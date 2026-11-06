//! Le scanner : pour chaque dossier source, liste tous les dossiers jusqu'à 3 niveaux de
//! profondeur (seulement des dossiers), et relève pour chacun ce qui aide à décider si c'est un
//! projet : marqueurs (`.git`, `Cargo.toml`…), sous-dossiers, activité, début du README, manifestes.
//!
//! Il ne décide rien de définitif : il signale les dossiers « probablement projets » (marqueur
//! fort) ; l'IA (Ollama ou un agent MCP) ou les règles locales tranchent ensuite.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;

use crate::model::Config;

/// Au-delà, on arrête (sécurité sur un disque énorme).
const MAX_DIRS: usize = 8000;
const README_BYTES: usize = 1600;

#[derive(Clone, Debug)]
pub struct ScanOptions {
    /// (index du dossier source, chemin, profondeur 1 à 3)
    pub roots: Vec<(usize, String, u32)>,
    pub excludes: Vec<String>,
    /// Marqueurs actifs : noms de fichiers ou dossiers, ou `*.ext`.
    pub markers: Vec<String>,
    /// Ne pas descendre dans un dossier déjà reconnu comme projet.
    pub skip_nested: bool,
    /// Ignorer les dossiers inactifs depuis plus de N jours.
    pub max_idle_days: Option<u32>,
    /// Rattacher les briques (frontend, api, packages/x…) à leur projet parent.
    pub group_bricks: bool,
    /// Chemins écartés (clés `path_key`) : ni relevés ni parcourus.
    pub ignored: Vec<String>,
}

impl ScanOptions {
    pub fn from_config(c: &Config) -> Self {
        Self {
            roots: c.roots.iter().enumerate().filter(|(_, r)| r.enabled && !r.path.trim().is_empty()).map(|(i, r)| (i, r.path.trim().to_owned(), r.depth.clamp(1, 3))).collect(),
            excludes: c.excludes.iter().map(|e| e.to_lowercase()).collect(),
            markers: c.markers.iter().filter(|(_, on)| *on).map(|(m, _)| m.clone()).collect(),
            skip_nested: c.skip_nested,
            max_idle_days: c.ignore_older.then_some(c.ignore_older_days),
            group_bricks: c.group_monorepos,
            ignored: c.ignored.iter().map(|p| crate::model::path_key(p)).collect(),
        }
    }
}

/// Ce que disent les manifestes (package.json, Cargo.toml…).
#[derive(Clone, Debug, Default, Serialize)]
pub struct Manifest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub stack: Vec<String>,
}

