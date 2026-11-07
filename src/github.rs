//! Les dépôts GitHub, via la CLI `gh` (déjà authentifiée : pas de jeton à saisir dans Rust Project).
//! On liste tous les dépôts accessibles : les siens, ceux des organisations, les collaborations.

use std::process::Command;

use serde::Deserialize;

use crate::classify::{Proposal, guess_domain};
use crate::model::{Data, path_key};

#[derive(Clone, Debug, Deserialize)]
pub struct Repo {
    pub name: String,
    pub full_name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub fork: bool,
    #[serde(default)]
    pub pushed_at: Option<String>,
    /// La fiche dont un dossier (ou un sous-dossier direct) a ce dépôt comme remote Git.
    #[serde(skip)]
    pub inside: Option<u64>,
}

const JQ: &str = ".[] | {name, full_name, description, html_url, language, archived, private, fork, pushed_at}";

fn gh() -> Command {
    let mut cmd = Command::new("gh");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// Les comptes connectés à gh, et le compte actif.
pub type Accounts = (Vec<String>, Option<String>);

/// Les comptes GitHub connectés à `gh` (`gh auth status`), et celui qui est actif.
pub fn accounts() -> Result<Accounts, String> {
    let out = gh().args(["auth", "status", "--hostname", "github.com"]).output().map_err(|e| format!("La CLI GitHub (gh) est introuvable : {e}"))?;
    // Selon la version de gh, l'état arrive sur la sortie standard ou sur celle des erreurs.
    let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let (list, active) = parse_accounts(&text);
    if list.is_empty() {
        return Err("Aucun compte GitHub connecté : lance « gh auth login » dans un terminal.".into());
    }
    Ok((list, active))
}

/// Lit la sortie de `gh auth status` : « Logged in to github.com account <login> », suivi de
/// « Active account: true » pour le compte actif.
pub fn parse_accounts(text: &str) -> (Vec<String>, Option<String>) {
    let (mut list, mut active) = (Vec::new(), None);
    for line in text.lines() {
        if let Some((_, rest)) = line.split_once(" account ") {
            if let Some(login) = rest.split_whitespace().next() {
                list.push(login.to_owned());
            }
        } else if line.contains("Active account: true") {
            active = list.last().cloned();
        }
    }
    (list, active)
}

/// Tous les dépôts accessibles (`gh api --paginate user/repos`), du plus récent au plus ancien.
/// `account` : un compte connecté à `gh` (son jeton est utilisé sans changer le compte actif) ;
/// `None` : le compte actif.
pub fn list_repos(account: Option<&str>) -> Result<Vec<Repo>, String> {
    let mut cmd = gh();
    if let Some(login) = account.filter(|a| !a.trim().is_empty()) {
        let token = gh().args(["auth", "token", "--hostname", "github.com", "--user", login]).output().map_err(|e| format!("La CLI GitHub (gh) est introuvable : {e}"))?;
        if !token.status.success() {
            return Err(format!("Pas de jeton pour le compte {login} : {}", String::from_utf8_lossy(&token.stderr).trim()));
        }
        cmd.env("GH_TOKEN", String::from_utf8_lossy(&token.stdout).trim());
    }
    let out = cmd
        .args(["api", "--paginate", "user/repos?per_page=100&sort=pushed&affiliation=owner,collaborator,organization_member", "--jq", JQ])
        .output()
        .map_err(|e| format!("La CLI GitHub (gh) est introuvable : {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("gh a échoué : {}", err.trim()));
    }
    parse_lines(&String::from_utf8_lossy(&out.stdout))
}

/// Une ligne JSON par dépôt (sortie de `--jq`).
pub fn parse_lines(text: &str) -> Result<Vec<Repo>, String> {
    text.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str::<Repo>(l).map_err(|e| format!("Ligne illisible : {e}"))).collect()
}

/// Une adresse de dépôt comparable : `https://github.com/Moi/Projet.git/` → `github.com/moi/projet`.
fn url_key(url: &str) -> String {
    let web = crate::scan::web_url(url).unwrap_or_else(|| url.to_owned());
    web.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').trim_end_matches(".git").to_lowercase()
}

/// Relie chaque dépôt à la fiche qui le contient : un de ses dossiers, ou un sous-dossier direct
/// (un projet fait de plusieurs dépôts, comme `framework`, `installer`… sous un même dossier).
/// Lit les `.git/config` : à appeler hors de l'interface.
pub fn link_local(repos: &mut [Repo], projects: &[(u64, Vec<String>)]) {
    let mut owner: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for (id, folders) in projects {
        for folder in folders {
            let dir = std::path::Path::new(folder.trim());
            let children = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir());
            for d in std::iter::once(dir.to_path_buf()).chain(children) {
                if let Some(remote) = crate::scan::git_remote(&d) {
                    owner.entry(url_key(&remote)).or_insert(*id);
                }
            }
        }
    }
    for repo in repos {
        repo.inside = owner.get(&url_key(&repo.html_url)).copied();
    }
}

