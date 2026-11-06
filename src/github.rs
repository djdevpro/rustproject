//! Les dépôts GitHub, via la CLI `gh` (déjà authentifiée : pas de jeton à saisir dans Atelier).
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

/// Tous les dépôts accessibles (`gh api --paginate user/repos`), du plus récent au plus ancien.
pub fn list_repos() -> Result<Vec<Repo>, String> {
    let out = gh()
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

/// Le dossier local d'un dépôt, s'il existe sous un des dossiers suivis (même nom).
pub fn local_match(data: &Data, repo: &Repo) -> Option<usize> {
    let name = repo.name.to_lowercase();
    data.projects.iter().position(|p| {
        p.remote.as_deref().is_some_and(|r| r.eq_ignore_ascii_case(&repo.html_url)) || p.folders.iter().any(|f| path_key(f).rsplit('\\').next() == Some(name.as_str()))
    })
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
}
