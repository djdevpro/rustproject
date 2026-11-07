//! « En ce moment » : les fichiers modifiés récemment dans les dossiers des projets.
//!
//! Le relevé tourne en arrière-plan (toutes les 30 s tant que le panneau est visible) : il
//! parcourt chaque dossier de projet en sautant les dossiers cachés, les exclusions du scan
//! (`node_modules`, `target`…) et les dossiers d'autres projets (pas de doublons quand un projet
//! en contient un autre). Il garde, par projet, les fichiers touchés dans la période choisie, le
//! README, la branche Git et le nombre de fichiers non commités.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

/// Fichiers gardés par projet (les plus récents).
pub const MAX_FILES: usize = 40;
/// Entrées parcourues au plus par projet : un dossier énorme ne bloque pas le relevé.
const MAX_ENTRIES: usize = 40_000;
const MAX_DEPTH: usize = 16;
/// Un nouveau relevé toutes les tant de secondes, tant que le panneau est affiché.
pub const REFRESH_SECS: f64 = 30.0;

/// La période regardée.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Window {
    #[default]
    Day,
    Week,
    Month,
}

impl Window {
    pub fn secs(self) -> u64 {
        match self {
            Self::Day => 86_400,
            Self::Week => 7 * 86_400,
            Self::Month => 30 * 86_400,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FileHit {
    /// Chemin complet.
    pub path: String,
    /// Chemin dans le projet (préfixé du nom du dossier quand le projet en a plusieurs).
    pub rel: String,
    pub mtime: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GitInfo {
    pub branch: String,
    /// Fichiers modifiés et pas encore commités (`None` : git introuvable).
    pub dirty: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectActivity {
    pub id: u64,
    /// La modification la plus récente.
    pub last: u64,
    /// Fichiers modifiés dans la période (tous, même au-delà de `files`).
    pub count: usize,
    /// Les plus récents d'abord, `MAX_FILES` au plus.
    pub files: Vec<FileHit>,
    pub readme: Option<String>,
    pub git: Option<GitInfo>,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// Quand le relevé a été fait (secondes Unix).
    pub at: u64,
    /// Les projets qui ont bougé, du plus récent au plus ancien.
    pub projects: Vec<ProjectActivity>,
}

impl Snapshot {
    pub fn files(&self) -> usize {
        self.projects.iter().map(|p| p.count).sum()
    }
}

/// Un projet à relever : son identifiant et ses dossiers.
pub struct Job {
    pub id: u64,
    pub folders: Vec<String>,
}

/// Le relevé en arrière-plan : le dernier résultat, et quand relancer.
#[derive(Default)]
pub struct Watcher {
    shared: Arc<Mutex<Option<Snapshot>>>,
    busy: Arc<AtomicBool>,
    pub snapshot: Option<Snapshot>,
    /// Dernière demande : (instant de l'interface, période).
    last: Option<(f64, Window)>,
}

impl Watcher {
    /// Reprend le dernier relevé arrivé, et en lance un nouveau s'il est temps. `jobs` n'est
    /// appelé que dans ce cas : (projets, exclusions en minuscules).
    pub fn poll(&mut self, ctx: &egui::Context, now: f64, window: Window, jobs: impl FnOnce() -> (Vec<Job>, Vec<String>)) {
        if let Ok(mut shared) = self.shared.lock()
            && let Some(snap) = shared.take()
        {
            self.snapshot = Some(snap);
        }
        let due = self.last.is_none_or(|(at, w)| w != window || now - at >= REFRESH_SECS);
        if !due || self.busy.load(Ordering::Acquire) {
            return;
        }
        self.last = Some((now, window));
        let (jobs, excludes) = jobs();
        self.busy.store(true, Ordering::Release);
        let (shared, busy, ctx) = (self.shared.clone(), self.busy.clone(), ctx.clone());
        std::thread::spawn(move || {
            let snap = collect(&jobs, &excludes, window, crate::model::now());
            if let Ok(mut s) = shared.lock() {
                *s = Some(snap);
            }
            busy.store(false, Ordering::Release);
            ctx.request_repaint();
        });
    }

    pub fn busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }

    /// Relancer le relevé au prochain passage.
    pub fn refresh(&mut self) {
        self.last = None;
    }

    /// Tests : un relevé posé à la main, que le relevé en arrière-plan ne remplace plus.
    #[cfg(test)]
    pub fn freeze(&mut self, snap: Snapshot) {
        while self.busy() {
            std::thread::yield_now();
        }
        if let Ok(mut shared) = self.shared.lock() {
            shared.take();
        }
        self.snapshot = Some(snap);
        self.last = Some((f64::INFINITY, Window::Day));
    }
}

/// Le relevé lui-même (synchrone).
pub fn collect(jobs: &[Job], excludes: &[String], window: Window, now: u64) -> Snapshot {
    let since = now.saturating_sub(window.secs());
    // Les dossiers de tous les projets : on ne descend pas dans celui d'un autre projet.
    let roots: Vec<String> = jobs.iter().flat_map(|j| j.folders.iter().map(|f| norm(f))).collect();
    let mut projects = Vec::new();
    for job in jobs {
        let multi = job.folders.len() > 1;
        let mut files = Vec::new();
        let mut count = 0;
        for folder in &job.folders {
            let root = Path::new(folder);
            if !root.is_dir() {
                continue;
            }
            let prefix = if multi { format!("{}/", root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()) } else { String::new() };
            let own: Vec<&String> = job.folders.iter().collect();
            let mut hits = Vec::new();
            let overflow = walk(root, since, excludes, &roots, &own, &mut hits);
            // Ce que git ignore (fichiers générés, builds, caches) n'est pas du travail.
            let ignored = git_ignored(root, &hits);
            hits.retain(|h| !ignored.contains(&h.rel));
            count += hits.len() + overflow;
            files.extend(hits.into_iter().map(|h| FileHit { rel: format!("{prefix}{}", h.rel), ..h }));
        }
        if files.is_empty() {
            continue;
        }
        files.sort_by(|a: &FileHit, b| b.mtime.cmp(&a.mtime).then_with(|| a.rel.cmp(&b.rel)));
        files.truncate(MAX_FILES);
        let first = job.folders.iter().map(Path::new).find(|p| p.is_dir());
        projects.push(ProjectActivity {
            id: job.id,
            last: files[0].mtime,
            count,
            files,
            readme: job.folders.iter().find_map(|f| find_readme(Path::new(f))),
            git: first.and_then(git_info),
        });
    }
    projects.sort_by_key(|p| std::cmp::Reverse(p.last));
    Snapshot { at: now, projects }
}

/// Un chemin comparable : minuscules, séparateurs `/`, sans `/` final.
fn norm(p: &str) -> String {
    p.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// Fichiers récents gardés au plus par dossier avant le filtre de git (au-delà, seulement comptés).
const MAX_HITS: usize = 20_000;

/// Parcourt `root` ; `files` reçoit les fichiers récents (chemin relatif à `root`). Renvoie le
/// nombre de fichiers récents au-delà de `MAX_HITS`.
fn walk(root: &Path, since: u64, excludes: &[String], roots: &[String], own: &[&String], files: &mut Vec<FileHit>) -> usize {
    let mut overflow = 0;
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut seen = 0;
    let own: Vec<String> = own.iter().map(|f| norm(f)).collect();
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                return overflow;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else { continue };
            let lower = name.to_lowercase();
            if kind.is_dir() {
                if depth >= MAX_DEPTH || lower.starts_with('.') || lower.starts_with('$') || excludes.contains(&lower) {
                    continue;
                }
                let path = entry.path();
                let key = norm(&path.to_string_lossy());
                // Le dossier d'un autre projet : il a sa propre carte.
                if roots.contains(&key) && !own.contains(&key) {
                    continue;
                }
                stack.push((path, depth + 1));
            } else if kind.is_file() {
                if lower.starts_with("~$") || lower.ends_with(".tmp") || lower.ends_with(".swp") || lower.starts_with(".#") {
                    continue;
                }
                let Some(mtime) = entry.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()) else { continue };
                if mtime < since {
                    continue;
                }
                if files.len() >= MAX_HITS {
                    overflow += 1;
                    continue;
                }
                let path = entry.path();
                let rel = path.strip_prefix(root).map(|r| r.to_string_lossy().replace('\\', "/")).unwrap_or_else(|_| name.clone());
                files.push(FileHit { path: path.to_string_lossy().into_owned(), rel, mtime });
            }
        }
    }
    overflow
}

/// Parmi `hits` (chemins relatifs à `root`), ceux que git ignore. Vide hors d'un dépôt ou sans git.
fn git_ignored(root: &Path, hits: &[FileHit]) -> std::collections::HashSet<String> {
    use std::io::Write as _;
    let mut set = std::collections::HashSet::new();
    if hits.is_empty() {
        return set;
    }
    let mut cmd = std::process::Command::new("git");
    // `-z` : chemins séparés par un octet nul, à l'aller comme au retour (jamais entre guillemets).
    cmd.arg("-C").arg(root).args(["check-ignore", "-z", "--stdin"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let Ok(mut child) = cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn() else { return set };
    let input: Vec<u8> = hits.iter().flat_map(|h| h.rel.bytes().chain([0])).collect();
    let mut stdin = child.stdin.take();
    // Écrire dans un autre fil : git répond pendant qu'on écrit, sans blocage.
    let writer = std::thread::spawn(move || {
        if let Some(s) = stdin.as_mut() {
            let _ = s.write_all(&input);
        }
    });
    let out = child.wait_with_output();
    let _ = writer.join();
    if let Ok(out) = out {
        set.extend(out.stdout.split(|b| *b == 0).filter(|p| !p.is_empty()).map(|p| String::from_utf8_lossy(p).into_owned()));
    }
    set
}

/// Le README à la racine du dossier (`README.md` de préférence).
pub fn find_readme(dir: &Path) -> Option<String> {
    let mut found: Vec<(u8, String)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|k| k.is_file()))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            let rank = match name.as_str() {
                "readme.md" => 0,
                n if n.starts_with("readme") => 1,
                _ => return None,
            };
            Some((rank, e.path().to_string_lossy().into_owned()))
        })
        .collect();
    found.sort();
    found.into_iter().next().map(|(_, p)| p)
}