/// Un dossier relevé par le scan.
#[derive(Clone, Debug, Serialize)]
pub struct Found {
    pub path: String,
    pub name: String,
    /// Index du dossier source.
    pub root: usize,
    /// 1 = enfant direct du dossier source.
    pub depth: u32,
    pub markers: Vec<String>,
    /// Noms des sous-dossiers (au plus 15), pour aider l'IA à juger.
    pub children: Vec<String>,
    pub files: usize,
    pub idle_days: u32,
    pub readme: Option<String>,
    pub manifest: Manifest,
    /// Un marqueur coché est présent : projet probable (l'IA ou les règles confirment).
    pub likely_project: bool,
    /// Un marqueur fort (autre que le README) : on ne descend pas dans ce dossier.
    pub strong: bool,
    /// Le dossier parent, s'il fait partie du scan (niveau 2 ou 3).
    pub parent: Option<String>,
    /// Si ce dossier n'est qu'une brique d'un projet (frontend, api, packages/x…) : le chemin du projet.
    pub brick_of: Option<String>,
    /// Pour un projet fait de briques : leurs chemins.
    pub bricks: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ScanResult {
    pub dirs: Vec<Found>,
    /// Nombre de dossiers relevés par dossier source (même index que la configuration).
    pub per_root: Vec<(usize, usize)>,
    pub errors: Vec<String>,
    pub truncated: bool,
}

impl ScanResult {
    pub fn likely(&self) -> impl Iterator<Item = &Found> {
        self.dirs.iter().filter(|d| d.likely_project)
    }
}

fn days_since(t: SystemTime) -> u32 {
    SystemTime::now().duration_since(t).map(|d| (d.as_secs() / 86_400) as u32).unwrap_or(0)
}

fn matches_marker(marker: &str, name: &str) -> bool {
    let (m, n) = (marker.to_lowercase(), name.to_lowercase());
    match m.strip_prefix("*.") {
        Some(ext) => n.ends_with(&format!(".{ext}")),
        None => m == n,
    }
}

/// Parcourt tous les dossiers sources. `progress` reçoit le nombre de dossiers déjà relevés.
pub fn scan(opts: &ScanOptions, mut progress: impl FnMut(usize)) -> ScanResult {
    let mut result = ScanResult::default();
    for (root_idx, root, depth) in &opts.roots {
        let before = result.dirs.len();
        let root_path = PathBuf::from(root);
        if !root_path.is_dir() {
            result.errors.push(format!("Dossier source introuvable : {root}"));
            result.per_root.push((*root_idx, 0));
            continue;
        }
        // Parcours en largeur : les dossiers de niveau 1 d'abord, puis 2, puis 3.
        let mut queue: std::collections::VecDeque<(PathBuf, u32, Option<String>)> = subdirs(&root_path, opts).into_iter().map(|p| (p, 1, None)).collect();
        while let Some((dir, level, parent)) = queue.pop_front() {
            if result.dirs.len() >= MAX_DIRS {
                result.truncated = true;
                break;
            }
            let mut found = inspect(&dir, *root_idx, level, opts);
            found.parent = parent;
            if let Some(max) = opts.max_idle_days
                && found.idle_days > max
            {
                continue;
            }
            // Un README seul n'arrête pas la descente : un dossier de rangement en a souvent un.
            let descend = level < *depth && !(opts.skip_nested && found.strong);
            if descend {
                let me = found.path.clone();
                queue.extend(subdirs(&dir, opts).into_iter().map(|p| (p, level + 1, Some(me.clone()))));
            }
            result.dirs.push(found);
            if result.dirs.len().is_multiple_of(25) {
                progress(result.dirs.len());
            }
        }
        result.per_root.push((*root_idx, result.dirs.len() - before));
    }
    if opts.group_bricks {
        detect_bricks(&mut result.dirs);
    }
    progress(result.dirs.len());
    result
}

/// Les noms typiques d'une partie de projet (et non d'un projet à part entière).
const BRICK_NAMES: [&str; 44] = [
    "frontend", "front", "front-end", "backend", "back", "back-end", "api", "server", "serveur", "client", "web", "webapp", "app", "mobile", "desktop",
    "admin", "dashboard", "worker", "workers", "core", "shared", "common", "lib", "ui", "docs", "doc", "infra", "deploy", "scripts", "cli", "sdk",
    "engine", "plugin", "extension", "gateway", "db", "database", "migrations", "e2e", "tests", "examples", "landing", "site", "www",
];
/// Les dossiers qui contiennent les briques d'un monorepo.
const BRICK_CONTAINERS: [&str; 8] = ["packages", "apps", "services", "libs", "modules", "crates", "workspaces", "projects"];

/// Le nom d'un dossier évoque une partie de projet (frontend, api, site, `packages/x`, `monprojet-api`…).
pub fn brick_like(name: &str, parent_name: &str) -> bool {
    let (n, p) = (name.to_lowercase(), parent_name.to_lowercase());
    BRICK_NAMES.contains(&n.as_str())
        || BRICK_CONTAINERS.contains(&n.as_str())
        || BRICK_CONTAINERS.contains(&p.as_str())
        || (p.len() >= 3 && (n.starts_with(&format!("{p}-")) || n.starts_with(&format!("{p}_"))))
}

/// Le nom d'un dossier évoque une copie, une sauvegarde ou un déploiement figé
/// (« BunFlow - Copie », « TopFoot-save », « site-sauv », « site-deploy-64a6597 »…).
pub fn copy_like(name: &str) -> bool {
    let n = name.to_lowercase();
    const WORDS: [&str; 9] = ["copie", "copy", "backup", "-save", "_save", "-sauv", "_sauv", "-old", "_old"];
    let commit_suffix = n.rsplit_once('-').is_some_and(|(_, h)| (7..=12).contains(&h.len()) && h.chars().all(|c| c.is_ascii_hexdigit()) && h.chars().any(|c| c.is_ascii_digit()));
    WORDS.iter().any(|w| n.contains(w)) || n.ends_with(" (2)") || commit_suffix
}

/// Repère les briques : un dossier qui n'est qu'une partie de son parent (frontend, api,
/// `packages/x`…). Le parent devient le projet (avec les technos de ses briques) ; la brique ne
/// doit pas devenir un projet à part.
///
/// Une brique est confirmée si son parent est déjà un projet probable, s'il contient au moins deux
/// briques, ou si elle est dans un dossier de monorepo (`packages`, `apps`…). Un dossier de rangement
/// qui contient des projets aux noms quelconques n'est donc pas regroupé.
fn detect_bricks(dirs: &mut [Found]) {
    use std::collections::HashMap;
    let index: HashMap<String, usize> = dirs.iter().enumerate().map(|(i, d)| (crate::model::path_key(&d.path), i)).collect();
    let parent_of = |d: &Found| d.parent.as_deref().and_then(|p| index.get(&crate::model::path_key(p)).copied());
    // Candidats : (brique, projet).
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    for (i, d) in dirs.iter().enumerate() {
        let Some(p) = parent_of(d) else { continue };
        let name = d.name.to_lowercase();
        let parent_name = dirs[p].name.to_lowercase();
        if BRICK_CONTAINERS.contains(&parent_name.as_str()) {
            // packages/x : brique du grand-parent (le monorepo), comme « packages » lui-même.
            if let Some(g) = parent_of(&dirs[p]) {
                candidates.push((i, g));
                candidates.push((p, g));
            }
        } else if brick_like(&name, &parent_name) {
            candidates.push((i, p));
        }
    }
    candidates.sort_unstable();
    candidates.dedup();
    // Confirmer, projet par projet.
    let mut by_project: HashMap<usize, Vec<usize>> = HashMap::new();
    for (b, p) in candidates {
        by_project.entry(p).or_default().push(b);
    }
    for (p, bricks) in by_project {
        let monorepo = bricks.iter().any(|&b| BRICK_CONTAINERS.contains(&dirs[b].name.to_lowercase().as_str()));
        if !(dirs[p].likely_project || bricks.len() >= 2 || monorepo) {
            continue;
        }
        let project_path = dirs[p].path.clone();
        let mut stack = dirs[p].manifest.stack.clone();
        let mut description = dirs[p].manifest.description.clone();
        for &b in &bricks {
            dirs[b].brick_of = Some(project_path.clone());
            for s in dirs[b].manifest.stack.clone() {
                push(&mut stack, &s);
            }
            if description.is_none() {
                description = dirs[b].manifest.description.clone();
            }
        }
        let paths: Vec<String> = bricks.iter().map(|&b| dirs[b].path.clone()).collect();
        let project = &mut dirs[p];
        project.likely_project = true;
        project.manifest.stack = stack;
        project.manifest.description = description;
        project.bricks = paths;
    }
}

/// Les sous-dossiers à explorer (ni cachés, ni exclus, ni liens symboliques), triés par nom.
fn subdirs(dir: &Path, opts: &ScanOptions) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir() && !t.is_symlink()).unwrap_or(false))
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            !name.starts_with('.') && !name.starts_with('$') && !opts.excludes.contains(&name)
        })
        .filter(|e| opts.ignored.is_empty() || !opts.ignored.contains(&crate::model::path_key(&e.path().to_string_lossy())))
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

