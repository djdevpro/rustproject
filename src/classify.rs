//! Le classement : décider si un dossier relevé par le scan est un projet, et en tirer une fiche
//! (nom, description en une phrase, domaine, type, technos).
//!
//! Deux façons : les règles locales (sans IA : marqueurs, manifestes, README), ou un modèle Ollama
//! à qui l'on envoie un résumé du dossier et qui répond en JSON. Un agent MCP, lui, crée les
//! fiches directement (voir `mcp.rs`).

use lucide_icons::Icon;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::model::{ActivityKind, AddedBy, Data, Project, ReadDepth, Source};
use crate::scan::Found;

/// Une fiche proposée pour un dossier (ou un dépôt).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Proposal {
    pub is_project: bool,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub kind: String,
    pub stack: Vec<String>,
    pub folder: Option<String>,
    /// Les dossiers de ses briques (frontend, api…), rattachés à la même fiche.
    #[serde(default)]
    pub extra_folders: Vec<String>,
    pub remote: Option<String>,
    pub idle_days: u32,
    pub key_files: Vec<String>,
    pub confidence: f32,
    /// Les mots-clés proposés par l'IA (vide avec les règles locales).
    #[serde(default)]
    pub keywords: Vec<String>,
}

// ---- règles locales ----------------------------------------------------------------------------

/// La première phrase « utile » d'un README : on saute titres, badges, images et listes.
pub fn first_sentence(text: &str) -> Option<String> {
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with('!') || l.starts_with('[') || l.starts_with('<') || l.starts_with("```") || l.starts_with('|') || l.starts_with('-') || l.starts_with('*') || l.starts_with('>') {
            continue;
        }
        let sentence = match l.find(". ") {
            Some(i) => &l[..=i],
            None => l,
        };
        let mut s: String = sentence.chars().take(140).collect();
        if sentence.chars().count() > 140 {
            s.push('…');
        }
        if s.chars().count() >= 12 {
            return Some(s);
        }
    }
    None
}

/// Un domaine d'après des mots-clés (nom, description, technos).
pub fn guess_domain(text: &str) -> &'static str {
    let t = crate::search::fold(text);
    let any = |words: &[&str]| words.iter().any(|w| t.contains(w));
    if any(&["formation", "cours", "learning", "quiz", "tuto", "lecon", "education"]) {
        "Formation"
    } else if any(&["mcp", "agent", "llm", "claude", "openai", "gpt", "ia ", " ia", "ai-", "-ai", "ollama", "rag", "embedding", "prompt"]) {
        "IA & agents"
    } else if any(&["3d", "three", "bevy", "voxel", "jeu", "game", "unity", "godot", "wgpu"]) {
        "3D & jeux"
    } else if any(&["audio", "voix", "voice", "tts", "whisper", "son", "music", "musique", "transcri"]) {
        "Audio & voix"
    } else if any(&["saas", "site", "web", "next", "laravel", "shop", "boutique", "facture", "api", "vue", "react", "astro", "e-commerce"]) {
        "Web & SaaS"
    } else if any(&["cv", "blog", "portfolio", "youtube", "perso", "notes", "trading"]) {
        "Perso & contenu"
    } else {
        "Outils de dev"
    }
}

/// Le type de projet d'après les technos.
fn guess_kind(stack: &[String], name: &str) -> String {
    let has = |s: &str| stack.iter().any(|x| x == s);
    let n = name.to_lowercase();
    if n.contains("mcp") {
        "Serveur MCP".into()
    } else if has("egui") || has("Tauri") || has("Electron") {
        "Application desktop".into()
    } else if has("Next.js") || has("Astro") || has("Laravel") || has("Vue") || has("React") || has("Svelte") {
        "Application web".into()
    } else if has("Bevy") {
        "Jeu".into()
    } else if has("Flutter") {
        "Application mobile".into()
    } else if has("Python") {
        "Outil Python".into()
    } else if has("Rust") || has("Go") {
        "Outil en ligne de commande".into()
    } else if !stack.is_empty() {
        "Projet de code".into()
    } else {
        "Dossier de projet".into()
    }
}

