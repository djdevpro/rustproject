//! Rust Project déclaré dans Claude Code : où il l'est (lu dans `~/.claude.json`), et l'ajout en
//! un clic pour tous les projets (portée « user »), par la CLI `claude`.
//!
//! Une déclaration locale (un seul dossier) l'emporte sur la globale dans ce dossier : à
//! l'installation, on les retire pour que tous les projets voient le même serveur.

use std::path::PathBuf;

use serde_json::Value;

/// Le nom du serveur dans Claude Code.
pub const NAME: &str = "rust-project";

/// Où Claude Code connaît Rust Project.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClaudeSetup {
    /// La commande déclarée pour tous les projets (portée « user »).
    pub global: Option<String>,
    /// Les dossiers où il est déclaré pour ce seul dossier (portée « local »).
    pub local: Vec<String>,
}

impl ClaudeSetup {
    /// Déclaré pour tous les projets, vers cet exécutable, sans déclaration locale qui le masque.
    pub fn is_ok(&self, exe: &str) -> bool {
        self.local.is_empty() && self.global.as_deref().is_some_and(|c| same_path(c, exe))
    }
}

/// Le fichier de configuration de Claude Code.
fn config_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        return Some(PathBuf::from(dir).join(".claude.json"));
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".claude.json"))
}

/// Lit la configuration de Claude Code (rien de déclaré si elle est absente ou illisible).
pub fn read() -> ClaudeSetup {
    config_path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<Value>(&t).ok()).map(|v| from_config(&v)).unwrap_or_default()
}

pub fn from_config(config: &Value) -> ClaudeSetup {
    let global = config["mcpServers"][NAME]["command"].as_str().map(str::to_owned);
    let mut local: Vec<String> = config["projects"]
        .as_object()
        .map(|projects| projects.iter().filter(|(_, p)| !p["mcpServers"][NAME].is_null()).map(|(dir, _)| dir.clone()).collect())
        .unwrap_or_default();
    local.sort();
    ClaudeSetup { global, local }
}

fn same_path(a: &str, b: &str) -> bool {
    let n = |s: &str| s.trim().trim_matches('"').replace('/', "\\").to_lowercase();
    n(a) == n(b)
}

/// La commande à taper soi-même (celle que fait `install`).
pub fn command(exe: &str) -> String {
    format!(r#"claude mcp add --scope user {NAME} -- "{exe}" --mcp"#)
}

fn claude(args: &[&str], dir: Option<&str>) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        // `claude` peut être un `.cmd` (installation npm) : `cmd /C` le trouve dans le PATH.
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C").arg("claude").creation_flags(0x0800_0000);
        cmd
    };
    #[cfg(not(windows))]
    let mut cmd = std::process::Command::new("claude");
    cmd.args(args).stdin(std::process::Stdio::null());
    if let Some(dir) = dir.filter(|d| std::path::Path::new(d).is_dir()) {
        cmd.current_dir(dir);
    }
    cmd.output()
}

/// Déclare Rust Project pour tous les projets : retire les déclarations locales et l'ancienne
/// globale, puis ajoute celle-ci (cet exécutable).
pub fn install(exe: &str, setup: &ClaudeSetup) -> Result<(), String> {
    for dir in &setup.local {
        let _ = claude(&["mcp", "remove", NAME, "-s", "local"], Some(dir));
    }
    if setup.global.is_some() {
        let _ = claude(&["mcp", "remove", NAME, "-s", "user"], None);
    }
    let out = claude(&["mcp", "add", "--scope", "user", NAME, "--", exe, "--mcp"], None).map_err(|e| format!("Claude Code introuvable : {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.lines().find(|l| !l.trim().is_empty()).unwrap_or("Claude Code introuvable (commande « claude »).");
        Err(err.trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lire_la_configuration_de_claude_code() {
        let exe = r"C:\Apps\rust-project-0.2.0.exe";
        let config = json!({
            "mcpServers": { "rust-project": { "command": r"C:\Apps\rust-project-0.1.0.exe", "args": ["--mcp"] }, "autre": { "command": "x" } },
            "projects": {
                "C:/b": { "mcpServers": { "rust-project": { "command": exe } } },
                "C:/a": { "mcpServers": {} },
                "C:/c": {}
            }
        });
        let s = from_config(&config);
        assert_eq!(s.global.as_deref(), Some(r"C:\Apps\rust-project-0.1.0.exe"));
        assert_eq!(s.local, ["C:/b"]);
        assert!(!s.is_ok(exe), "ancien exécutable, et une déclaration locale");
        let s = ClaudeSetup { global: Some("c:/apps/RUST-PROJECT-0.2.0.exe".into()), local: vec![] };
        assert!(s.is_ok(exe), "même chemin, casse et séparateurs à part");
        assert_eq!(from_config(&json!({})), ClaudeSetup::default());
        assert_eq!(command(exe), r#"claude mcp add --scope user rust-project -- "C:\Apps\rust-project-0.2.0.exe" --mcp"#);
    }
}