/// Relève ce qu'un dossier contient d'utile (sans descendre plus bas).
fn inspect(dir: &Path, root: usize, depth: u32, opts: &ScanOptions) -> Found {
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut markers = Vec::new();
    let mut children = Vec::new();
    let mut files = 0;
    let mut newest = std::fs::metadata(dir).and_then(|m| m.modified()).ok();
    let mut readme = None;
    let mut entry_names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let entry_name = e.file_name().to_string_lossy().into_owned();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            for m in &opts.markers {
                if matches_marker(m, &entry_name) && !markers.contains(m) {
                    markers.push(m.clone());
                }
            }
            if is_dir {
                let lower = entry_name.to_lowercase();
                if !lower.starts_with('.') && !opts.excludes.contains(&lower) && children.len() < 15 {
                    children.push(entry_name.clone());
                }
            } else {
                files += 1;
                if let Ok(t) = e.metadata().and_then(|m| m.modified())
                    && newest.is_none_or(|n| t > n)
                {
                    newest = Some(t);
                }
                if readme.is_none() && entry_name.to_lowercase().starts_with("readme") {
                    readme = read_head(&e.path());
                }
            }
            entry_names.push(entry_name);
        }
    }
    children.sort();
    let manifest = read_manifests(dir, &entry_names);
    // Tout marqueur coché fait un projet probable ; « fort » : autre chose que le README.
    let likely_project = !markers.is_empty();
    let strong = markers.iter().any(|m| !m.to_lowercase().starts_with("readme"));
    Found {
        path: dir.to_string_lossy().into_owned(),
        name,
        root,
        depth,
        markers,
        children,
        files,
        idle_days: newest.map(days_since).unwrap_or(0),
        readme,
        manifest,
        likely_project,
        strong,
        parent: None,
        brick_of: None,
        bricks: Vec::new(),
    }
}

