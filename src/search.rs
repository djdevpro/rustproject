//! La recherche : floue (tolère les fautes : « imo » trouve ImmoFormation) et sémantique
//! (embeddings Ollama : « logement » trouve ImmoFormation). Les deux se combinent.

use std::collections::HashMap;

use crate::model::Data;
use crate::store::SemanticIndex;

/// Minuscules et sans accents : « Éducation » → « education ».
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            'œ' => 'o',
            c => c,
        })
        .collect()
}

fn words(s: &str) -> Vec<String> {
    fold(s).split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_owned).collect()
}

fn is_subsequence(needle: &str, hay: &str) -> bool {
    let mut it = hay.chars();
    needle.chars().all(|c| it.any(|h| h == c))
}

/// Proximité d'un mot de la requête avec un mot du texte (0–1).
fn word_score(q: &str, w: &str) -> f32 {
    if w.contains(q) {
        return if w.starts_with(q) { 1.0 } else { 0.9 };
    }
    let qn = q.chars().count();
    if qn >= 3 && is_subsequence(q, w) && w.chars().next() == q.chars().next() {
        return 0.8;
    }
    if qn >= 4 {
        // Faute de frappe : comparer au début du mot de même longueur (« immofromation »).
        let prefix: String = w.chars().take(qn + 1).collect();
        let d = strsim::normalized_damerau_levenshtein(q, &prefix).max(strsim::normalized_damerau_levenshtein(q, w));
        if d >= 0.75 {
            return d as f32 * 0.85;
        }
    }
    0.0
}

/// Les petits mots qui ne cherchent rien (« formation de cours » = « formation cours »).
const STOPWORDS: &[&str] = &["et", "de", "des", "du", "la", "le", "les", "un", "une", "pour", "en", "a", "au", "aux", "d", "l", "sur", "avec", "the", "of", "and", "for"];

/// Les alternatives d'une requête : « formation ou immo » → [formation], [immo]. Dans chacune,
/// les petits mots sont ignorés (sauf si elle n'a que ça).
fn alternatives(query: &str) -> Vec<Vec<String>> {
    let mut out = vec![Vec::new()];
    for w in words(query) {
        if w == "ou" || w == "or" {
            out.push(Vec::new());
        } else if let Some(last) = out.last_mut() {
            last.push(w);
        }
    }
    out.into_iter()
        .filter(|a| !a.is_empty())
        .map(|a| {
            let meaningful: Vec<String> = a.iter().filter(|w| !STOPWORDS.contains(&w.as_str())).cloned().collect();
            if meaningful.is_empty() { a } else { meaningful }
        })
        .collect()
}

/// Score flou d'une alternative (ses mots) dans un texte : la moyenne, sur ses mots, du meilleur
/// mot du texte.
fn lexical_alt(q: &[String], text: &str) -> f32 {
    let t = words(text);
    let joined: String = t.concat();
    let total: f32 = q
        .iter()
        .map(|qw| {
            let best = t.iter().map(|w| word_score(qw, w)).fold(0.0, f32::max);
            // Mots collés (« immo formation » vs « immoformation ») : chercher dans le texte entier.
            if best < 0.9 && joined.contains(qw.as_str()) { 0.9 } else { best }
        })
        .sum();
    total / q.len().max(1) as f32
}

/// Score flou d'un texte pour une requête : la meilleure alternative (« … ou … »).
pub fn lexical(query: &str, text: &str) -> f32 {
    alternatives(query).iter().map(|q| lexical_alt(q, text)).fold(0.0, f32::max)
}