/// La branche (lue dans `.git/HEAD`, sans lancer git) et les fichiers non commités.
fn git_info(dir: &Path) -> Option<GitInfo> {
    let dot = dir.join(".git");
    let git_dir = if dot.is_dir() {
        dot
    } else {
        // Un worktree : `.git` est un fichier « gitdir: <chemin> ».
        let text = std::fs::read_to_string(&dot).ok()?;
        let p = text.trim().strip_prefix("gitdir:")?.trim().to_owned();
        let p = Path::new(&p);
        if p.is_absolute() { p.to_path_buf() } else { dir.join(p) }
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    let branch = match head.strip_prefix("ref: refs/heads/") {
        Some(b) => b.to_owned(),
        None => head.chars().take(7).collect(),
    };
    Some(GitInfo { branch, dirty: dirty_count(dir) })
}

fn dirty_count(dir: &Path) -> Option<usize> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C").arg(dir).args(["status", "--porcelain"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).lines().filter(|l| !l.trim().is_empty()).count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releve_des_fichiers_recents() {
        let base = std::env::temp_dir().join(format!("rp-recent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let a = base.join("alpha");
        let b = a.join("beta"); // un projet dans un autre : pas compté deux fois
        for d in [a.join("src"), a.join("node_modules/x"), a.join(".cache"), b.clone()] {
            std::fs::create_dir_all(d).unwrap();
        }
        for f in [a.join("README.md"), a.join("src/main.rs"), a.join("node_modules/x/i.js"), a.join(".cache/c"), b.join("lib.rs"), a.join("~$brouillon.docx")] {
            std::fs::write(f, "x").unwrap();
        }
        let jobs = vec![
            Job { id: 1, folders: vec![a.to_string_lossy().into_owned()] },
            Job { id: 2, folders: vec![b.to_string_lossy().into_owned()] },
            Job { id: 3, folders: vec![base.join("absent").to_string_lossy().into_owned()] },
        ];
        let snap = collect(&jobs, &["node_modules".to_owned()], Window::Day, crate::model::now() + 5);
        let alpha = snap.projects.iter().find(|p| p.id == 1).unwrap();
        let mut rels: Vec<&str> = alpha.files.iter().map(|f| f.rel.as_str()).collect();
        rels.sort();
        assert_eq!(rels, ["README.md", "src/main.rs"]);
        assert_eq!(alpha.count, 2);
        assert!(alpha.readme.as_deref().is_some_and(|r| r.ends_with("README.md")));
        let beta = snap.projects.iter().find(|p| p.id == 2).unwrap();
        assert_eq!(beta.files[0].rel, "lib.rs");
        assert!(!snap.projects.iter().any(|p| p.id == 3));
        // Une période passée : rien.
        let old = collect(&jobs, &[], Window::Day, crate::model::now() + 3 * 86_400);
        assert!(old.projects.is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Les fichiers ignorés par git (générés, caches) ne comptent pas.
    #[test]
    fn fichiers_ignores_par_git() {
        let base = std::env::temp_dir().join(format!("rp-recent-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("gen")).unwrap();
        let init = std::process::Command::new("git").arg("-C").arg(&base).arg("init").output();
        if !init.is_ok_and(|o| o.status.success()) {
            return; // pas de git sur cette machine
        }
        std::fs::write(base.join(".gitignore"), "gen/\n*.log\n").unwrap();
        for f in ["main.rs", "gen/a.json", "trace.log"] {
            std::fs::write(base.join(f), "x").unwrap();
        }
        let jobs = vec![Job { id: 1, folders: vec![base.to_string_lossy().into_owned()] }];
        let snap = collect(&jobs, &[], Window::Day, crate::model::now() + 5);
        let p = &snap.projects[0];
        let mut rels: Vec<&str> = p.files.iter().map(|f| f.rel.as_str()).collect();
        rels.sort();
        assert_eq!(rels, [".gitignore", "main.rs"], "ni gen/ ni *.log");
        assert_eq!(p.count, 2);
        assert_eq!(p.git.as_ref().map(|g| g.dirty), Some(Some(2)), ".gitignore et main.rs non commités");
        let _ = std::fs::remove_dir_all(&base);
    }
}
