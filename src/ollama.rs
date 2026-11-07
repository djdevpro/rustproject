//! Le client Ollama (API HTTP locale, `http://localhost:11434` par défaut) : liste des modèles
//! (l'équivalent de `ollama list`), réponses JSON structurées, embeddings et téléchargement.

use std::io::BufRead;
use std::time::Duration;

use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub family: String,
    pub parameters: String,
}

impl ModelInfo {
    /// Un modèle d'embeddings (d'après son nom ou sa famille) : à proposer pour la recherche.
    pub fn is_embedding(&self) -> bool {
        let n = self.name.to_lowercase();
        let f = self.family.to_lowercase();
        n.contains("embed") || n.contains("bge") || n.contains("minilm") || n.contains("e5") || f.contains("bert")
    }

    pub fn size_label(&self) -> String {
        let gb = self.size as f64 / 1e9;
        if gb >= 1.0 { format!("{gb:.1} Go") } else { format!("{:.0} Mo", self.size as f64 / 1e6) }
    }
}

/// Modèles conseillés, à télécharger depuis la configuration.
pub const SUGGESTED_CHAT: [(&str, &str); 3] = [
    ("qwen2.5:3b", "≈ 1,9 Go · rapide, bon en français et en JSON"),
    ("llama3.2:3b", "≈ 2,0 Go · léger et polyvalent"),
    ("qwen2.5:7b", "≈ 4,7 Go · plus fin, plus lent"),
];
pub const SUGGESTED_EMBED: [(&str, &str); 3] = [
    ("qwen3-embedding:4b", "≈ 2,5 Go · le meilleur en français pour sa taille, recommandé"),
    ("embeddinggemma", "≈ 0,6 Go · le meilleur petit modèle en français"),
    ("bge-m3", "≈ 1,2 Go · multilingue, un cran en dessous"),
];

/// Contexte des embeddings, en jetons : une fiche ou une requête y tient largement.
pub const EMBED_CTX: u32 = 2048;

/// La requête telle que le modèle d'embeddings l'attend : Qwen3 et EmbeddingGemma veulent une
/// consigne devant la requête (pas devant les fiches), sinon ils perdent quelques points.
pub fn query_text(model: &str, query: &str) -> String {
    let m = model.to_lowercase();
    if m.contains("qwen3-embedding") {
        format!("Instruct: Given a search query, retrieve relevant personal project descriptions\nQuery: {query}")
    } else if m.contains("embeddinggemma") {
        format!("task: search result | query: {query}")
    } else {
        query.to_owned()
    }
}

pub struct Ollama {
    base: String,
}

fn agent(timeout: Option<Duration>) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(timeout).http_status_as_error(false).build().into()
}

/// Le message d'erreur d'une réponse non 2xx (Ollama renvoie `{"error": "…"}`).
fn status_error(status: u16, body: &str) -> String {
    let msg = serde_json::from_str::<Value>(body).ok().and_then(|v| v["error"].as_str().map(str::to_owned)).unwrap_or_else(|| body.chars().take(200).collect());
    format!("Ollama a répondu {status} : {msg}")
}

impl Ollama {
    pub fn new(url: &str) -> Self {
        Self { base: url.trim().trim_end_matches('/').to_owned() }
    }

    fn get(&self, path: &str, timeout: u64) -> Result<Value, String> {
        let mut resp = agent(Some(Duration::from_secs(timeout))).get(format!("{}{path}", self.base)).call().map_err(|e| format!("Ollama injoignable ({}) : {e}", self.base))?;
        let status = resp.status().as_u16();
        let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
        if status >= 300 {
            return Err(status_error(status, &body));
        }
        serde_json::from_str(&body).map_err(|e| format!("Réponse illisible : {e}"))
    }

    /// Un `POST` JSON (aussi pour les serveurs à l'API proche, comme Ollaya).
    pub fn post_json(&self, path: &str, payload: &Value, timeout: u64) -> Result<Value, String> {
        self.post(path, payload, timeout)
    }

    fn post(&self, path: &str, payload: &Value, timeout: u64) -> Result<Value, String> {
        let mut resp = agent(Some(Duration::from_secs(timeout))).post(format!("{}{path}", self.base)).send_json(payload).map_err(|e| format!("Ollama injoignable ({}) : {e}", self.base))?;
        let status = resp.status().as_u16();
        let body = resp.body_mut().with_config().limit(64 * 1024 * 1024).read_to_string().map_err(|e| e.to_string())?;
        if status >= 300 {
            return Err(status_error(status, &body));
        }
        serde_json::from_str(&body).map_err(|e| format!("Réponse illisible : {e}"))
    }

