//! L'enregistrement : un fichier JSON lisible (`%USERPROFILE%\.rust-project\rust-project.json`), écrit de façon
//! atomique (fichier temporaire, puis renommage). L'interface et le serveur MCP partagent ce
//! fichier : chacun relit ce que l'autre a écrit.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::model::Data;

/// Reprend les données d'un ancien emplacement (`old` et l'`embeddings.json` voisin), une seule
/// fois : rien n'est fait si le nouveau fichier existe déjà. L'ancien fichier reste en sauvegarde.
fn migrate(old: &Path, path: &Path) {
    if path.exists() || !old.exists() {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::copy(old, path);
    let embeddings = old.with_file_name("embeddings.json");
    if embeddings.exists() {
        let _ = std::fs::copy(embeddings, path.with_file_name("embeddings.json"));
    }
}

pub struct Store {
    path: PathBuf,
    /// Date de modification du fichier quand on l'a lu ou écrit pour la dernière fois.
    seen: Option<SystemTime>,
}

impl Store {
    /// `ATELIER_DATA` s'il est défini (tests, plusieurs bases), sinon le dossier de l'utilisateur.
    pub fn default_path() -> PathBuf {
        if let Ok(p) = std::env::var("ATELIER_DATA") {
            return PathBuf::from(p);
        }
        // Windows : `%USERPROFILE%\.rust-project`, pas `%APPDATA%`. Pour une application du Store
        // comme Claude Desktop, Windows détourne les écritures dans AppData vers un dossier privé :
        // le serveur MCP qu'elle lance et l'interface n'y verraient pas les mêmes données.
        if cfg!(windows)
            && let Ok(home) = std::env::var("USERPROFILE")
        {
            let path = PathBuf::from(home).join(".rust-project").join("rust-project.json");
            if let Ok(appdata) = std::env::var("APPDATA") {
                let appdata = PathBuf::from(appdata);
                migrate(&appdata.join("Rust Project").join("rust-project.json"), &path);
                migrate(&appdata.join("Atelier").join("atelier.json"), &path);
            }
            return path;
        }
        // Linux : $XDG_DATA_HOME, sinon ~/.local/share.
        let base = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
            .unwrap_or_else(|_| std::env::temp_dir());
        let path = base.join("Rust Project").join("rust-project.json");
        migrate(&base.join("Atelier").join("atelier.json"), &path);
        path
    }

    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), seen: None }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn mtime(&self) -> Option<SystemTime> {
        std::fs::metadata(&self.path).and_then(|m| m.modified()).ok()
    }

    /// Lit le fichier. `Ok(None)` s'il n'existe pas encore.
    pub fn load(&mut self) -> Result<Option<Data>, String> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("Lecture de {} impossible : {e}", self.path.display())),
        };
        let mut data: Data = serde_json::from_str(&text).map_err(|e| format!("{} est illisible : {e}", self.path.display()))?;
        data.normalize();
        self.seen = self.mtime();
        Ok(Some(data))
    }

    /// Écrit le fichier (temporaire puis renommage : jamais de fichier à moitié écrit).
    pub fn save(&mut self, data: &Data) -> Result<(), String> {
        let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        self.save_text(&json)
    }

    pub fn save_text(&mut self, json: &str) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Création de {} impossible : {e}", dir.display()))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| format!("Écriture impossible : {e}"))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| format!("Écriture impossible : {e}"))?;
        self.seen = self.mtime();
        Ok(())
    }

    /// Le fichier a-t-il été modifié par quelqu'un d'autre (le serveur MCP…) depuis notre dernier accès ?
    pub fn changed_on_disk(&self) -> bool {
        match (self.mtime(), self.seen) {
            (Some(now), Some(seen)) => now != seen,
            (Some(_), None) => true,
            _ => false,
        }
    }

    /// Le fichier des embeddings, à côté du fichier principal.
    pub fn embeddings_path(&self) -> PathBuf {
        self.path.with_file_name("embeddings.json")
    }
}

// ---- index sémantique --------------------------------------------------------------------------

/// Les vecteurs d'embeddings des projets, par identifiant. `hash` est celui du texte embarqué :
/// un projet dont le texte change est réindexé.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SemanticIndex {
    pub model: String,
    pub vectors: HashMap<u64, (u64, Vec<f32>)>,
}

impl SemanticIndex {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, serde_json::to_string(self).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
}

/// Une empreinte stable d'un texte (FNV-1a 64 bits).
pub fn text_hash(text: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les données d'un ancien emplacement (Atelier, AppData) passent dans le nouveau, une seule
    /// fois, sans rien écraser, et l'ancien dossier reste en place.
    #[test]
    fn reprise_des_anciennes_donnees() {
        let base = std::env::temp_dir().join(format!("rust-project-migration-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let old = base.join("Atelier");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("atelier.json"), "{\"projects\":[]}").unwrap();
        std::fs::write(old.join("embeddings.json"), "{}").unwrap();
        let path = base.join("Rust Project").join("rust-project.json");
        migrate(&old.join("atelier.json"), &path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"projects\":[]}");
        assert!(path.with_file_name("embeddings.json").exists());
        assert!(old.join("atelier.json").exists(), "l'ancien fichier reste en sauvegarde");
        // Une seconde fois : le nouveau fichier n'est jamais écrasé.
        std::fs::write(&path, "nouveau").unwrap();
        migrate(&old.join("atelier.json"), &path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "nouveau");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn aller_retour_et_detection_des_changements() {
        let dir = std::env::temp_dir().join(format!("atelier-store-{}", std::process::id()));
        let path = dir.join("atelier.json");
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = Store::open(&path);
        assert!(store.load().unwrap().is_none(), "pas de fichier au départ");

        let mut data = crate::model::demo();
        data.projects[0].steps.push(crate::model::Step { text: "Nouvelle étape".into(), done: true });
        store.save(&data).unwrap();
        assert!(!store.changed_on_disk());

        let back = Store::open(&path).load().unwrap().unwrap();
        assert_eq!(back.projects.len(), data.projects.len());
        assert_eq!(back.projects[0].steps.last().unwrap().text, "Nouvelle étape");
        assert_eq!(back.projects[0].icon.to_string(), data.projects[0].icon.to_string());

        // Un autre processus écrit le fichier : on le voit.
        std::thread::sleep(std::time::Duration::from_millis(20));
        Store::open(&path).save(&back).unwrap();
        assert!(store.changed_on_disk());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
