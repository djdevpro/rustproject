//! Le client Ollaya : des modèles de **décision** (ils répondent à des questions typées par des
//! probabilités, sans générer de texte), servis en local sur `http://127.0.0.1:11435`.
//!
//! La gestion des modèles suit l'API d'Ollama (`/api/tags`, `/api/pull`) : on réutilise ce client.
//! Les décisions passent par `POST /api/decide` : un `state` (le projet) et des `questions`
//! (`noul` = oui/non, `choice`, `score`), qui reviennent dans `answers`.

use serde_json::{Value, json};

use crate::ollama::{ModelInfo, Ollama};
use crate::ontology::{self, Concept, Ontology, QUESTIONS_PER_CALL};

/// Modèles conseillés pour classer des projets en français.
pub const SUGGESTED: [(&str, &str); 3] = [
    ("decider:2b", "≈ 3,8 Go · recommandé : juste en français, rapide"),
    ("decider:4b", "≈ 8,4 Go · le plus précis, plus lourd"),
    ("laya:multilingual", "≈ 0,7 Go · très léger, moins juste en français"),
];

pub struct Ollaya {
    inner: Ollama,
}

impl Ollaya {
    pub fn new(url: &str) -> Self {
        Self { inner: Ollama::new(url) }
    }

    /// Les modèles installés.
    pub fn models(&self) -> Result<Vec<ModelInfo>, String> {
        self.inner.models().map_err(|e| e.replace("Ollama", "Ollaya"))
    }

    /// Télécharge un modèle (`progress(statut, fraction 0–1)`).
    pub fn pull(&self, model: &str, progress: impl FnMut(&str, f32)) -> Result<(), String> {
        self.inner.pull(model, progress).map_err(|e| e.replace("Ollama", "Ollaya"))
    }

    /// Pose des questions sur un état ; renvoie l'objet `answers`. Le premier appel charge le
    /// modèle (une vingtaine de secondes).
    pub fn decide(&self, model: &str, state: &Value, questions: &Value) -> Result<Value, String> {
        match self.decide_once(model, state, questions, None) {
            // Mémoire graphique saturée : le moteur ONNX du modèle reste en erreur. On le décharge
            // (`keep_alive: 0`) et on réessaie une fois, ce qui le recharge proprement.
            Err(e) if e.contains("onnx runtime") || e.contains("INFERENCE_FAILED") => {
                let _ = self.decide_once(model, state, questions, Some(0));
                self.decide_once(model, state, questions, None)
            }
            r => r,
        }
    }

    fn decide_once(&self, model: &str, state: &Value, questions: &Value, keep_alive: Option<u32>) -> Result<Value, String> {
        let mut payload = json!({ "model": model, "state": state, "questions": questions });
        if let Some(k) = keep_alive {
            payload["keep_alive"] = json!(k);
        }
        let v = self.inner.post_json("/api/decide", &payload, 300).map_err(|e| e.replace("Ollama", "Ollaya"))?;
        if let Some(e) = v["error"].as_str() {
            return Err(format!("Ollaya : {e}"));
        }
        v.get("answers").cloned().ok_or_else(|| "Ollaya : réponse sans « answers »".into())
    }

    /// Classe un projet dans l'ontologie : (concept, probabilité) pour chaque concept.
    pub fn classify(&self, model: &str, onto: &Ontology, state: &Value) -> Result<Vec<(u64, f32)>, String> {
        let concepts: Vec<&Concept> = onto.concepts.iter().filter(|c| !c.name.trim().is_empty()).collect();
        let mut scores = Vec::new();
        for chunk in concepts.chunks(QUESTIONS_PER_CALL) {
            let answers = self.decide(model, state, &ontology::questions(onto, chunk))?;
            scores.extend(ontology::parse_answers(&answers));
        }
        Ok(scores)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ollama::tests::fake_server;

    #[test]
    fn decision_et_classement() {
        let mut o = Ontology::default();
        let f = o.add_facet("Domaine", "");
        let immo = o.add_concept(f, "Immobilier", "Logement, agences.");
        let jeux = o.add_concept(f, "Jeux", "Jeux vidéo.");
        let reply = json!({ "model": "decider:2b", "answers": {
            format!("c{immo}"): { "type": "noul", "noul": 0.93 },
            format!("c{jeux}"): { "type": "noul", "noul": 0.04 }
        }});
        let url = fake_server(vec![reply.to_string(), r#"{"error":"model not found","code":"NOT_FOUND"}"#.into()]);
        let ollaya = Ollaya::new(&url);
        let state = json!({ "nom": "ImmoFormation" });
        let mut scores = ollaya.classify("decider:2b", &o, &state).unwrap();
        scores.sort_by_key(|s| s.0);
        assert_eq!(scores, vec![(immo, 0.93), (jeux, 0.04)]);
        let err = ollaya.decide("inconnu", &state, &json!({})).unwrap_err();
        assert!(err.contains("model not found"), "{err}");
    }
}