/// Une fiche sans IA, d'après ce que le scan a relevé.
pub fn by_rules(f: &Found) -> Proposal {
    let stack = f.manifest.stack.clone();
    let description = f
        .manifest
        .description
        .clone()
        .filter(|d| d.chars().count() >= 8)
        .or_else(|| f.readme.as_deref().and_then(first_sentence))
        .unwrap_or_else(|| if stack.is_empty() { format!("Projet « {} ».", f.name) } else { format!("Projet {} sans description.", stack.join(", ")) });
    let context = format!("{} {} {} {}", f.name, description, stack.join(" "), f.readme.as_deref().unwrap_or(""));
    Proposal {
        is_project: f.likely_project,
        name: f.name.clone(),
        description,
        domain: guess_domain(&context).into(),
        kind: guess_kind(&stack, &f.name),
        stack,
        folder: Some(f.path.clone()),
        extra_folders: f.bricks.clone(),
        remote: crate::scan::git_remote(std::path::Path::new(&f.path)),
        idle_days: f.idle_days,
        key_files: f.markers.clone(),
        // Un README seul est un indice faible.
        confidence: if f.strong || !f.bricks.is_empty() { 0.75 } else if f.likely_project { 0.5 } else { 0.3 },
        keywords: Vec::new(),
    }
}

// ---- Ollama ------------------------------------------------------------------------------------

pub const SYSTEM_PROMPT: &str = "Tu ranges les dossiers d'un développeur. Pour chaque dossier, tu décides s'il s'agit d'un \
projet (un logiciel, un site, un outil, une formation, un contenu sur lequel on travaille) ou seulement d'un dossier de \
rangement (qui regroupe d'autres projets), d'archives, de données ou de dépendances. Tu réponds uniquement en JSON, en \
français. La description tient en UNE phrase courte (15 mots au plus) qui dit ce qu'est le projet, sans répéter son nom. \
Attention aux BRIQUES : un dossier qui n'est qu'une partie d'un projet plus grand (frontend, backend, api, serveur, \
client, admin, docs, un paquet dans packages/ ou apps/, un module…) n'est PAS un projet : réponds est_projet = false. \
À l'inverse, le dossier qui regroupe ces briques EST le projet, même sans marqueur à sa racine. Les mots_cles : 8 à 15 \
mots ou courtes expressions en français qu'on taperait pour retrouver le projet (son sujet, son public, son usage, son \
secteur : « emploi », « entretien d'embauche », « candidats »), pas sa stack technique.";

/// La consigne pour proposer les mots-clés d'une fiche existante.
pub const KEYWORDS_PROMPT: &str = "Tu reçois la fiche d'un projet d'un développeur. Propose 8 à 15 mots-clés en français : \
les mots ou courtes expressions qu'on taperait pour retrouver ce projet (son sujet, son public, son usage, son secteur, \
ex. « emploi », « entretien d'embauche », « candidats », « immobilier »), pas sa stack technique ni son nom. Réponds \
uniquement en JSON : {\"mots_cles\": [...]}.";

/// Des mots-clés propres : sans espaces en trop, sans doublons (casse et accents ignorés), 20 au plus.
pub fn clean_keywords(words: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for w in words {
        let w = w.trim().trim_matches(|c: char| c == ',' || c == ';' || c == '.').trim().to_owned();
        if !w.is_empty() && w.chars().count() <= 60 && !out.iter().any(|x| crate::search::fold(x) == crate::search::fold(&w)) {
            out.push(w);
        }
    }
    out.truncate(20);
    out
}

/// Le schéma JSON imposé à la réponse (sorties structurées d'Ollama).
pub fn answer_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "est_projet": { "type": "boolean" },
            "nom": { "type": "string" },
            "description": { "type": "string" },
            "domaine": { "type": "string" },
            "type": { "type": "string" },
            "technos": { "type": "array", "items": { "type": "string" } },
            "mots_cles": { "type": "array", "items": { "type": "string" } },
            "confiance": { "type": "number" }
        },
        "required": ["est_projet", "nom", "description", "domaine", "type", "technos", "mots_cles", "confiance"]
    })
}