/// Les noms de dossiers d'un projet qui disent quelque chose : le dossier lui-même et son parent
/// (« ImmoFormation › site internet »), sauf si ce parent est un dossier source (« Projet perso »).
pub fn folder_context(data: &Data, p: &crate::model::Project) -> String {
    let roots: Vec<String> = data.config.roots.iter().map(|r| crate::model::path_key(&r.path)).collect();
    let mut parts: Vec<String> = Vec::new();
    for f in &p.folders {
        // Séparateurs `\` ou `/` : les chemins viennent de Windows comme de Linux.
        let f = f.trim();
        let mut names = Vec::new();
        if let Some(parent) = crate::model::parent_path(f)
            && !roots.contains(&crate::model::path_key(parent))
            && !crate::model::file_name(parent).is_empty()
        {
            names.push(crate::model::file_name(parent).to_owned());
        }
        if !crate::model::file_name(f).is_empty() {
            names.push(crate::model::file_name(f).to_owned());
        }
        let text = names.join(" › ");
        if !text.is_empty() && !parts.contains(&text) {
            parts.push(text);
        }
    }
    parts.join(" ; ")
}

/// Le texte d'un projet pour les embeddings : sa fiche, et les noms de ses dossiers
/// (« ImmoFormation › Qualiopi » dit ce que la fiche ne dit pas toujours).
pub fn embed_text(data: &Data, p: &crate::model::Project) -> String {
    let domain = data.domains.get(p.domain).map(|d| d.name.as_str()).unwrap_or("");
    format!("{}. Dossier : {}", p.search_text(domain), folder_context(data, p))
}

/// Le seuil sémantique d'une requête : chaque modèle a son échelle (qwen3 donne ≈ 0,3 entre deux
/// textes sans rapport, d'autres ≈ 0,5). On garde ce qui se détache nettement du lot : au-dessus
/// de la moyenne plus deux écarts-types des projets. Renvoie (seuil, meilleur score).
fn semantic_cutoff(scores: &[f32]) -> (f32, f32) {
    if scores.len() < 8 {
        return (SEMANTIC_MIN, 1.0);
    }
    let n = scores.len() as f32;
    let mean = scores.iter().sum::<f32>() / n;
    let sd = (scores.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / n).sqrt();
    let top = scores.iter().copied().fold(f32::MIN, f32::max);
    ((mean + 2.0 * sd).max(0.25), top)
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na.sqrt() * nb.sqrt()) }
}

/// Seuils : un résultat flou à partir de 0,6 ; un résultat sémantique à partir de 0,5 de
/// similarité (les modèles d'embeddings donnent rarement moins de 0,3 entre deux textes).
pub const LEXICAL_MIN: f32 = 0.6;
pub const SEMANTIC_MIN: f32 = 0.5;

/// Dans l'index sémantique, la clé d'un concept de l'ontologie : son identifiant avec ce bit
/// (les identifiants de projets n'y arrivent jamais).
pub const CONCEPT_KEY: u64 = 1 << 63;

/// Un résultat : index du projet, score, s'il ne vient que du sens (pas des mots), et le concept
/// de l'ontologie par lequel il a été trouvé, s'il y en a un (« via Immobilier »).
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub index: usize,
    pub score: f32,
    pub semantic_only: bool,
    pub via: Option<u64>,
}

/// Le score d'un concept pour la requête : par ses mots (nom, synonymes), ou par le sens.
/// Un score sémantique ramené à 0,2–0,9 : 0,2 au seuil, 0,9 pour le meilleur projet.
fn sem_scale(sem: f32, (cutoff, top): (f32, f32)) -> f32 {
    if sem < cutoff {
        return 0.0;
    }
    0.2 + 0.7 * ((sem - cutoff) / (top - cutoff).max(1e-3)).clamp(0.0, 1.0)
}

fn concept_score(data: &Data, id: u64, query: &str, query_vec: Option<&[f32]>, index: &SemanticIndex, cutoff: (f32, f32)) -> f32 {
    let Some(c) = data.ontology.concept(id) else { return 0.0 };
    let words = std::iter::once(c.name.as_str()).chain(c.synonyms.iter().map(String::as_str)).map(|w| lexical(query, w)).fold(0.0, f32::max);
    let lex = if words >= LEXICAL_MIN { words } else { 0.0 };
    let sem = query_vec.and_then(|q| index.vectors.get(&(CONCEPT_KEY | id)).map(|(_, v)| cosine(q, v))).unwrap_or(0.0);
    lex.max(sem_scale(sem, cutoff))
}

