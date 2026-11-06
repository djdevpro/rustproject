//! L'enregistrement : un fichier JSON lisible (`%APPDATA%\Atelier\atelier.json`), écrit de façon
//! atomique (fichier temporaire, puis renommage). L'interface et le serveur MCP partagent ce
//! fichier : chacun relit ce que l'autre a écrit.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::model::Data;

pub struct Store {
    path: PathBuf,
    /// Date de modification du fichier quand on l'a lu ou écrit pour la dernière fois.
    seen: Option<SystemTime>,
}

impl Store {
    /// `ATELIER_DATA` s'il est défini (tests, plusieurs bases), sinon `%APPDATA%\Atelier\atelier.json`.
    pub fn default_path() -> PathBuf {
        if let Ok(p) = std::env::var("ATELIER_DATA") {
            return PathBuf::from(p);
        }
        // Windows : %APPDATA%. Linux : $XDG_DATA_HOME, sinon ~/.local/share.
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("XDG_DATA_HOME").map(PathBuf::from))
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
            .unwrap_or_else(|_| std::env::temp_dir());
        base.join("Atelier").join("atelier.json")
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