/// Ce qu'on dit du dossier au modèle, selon la profondeur de lecture autorisée.
pub fn prompt_for(f: &Found, domains: &[String], depth: ReadDepth) -> String {
    let mut s = format!("Dossier : {}\nChemin : {}\nNiveau sous le dossier source : {}\n", f.name, f.path, f.depth);
    s += &format!("Marqueurs trouvés : {}\n", if f.markers.is_empty() { "aucun".into() } else { f.markers.join(", ") });
    if depth != ReadDepth::Readme {
        if let Some(n) = &f.manifest.name {
            s += &format!("Nom dans le manifeste : {n}\n");
        }
        if let Some(d) = &f.manifest.description {
            s += &format!("Description dans le manifeste : {d}\n");
        }
        if !f.manifest.stack.is_empty() {
            s += &format!("Technos détectées : {}\n", f.manifest.stack.join(", "));
        }
    }
    if depth == ReadDepth::Tree || f.markers.is_empty() {
        s += &format!("Sous-dossiers : {}\nFichiers : {}\n", if f.children.is_empty() { "aucun".into() } else { f.children.join(", ") }, f.files);
    }
    s += &format!("Dernière activité : il y a {} jours\n", f.idle_days);
    if let Some(r) = &f.readme {
        let excerpt: String = r.chars().take(1200).collect();
        s += &format!("Début du README :\n---\n{excerpt}\n---\n");
    }
    s += &format!(
        "\nDomaines existants (réutilise-en un si possible, sinon propose un nom court) : {}.\n\
         Réponds avec : est_projet, nom (lisible), description (une phrase), domaine, type (ex. « Application web »), \
         technos (liste), confiance (0 à 1).",
        domains.join(", ")
    );
    s
}

