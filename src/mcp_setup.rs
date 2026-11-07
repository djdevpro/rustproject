//! Rust Project déclaré dans Claude Code : où il l'est (lu dans `~/.claude.json`), et l'ajout en
//! un clic pour tous les projets (portée « user »), par la CLI `claude`.
//!
//! Une déclaration locale (un seul dossier) l'emporte sur la globale dans ce dossier : à
//! l'installation, on les retire pour que tous les projets voient le même serveur.
//!
//! Codex aussi : il le lit dans `~/.codex/config.toml` (`[mcp_servers.rust-project]`), valable
//! pour tous les projets, et l'ajoute par sa CLI `codex mcp add`.

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
    cli("claude", args, dir)
}

/// Une CLI (`claude`, `codex`) lancée sans console, qui répond sur sa sortie.
fn cli(program: &str, args: &[&str], dir: Option<&str>) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        // La CLI peut être un `.cmd` (installation npm) : `cmd /C` la trouve dans le PATH.
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C").arg(program).creation_flags(0x0800_0000);
        cmd
    };
    #[cfg(not(windows))]
    let mut cmd = std::process::Command::new(program);
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

// ---- Codex -------------------------------------------------------------------------------------

/// Le fichier de configuration de Codex.
fn codex_config_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CODEX_HOME") {
        return Some(PathBuf::from(dir).join("config.toml"));
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".codex").join("config.toml"))
}

/// La commande avec laquelle Codex lance Rust Project (`None` : pas déclaré, ou pas de Codex).
pub fn codex_read() -> Option<String> {
    codex_config_path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| codex_from_config(&t))
}

/// La commande de `[mcp_servers.rust-project]` dans un `config.toml` de Codex.
pub fn codex_from_config(toml: &str) -> Option<String> {
    let headers = [format!("[mcp_servers.{NAME}]"), format!("[mcp_servers.\"{NAME}\"]"), format!("[mcp_servers.'{NAME}']")];
    let mut inside = false;
    for line in toml.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = headers.iter().any(|h| line.replace(' ', "") == *h);
            continue;
        }
        if !inside {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        if key.trim() != "command" {
            continue;
        }
        let value = value.trim();
        // Chaîne littérale ('…') telle quelle ; chaîne de base ("…") avec ses échappements.
        if let Some(v) = value.strip_prefix('\'').and_then(|v| v.split_once('\'')).map(|(v, _)| v) {
            return Some(v.to_owned());
        }
        if let Some(v) = value.strip_prefix('"') {
            let mut out = String::new();
            let mut chars = v.chars();
            while let Some(c) = chars.next() {
                match c {
                    '"' => return Some(out),
                    '\\' => match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some(other) => out.push(other),
                        None => break,
                    },
                    c => out.push(c),
                }
            }
        }
        return None;
    }
    None
}

/// Déclaré dans Codex, vers cet exécutable ?
pub fn codex_is_ok(command: Option<&str>, exe: &str) -> bool {
    command.is_some_and(|c| same_path(c, exe))
}

/// La commande à taper soi-même (celle que fait `codex_install`).
pub fn codex_command(exe: &str) -> String {
    format!(r#"codex mcp add {NAME} -- "{exe}" --mcp"#)
}

/// Déclare Rust Project dans Codex, pour tous les projets : retire l'ancienne déclaration, puis
/// ajoute celle-ci (cet exécutable).
pub fn codex_install(exe: &str, declared: bool) -> Result<(), String> {
    if declared {
        let _ = cli("codex", &["mcp", "remove", NAME], None);
    }
    let out = cli("codex", &["mcp", "add", NAME, "--", exe, "--mcp"], None).map_err(|e| format!("Codex introuvable : {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.lines().find(|l| !l.trim().is_empty()).unwrap_or("Codex introuvable (commande « codex »).");
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

    #[test]
    fn lire_la_configuration_de_codex() {
        let exe = r"C:\Apps\rust-project.exe";
        let toml = r#"model = "x"
[mcp_servers.magic]
command = "npx"

[mcp_servers.rust-project]
args = ["--mcp"]
command = 'C:\Apps\rust-project.exe'
[mcp_servers.rust-project.env]
command = "faux"
"#;
        assert_eq!(codex_from_config(toml).as_deref(), Some(exe));
        let toml = r#"[mcp_servers."rust-project"]
command = "C:\\Apps\\rust-project.exe"
"#;
        assert_eq!(codex_from_config(toml).as_deref(), Some(exe), "chaîne de base, échappements");
        assert_eq!(codex_from_config("[mcp_servers.magic]\ncommand = \"npx\"\n"), None);
        assert!(codex_is_ok(Some("c:/apps/RUST-PROJECT.exe"), exe));
        assert!(!codex_is_ok(None, exe));
        assert_eq!(codex_command(exe), r#"codex mcp add rust-project -- "C:\Apps\rust-project.exe" --mcp"#);
    }
}