fn read_head(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0; README_BYTES];
    let n = f.read(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf[..n]).into_owned();
    let text = text.trim().to_owned();
    (!text.is_empty()).then_some(text)
}

fn read_text(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > 512 * 1024 {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn push(stack: &mut Vec<String>, s: &str) {
    if !stack.iter().any(|x| x == s) {
        stack.push(s.to_owned());
    }
}

/// Nom, description et technos d'après les manifestes présents.
fn read_manifests(dir: &Path, names: &[String]) -> Manifest {
    let has = |n: &str| names.iter().any(|x| x.eq_ignore_ascii_case(n));
    let mut m = Manifest::default();
    if has("package.json")
        && let Some(v) = read_text(&dir.join("package.json")).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
    {
        m.name = v["name"].as_str().map(str::to_owned);
        m.description = v["description"].as_str().filter(|s| !s.is_empty()).map(str::to_owned);
        let mut deps: Vec<String> = Vec::new();
        for key in ["dependencies", "devDependencies"] {
            if let Some(obj) = v[key].as_object() {
                deps.extend(obj.keys().cloned());
            }
        }
        let dep = |n: &str| deps.iter().any(|d| d == n || d.starts_with(&format!("{n}/")));
        for (d, label) in [
            ("next", "Next.js"),
            ("react", "React"),
            ("vue", "Vue"),
            ("svelte", "Svelte"),
            ("astro", "Astro"),
            ("three", "Three.js"),
            ("electron", "Electron"),
            ("@tauri-apps", "Tauri"),
            ("express", "Express"),
            ("convex", "Convex"),
            ("@anthropic-ai", "Claude"),
            ("openai", "OpenAI"),
        ] {
            if dep(d) {
                push(&mut m.stack, label);
            }
        }
        push(&mut m.stack, if dep("typescript") || has("tsconfig.json") { "TypeScript" } else { "JavaScript" });
        if has("bun.lockb") || has("bun.lock") {
            push(&mut m.stack, "Bun");
        }
    }
    if has("Cargo.toml")
        && let Some(t) = read_text(&dir.join("Cargo.toml"))
    {
        push(&mut m.stack, "Rust");
        let mut in_package = false;
        for line in t.lines() {
            let l = line.trim();
            if l.starts_with('[') {
                in_package = l == "[package]";
            } else if in_package && let Some((k, v)) = l.split_once('=') {
                let v = v.trim().trim_matches('"').to_owned();
                match k.trim() {
                    "name" if m.name.is_none() => m.name = Some(v),
                    "description" if m.description.is_none() => m.description = Some(v),
                    _ => {}
                }
            }
        }
        for (d, label) in [("egui", "egui"), ("bevy", "Bevy"), ("tauri", "Tauri"), ("axum", "Axum"), ("wgpu", "wgpu")] {
            if t.contains(&format!("\n{d} ")) || t.contains(&format!("\n{d}=")) || t.contains(&format!("\n{d} =")) {
                push(&mut m.stack, label);
            }
        }
    }
    if has("pyproject.toml") || has("requirements.txt") || has("setup.py") {
        push(&mut m.stack, "Python");
        let reqs = read_text(&dir.join("requirements.txt")).unwrap_or_default() + &read_text(&dir.join("pyproject.toml")).unwrap_or_default();
        let reqs = reqs.to_lowercase();
        for (d, label) in [("torch", "PyTorch"), ("fastapi", "FastAPI"), ("flask", "Flask"), ("django", "Django"), ("whisper", "Whisper"), ("streamlit", "Streamlit")] {
            if reqs.contains(d) {
                push(&mut m.stack, label);
            }
        }
    }
    if has("composer.json") {
        push(&mut m.stack, "PHP");
        if let Some(v) = read_text(&dir.join("composer.json")).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
            if m.description.is_none() {
                m.description = v["description"].as_str().filter(|s| !s.is_empty()).map(str::to_owned);
            }
            if v["require"].get("laravel/framework").is_some() {
                push(&mut m.stack, "Laravel");
            }
        }
    }
    if has("go.mod") {
        push(&mut m.stack, "Go");
    }
    if names.iter().any(|n| n.to_lowercase().ends_with(".sln") || n.to_lowercase().ends_with(".csproj")) {
        push(&mut m.stack, ".NET");
    }
    if has("pubspec.yaml") {
        push(&mut m.stack, "Flutter");
    }
    if has("docker-compose.yml") || has("compose.yaml") || has("Dockerfile") {
        push(&mut m.stack, "Docker");
    }
    m
}

// ---- dépôt Git distant -------------------------------------------------------------------------

/// L'adresse web du dépôt distant d'un dossier (`origin` de préférence), lue dans `.git/config`.
/// Le dossier peut être un sous-dossier du dépôt (deux niveaux au plus).
pub fn git_remote(dir: &Path) -> Option<String> {
    let repo = dir.ancestors().take(3).find(|d| d.join(".git").exists())?;
    let config = std::fs::read_to_string(git_dir(&repo.join(".git"))?.join("config")).ok()?;
    let mut remotes: Vec<(String, String)> = Vec::new();
    let mut section = String::new();
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line.to_owned();
        } else if let Some(name) = section.strip_prefix("[remote \"").and_then(|s| s.strip_suffix("\"]"))
            && let Some(url) = line.strip_prefix("url").map(str::trim_start).and_then(|s| s.strip_prefix('='))
        {
            remotes.push((name.to_owned(), url.trim().to_owned()));
        }
    }
    let (_, url) = remotes.iter().find(|(n, _)| n == "origin").or(remotes.first())?;
    web_url(url)
}

