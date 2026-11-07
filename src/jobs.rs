//! Les tâches de fond : scan, analyse par l'IA, modèles Ollama, téléchargement, GitHub,
//! embeddings. Chacune tourne dans son propre fil et envoie des `JobMsg` que l'interface lit à
//! chaque image : l'interface ne se fige jamais.

use std::sync::mpsc::Sender;

use crate::classify::{Proposal, SYSTEM_PROMPT, answer_schema, by_rules, parse_answer, prompt_for};
use crate::github::Repo;
use crate::model::{AddedBy, ReadDepth};
use crate::ollama::{ModelInfo, Ollama};
use crate::ollaya::Ollaya;
use crate::ontology::Ontology;
use crate::scan::{Found, ScanOptions, ScanResult, scan};

pub enum JobMsg {
    ScanProgress(usize),
    ScanDone(ScanResult),
    /// Une fiche proposée pour un dossier (`Err` : l'IA a échoué, la fiche vient des règles).
    Classified { proposal: Proposal, by: AddedBy, warning: Option<String> },
    AnalysisProgress { done: usize, total: usize, current: String },
    AnalysisDone,
    /// Analyse d'un seul dossier (fenêtre « Ajouter un projet »).
    OneAnalyzed(Result<(Proposal, AddedBy), String>),
    Models(Result<Vec<ModelInfo>, String>),
    PullProgress { status: String, fraction: f32 },
    PullDone(Result<String, String>),
    Github(Result<Vec<Repo>, String>),
    /// Les paquets de chaque projet, relus dans ses manifestes : (identifiant, paquets).
    Packages(Vec<(u64, Vec<crate::packages::Package>)>),
    /// Les mots-clés proposés par Ollama pour un projet.
    Keywords { id: u64, result: Result<Vec<String>, String> },
    KeywordsDone,
    /// Comptes connectés à gh, et le compte actif.
    GithubAccounts(Result<crate::github::Accounts, String>),
    /// Assistant « Dossiers sources » : projets probables d'un dossier proposé.
    SetupCount { path: String, depth: u32, likely: usize, error: Option<String> },
    Embedded { model: String, result: Result<Vec<(u64, u64, Vec<f32>)>, String> },
    QueryVec { query: String, result: Result<Vec<f32>, String> },
    // ---- Ollaya et ontologie
    OllayaModels(Result<Vec<ModelInfo>, String>),
    OllayaPullProgress { status: String, fraction: f32 },
    OllayaPullDone(Result<String, String>),
    TaggingProgress { done: usize, total: usize, current: String },
    /// Un projet classé : (concept, probabilité), avec l'empreinte de l'ontologie utilisée.
    Tagged { id: u64, fingerprint: u64, result: Result<Vec<(u64, f32)>, String> },
    TaggingDone,
    OntologyProposed { model: String, result: Result<Ontology, String> },
    /// Rust Project déclaré dans Claude Code pour tous les projets (ou pourquoi pas).
    ClaudeMcp(Result<(), String>),
}

/// De quoi envoyer des messages et réveiller l'interface.
#[derive(Clone)]
pub struct Bus {
    pub tx: Sender<JobMsg>,
    pub ctx: egui::Context,
}

impl Bus {
    fn send(&self, msg: JobMsg) {
        let _ = self.tx.send(msg);
        self.ctx.request_repaint();
    }
}

/// Comment classer : avec Ollama (url, modèle) ou avec les règles locales.
#[derive(Clone)]
pub struct Classifier {
    pub ollama: Option<(String, String)>,
    pub domains: Vec<String>,
    pub depth: ReadDepth,
}

impl Classifier {
    /// Classe un dossier ; si l'IA échoue, retombe sur les règles et le signale.
    pub fn classify(&self, f: &Found) -> (Proposal, AddedBy, Option<String>) {
        if let Some((url, model)) = &self.ollama {
            let prompt = prompt_for(f, &self.domains, self.depth);
            match Ollama::new(url).chat_json(model, SYSTEM_PROMPT, &prompt, &answer_schema()).and_then(|t| parse_answer(&t, f)) {
                Ok(p) => return (p, AddedBy::Ollama, None),
                Err(e) => return (by_rules(f), AddedBy::Rules, Some(format!("{} : {e}", f.name))),
            }
        }
        (by_rules(f), AddedBy::Rules, None)
    }
}

pub fn spawn_scan(bus: Bus, opts: ScanOptions) {
    std::thread::spawn(move || {
        let progress_bus = bus.clone();
        let result = scan(&opts, |n| progress_bus.send(JobMsg::ScanProgress(n)));
        bus.send(JobMsg::ScanDone(result));
    });
}