/// Classe les projets pour une requête : recherche hybride. Un projet ressort par ses mots
/// (flou, dossiers compris), par son sens (embeddings), ou par un concept de l'ontologie auquel il
/// est rattaché. `query_vec` : l'embedding de la requête, s'il est prêt.
pub fn rank(data: &Data, query: &str, query_vec: Option<&[f32]>, index: &SemanticIndex) -> Vec<Hit> {
    // Le sens : la similarité de chaque projet, et le seuil propre à cette requête.
    let sems: Vec<f32> = data.projects.iter().map(|p| query_vec.and_then(|q| index.vectors.get(&p.id).map(|(_, v)| cosine(q, v))).unwrap_or(0.0)).collect();
    let indexed: Vec<f32> = data.projects.iter().zip(&sems).filter(|(p, _)| index.vectors.contains_key(&p.id)).map(|(_, s)| *s).collect();
    let cutoff = if query_vec.is_some() { semantic_cutoff(&indexed) } else { (f32::MAX, 1.0) };
    // Les concepts qui répondent à la requête, une fois pour toutes.
    let concepts: Vec<(u64, f32)> = data.ontology.concepts.iter().map(|c| (c.id, concept_score(data, c.id, query, query_vec, index, cutoff))).filter(|(_, s)| *s > 0.0).collect();
    let alts = alternatives(query);
    let mut hits: Vec<Hit> = data
        .projects
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let domain = data.domains.get(p.domain).map(|d| d.name.as_str()).unwrap_or("");
            let text = format!("{} {} {} {} {} {}", p.name, p.description, p.kind, p.stack.join(" "), domain, p.outcome);
            // Le nom compte double ; les noms de dossiers (« ImmoFormation › site internet ») un peu moins.
            let folders = folder_context(data, p);
            let fields = [(p.name.as_str(), 1.0), (text.as_str(), 0.9), (folders.as_str(), 0.85)];
            let per_alt: Vec<f32> = alts.iter().map(|a| fields.iter().map(|(f, w)| lexical_alt(a, f) * w).fold(0.0, f32::max)).collect();
            // « formation ou immo » : un projet qui répond aux deux passe devant ceux qui n'ont que l'un.
            let matched = per_alt.iter().filter(|s| **s >= LEXICAL_MIN).count();
            let bonus = 0.04 * matched.saturating_sub(1) as f32;
            let lex = per_alt.iter().copied().fold(0.0, f32::max);
            let sem = sems[i];
            let sem_score = sem_scale(sem, cutoff);
            // Par l'ontologie : le meilleur concept rattaché, pondéré par la confiance du rattachement.
            let via = p
                .tags
                .iter()
                .filter(|t| t.weight >= crate::ontology::LINK_MIN)
                .filter_map(|t| concepts.iter().find(|(id, _)| *id == t.concept).map(|(id, s)| (*id, s * (0.6 + 0.35 * t.weight))))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            let onto = via.map(|v| v.1).unwrap_or(0.0);
            match (lex >= LEXICAL_MIN, sem_score > 0.0, onto > 0.0) {
                (true, _, _) => Some(Hit { index: i, score: lex.max(sem_score).max(onto) + bonus, semantic_only: false, via: None }),
                (false, s, true) if !s || onto >= sem_score => Some(Hit { index: i, score: onto, semantic_only: false, via: via.map(|v| v.0) }),
                (false, true, _) => Some(Hit { index: i, score: sem_score, semantic_only: true, via: None }),
                _ => None,
            }
        })
        .collect();
    // Ce qui est trouvé par les mots ou l'ontologie d'abord, puis ce qui ne l'est que par le sens.
    hits.sort_by(|a, b| a.semantic_only.cmp(&b.semantic_only).then(b.score.total_cmp(&a.score)));
    hits
}