/// Le premier dépôt distant trouvé parmi les dossiers d'un projet. À défaut, celui des
/// sous-dossiers directs du premier dossier, s'ils n'en ont qu'un (un projet dont le code est
/// rangé un cran plus bas) ; plusieurs dépôts différents : aucun n'est choisi.
pub fn remote_of(folders: &[String]) -> Option<String> {
    let folders: Vec<&str> = folders.iter().map(|f| f.trim()).filter(|f| !f.is_empty()).collect();
    if let Some(r) = folders.iter().find_map(|f| git_remote(Path::new(f))) {
        return Some(r);
    }
    let first = folders.first()?;
    let mut below: Vec<String> = std::fs::read_dir(first).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).filter_map(|p| git_remote(&p)).collect();
    below.sort();
    below.dedup();
    (below.len() == 1).then(|| below.remove(0))
}

/// Le vrai dossier Git : `.git` lui-même, ou celui qu'indique un fichier `.git` (worktree,
/// sous-module), en remontant au dépôt commun pour un worktree.
fn git_dir(dot_git: &Path) -> Option<PathBuf> {
    if dot_git.is_dir() {
        return Some(dot_git.to_path_buf());
    }
    let text = std::fs::read_to_string(dot_git).ok()?;
    let target = text.trim().strip_prefix("gitdir:")?.trim();
    let mut dir = PathBuf::from(target);
    if dir.is_relative() {
        dir = dot_git.parent()?.join(dir);
    }
    if let Ok(common) = std::fs::read_to_string(dir.join("commondir")) {
        dir = dir.join(common.trim());
    }
    Some(dir)
}

