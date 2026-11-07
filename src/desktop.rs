//! Sous Linux, un exécutable n'a pas d'icône : Rust Project se déclare au bureau (menu des
//! applications, dock) par un fichier `.desktop` et son icône, dans le dossier de l'utilisateur.
//! Le nom `rust-project` est celui de la fenêtre (app_id Wayland, classe X11).

use std::path::{Path, PathBuf};

pub const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Le fichier `.desktop` qui lance cet exécutable.
pub fn entry(exe: &str) -> String {
    // Guillemets de la spécification : `"`, `` ` ``, `$` et `\` y sont échappés.
    let mut quoted = String::from("\"");
    for c in exe.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    format!(
        "[Desktop Entry]\nType=Application\nName=Rust Project\nExec={quoted}\nIcon=rust-project\nTerminal=false\nCategories=Development;\nStartupWMClass=rust-project\n"
    )
}

/// Écrit (ou met à jour) l'icône et le `.desktop` dans `~/.local/share`. Silencieux en cas d'échec.
#[cfg(target_os = "linux")]
pub fn install() {
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/share")));
    let (Some(data), Ok(exe)) = (data, std::env::current_exe()) else { return };
    write_if_changed(&data.join("icons/hicolor/256x256/apps/rust-project.png"), ICON_PNG);
    write_if_changed(&data.join("applications/rust-project.desktop"), entry(&exe.display().to_string()).as_bytes());
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn write_if_changed(path: &Path, bytes: &[u8]) {
    if std::fs::read(path).is_ok_and(|old| old == bytes) {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fichier_desktop() {
        let e = entry("/home/moi/Téléchargements/rust project $1");
        assert!(e.contains("Exec=\"/home/moi/Téléchargements/rust project \\$1\"\n"), "{e}");
        assert!(e.contains("Icon=rust-project\n") && e.contains("StartupWMClass=rust-project\n"));
        // L'icône intégrée est bien un PNG carré de 256 px.
        let icon = eframe::icon_data::from_png_bytes(ICON_PNG).unwrap();
        assert_eq!((icon.width, icon.height), (256, 256));
        let dir = std::env::temp_dir().join(format!("rp-desktop-{}", std::process::id()));
        let file: PathBuf = dir.join("a/b.txt");
        write_if_changed(&file, b"un");
        write_if_changed(&file, b"un");
        assert_eq!(std::fs::read(&file).unwrap(), b"un");
        let _ = std::fs::remove_dir_all(dir);
    }
}
