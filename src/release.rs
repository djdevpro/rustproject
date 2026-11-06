//! Les versions de Rust Project, publiées comme releases GitHub : vérifier, télécharger, installer.
//!
//! Même principe que le lanceur d'ArtCraft :
//! - la release est la source de vérité (`releases/latest`, ou la liste pour le canal bêta),
//!   interrogée avec son ETag pour ne pas consommer le quota de l'API ;
//! - ses notes sont les nouveautés affichées dans la fenêtre de mise à jour ;
//! - chaque fichier a son empreinte SHA-256 (champ `digest` de GitHub, sinon `SHA256SUMS.txt`),
//!   vérifiée avant l'installation ;
//! - l'installation renomme l'exécutable en cours (`rust-project.exe.old`), met le nouveau à sa
//!   place, puis Rust Project redémarre ; l'ancien est effacé au lancement suivant.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Le dépôt des releases.
pub const REPO: &str = "djdevpro/rustproject";

/// La version de cet exécutable : celle du tag de release (posée par la CI au moment de la
/// compilation), sinon celle de Cargo.toml (compilation locale).
pub const VERSION: &str = match option_env!("ATELIER_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

/// La fin du nom de fichier publié pour cette plateforme.
pub const PLATFORM: &str = if cfg!(windows) { "windows-x64.exe" } else { "linux-x64" };

/// Le nom du fichier d'une version pour cette plateforme, tel que la CI le publie.
pub fn asset_name(version: &str) -> String {
    format!("rust-project-{version}-{PLATFORM}")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    /// Empreinte SHA-256 en hexadécimal, si GitHub la donne.
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub name: String,
    /// Les points de version (lignes « - … » des notes de la release).
    pub notes: Vec<String>,
    /// Date de publication, ex. « 4 nov. 2026 ».
    pub published: String,
    pub page: String,
    pub prerelease: bool,
    pub asset: Option<Asset>,
    /// `SHA256SUMS.txt`, si la release en a un (empreintes de secours).
    pub sums: Option<String>,
}

/// Où en est la mise à jour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Pas encore vérifié.
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Ready,
    Failed,
}

// ---- versions ----------------------------------------------------------------------------------

/// `1.2.3`, `v1.2.3` ou `1.2.3-beta.1` : (numéros, pré-version éventuelle).
fn parse_version(v: &str) -> Option<([u64; 3], Option<String>)> {
    let v = v.trim().trim_start_matches('v');
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p.to_owned())),
        None => (v, None),
    };
    let mut it = core.split('.').map(|x| x.parse::<u64>().ok());
    let n = [it.next()??, it.next().flatten().unwrap_or(0), it.next().flatten().unwrap_or(0)];
    Some((n, pre))
}

/// `remote` est plus récente que `local` (une pré-version passe avant la version finale).
pub fn newer(remote: &str, local: &str) -> bool {
    let (Some((a, pa)), Some((b, pb))) = (parse_version(remote), parse_version(local)) else { return false };
    match a.cmp(&b) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => match (pa, pb) {
            (None, Some(_)) => true,
            (Some(x), Some(y)) => x > y,
            _ => false,
        },
    }
}

// ---- lecture d'une release ---------------------------------------------------------------------

/// Les points de version : les lignes en liste des notes, sans liens ni mentions d'auteur.
pub fn notes_points(body: &str) -> Vec<String> {
    let clean = |line: &str| {
        let mut s = line.to_owned();
        // [texte](url) → texte
        while let (Some(a), Some(b)) = (s.find('['), s.find("](")) {
            let Some(c) = s[b..].find(')').map(|c| b + c) else { break };
            if a >= b {
                break;
            }
            s = format!("{}{}{}", &s[..a], &s[a + 1..b], &s[c + 1..]);
        }
        // Notes générées par GitHub : « … by @nom in https://… »
        if let Some(i) = s.find(" by @") {
            s.truncate(i);
        }
        s.trim().trim_end_matches('.').trim().to_owned()
    };
    let bullets: Vec<String> = body
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")))
        .map(clean)
        .filter(|l| !l.is_empty() && !l.contains("Full Changelog"))
        .collect();
    if !bullets.is_empty() {
        return bullets;
    }
    body.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#') && !l.contains("Full Changelog")).map(clean).filter(|l| !l.is_empty()).collect()
}