    /// Les modèles installés (`GET /api/tags`).
    pub fn models(&self) -> Result<Vec<ModelInfo>, String> {
        let v = self.get("/api/tags", 5)?;
        let mut out: Vec<ModelInfo> = v["models"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|m| ModelInfo {
                        name: m["name"].as_str().unwrap_or("").to_owned(),
                        size: m["size"].as_u64().unwrap_or(0),
                        family: m["details"]["family"].as_str().unwrap_or("").to_owned(),
                        parameters: m["details"]["parameter_size"].as_str().unwrap_or("").to_owned(),
                    })
                    .filter(|m| !m.name.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Une réponse JSON contrainte par `schema` (`POST /api/chat`, sorties structurées).
    pub fn chat_json(&self, model: &str, system: &str, user: &str, schema: &Value) -> Result<String, String> {
        let payload = json!({
            "model": model,
            "messages": [{ "role": "system", "content": system }, { "role": "user", "content": user }],
            "stream": false,
            "format": schema,
            "options": { "temperature": 0.2 }
        });
        let v = self.post("/api/chat", &payload, 180)?;
        v["message"]["content"].as_str().map(str::to_owned).ok_or_else(|| "réponse vide".into())
    }

    /// Les vecteurs d'embeddings de plusieurs textes (`POST /api/embed`).
    pub fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, String> {
        // Les fiches sont courtes : un petit contexte suffit. Sans lui, Ollama prend celui du modèle
        // (40 960 jetons pour qwen3-embedding) et réserve ~10 Go de mémoire graphique au lieu de ~3.
        let payload = json!({ "model": model, "input": inputs, "truncate": true, "options": { "num_ctx": EMBED_CTX } });
        let v = self.post("/api/embed", &payload, 120)?;
        let vectors: Vec<Vec<f32>> = v["embeddings"]
            .as_array()
            .ok_or("réponse sans embeddings")?
            .iter()
            .map(|e| e.as_array().map(|x| x.iter().map(|f| f.as_f64().unwrap_or(0.0) as f32).collect()).unwrap_or_default())
            .collect();
        if vectors.len() != inputs.len() {
            return Err(format!("{} vecteurs reçus pour {} textes", vectors.len(), inputs.len()));
        }
        Ok(vectors)
    }

    /// Télécharge un modèle (`POST /api/pull`, en flux) ; `progress(statut, fraction 0–1)`.
    pub fn pull(&self, model: &str, mut progress: impl FnMut(&str, f32)) -> Result<(), String> {
        let mut resp = agent(None).post(format!("{}/api/pull", self.base)).send_json(json!({ "model": model, "stream": true })).map_err(|e| format!("Ollama injoignable : {e}"))?;
        let status = resp.status().as_u16();
        if status >= 300 {
            let body = resp.body_mut().read_to_string().unwrap_or_default();
            return Err(status_error(status, &body));
        }
        let reader = std::io::BufReader::new(resp.body_mut().as_reader());
        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
            if let Some(err) = v["error"].as_str() {
                return Err(err.to_owned());
            }
            let status = v["status"].as_str().unwrap_or("");
            let fraction = match (v["completed"].as_f64(), v["total"].as_f64()) {
                (Some(c), Some(t)) if t > 0.0 => (c / t) as f32,
                _ => -1.0,
            };
            progress(status, fraction);
            if status == "success" {
                return Ok(());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use super::*;

    /// Un faux serveur Ollama : répond `body` (JSON) à chaque requête, pendant `requests` requêtes.
    pub fn fake_server(responses: Vec<String>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for body in responses {
                let Ok((mut stream, _)) = listener.accept() else { return };
                // Lire toute la requête (en-têtes, puis le corps annoncé) avant de répondre.
                let mut req = Vec::new();
                let mut buf = [0u8; 8192];
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    req.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&req);
                    if let Some(end) = text.find("\r\n\r\n") {
                        let length = text[..end]
                            .lines()
                            .find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                            .unwrap_or(0);
                        if req.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                let reply = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        format!("http://{addr}")
    }

    #[test]
    fn liste_des_modeles() {
        let url = fake_server(vec![
            r#"{"models":[{"name":"qwen2.5:3b","size":1900000000,"details":{"family":"qwen2","parameter_size":"3.1B"}},{"name":"bge-m3:latest","size":1200000000,"details":{"family":"bert","parameter_size":"567M"}}]}"#.into(),
        ]);
        let models = Ollama::new(&url).models().unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "bge-m3:latest");
        assert!(models[0].is_embedding() && !models[1].is_embedding());
        assert_eq!(models[1].size_label(), "1.9 Go");
    }

    #[test]
    fn chat_et_embeddings() {
        let url = fake_server(vec![
            r#"{"message":{"role":"assistant","content":"{\"est_projet\":true}"}}"#.into(),
            r#"{"embeddings":[[0.1,0.2],[0.3,0.4]]}"#.into(),
        ]);
        let o = Ollama::new(&url);
        assert_eq!(o.chat_json("m", "s", "u", &json!({})).unwrap(), "{\"est_projet\":true}");
        let v = o.embed("e", &["a".into(), "b".into()]).unwrap();
        assert_eq!(v, vec![vec![0.1, 0.2], vec![0.3, 0.4]]);
    }

    #[test]
    fn serveur_injoignable() {
        let err = Ollama::new("http://127.0.0.1:9").models().unwrap_err();
        assert!(err.contains("injoignable"), "{err}");
    }
}