/// La fiche d'un dépôt : même adresse, un dossier du même nom, ou un dossier qui le contient.
pub fn local_match(data: &Data, repo: &Repo) -> Option<usize> {
    let name = repo.name.to_lowercase();
    let key = url_key(&repo.html_url);
    data.projects
        .iter()
        .position(|p| p.remote.as_deref().is_some_and(|r| url_key(r) == key) || p.folders.iter().any(|f| path_key(f).rsplit('\\').next() == Some(name.as_str())))
        .or_else(|| repo.inside.and_then(|id| data.index_of(id)))
}

/// Une fiche pour un dépôt sans dossier local.
pub fn proposal(repo: &Repo) -> Proposal {
    let description = repo.description.clone().filter(|d| !d.trim().is_empty()).unwrap_or_else(|| format!("Dépôt GitHub {}.", repo.full_name));
    let stack: Vec<String> = repo.language.iter().cloned().collect();
    Proposal {
        is_project: !repo.fork,
        name: repo.name.clone(),
        domain: guess_domain(&format!("{} {} {}", repo.name, description, stack.join(" "))).into(),
        description,
        kind: if repo.archived { "Dépôt archivé".into() } else { "Dépôt GitHub".into() },
        stack,
        folder: None,
        extra_folders: Vec::new(),
        remote: Some(repo.html_url.clone()),
        idle_days: repo.pushed_at.as_deref().map(days_since_iso).unwrap_or(0),
        key_files: Vec::new(),
        confidence: 0.6,
        keywords: Vec::new(),
    }
}

/// Jours écoulés depuis une date ISO 8601 (`2026-10-04T12:00:00Z`), approximation au jour.
fn days_since_iso(s: &str) -> u32 {
    let parse = || -> Option<u64> {
        let (date, _) = s.split_once('T')?;
        let mut it = date.split('-').map(|x| x.parse::<i64>().ok());
        let (y, m, d) = (it.next()??, it.next()??, it.next()??);
        // Jours depuis 1970 (algorithme de Howard Hinnant).
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some((era * 146_097 + doe - 719_468) as u64)
    };
    let today = crate::model::now() / 86_400;
    parse().map(|days| today.saturating_sub(days) as u32).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_des_depots() {
        let text = r#"{"name":"atelier","full_name":"djdevpro/atelier","description":"Gestionnaire de projets","html_url":"https://github.com/djdevpro/atelier","language":"Rust","archived":false,"private":true,"fork":false,"pushed_at":"2026-11-01T10:00:00Z"}
{"name":"vieux","full_name":"org/vieux","description":null,"html_url":"https://github.com/org/vieux","language":null,"archived":true,"private":false,"fork":true,"pushed_at":null}"#;
        let repos = parse_lines(text).unwrap();
        assert_eq!(repos.len(), 2);
        let p = proposal(&repos[0]);
        assert!(p.is_project && p.remote.as_deref() == Some("https://github.com/djdevpro/atelier"));
        assert_eq!(p.stack, vec!["Rust".to_owned()]);
        assert!(!proposal(&repos[1]).is_project, "un fork n'est pas proposé comme projet");
        assert_eq!(days_since_iso("1970-01-02T00:00:00Z"), (crate::model::now() / 86_400 - 1) as u32);
    }

    #[test]
    fn comptes_connectes_a_gh() {
        let text = "github.com\n  ✓ Logged in to github.com account djdevpro (keyring)\n  - Active account: true\n  - Token: gho_***\n\n  ✓ Logged in to github.com account autre-compte (keyring)\n  - Active account: false\n";
        let (list, active) = parse_accounts(text);
        assert_eq!(list, vec!["djdevpro".to_owned(), "autre-compte".to_owned()]);
        assert_eq!(active.as_deref(), Some("djdevpro"));
        assert_eq!(parse_accounts("You are not logged into any GitHub hosts."), (Vec::new(), None));
    }

    /// Un dépôt rangé dans un sous-dossier d'une fiche (projet fait de plusieurs dépôts) est suivi.
    #[test]
    fn depot_dans_un_sous_dossier_de_fiche() {
        let root = std::env::temp_dir().join(format!("atelier-sous-depot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("framework").join(".git")).unwrap();
        std::fs::write(root.join("framework").join(".git").join("config"), "[remote \"origin\"]\n\turl = git@github.com:Moi/Framework.git\n").unwrap();
        let mut repos = parse_lines(r#"{"name":"framework","full_name":"moi/framework","html_url":"https://github.com/moi/framework"}"#).unwrap();
        link_local(&mut repos, &[(7, vec![root.display().to_string()])]);
        assert_eq!(repos[0].inside, Some(7));
        let _ = std::fs::remove_dir_all(&root);
    }
}