pub fn spawn_analysis(bus: Bus, candidates: Vec<Found>, classifier: Classifier) {
    std::thread::spawn(move || {
        let total = candidates.len();
        for (i, f) in candidates.iter().enumerate() {
            bus.send(JobMsg::AnalysisProgress { done: i, total, current: f.name.clone() });
            let (proposal, by, warning) = classifier.classify(f);
            bus.send(JobMsg::Classified { proposal, by, warning });
        }
        bus.send(JobMsg::AnalysisProgress { done: total, total, current: String::new() });
        bus.send(JobMsg::AnalysisDone);
    });
}

/// Analyse un dossier choisi à la main.
pub fn spawn_analyze_one(bus: Bus, path: String, opts: ScanOptions, classifier: Classifier) {
    std::thread::spawn(move || {
        let p = std::path::Path::new(path.trim());
        let result = (|| {
            if !p.is_dir() {
                return Err(format!("Dossier introuvable : {}", p.display()));
            }
            let parent = p.parent().ok_or("Ce dossier n'a pas de parent")?.to_string_lossy().into_owned();
            let scanned = scan(&ScanOptions { roots: vec![(0, parent, 1)], skip_nested: true, max_idle_days: None, ..opts }, |_| {});
            let key = crate::model::path_key(&p.to_string_lossy());
            let f = scanned.dirs.into_iter().find(|d| crate::model::path_key(&d.path) == key).ok_or("Dossier non lisible (exclu ou caché ?)")?;
            let (mut proposal, by, _) = classifier.classify(&f);
            // Choisi à la main : c'est un projet, même sans marqueur.
            proposal.is_project = true;
            Ok((proposal, by))
        })();
        bus.send(JobMsg::OneAnalyzed(result));
    });
}

pub fn spawn_models(bus: Bus, url: String) {
    std::thread::spawn(move || bus.send(JobMsg::Models(Ollama::new(&url).models())));
}

pub fn spawn_pull(bus: Bus, url: String, model: String) {
    std::thread::spawn(move || {
        let progress_bus = bus.clone();
        let mut last = std::time::Instant::now();
        let result = Ollama::new(&url).pull(&model, |status, fraction| {
            // Pas plus de 10 messages par seconde.
            if last.elapsed().as_millis() > 100 {
                last = std::time::Instant::now();
                progress_bus.send(JobMsg::PullProgress { status: status.to_owned(), fraction });
            }
        });
        bus.send(JobMsg::PullDone(result.map(|_| model)));
    });
}

/// Les dépôts du compte choisi (vide : le compte actif de gh), reliés aux fiches qui les contiennent.
pub fn spawn_github(bus: Bus, account: String, projects: Vec<(u64, Vec<String>)>) {
    std::thread::spawn(move || {
        let result = crate::github::list_repos(Some(&account)).map(|mut repos| {
            crate::github::link_local(&mut repos, &projects);
            repos
        });
        bus.send(JobMsg::Github(result));
    });
}

/// Fait proposer des mots-clés par le modèle de langage d'Ollama : (identifiant, texte de la fiche).
pub fn spawn_keywords(bus: Bus, url: String, model: String, projects: Vec<(u64, String)>) {
    std::thread::spawn(move || {
        let ollama = Ollama::new(&url);
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "mots_cles": { "type": "array", "items": { "type": "string" } } },
            "required": ["mots_cles"]
        });
        for (id, text) in projects {
            let result = ollama.chat_json(&model, crate::classify::KEYWORDS_PROMPT, &text, &schema).and_then(|answer| {
                let start = answer.find('{').ok_or("réponse sans JSON")?;
                let end = answer.rfind('}').ok_or("réponse sans JSON")?;
                let v: serde_json::Value = serde_json::from_str(&answer[start..=end]).map_err(|e| format!("JSON invalide : {e}"))?;
                Ok(crate::classify::clean_keywords(v["mots_cles"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default()))
            });
            let failed = result.is_err();
            bus.send(JobMsg::Keywords { id, result });
            if failed {
                break;
            }
        }
        bus.send(JobMsg::KeywordsDone);
    });
}

/// Relit les paquets des projets dans leurs manifestes (identifiant, dossiers).
pub fn spawn_packages(bus: Bus, projects: Vec<(u64, Vec<String>)>) {
    std::thread::spawn(move || {
        let out = projects.into_iter().map(|(id, folders)| (id, crate::packages::of_folders(&folders))).collect();
        bus.send(JobMsg::Packages(out));
    });
}

/// Les comptes connectés à gh, et l'actif.
/// Déclare Rust Project dans Claude Code, pour tous les projets.
pub fn spawn_claude_mcp(bus: Bus, exe: String, setup: crate::mcp_setup::ClaudeSetup) {
    std::thread::spawn(move || bus.send(JobMsg::ClaudeMcp(crate::mcp_setup::install(&exe, &setup))));
}

