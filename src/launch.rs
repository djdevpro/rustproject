//! Le lancement réel : chaque lanceur devient un processus Windows, sans attendre sa fin.
//!
//! Tout passe par `cmd /S /C "…"`, qui sait trouver `code.cmd`, `claude.exe`, `start`… dans le
//! PATH. La console de `cmd` est cachée ; les commandes « dans un terminal » ouvrent leur propre
//! fenêtre PowerShell, placée dans le dossier du projet.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::model::{Launcher, LauncherKind, Project};

/// Ce qui sera exécuté : une ligne pour `cmd`, et le dossier de travail.
#[derive(Debug, Clone)]
pub struct Plan {
    pub line: String,
    pub dir: Option<String>,
}

/// Prépare le lancement, ou explique pourquoi c'est impossible (dossier introuvable…).
pub fn plan(l: &Launcher, p: &Project) -> Result<Plan, String> {
    let folder = p.folders.first().cloned().unwrap_or_default();
    let resolved = l.resolve(p);
    let dir_ok = !folder.is_empty() && Path::new(&folder).is_dir();
    let needs_dir = l.kind != LauncherKind::Url && (l.in_terminal || l.kind == LauncherKind::Folder || l.command.contains("{dossier}"));
    if needs_dir && !dir_ok {
        return Err(if folder.is_empty() { "Ce projet n'a pas de dossier.".to_owned() } else { format!("Dossier introuvable : {folder}") });
    }
    if resolved.trim().is_empty() {
        return Err("La commande du lanceur est vide.".to_owned());
    }
    let line = match l.kind {
        // `start "" "<adresse>"` ouvre le programme associé (navigateur, obsidian://…).
        LauncherKind::Url => format!(r#"start "" "{resolved}""#),
        LauncherKind::Command if l.in_terminal => {
            let title = l.name.replace('"', "");
            format!(r#"start "{title}" /D "{folder}" powershell -NoExit -Command {resolved}"#)
        }
        _ => resolved,
    };
    Ok(Plan { line, dir: dir_ok.then_some(folder) })
}

#[cfg(windows)]
fn shell(line: &str) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = Command::new("cmd");
    // `/S` : cmd retire exactement la première et la dernière guillemet, le reste est intact.
    // `chcp 65001` : les messages d'erreur de cmd arrivent en UTF-8.
    cmd.raw_arg(format!(r#"/S /C "chcp 65001>nul & {line}""#));
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn shell(line: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(line);
    cmd
}

/// Lance sans attendre. Si la commande échoue en écrivant une erreur, `on_error` la reçoit
/// (depuis un autre fil d'exécution).
pub fn spawn(plan: &Plan, on_error: impl FnOnce(String) + Send + 'static) -> Result<(), String> {
    let mut cmd = shell(&plan.line);
    if let Some(dir) = &plan.dir {
        cmd.current_dir(dir);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    let child = cmd.spawn().map_err(|e| format!("Impossible de lancer : {e}"))?;
    std::thread::spawn(move || {
        if let Ok(out) = child.wait_with_output() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_owned();
            if !out.status.success() && !err.is_empty() {
                on_error(err.lines().next().unwrap_or_default().to_owned());
            }
        }
    });
    Ok(())
}

/// Argument passé à l'instance relancée : elle sait qu'elle sort d'une mise à jour.
pub const AFTER_UPDATE: &str = "--apres-mise-a-jour";

/// Relance Atelier (le même exécutable), pour appliquer une mise à jour.
pub fn restart() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Command::new(exe).arg(AFTER_UPDATE).spawn().map(|_| ()).map_err(|e| format!("Redémarrage impossible : {e}"))
}