/// `2026-11-04T10:00:00Z` → `4 nov. 2026`.
fn short_date(iso: &str) -> String {
    const MONTHS: [&str; 12] = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
    let date = iso.split('T').next().unwrap_or("");
    let mut it = date.split('-');
    match (it.next(), it.next().and_then(|m| m.parse::<usize>().ok()), it.next().and_then(|d| d.parse::<u32>().ok())) {
        (Some(y), Some(m @ 1..=12), Some(d)) => format!("{d} {} {y}", MONTHS[m - 1]),
        _ => String::new(),
    }
}

/// Une release de l'API GitHub. `None` pour un brouillon ou une réponse sans tag.
pub fn parse_release(v: &Value) -> Option<Release> {
    if v["draft"].as_bool().unwrap_or(false) {
        return None;
    }
    let tag = v["tag_name"].as_str()?;
    let version = tag.trim_start_matches('v').to_owned();
    parse_version(&version)?;
    let wanted = asset_name(&version);
    let assets = v["assets"].as_array().cloned().unwrap_or_default();
    let asset = assets.iter().find(|a| a["name"].as_str() == Some(wanted.as_str())).or_else(|| assets.iter().find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(&format!("-{PLATFORM}"))))).map(|a| Asset {
        name: a["name"].as_str().unwrap_or_default().to_owned(),
        url: a["browser_download_url"].as_str().unwrap_or_default().to_owned(),
        size: a["size"].as_u64().unwrap_or(0),
        sha256: a["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(str::to_lowercase),
    });
    let sums = assets.iter().find(|a| a["name"].as_str() == Some("SHA256SUMS.txt")).and_then(|a| a["browser_download_url"].as_str()).map(str::to_owned);
    Some(Release {
        name: v["name"].as_str().filter(|n| !n.is_empty()).unwrap_or(tag).to_owned(),
        notes: notes_points(v["body"].as_str().unwrap_or("")),
        published: short_date(v["published_at"].as_str().unwrap_or("")),
        page: v["html_url"].as_str().unwrap_or_default().to_owned(),
        prerelease: v["prerelease"].as_bool().unwrap_or(false),
        version,
        asset,
        sums,
    })
}

/// L'empreinte d'un fichier dans un `SHA256SUMS.txt` (« empreinte  nom »).
fn sum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let (hash, file) = (it.next()?, it.next()?);
        (file.trim_start_matches('*') == name).then(|| hash.to_lowercase())
    })
}

// ---- réseau ------------------------------------------------------------------------------------

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).build().into()
}

const USER_AGENT: &str = concat!("rust-project/", env!("CARGO_PKG_VERSION"));

enum Fetch {
    NotModified,
    Latest(Option<Box<Release>>, Option<String>),
}

fn fetch(api: &str, beta: bool, etag: Option<&str>) -> Result<Fetch, String> {
    let url = if beta { format!("{api}/repos/{REPO}/releases?per_page=10") } else { format!("{api}/repos/{REPO}/releases/latest") };
    let mut req = agent(Duration::from_secs(15)).get(&url).header("User-Agent", USER_AGENT).header("Accept", "application/vnd.github+json");
    if let Some(e) = etag {
        req = req.header("If-None-Match", e);
    }
    let mut resp = req.call().map_err(|e| format!("GitHub injoignable : {e}"))?;
    let status = resp.status().as_u16();
    let etag = resp.headers().get("etag").and_then(|v| v.to_str().ok()).map(str::to_owned);
    match status {
        304 => return Ok(Fetch::NotModified),
        // Aucune release publiée pour l'instant.
        404 => return Ok(Fetch::Latest(None, None)),
        200 => {}
        403 | 429 => return Err("GitHub limite les vérifications pour l'instant, réessaie plus tard".into()),
        s => return Err(format!("GitHub a répondu {s}")),
    }
    let body: Value = serde_json::from_str(&resp.body_mut().with_config().limit(8 * 1024 * 1024).read_to_string().map_err(|e| e.to_string())?).map_err(|e| format!("Réponse illisible : {e}"))?;
    let release = if beta { body.as_array().and_then(|list| list.iter().find_map(parse_release)) } else { parse_release(&body) };
    Ok(Fetch::Latest(release.map(Box::new), etag))
}