pub fn spawn_github_accounts(bus: Bus) {
    std::thread::spawn(move || bus.send(JobMsg::GithubAccounts(crate::github::accounts())));
}

/// Compte les projets probables d'un dossier proposé par l'assistant (même règles que le scan).
pub fn spawn_setup_count(bus: Bus, mut opts: ScanOptions, path: String, depth: u32) {
    std::thread::spawn(move || {
        opts.roots = vec![(0, path.clone(), depth)];
        let result = scan(&opts, |_| {});
        let likely = result.dirs.iter().filter(|d| d.likely_project && d.brick_of.is_none()).count();
        bus.send(JobMsg::SetupCount { path, depth, likely, error: result.errors.first().cloned() });
    });
}

/// Calcule les embeddings des projets dont le texte a changé : (id, empreinte, texte).
pub fn spawn_embed(bus: Bus, url: String, model: String, items: Vec<(u64, u64, String)>) {
    std::thread::spawn(move || {
        let ollama = Ollama::new(&url);
        let mut out = Vec::new();
        let mut error = None;
        for chunk in items.chunks(16) {
            let texts: Vec<String> = chunk.iter().map(|(_, _, t)| t.clone()).collect();
            match ollama.embed(&model, &texts) {
                Ok(vectors) => out.extend(chunk.iter().zip(vectors).map(|((id, hash, _), v)| (*id, *hash, v))),
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }
        let result = match error {
            Some(e) if out.is_empty() => Err(e),
            _ => Ok(out),
        };
        bus.send(JobMsg::Embedded { model, result });
    });
}

pub fn spawn_query_embed(bus: Bus, url: String, model: String, query: String) {
    std::thread::spawn(move || {
        let text = crate::ollama::query_text(&model, &query);
        let result = Ollama::new(&url).embed(&model, std::slice::from_ref(&text)).map(|mut v| v.remove(0));
        bus.send(JobMsg::QueryVec { query, result });
    });
}

// ---- Ollaya et ontologie -------------------------------------------------------------------------

pub fn spawn_ollaya_models(bus: Bus, url: String) {
    std::thread::spawn(move || bus.send(JobMsg::OllayaModels(Ollaya::new(&url).models())));
}

pub fn spawn_ollaya_pull(bus: Bus, url: String, model: String) {
    std::thread::spawn(move || {
        let progress_bus = bus.clone();
        let mut last = std::time::Instant::now();
        let result = Ollaya::new(&url).pull(&model, |status, fraction| {
            if last.elapsed().as_millis() > 100 {
                last = std::time::Instant::now();
                progress_bus.send(JobMsg::OllayaPullProgress { status: status.to_owned(), fraction });
            }
        });
        bus.send(JobMsg::OllayaPullDone(result.map(|_| model)));
    });
}

/// Pilotage d'un classement en cours : il tourne, il est en pause, ou il doit s'arrêter.
pub const RUN: u8 = 0;
pub const PAUSE: u8 = 1;
pub const STOP: u8 = 2;

/// Classe des projets dans l'ontologie avec Ollaya : (identifiant, nom, état du projet).
/// `control` : `RUN`, `PAUSE` (attend entre deux projets) ou `STOP`.
pub fn spawn_tagging(bus: Bus, url: String, model: String, onto: Ontology, projects: Vec<(u64, String, serde_json::Value)>, control: std::sync::Arc<std::sync::atomic::AtomicU8>) {
    use std::sync::atomic::Ordering;
    std::thread::spawn(move || {
        let ollaya = Ollaya::new(&url);
        let fingerprint = onto.fingerprint();
        let total = projects.len();
        for (i, (id, name, state)) in projects.into_iter().enumerate() {
            while control.load(Ordering::Relaxed) == PAUSE {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            if control.load(Ordering::Relaxed) == STOP {
                break;
            }
            bus.send(JobMsg::TaggingProgress { done: i, total, current: name });
            let result = ollaya.classify(&model, &onto, &state);
            let stop = result.is_err();
            bus.send(JobMsg::Tagged { id, fingerprint, result });
            if stop {
                break;
            }
        }
        bus.send(JobMsg::TaggingDone);
    });
}

/// Demande une ontologie à un modèle de langage, qui voit tous les projets d'un coup.
pub fn spawn_generate_ontology(bus: Bus, url: String, model: String, prompt: String) {
    std::thread::spawn(move || {
        let result = Ollama::new(&url)
            .chat_json(&model, crate::ontology::GENERATION_SYSTEM, &prompt, &crate::ontology::generation_schema())
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).map_err(|e| format!("Réponse illisible : {e}")))
            .and_then(|v| crate::ontology::from_json(&v));
        bus.send(JobMsg::OntologyProposed { model, result });
    });
}