/// Lit la réponse JSON du modèle (tolère du texte autour de l'objet).
pub fn parse_answer(text: &str, f: &Found) -> Result<Proposal, String> {
    let start = text.find('{').ok_or("réponse sans JSON")?;
    let end = text.rfind('}').ok_or("réponse sans JSON")?;
    let v: Value = serde_json::from_str(&text[start..=end]).map_err(|e| format!("JSON invalide : {e}"))?;
    let s = |k: &str| v[k].as_str().unwrap_or("").trim().to_owned();
    let mut description = s("description");
    // Une seule phrase, coûte que coûte.
    if let Some(i) = description.find(". ") {
        description.truncate(i + 1);
    }
    let name = Some(s("nom")).filter(|n| !n.is_empty()).unwrap_or_else(|| f.name.clone());
    let mut stack: Vec<String> = v["technos"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default();
    for t in &f.manifest.stack {
        if !stack.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            stack.push(t.clone());
        }
    }
    Ok(Proposal {
        is_project: v["est_projet"].as_bool().unwrap_or(false),
        name,
        description,
        domain: Some(s("domaine")).filter(|d| !d.is_empty()).unwrap_or_else(|| guess_domain(&f.name).into()),
        kind: s("type"),
        stack,
        folder: Some(f.path.clone()),
        extra_folders: f.bricks.clone(),
        remote: crate::scan::git_remote(std::path::Path::new(&f.path)),
        idle_days: f.idle_days,
        key_files: f.markers.clone(),
        confidence: v["confiance"].as_f64().unwrap_or(0.6) as f32,
        keywords: clean_keywords(v["mots_cles"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default()),
    })
}

// ---- création de la fiche ----------------------------------------------------------------------

/// Une icône d'après le type et les technos.
pub fn icon_for(kind: &str, stack: &[String], domain: &str) -> Icon {
    let k = kind.to_lowercase();
    let has = |s: &str| stack.iter().any(|x| x.eq_ignore_ascii_case(s));
    if k.contains("mcp") {
        Icon::Network
    } else if k.contains("desktop") {
        Icon::AppWindow
    } else if k.contains("mobile") {
        Icon::Smartphone
    } else if k.contains("jeu") || has("Bevy") {
        Icon::Gamepad2
    } else if k.contains("formation") || domain == "Formation" {
        Icon::GraduationCap
    } else if domain == "IA & agents" {
        Icon::Sparkles
    } else if domain == "Audio & voix" {
        Icon::AudioWaveform
    } else if domain == "3D & jeux" {
        Icon::Box
    } else if k.contains("web") || k.contains("site") || k.contains("saas") {
        Icon::Globe
    } else if k.contains("ligne de commande") || k.contains("cli") {
        Icon::Terminal
    } else {
        Icon::Code
    }
}

/// Crée la fiche d'un projet proposé, s'il n'est pas déjà suivi. Renvoie son index.
pub fn apply(data: &mut Data, p: &Proposal, by: AddedBy) -> Option<usize> {
    if let Some(folder) = &p.folder
        && data.find_by_folder(folder).is_some()
    {
        return None;
    }
    if p.folder.is_none()
        && let Some(remote) = &p.remote
        && data.projects.iter().any(|x| x.remote.as_deref() == Some(remote.as_str()))
    {
        return None;
    }
    let domain = data.domain_named(&p.domain);
    let domain_name = data.domains[domain].name.clone();
    let mut project = Project {
        name: p.name.clone(),
        icon: icon_for(&p.kind, &p.stack, &domain_name),
        description: p.description.clone(),
        domain,
        kind: p.kind.clone(),
        stack: p.stack.clone(),
        folders: p.folder.iter().chain(p.extra_folders.iter()).cloned().collect(),
        key_files: p.key_files.clone(),
        idle_days: p.idle_days,
        confidence: p.confidence,
        remote: p.remote.clone(),
        keywords: p.keywords.clone(),
        source: if p.folder.is_some() { Source::Local } else { Source::GitHub },
        added_by: by,
        // Les projets endormis depuis longtemps arrivent directement en réserve.
        attention: if p.idle_days > 120 { crate::model::Attention::Reserve } else { crate::model::Attention::Flow },
        status: if p.idle_days > 120 { crate::model::Status::Paused } else { crate::model::Status::Active },
        ..Default::default()
    };
    project.log(ActivityKind::Scan, format!("Fiche créée : {}", by.label()));
    Some(data.add_project(project))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::Manifest;

    fn found(name: &str, likely: bool) -> Found {
        Found {
            path: format!(r"C:\projets\{name}"),
            name: name.into(),
            root: 0,
            depth: 1,
            markers: if likely { vec!["package.json".into()] } else { vec![] },
            children: vec!["src".into()],
            files: 4,
            idle_days: 3,
            readme: Some("# ImmoFormation\n\n![badge](x)\n\nPlateforme de formation aux métiers de l'immobilier. Elle propose des modules.".into()),
            manifest: Manifest { name: None, description: None, stack: vec!["Laravel".into(), "PHP".into()] },
            likely_project: likely,
            strong: likely,
            parent: None,
            brick_of: None,
            bricks: Vec::new(),
        }
    }

    #[test]
    fn regles_premiere_phrase_et_domaine() {
        let p = by_rules(&found("ImmoFormation", true));
        assert!(p.is_project);
        assert_eq!(p.description, "Plateforme de formation aux métiers de l'immobilier.");
        assert_eq!(p.domain, "Formation");
        assert_eq!(p.kind, "Application web");
        assert!(!by_rules(&found("Archives", false)).is_project);
    }

    #[test]
    fn reponse_du_modele() {
        let f = found("tempo", true);
        let text = "Voici : {\"est_projet\": true, \"nom\": \"Tempo\", \"description\": \"Outil de diagnostic en ligne. Il fait aussi X.\", \"domaine\": \"Web & SaaS\", \"type\": \"Application web\", \"technos\": [\"Next.js\"], \"confiance\": 0.9}";
        let p = parse_answer(text, &f).unwrap();
        assert!(p.is_project);
        assert_eq!(p.name, "Tempo");
        assert_eq!(p.description, "Outil de diagnostic en ligne.", "une seule phrase");
        assert!(p.stack.contains(&"Next.js".to_owned()) && p.stack.contains(&"Laravel".to_owned()));
        assert!(parse_answer("pas de json", &f).is_err());
    }

    #[test]
    fn creation_sans_doublon() {
        let mut data = Data::fresh();
        let p = by_rules(&found("ImmoFormation", true));
        let i = apply(&mut data, &p, AddedBy::Rules).expect("créé");
        assert_eq!(data.projects[i].folders, vec![r"C:\projets\ImmoFormation".to_owned()]);
        assert_eq!(data.domains[data.projects[i].domain].name, "Formation");
        assert!(apply(&mut data, &p, AddedBy::Rules).is_none(), "déjà suivi");
    }
}