/// Les embeddings de requêtes déjà calculés (pour ne pas réinterroger Ollama à chaque image).
#[derive(Default)]
pub struct QueryCache {
    pub vectors: HashMap<String, Vec<f32>>,
    pub pending: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fautes_et_accents() {
        assert!(lexical("imo", "ImmoFormation") >= LEXICAL_MIN, "« imo » ⊂ « immo »");
        assert!(lexical("immofromation", "ImmoFormation") >= LEXICAL_MIN, "lettres inversées");
        assert!(lexical("formation immo", "ImmoFormation · Plateforme de formation") >= LEXICAL_MIN);
        assert!(lexical("education", "Éducation") >= LEXICAL_MIN, "accents ignorés");
        assert!(lexical("tempo", "ImmoFormation") < LEXICAL_MIN);
        assert!(lexical("logement", "ImmoFormation · formation immobilier") < LEXICAL_MIN, "sans embeddings, pas de synonymes");
    }

    #[test]
    fn recherche_semantique() {
        let data = crate::model::demo();
        let immo = data.projects.iter().position(|p| p.name == "ImmoFormation").unwrap();
        let mut index = SemanticIndex { model: "test".into(), ..Default::default() };
        // Des vecteurs jouets : l'axe 0 = « immobilier/logement », l'axe 1 = « autre chose ».
        for p in &data.projects {
            let v = if p.name == "ImmoFormation" { vec![0.9, 0.1] } else { vec![0.1, 0.9] };
            index.vectors.insert(p.id, (0, v));
        }
        let query = [0.95, 0.05]; // « logement »
        let hits = rank(&data, "logement", Some(&query), &index);
        assert_eq!(hits.first().map(|h| h.index), Some(immo));
        assert!(hits[0].semantic_only);
        // Sans embedding de requête, « logement » ne trouve rien ; « imo » trouve quand même.
        assert!(rank(&data, "logement", None, &index).is_empty());
        assert_eq!(rank(&data, "imo", None, &index).first().map(|h| h.index), Some(immo));
    }