/// Télécharge le fichier de la release, vérifie son empreinte, et le range dans `dir`.
fn download(release: &Release, dir: &Path, mut progress: impl FnMut(u64, u64)) -> Result<PathBuf, String> {
    let asset = release.asset.as_ref().ok_or("Cette version n'a pas de fichier pour cette plateforme")?;
    let expected = match (&asset.sha256, &release.sums) {
        (Some(h), _) => Some(h.clone()),
        (None, Some(url)) => {
            let mut resp = agent(Duration::from_secs(30)).get(url).header("User-Agent", USER_AGENT).call().map_err(|e| e.to_string())?;
            sum_for(&resp.body_mut().read_to_string().map_err(|e| e.to_string())?, &asset.name)
        }
        _ => None,
    };
    let expected = expected.ok_or("Pas d'empreinte SHA-256 publiée pour ce fichier : installation refusée")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let dest = dir.join(&asset.name);
    let part = dir.join(format!("{}.part", asset.name));
    let mut resp = agent(Duration::from_secs(600)).get(&asset.url).header("User-Agent", USER_AGENT).call().map_err(|e| format!("Téléchargement impossible : {e}"))?;
    if resp.status().as_u16() != 200 {
        return Err(format!("Téléchargement impossible : GitHub a répondu {}", resp.status().as_u16()));
    }
    let total = asset.size.max(1);
    let mut reader = resp.body_mut().with_config().limit(1024 * 1024 * 1024).reader();
    let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("Téléchargement interrompu : {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hasher.update(&buf[..n]);
        done += n as u64;
        progress(done, total);
    }
    drop(file);
    let got = format!("{:x}", hasher.finalize());
    if got != expected {
        let _ = std::fs::remove_file(&part);
        return Err("L'empreinte du fichier ne correspond pas : installation refusée".into());
    }
    std::fs::rename(&part, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

// ---- installation ------------------------------------------------------------------------------

/// Un exécutable compilé sur place (`target\debug`, `target\release`) : on ne le remplace pas.
pub fn is_dev_build() -> bool {
    std::env::current_exe().map(|p| crate::model::path_key(&p.to_string_lossy())).is_ok_and(|p| p.contains("\\target\\debug\\") || p.contains("\\target\\release\\"))
}

fn old_exe(exe: &Path) -> PathBuf {
    PathBuf::from(format!("{}.old", exe.display()))
}

/// Met le nouvel exécutable à la place de celui qui tourne (il reste utilisable jusqu'à la fin).
pub fn install(new_exe: &Path) -> Result<(), String> {
    if is_dev_build() {
        return Err("Version de développement : installe Rust Project depuis une release pour les mises à jour".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let old = old_exe(&exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| format!("Impossible de remplacer {} : {e}", exe.display()))?;
    if let Err(e) = std::fs::copy(new_exe, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("Installation impossible : {e}"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

/// Après un redémarrage : efface l'ancien exécutable (dès que l'ancien processus l'a lâché).
pub fn cleanup_after_update() {
    let Ok(exe) = std::env::current_exe() else { return };
    let old = old_exe(&exe);
    std::thread::spawn(move || {
        for _ in 0..20 {
            if !old.exists() || std::fs::remove_file(&old).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

// ---- l'état vu par l'interface -----------------------------------------------------------------

enum Msg {
    Checked(Result<Fetch, String>),
    Progress(u64, u64),
    Downloaded(Result<PathBuf, String>),
}

/// La mise à jour : phase, release trouvée, progression. Le travail réseau tourne dans un fil.
pub struct Updater {
    pub phase: Phase,
    pub release: Option<Release>,
    /// (reçu, total) en octets.
    pub progress: (u64, u64),
    pub error: Option<String>,
    pub downloaded: Option<PathBuf>,
    /// L'adresse de l'API (un faux serveur dans les tests).
    pub api: String,
    etag: Option<String>,
    cached: Option<Release>,
    rx: Option<Receiver<Msg>>,
}

impl Default for Updater {
    fn default() -> Self {
        Self { phase: Phase::Idle, release: None, progress: (0, 0), error: None, downloaded: None, api: "https://api.github.com".into(), etag: None, cached: None, rx: None }
    }
}

impl Updater {
    pub fn busy(&self) -> bool {
        matches!(self.phase, Phase::Checking | Phase::Downloading)
    }

    pub fn check(&mut self, ctx: &egui::Context, beta: bool) {
        if self.busy() {
            return;
        }
        self.phase = Phase::Checking;
        self.error = None;
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let (api, etag, ctx) = (self.api.clone(), self.etag.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Checked(fetch(&api, beta, etag.as_deref())));
            ctx.request_repaint();
        });
    }

    pub fn download(&mut self, ctx: &egui::Context, dir: PathBuf) {
        let Some(release) = self.release.clone().filter(|_| self.phase == Phase::Available || self.phase == Phase::Failed) else { return };
        self.phase = Phase::Downloading;
        self.error = None;
        self.progress = (0, release.asset.as_ref().map(|a| a.size).unwrap_or(0));
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut last = 0u64;
            let result = download(&release, &dir, |done, total| {
                // Pas plus d'un message par 256 Ko : l'interface suit sans être inondée.
                if done - last >= 256 * 1024 || done == total {
                    last = done;
                    let _ = tx.send(Msg::Progress(done, total));
                    ctx.request_repaint();
                }
            });
            let _ = tx.send(Msg::Downloaded(result));
            ctx.request_repaint();
        });
    }

    /// Une release trouvée : disponible si plus récente que cette version.
    pub fn apply(&mut self, release: Option<Release>) {
        self.phase = match &release {
            Some(r) if newer(&r.version, VERSION) => Phase::Available,
            _ => Phase::UpToDate,
        };
        self.release = release;
    }

    /// Relève les messages du fil de travail. Renvoie `true` si quelque chose a changé.
    pub fn poll(&mut self) -> bool {
        let Some(rx) = &self.rx else { return false };
        let msgs: Vec<Msg> = rx.try_iter().collect();
        let changed = !msgs.is_empty();
        for msg in msgs {
            match msg {
                Msg::Checked(Ok(Fetch::Latest(release, etag))) => {
                    self.etag = etag;
                    self.cached = release.as_deref().cloned();
                    self.apply(release.map(|r| *r));
                }
                Msg::Checked(Ok(Fetch::NotModified)) => {
                    let cached = self.cached.clone();
                    self.apply(cached);
                }
                Msg::Checked(Err(e)) | Msg::Downloaded(Err(e)) => {
                    self.phase = Phase::Failed;
                    self.error = Some(e);
                }
                Msg::Progress(done, total) => self.progress = (done, total),
                Msg::Downloaded(Ok(path)) => {
                    self.downloaded = Some(path);
                    self.phase = Phase::Ready;
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn release_json(tag: &str) -> Value {
        json!({
            "tag_name": tag, "name": format!("Rust Project {}", tag.trim_start_matches('v')), "draft": false, "prerelease": false,
            "published_at": "2026-11-04T10:00:00Z", "html_url": format!("https://github.com/{REPO}/releases/tag/{tag}"),
            "body": "- Carte en graphe animé\n- Glisser un projet vers une attention\n\n**Full Changelog**: https://github.com/x/y/compare/v0.1.0...v0.2.0",
            "assets": [
                { "name": asset_name(tag.trim_start_matches('v')), "size": 19_300_000, "browser_download_url": "https://example.invalid/rust-project.exe", "digest": "sha256:ABCDEF" },
                { "name": "SHA256SUMS.txt", "size": 120, "browser_download_url": "https://example.invalid/SHA256SUMS.txt" }
            ]
        })
    }

    #[test]
    fn comparer_les_versions() {
        assert!(newer("0.2.0", "0.1.9") && newer("v1.0.0", "0.9.9") && newer("0.1.10", "0.1.9"));
        assert!(!newer("0.1.0", "0.1.0") && !newer("0.1.0", "0.2.0"));
        assert!(newer("0.2.0", "0.2.0-beta.1"), "la version finale passe après la bêta");
        assert!(!newer("0.2.0-beta.1", "0.2.0"));
        assert!(!newer("pas-une-version", "0.1.0"));
    }

    #[test]
    fn lire_une_release_github() {
        let r = parse_release(&release_json("v0.2.0")).unwrap();
        assert_eq!(r.version, "0.2.0");
        assert_eq!(r.notes, ["Carte en graphe animé", "Glisser un projet vers une attention"]);
        assert_eq!(r.published, "4 nov. 2026");
        let a = r.asset.unwrap();
        assert_eq!((a.name.clone(), a.size, a.sha256.as_deref()), (asset_name("0.2.0"), 19_300_000, Some("abcdef")));
        assert!(r.sums.is_some());
        assert!(parse_release(&json!({ "tag_name": "v0.3.0", "draft": true })).is_none(), "un brouillon n'est pas une version");
    }

    #[test]
    fn notes_generees_par_github() {
        let body = "## What's Changed\n* Real updater by @djdevpro in https://github.com/x/y/pull/3\n* [Docs](https://x.y) refreshed\n\n**Full Changelog**: https://x";
        assert_eq!(notes_points(body), ["Real updater", "Docs refreshed"]);
        assert_eq!(notes_points("Une seule phrase sans liste."), ["Une seule phrase sans liste"]);
    }

    #[test]
    fn empreintes_de_secours() {
        let sums = "abc123  atelier-0.2.0-windows-x64.exe\nfff  autre.zip\n";
        assert_eq!(sum_for(sums, "atelier-0.2.0-windows-x64.exe").as_deref(), Some("abc123"));
        assert_eq!(sum_for(sums, "absent.exe"), None);
    }

    #[test]
    fn disponible_seulement_si_plus_recente() {
        let mut u = Updater::default();
        let mut r = parse_release(&release_json("v0.2.0")).unwrap();
        r.version = "999.0.0".into();
        u.apply(Some(r.clone()));
        assert_eq!(u.phase, Phase::Available);
        r.version = VERSION.into();
        u.apply(Some(r));
        assert_eq!(u.phase, Phase::UpToDate);
        u.apply(None);
        assert_eq!(u.phase, Phase::UpToDate, "aucune release publiée");
    }

    /// Un faux serveur GitHub : la release, puis le fichier ; l'empreinte est vérifiée.
    #[test]
    fn verifier_puis_telecharger_depuis_un_faux_github() {
        use std::net::TcpListener;
        let payload = b"MZ faux executable".to_vec();
        let digest = format!("{:x}", Sha256::digest(&payload));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let release = json!({
            "tag_name": "v999.0.0", "name": "Rust Project 999.0.0", "draft": false, "prerelease": false, "published_at": "2026-11-04T10:00:00Z",
            "html_url": "https://example.invalid", "body": "- Une nouveauté",
            "assets": [{ "name": asset_name("999.0.0"), "size": payload.len(), "browser_download_url": format!("{base}/fichier"), "digest": format!("sha256:{digest}") }]
        })
        .to_string();
        let served = payload.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2).flatten() {
                let mut stream = stream;
                let mut req = [0u8; 4096];
                let n = stream.read(&mut req).unwrap_or(0);
                let first = String::from_utf8_lossy(&req[..n]).lines().next().unwrap_or_default().to_owned();
                let (ctype, body) = if first.contains("/fichier") { ("application/octet-stream", served.clone()) } else { ("application/json", release.clone().into_bytes()) };
                let head = format!("HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nETag: \"e1\"\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        let ctx = egui::Context::default();
        let mut u = Updater { api: base.clone(), ..Default::default() };
        u.check(&ctx, false);
        let wait = |u: &mut Updater, until: Phase| {
            for _ in 0..200 {
                u.poll();
                if u.phase == until || u.phase == Phase::Failed {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        };
        wait(&mut u, Phase::Available);
        assert_eq!(u.phase, Phase::Available, "{:?}", u.error);
        assert_eq!(u.release.as_ref().unwrap().notes, ["Une nouveauté"]);
        let dir = std::env::temp_dir().join(format!("atelier-maj-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        u.download(&ctx, dir.clone());
        wait(&mut u, Phase::Ready);
        assert_eq!(u.phase, Phase::Ready, "{:?}", u.error);
        assert_eq!(std::fs::read(u.downloaded.as_ref().unwrap()).unwrap(), payload, "fichier complet et vérifié");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