/// L'adresse d'une page web à partir d'une URL de dépôt : `git@github.com:moi/projet.git`,
/// `ssh://git@github.com/moi/projet.git` ou `https://jeton@github.com/moi/projet.git` donnent
/// `https://github.com/moi/projet`. Un dépôt local (chemin, `file://`) n'en a pas.
pub fn web_url(remote: &str) -> Option<String> {
    let r = remote.trim();
    let (host, path) = if let Some(rest) = r.strip_prefix("https://").or_else(|| r.strip_prefix("http://")) {
        rest.split_once('/')?
    } else if let Some(rest) = r.strip_prefix("ssh://").or_else(|| r.strip_prefix("git://")) {
        rest.split_once('/')?
    } else if let Some((host, path)) = r.split_once(':')
        && host.contains('.')
        && !host.contains('/')
    {
        // Forme courte de SSH : `git@github.com:moi/projet.git`.
        (host, path)
    } else {
        return None;
    };
    // Sans identifiants ni port : `jeton@github.com:22` → `github.com`.
    let host = host.rsplit('@').next()?.split(':').next()?;
    let path = path.trim_matches('/').trim_end_matches(".git");
    (host.contains('.') && !path.is_empty()).then(|| format!("https://{host}/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresse_web_du_depot() {
        let want = Some("https://github.com/djdevpro/atelier".to_owned());
        assert_eq!(web_url("git@github.com:djdevpro/atelier.git"), want);
        assert_eq!(web_url("https://github.com/djdevpro/atelier.git"), want);
        assert_eq!(web_url("https://github.com/djdevpro/atelier"), want);
        assert_eq!(web_url("https://ghp_jeton@github.com/djdevpro/atelier.git"), want);
        assert_eq!(web_url("ssh://git@github.com:22/djdevpro/atelier.git"), want);
        assert_eq!(web_url("https://gitlab.com/groupe/sous/projet.git").as_deref(), Some("https://gitlab.com/groupe/sous/projet"));
        assert_eq!(web_url(r"C:\depots\atelier"), None);
        assert_eq!(web_url("/srv/git/atelier.git"), None);
        assert_eq!(web_url("file:///srv/git/atelier.git"), None);
    }

    #[test]
    fn depot_lu_dans_git_config() {
        let root = std::env::temp_dir().join(format!("atelier-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("frontend")).unwrap();
        std::fs::write(
            root.join(".git").join("config"),
            "[core]\n\tbare = false\n[remote \"upstream\"]\n\turl = https://github.com/autre/projet.git\n[remote \"origin\"]\n\turl = git@github.com:moi/projet.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n",
        )
        .unwrap();
        // `origin` passe avant les autres, et un sous-dossier du dépôt le trouve aussi.
        assert_eq!(git_remote(&root).as_deref(), Some("https://github.com/moi/projet"));
        assert_eq!(remote_of(&[String::new(), root.join("frontend").display().to_string()]).as_deref(), Some("https://github.com/moi/projet"));
        assert_eq!(git_remote(&std::env::temp_dir().join("atelier-pas-de-depot-ici")), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Un projet sans dépôt à sa racine prend celui de son unique sous-dossier versionné ;
    /// plusieurs dépôts différents dessous : aucun n'est choisi.
    #[test]
    fn depot_d_un_sous_dossier() {
        let root = std::env::temp_dir().join(format!("atelier-git-dessous-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = |name: &str, url: &str| {
            std::fs::create_dir_all(root.join(name).join(".git")).unwrap();
            std::fs::write(root.join(name).join(".git").join("config"), format!("[remote \"origin\"]\n\turl = {url}\n")).unwrap();
        };
        repo("code", "git@github.com:moi/site.git");
        std::fs::create_dir_all(root.join("maquettes")).unwrap();
        let folders = [root.display().to_string()];
        assert_eq!(remote_of(&folders).as_deref(), Some("https://github.com/moi/site"));
        repo("autre", "git@github.com:moi/autre.git");
        assert_eq!(remote_of(&folders), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Un petit arbre de dossiers temporaire :
    /// root/
    ///   app-rust/ (Cargo.toml, README.md) + app-rust/src/
    ///   site/ (package.json avec next)
    ///   clients/            (groupe, sans marqueur)
    ///     client-a/ (.git)
    ///       sous-projet/ (package.json)   ← ignoré : « pas de projet dans un projet »
    ///     archives/
    ///       vieux/ (pyproject.toml)       ← niveau 3
    ///   node_modules/ (exclu)
    ///   .cache/ (caché)
    fn tree() -> PathBuf {
        let root = std::env::temp_dir().join(format!("atelier-scan-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&root);
        let mk = |p: &str| std::fs::create_dir_all(root.join(p)).unwrap();
        let file = |p: &str, c: &str| std::fs::write(root.join(p), c).unwrap();
        mk("app-rust/src");
        file("app-rust/Cargo.toml", "[package]\nname = \"app-rust\"\ndescription = \"Une appli de test\"\n\n[dependencies]\negui = \"0.37\"\n");
        file("app-rust/README.md", "# App Rust\n\nUne petite appli de démonstration.");
        mk("site");
        file("site/package.json", r#"{"name":"site","description":"Site vitrine","dependencies":{"next":"15","react":"19"},"devDependencies":{"typescript":"5"}}"#);
        mk("clients/client-a/.git");
        mk("clients/client-a/sous-projet");
        file("clients/client-a/sous-projet/package.json", "{}");
        mk("clients/archives/vieux");
        file("clients/archives/vieux/pyproject.toml", "[project]\nname='vieux'\ndependencies=['torch']\n");
        mk("node_modules/paquet");
        mk(".cache/x");
        root
    }

    fn options(root: &Path, depth: u32) -> ScanOptions {
        let c = Config::default();
        ScanOptions { roots: vec![(0, root.to_string_lossy().into_owned(), depth)], ..ScanOptions::from_config(&c) }
    }

    #[test]
    fn liste_trois_niveaux_et_reconnait_les_projets() {
        let root = tree();
        let r = scan(&options(&root, 3), |_| {});
        let names: Vec<&str> = r.dirs.iter().map(|d| d.name.as_str()).collect();
        // Niveau 1, 2 et 3 ; ni node_modules, ni .cache ; pas de « sous-projet » (dans un projet) ;
        // pas de « src » (dans un projet non plus).
        for n in ["app-rust", "site", "clients", "client-a", "archives", "vieux"] {
            assert!(names.contains(&n), "{n} manque : {names:?}");
        }
        for n in ["node_modules", ".cache", "sous-projet", "src", "paquet"] {
            assert!(!names.contains(&n), "{n} ne devrait pas apparaître : {names:?}");
        }
        let get = |n: &str| r.dirs.iter().find(|d| d.name == n).unwrap();
        assert_eq!(get("vieux").depth, 3);
        assert!(get("app-rust").likely_project && get("client-a").likely_project && get("vieux").likely_project);
        assert!(!get("clients").likely_project && !get("archives").likely_project);
        assert_eq!(get("app-rust").manifest.description.as_deref(), Some("Une appli de test"));
        assert!(get("app-rust").manifest.stack.contains(&"egui".to_owned()));
        assert!(get("site").manifest.stack.iter().any(|s| s == "Next.js") && get("site").manifest.stack.iter().any(|s| s == "TypeScript"));
        assert!(get("vieux").manifest.stack.contains(&"PyTorch".to_owned()));
        assert!(get("app-rust").readme.as_deref().unwrap().contains("démonstration"));
        assert_eq!(r.per_root, vec![(0, r.dirs.len())]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn la_profondeur_limite_le_parcours() {
        let root = tree();
        let level1 = scan(&options(&root, 1), |_| {});
        assert!(level1.dirs.iter().all(|d| d.depth == 1));
        assert_eq!(level1.dirs.len(), 3, "app-rust, clients, site");
        let level2 = scan(&options(&root, 2), |_| {});
        assert!(level2.dirs.iter().any(|d| d.name == "client-a") && !level2.dirs.iter().any(|d| d.name == "vieux"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// root/
    ///   saas/ (README)      frontend/ (package.json)   backend/ (composer.json)   → 1 projet, 2 briques
    ///   mono/               packages/ ui/ (package.json)  api/ (package.json)   → monorepo
    ///   clients/            acme/ (.git)   globex/ (.git)                        → 2 projets (rangement)
    #[test]
    fn les_briques_sont_rattachees_a_leur_projet() {
        let root = std::env::temp_dir().join(format!("atelier-bricks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mk = |p: &str| std::fs::create_dir_all(root.join(p)).unwrap();
        let file = |p: &str, c: &str| std::fs::write(root.join(p), c).unwrap();
        mk("saas/frontend");
        mk("saas/backend");
        file("saas/README.md", "# SaaS\n\nUn logiciel de facturation en ligne pour artisans.");
        file("saas/frontend/package.json", r#"{"dependencies":{"react":"19"}}"#);
        file("saas/backend/composer.json", r#"{"require":{"laravel/framework":"11"}}"#);
        mk("mono/packages/ui");
        mk("mono/packages/api");
        file("mono/packages/ui/package.json", "{}");
        file("mono/packages/api/package.json", "{}");
        mk("clients/acme/.git");
        mk("clients/globex/.git");
        let r = scan(&options(&root, 3), |_| {});
        let get = |n: &str| r.dirs.iter().find(|d| d.name == n).unwrap_or_else(|| panic!("{n} manque"));
        let saas = get("saas").path.clone();
        assert_eq!(get("frontend").brick_of.as_deref(), Some(saas.as_str()));
        assert_eq!(get("backend").brick_of.as_deref(), Some(saas.as_str()));
        assert!(get("saas").likely_project && get("saas").bricks.len() == 2);
        assert!(get("saas").manifest.stack.contains(&"React".to_owned()) && get("saas").manifest.stack.contains(&"Laravel".to_owned()), "technos des briques");
        let mono = get("mono").path.clone();
        assert_eq!(get("ui").brick_of.as_deref(), Some(mono.as_str()));
        assert_eq!(get("packages").brick_of.as_deref(), Some(mono.as_str()));
        assert!(get("mono").likely_project);
        // Un dossier de rangement n'est pas un projet, et ses projets restent des projets.
        assert!(get("acme").brick_of.is_none() && get("globex").brick_of.is_none());
        assert!(!get("clients").likely_project);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn noms_de_briques_et_de_copies() {
        assert!(brick_like("frontend", "Shop") && brick_like("ui", "packages") && brick_like("shop-api", "shop"));
        assert!(!brick_like("FactureX", "Projet perso"));
        for n in ["BunFlow - Copie", "TopFoot-save", "site-sauv", "site-deploy-64a6597", "release-notifications-70168a8"] {
            assert!(copy_like(n), "{n}");
        }
        for n in ["FactureX", "saas-spacetime", "universal-drive", "auto-theme-time-switcher"] {
            assert!(!copy_like(n), "{n}");
        }
    }

    #[test]
    fn dossier_source_introuvable() {
        let opts = ScanOptions { roots: vec![(0, r"C:\nexiste\pas\du\tout".into(), 2)], ..options(Path::new("."), 1) };
        let r = scan(&opts, |_| {});
        assert!(r.dirs.is_empty() && r.errors.len() == 1);
    }
}