    /// Essai réel : réindexe une COPIE des données avec le vrai modèle d'embeddings, puis cherche.
    /// `ATELIER_ESSAI=chemin\copie\atelier.json cargo test essai_reel_recherche -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn essai_reel_recherche() {
        let path = std::env::var("ATELIER_ESSAI").expect("ATELIER_ESSAI : le chemin d'une copie de atelier.json");
        let data = crate::store::Store::open(path).load().unwrap().unwrap();
        let model = data.config.ollama_embed_model.clone();
        let ollama = crate::ollama::Ollama::new(&data.config.ollama_url);
        let mut index = SemanticIndex { model: model.clone(), ..Default::default() };
        let texts: Vec<(u64, String)> = data.projects.iter().map(|p| (p.id, embed_text(&data, p))).collect();
        for chunk in texts.chunks(16) {
            let v = ollama.embed(&model, &chunk.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>()).unwrap();
            for ((id, _), v) in chunk.iter().zip(v) {
                index.vectors.insert(*id, (0, v));
            }
        }
        for q in ["immo", "formation ou immo", "logement", "immobilier", "formation immobilière"] {
            let qv = ollama.embed(&model, &[crate::ollama::query_text(&model, q)]).unwrap().remove(0);
            let hits = rank(&data, q, Some(&qv), &index);
            let shown: Vec<String> = hits.iter().take(6).map(|h| format!("{}{} ({:.2})", data.projects[h.index].name, if h.semantic_only { " ≈" } else { "" }, h.score)).collect();
            println!("« {q} » : {} résultats → {}", hits.len(), shown.join(", "));
        }
    }

    #[test]
    fn ou_mots_vides_et_dossiers() {
        // « ou » : une alternative suffit ; les petits mots ne comptent pas.
        assert!(lexical("formation ou immo", "Qualiopi ImmoFormation") >= LEXICAL_MIN);
        assert!(lexical("tempo ou immo", "ImmoFormation") >= LEXICAL_MIN);
        assert!(lexical("formation de la qualité", "Formation · qualité") >= LEXICAL_MIN);
        assert!(lexical("tempo ou rust", "ImmoFormation") < LEXICAL_MIN);

        // Une fiche dont seul le dossier parle d'immobilier : « immo » la trouve.
        let mut data = crate::model::demo();
        data.config.roots = vec![crate::model::ScanRoot { path: r"C:\Projets".into(), enabled: true, depth: 2, found: 0, last_scan: 0 }];
        let mut p = crate::model::Project { name: "site internet".into(), description: "Le calendrier et les inscriptions.".into(), ..Default::default() };
        p.folders = vec![r"C:\Projets\ImmoFormation\site internet".into()];
        let i = data.add_project(p);
        assert_eq!(folder_context(&data, &data.projects[i]), "ImmoFormation › site internet");
        let mut top = crate::model::Project { name: "atelier".into(), ..Default::default() };
        top.folders = vec![r"C:\Projets\atelier".into()];
        let j = data.add_project(top);
        assert_eq!(folder_context(&data, &data.projects[j]), "atelier", "le dossier source n'est pas un indice");
        for q in ["immo", "formation ou immo", "ImmoFormation"] {
            let hits = rank(&data, q, None, &SemanticIndex::default());
            assert!(hits.iter().any(|h| h.index == i), "« {q} » doit trouver la fiche du dossier ImmoFormation");
        }
    }

    #[test]
    fn seuil_semantique_propre_a_la_requete() {
        // Un modèle « à la qwen3 » : tout tourne autour de 0,3, le bon projet à 0,45 ; ce qui se
        // détache passe, même sous l'ancien seuil fixe de 0,5.
        let mut scores = vec![0.30f32; 40];
        scores.extend([0.28, 0.33, 0.31, 0.29, 0.32]);
        scores.push(0.45);
        let (cutoff, top) = semantic_cutoff(&scores);
        assert!(cutoff < 0.45 && cutoff > 0.32, "seuil {cutoff}");
        assert!((top - 0.45).abs() < 1e-6);
        assert!(sem_scale(0.31, (cutoff, top)) == 0.0, "le lot ne passe pas");
        assert!(sem_scale(0.45, (cutoff, top)) > 0.8);
    }

    #[test]
    fn recherche_par_l_ontologie() {
        let mut data = crate::model::demo();
        let facet = data.ontology.facet_named(crate::ontology::DOMAIN_FACET).unwrap();
        let concept = data.ontology.add_concept(facet, "Immobilier", "Logement, agences, biens.");
        data.ontology.concepts.iter_mut().find(|c| c.id == concept).unwrap().synonyms = vec!["logement".into(), "appartement".into()];
        let immo = data.projects.iter().position(|p| p.name == "ImmoFormation").unwrap();
        data.projects[immo].tags.push(crate::ontology::Tag { concept, weight: 0.9, manual: false });
        // Sans embeddings : « logement » est un synonyme du concept auquel ImmoFormation est rattaché.
        let hits = rank(&data, "logement", None, &SemanticIndex::default());
        assert_eq!(hits.first().map(|h| (h.index, h.via)), Some((immo, Some(concept))));
        // Avec embeddings : « maison » rejoint le concept par le sens.
        let mut index = SemanticIndex { model: "test".into(), ..Default::default() };
        index.vectors.insert(CONCEPT_KEY | concept, (0, vec![0.9, 0.1]));
        let hits = rank(&data, "maison", Some(&[0.95, 0.05]), &index);
        assert_eq!(hits.first().map(|h| (h.index, h.via)), Some((immo, Some(concept))));
    }
}
