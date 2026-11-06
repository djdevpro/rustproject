//! Le modèle : projets, taxonomie, lanceurs et configuration. Tout est sérialisable : c'est le
//! contenu du fichier `rust-project.json` (voir `store.rs`). Le jeu de démonstration sert aux tests.

use std::time::{SystemTime, UNIX_EPOCH};

use egui::Color32;
use lucide_icons::Icon;
use serde::{Deserialize, Serialize};

use crate::theme::{Accent, ThemeChoice, rgb};

// ---- temps -------------------------------------------------------------------------------------

/// Maintenant, en secondes Unix.
pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// « à l'instant », « il y a 5 min », « hier », « il y a 3 mois »… (`0` = « jamais »).
pub fn ago(ts: u64) -> String {
    use crate::i18n::{tf, tr};
    if ts == 0 {
        return tr("jamais").into();
    }
    let s = now().saturating_sub(ts);
    match s {
        0..60 => tr("à l'instant").into(),
        60..3600 => tf("il y a {0} min", &[("0", &(s / 60))]),
        3600..86_400 => tf("il y a {0} h", &[("0", &(s / 3600))]),
        86_400..172_800 => tr("hier").into(),
        _ if s < 60 * 86_400 => tf("il y a {d} j", &[("d", &(s / 86_400))]),
        _ => tf("il y a {0} mois", &[("0", &(s / (30 * 86_400)))]),
    }
}

// ---- état et attention -------------------------------------------------------------------------

/// Où en est le projet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Idea,
    Active,
    Paused,
    Done,
    Archived,
}

impl Status {
    pub const ALL: [Self; 5] = [Self::Idea, Self::Active, Self::Paused, Self::Done, Self::Archived];
    pub fn label(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::Idea => "Idée",
            Self::Active => "En cours",
            Self::Paused => "En pause",
            Self::Done => "Terminé",
            Self::Archived => "Archivé",
        })
    }
    /// Pour le serveur MCP : un état à partir de son nom (« en cours », « active »…).
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();
        Self::ALL.into_iter().find(|v| v.label().to_lowercase() == s || format!("{v:?}").to_lowercase() == s)
    }
}

/// Combien d'attention on lui donne en ce moment : c'est ce qui range le bureau.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Attention {
    /// Les projets du moment.
    Focus,
    /// Avance quand il y a de la place.
    Flow,
    /// Au repos : il ressortira par la revue « À revisiter ».
    Reserve,
}

impl Attention {
    pub const ALL: [Self; 3] = [Self::Focus, Self::Flow, Self::Reserve];
    pub fn label(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::Focus => "Focus",
            Self::Flow => "Au fil de l'eau",
            Self::Reserve => "En réserve",
        })
    }
    pub fn icon(self) -> Icon {
        match self {
            Self::Focus => Icon::Crosshair,
            Self::Flow => Icon::Waves,
            Self::Reserve => Icon::Archive,
        }
    }
    pub fn description(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::Focus => "Les projets du moment, en tête du bureau.",
            Self::Flow => "Ils avancent quand il reste de la place, sans pression.",
            Self::Reserve => "Au repos. Un projet par jour ressort dans « À revisiter ».",
        })
    }
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();
        Self::ALL.into_iter().find(|v| v.label().to_lowercase() == s || format!("{v:?}").to_lowercase() == s)
    }
}

// ---- projets -----------------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Step {
    pub text: String,
    pub done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityKind {
    Commit,
    Ai,
    Edit,
    Scan,
    Note,
}

impl ActivityKind {
    pub fn icon(self) -> Icon {
        match self {
            Self::Commit => Icon::GitCommitHorizontal,
            Self::Ai => Icon::Sparkles,
            Self::Edit => Icon::Pencil,
            Self::Scan => Icon::FolderSearch,
            Self::Note => Icon::NotebookPen,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Activity {
    /// Horodatage Unix.
    pub at: u64,
    pub kind: ActivityKind,
    pub text: String,
}

/// D'où vient le projet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    /// Trouvé dans un dossier source.
    #[default]
    Local,
    /// Un dépôt GitHub sans dossier local connu.
    GitHub,
    /// Ajouté à la main.
    Manual,
}

/// Qui a créé la fiche.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddedBy {
    #[default]
    Manual,
    /// Règles locales (marqueurs, README), sans IA.
    Rules,
    Ollama,
    Mcp,
    Demo,
}

impl AddedBy {
    pub fn label(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::Manual => "ajouté à la main",
            Self::Rules => "reconnu par les règles locales",
            Self::Ollama => "classé par Ollama",
            Self::Mcp => "ajouté par l'agent IA (MCP)",
            Self::Demo => "démonstration",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    /// Identifiant stable (le serveur MCP s'en sert).
    pub id: u64,
    pub name: String,
    pub icon: Icon,
    /// Une phrase courte qui dit ce qu'est le projet.
    pub description: String,
    /// « Résultat visé » : ce qu'on veut avoir entre les mains.
    pub outcome: String,
    pub objective: String,
    pub last_point: String,
    pub next_action: String,
    pub steps: Vec<Step>,
    pub status: Status,
    pub attention: Attention,
    /// Index dans `Data::domains`.
    pub domain: usize,
    /// Type de projet (« Application desktop », « SaaS »…).
    pub kind: String,
    pub stack: Vec<String>,
    pub folders: Vec<String>,
    pub links: Vec<(String, String)>,
    pub notes: Vec<String>,
    pub key_files: Vec<String>,
    /// Lanceurs propres à ce projet (index dans `Data::launchers`), en plus de ceux par défaut.
    pub launchers: Vec<usize>,
    /// Lanceurs par défaut masqués pour ce projet.
    pub hidden: Vec<usize>,
    /// Le lanceur mis en avant.
    pub favorite: Option<usize>,
    /// Jours depuis la dernière modification relevée dans son dossier.
    pub idle_days: u32,
    pub review: String,
    /// Confiance du classement (0–1).
    pub confidence: f32,
    /// Position parmi les projets en focus.
    pub order: i32,
    /// Journal, du plus récent au plus ancien.
    pub activity: Vec<Activity>,
    pub source: Source,
    /// Adresse du dépôt distant, si on la connaît.
    pub remote: Option<String>,
    pub added_by: AddedBy,
    /// Les concepts de l'ontologie auxquels il est rattaché (voir `ontology.rs`).
    pub tags: Vec<crate::ontology::Tag>,
    /// L'empreinte de l'ontologie lors du dernier classement par Ollaya (0 = jamais classé).
    pub tags_hash: u64,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            icon: Icon::Folder,
            description: String::new(),
            outcome: String::new(),
            objective: String::new(),
            last_point: String::new(),
            next_action: String::new(),
            steps: Vec::new(),
            status: Status::Active,
            attention: Attention::Flow,
            domain: 0,
            kind: String::new(),
            stack: Vec::new(),
            folders: Vec::new(),
            links: Vec::new(),
            notes: Vec::new(),
            key_files: Vec::new(),
            launchers: Vec::new(),
            hidden: Vec::new(),
            favorite: None,
            idle_days: 0,
            review: "Chaque semaine".into(),
            confidence: 1.0,
            order: 0,
            activity: Vec::new(),
            source: Source::Local,
            remote: None,
            added_by: AddedBy::Manual,
            tags: Vec::new(),
            tags_hash: 0,
        }
    }
}

impl Project {
    pub fn steps_done(&self) -> (usize, usize) {
        (self.steps.iter().filter(|s| s.done).count(), self.steps.len())
    }

    /// Ajoute une ligne au journal (les 50 dernières sont gardées).
    pub fn log(&mut self, kind: ActivityKind, text: impl Into<String>) {
        self.activity.insert(0, Activity { at: now(), kind, text: text.into() });
        self.activity.truncate(50);
    }

    /// Le texte qui représente le projet pour la recherche sémantique.
    pub fn search_text(&self, domain: &str) -> String {
        format!("{}. {}. {}. {}. {}. {}", self.name, self.description, self.kind, domain, self.stack.join(", "), self.outcome)
    }
}

/// Un domaine de la taxonomie (« Formation », « IA & agents »…).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Domain {
    pub name: String,
    pub icon: Icon,
    pub color: Color32,
}

// ---- lanceurs ----------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LauncherKind {
    App,
    Command,
    Url,
    Folder,
}

impl LauncherKind {
    pub const ALL: [Self; 4] = [Self::App, Self::Command, Self::Url, Self::Folder];
    pub fn label(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::App => "Application",
            Self::Command => "Commande",
            Self::Url => "Lien",
            Self::Folder => "Dossier",
        })
    }
    pub fn icon(self) -> Icon {
        match self {
            Self::App => Icon::AppWindow,
            Self::Command => Icon::SquareTerminal,
            Self::Url => Icon::Globe,
            Self::Folder => Icon::FolderOpen,
        }
    }
}

/// Familles de la galerie de lanceurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Group {
    Terminal,
    Editor,
    Ai,
    Web,
    Git,
    Tools,
}

impl Group {
    pub const ALL: [Self; 6] = [Self::Terminal, Self::Editor, Self::Ai, Self::Web, Self::Git, Self::Tools];
    pub fn label(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::Terminal => "Terminaux",
            Self::Editor => "Éditeurs",
            Self::Ai => "IA",
            Self::Web => "Navigateur",
            Self::Git => "Git",
            Self::Tools => "Outils",
        })
    }
}

/// Un bouton « ouvrir avec » : une application, une commande, un lien ou un dossier.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Launcher {
    pub name: String,
    pub icon: Icon,
    pub color: Color32,
    pub kind: LauncherKind,
    /// Modèle de commande, avec des variables : `{dossier}`, `{nom}`, `{remote}`, `{port}`.
    pub command: String,
    pub in_terminal: bool,
    pub shortcut: String,
    pub description: String,
    pub group: Group,
}

/// Les variables utilisables dans une commande, avec leur explication.
pub const VARIABLES: [(&str, &str); 5] = [
    ("{dossier}", "Chemin du dossier principal du projet"),
    ("{dossier_url}", "Le même chemin, encodé pour un lien (claude://…)"),
    ("{nom}", "Nom du projet"),
    ("{remote}", "URL du dépôt Git distant (détectée)"),
    ("{port}", "Port de dev (3000 par défaut)"),
];

impl Launcher {
    /// La commande telle qu'elle partirait pour ce projet.
    pub fn resolve(&self, project: &Project) -> String {
        let folder = project.folders.first().map(String::as_str).unwrap_or("");
        let remote = project.remote.clone().unwrap_or_default();
        self.command
            .replace("{dossier_url}", &url_encode(folder))
            .replace("{dossier}", folder)
            .replace("{nom}", &project.name)
            .replace("{remote}", &remote)
            .replace("{port}", "3000")
    }
}

/// Encodage d'URL : tout sauf les lettres, chiffres et `-_.~` devient `%XX`.
pub fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

// ---- configuration -----------------------------------------------------------------------------

/// Un dossier où se trouvent des projets.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanRoot {
    pub path: String,
    pub enabled: bool,
    /// Profondeur explorée : 1 à 3 niveaux de dossiers.
    pub depth: u32,
    /// Dossiers relevés au dernier scan.
    pub found: usize,
    /// Horodatage du dernier scan (0 = jamais).
    pub last_scan: u64,
}

impl Default for ScanRoot {
    fn default() -> Self {
        Self { path: String::new(), enabled: true, depth: 3, found: 0, last_scan: 0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanFreq {
    Startup,
    Hourly,
    Manual,
}

/// Comment l'IA classe les dossiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiMode {
    /// Rust Project interroge un serveur Ollama (API HTTP).
    Ollama,
    /// Un agent externe (Claude Code…) pilote Rust Project par le protocole MCP.
    Mcp,
    /// Pas d'IA : règles locales seulement.
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadDepth {
    Readme,
    Manifests,
    Tree,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub roots: Vec<ScanRoot>,
    pub excludes: Vec<String>,
    pub markers: Vec<(String, bool)>,
    pub freq: ScanFreq,
    pub skip_nested: bool,
    pub group_monorepos: bool,
    pub ignore_older: bool,
    pub ignore_older_days: u32,
    /// Créer automatiquement les projets reconnus après un scan ou une analyse.
    pub auto_add: bool,
    pub github_enabled: bool,
    /// Le compte GitHub dont on liste les dépôts (vide : le compte actif de la CLI gh).
    pub github_account: String,
    pub ai_mode: AiMode,
    pub ollama_url: String,
    /// Modèle de langage (classement, descriptions).
    pub ollama_model: String,
    /// Modèle d'embeddings (recherche sémantique).
    pub ollama_embed_model: String,
    /// Le serveur Ollaya (modèles de décision) et le modèle qui classe les projets dans l'ontologie.
    pub ollaya_url: String,
    pub ollaya_model: String,
    pub read_depth: ReadDepth,
    pub limit_focus: bool,
    pub focus_limit: usize,
    pub auto_reserve: bool,
    pub reserve_after: u32,
    pub reviews_per_day: usize,
    pub theme: ThemeChoice,
    pub accent: Accent,
    pub compact: bool,
    pub zoom: f32,
    pub demo_data: bool,
    pub update_auto: bool,
    pub update_beta: bool,
    /// Langue de l'interface (`None` : celle du système).
    pub language: Option<crate::i18n::Lang>,
    /// Version de la liste de marqueurs : les anciens fichiers reçoivent les nouveaux marqueurs.
    /// (Absent d'un ancien fichier → 0, d'où le `default` propre au champ.)
    #[serde(default = "zero")]
    pub markers_version: u32,
    /// Dossiers écartés à la main ou par l'agent (copies, sauvegardes…) : le scan ne les relève plus.
    pub ignored: Vec<String>,
}

fn zero() -> u32 {
    0
}

/// La version actuelle de la liste de marqueurs par défaut.
pub const MARKERS_VERSION: u32 = 2;

/// Les marqueurs par défaut : tous cochés.
pub fn default_markers() -> Vec<(String, bool)> {
    [".git", "Cargo.toml", "package.json", "pyproject.toml", "requirements.txt", "composer.json", "go.mod", "*.sln", "CLAUDE.md", "AGENTS.md", "AGENT.md", "README.md"]
        .map(|m| (m.to_string(), true))
        .to_vec()
}

/// Un dossier de départ pour le sélecteur : le dossier Documents (ou le dossier personnel).
/// (Ce n'est qu'un point de départ : aucun dossier source n'est ajouté sans l'utilisateur.)
pub fn default_root() -> String {
    let home = home_dir();
    let docs = std::path::Path::new(&home).join("Documents");
    if docs.is_dir() { docs.display().to_string() } else { home }
}

/// Le dossier personnel (`USERPROFILE` sous Windows, `HOME` ailleurs).
pub fn home_dir() -> String {
    std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_else(|_| if cfg!(windows) { r"C:\Users\Public".into() } else { "/".into() })
}

/// Les dossiers où l'on range souvent ses projets, parmi ceux qui existent : proposés par
/// l'assistant « Dossiers sources » (rien n'est coché d'office). (nom affiché, chemin)
pub fn suggested_roots() -> Vec<(String, String)> {
    use crate::i18n::tr;
    let home = std::path::PathBuf::from(home_dir());
    let mut out: Vec<(String, std::path::PathBuf)> = vec![(tr("Documents").into(), home.join("Documents")), (tr("Bureau").into(), home.join("Desktop"))];
    if !cfg!(windows) {
        out.push((tr("Bureau").into(), home.join("Bureau")));
    }
    for name in ["source", "Projects", "Projets", "dev", "code", "git", "repos", "src", "workspace", "Developer"] {
        out.push((name.to_owned(), home.join(name)));
    }
    out.push((tr("Bureau (OneDrive)").into(), home.join("OneDrive").join("Desktop")));
    out.push((tr("Documents (OneDrive)").into(), home.join("OneDrive").join("Documents")));
    if cfg!(windows) {
        for p in [r"C:\dev", r"C:\Projets", r"C:\projects", r"C:\code", r"D:\dev", r"D:\Projets"] {
            out.push((p.to_owned(), std::path::PathBuf::from(p)));
        }
    }
    let mut seen = Vec::new();
    out.into_iter()
        .filter(|(_, p)| p.is_dir())
        .filter_map(|(label, p)| {
            // Le même dossier sous deux noms (Windows ignore la casse) : une seule fois.
            let path = p.display().to_string();
            let key = path_key(&path);
            if seen.contains(&key) {
                return None;
            }
            seen.push(key);
            Some((label, path))
        })
        .collect()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Aucun dossier source par défaut : c'est à l'utilisateur de les choisir.
            roots: Vec::new(),
            excludes: ["node_modules", "target", ".git", "dist", "build", ".venv", "venv", "__pycache__", ".next", "vendor", "bin", "obj"].map(String::from).to_vec(),
            markers: default_markers(),
            freq: ScanFreq::Startup,
            skip_nested: true,
            group_monorepos: true,
            ignore_older: false,
            ignore_older_days: 730,
            auto_add: true,
            github_enabled: true,
            github_account: String::new(),
            ai_mode: AiMode::Ollama,
            ollama_url: "http://localhost:11434".into(),
            ollama_model: String::new(),
            ollama_embed_model: String::new(),
            ollaya_url: "http://127.0.0.1:11435".into(),
            ollaya_model: String::new(),
            read_depth: ReadDepth::Manifests,
            limit_focus: false,
            focus_limit: 4,
            auto_reserve: true,
            reserve_after: 45,
            reviews_per_day: 1,
            theme: ThemeChoice::Dark,
            accent: Accent::Amber,
            compact: false,
            zoom: 1.0,
            demo_data: false,
            update_auto: true,
            update_beta: false,
            language: None,
            markers_version: MARKERS_VERSION,
            ignored: Vec::new(),
        }
    }
}

// ---- données -----------------------------------------------------------------------------------

/// Tout ce que l'interface affiche, et ce qui est enregistré dans `rust-project.json`.
#[derive(Clone, Serialize, Deserialize)]
pub struct Data {
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default = "domains")]
    pub domains: Vec<Domain>,
    /// Les lanceurs configurés (par défaut ou propres à un projet).
    #[serde(default = "default_launchers")]
    pub launchers: Vec<Launcher>,
    /// Ceux affichés sur tous les projets, dans l'ordre.
    #[serde(default = "default_launcher_ids")]
    pub default_launchers: Vec<usize>,
    /// Les modèles proposés par la galerie (dans le code, pas dans le fichier).
    #[serde(skip, default = "gallery")]
    pub gallery: Vec<Launcher>,
    #[serde(default)]
    pub config: Config,
    #[serde(default)]
    pub next_id: u64,
    /// Facettes et concepts pour classer les projets (proposés par l'IA, retouchés à la main).
    #[serde(default)]
    pub ontology: crate::ontology::Ontology,
}

impl Data {
    /// Un fichier tout neuf : aucun projet, la configuration par défaut.
    pub fn fresh() -> Self {
        let mut data = Self {
            projects: Vec::new(),
            domains: domains(),
            launchers: default_launchers(),
            default_launchers: default_launcher_ids(),
            gallery: gallery(),
            config: Config::default(),
            next_id: 1,
            ontology: Default::default(),
        };
        data.ensure_ontology();
        data
    }

    /// Répare ce qui doit l'être après un chargement (identifiants manquants ou en double, index).
    pub fn normalize(&mut self) {
        // Ancien fichier sans ontologie : les domaines deviennent la facette « Domaine ».
        self.ensure_ontology();
        let mut seen = std::collections::HashSet::new();
        self.next_id = self.next_id.max(self.projects.iter().map(|p| p.id).max().unwrap_or(0) + 1);
        for p in &mut self.projects {
            if p.id == 0 || !seen.insert(p.id) {
                p.id = self.next_id;
                self.next_id += 1;
                seen.insert(p.id);
            }
            if p.domain >= self.domains.len() {
                p.domain = 0;
            }
            p.launchers.retain(|l| *l < self.launchers.len());
            p.hidden.retain(|l| *l < self.launchers.len());
            p.favorite = p.favorite.filter(|l| *l < self.launchers.len());
        }
        self.default_launchers.retain(|l| *l < self.launchers.len());
        // Ancien lanceur « Claude Code » resté tel quel (`claude` dans un terminal) : il ouvre
        // désormais Claude Desktop. Un lanceur modifié à la main n'est pas touché.
        for l in &mut self.launchers {
            if l.name == "Claude Code" && l.kind == LauncherKind::Command && l.command.trim() == "claude" {
                l.kind = LauncherKind::Url;
                l.command = CLAUDE_DESKTOP.into();
                l.in_terminal = false;
                l.description = "Nouvelle conversation dans Claude Desktop, sur le dossier".into();
            }
        }
        for r in &mut self.config.roots {
            r.depth = r.depth.clamp(1, 3);
        }
        // Ancien fichier : ajouter les nouveaux marqueurs (AGENTS.md…) et cocher le README.
        if self.config.markers_version < MARKERS_VERSION {
            for (m, _) in default_markers() {
                match self.config.markers.iter_mut().find(|(x, _)| x.eq_ignore_ascii_case(&m)) {
                    Some((_, on)) if m == "README.md" => *on = true,
                    Some(_) => {}
                    None => self.config.markers.push((m, true)),
                }
            }
            self.config.markers_version = MARKERS_VERSION;
        }
    }

    pub fn index_of(&self, id: u64) -> Option<usize> {
        self.projects.iter().position(|p| p.id == id)
    }

    /// Le projet qui suit déjà ce dossier (comparaison insensible à la casse et aux séparateurs).
    pub fn find_by_folder(&self, path: &str) -> Option<usize> {
        let key = path_key(path);
        self.projects.iter().position(|p| p.folders.iter().any(|f| path_key(f) == key))
    }

    /// Ajoute un projet, lui donne un identifiant, et renvoie son index.
    pub fn add_project(&mut self, mut p: Project) -> usize {
        p.id = self.next_id.max(1);
        self.next_id = p.id + 1;
        if p.domain >= self.domains.len() {
            p.domain = 0;
        }
        self.projects.push(p);
        self.projects.len() - 1
    }

    /// Les projets sans adresse de dépôt la reçoivent de leur dossier Git, s'il en a un.
    pub fn fill_remotes(&mut self) {
        for p in self.projects.iter_mut().filter(|p| p.remote.is_none()) {
            p.remote = crate::scan::remote_of(&p.folders);
        }
    }

    /// Le domaine qui porte ce nom (sans tenir compte de la casse), créé s'il n'existe pas.
    pub fn domain_named(&mut self, name: &str) -> usize {
        let name = name.trim();
        if name.is_empty() {
            return 0;
        }
        if let Some(i) = self.domains.iter().position(|d| d.name.to_lowercase() == name.to_lowercase()) {
            return i;
        }
        const PALETTE: [u32; 6] = [0x5fb3d9, 0xe08ab0, 0x9ccc65, 0xc49cff, 0xf2b866, 0x7fd1c7];
        let color = rgb(PALETTE[self.domains.len() % PALETTE.len()]);
        self.domains.push(Domain { name: name.to_owned(), icon: Icon::Tag, color });
        self.domains.len() - 1
    }

    /// Les lanceurs d'un projet, dans l'ordre d'affichage : ceux par défaut (sauf masqués), puis les siens.
    pub fn launchers_of(&self, p: usize) -> Vec<usize> {
        let project = &self.projects[p];
        let mut ids: Vec<usize> = self.default_launchers.iter().copied().filter(|id| !project.hidden.contains(id)).collect();
        for &id in &project.launchers {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }

    pub fn favorite_of(&self, p: usize) -> Option<usize> {
        let ids = self.launchers_of(p);
        self.projects[p].favorite.filter(|f| ids.contains(f)).or_else(|| ids.first().copied())
    }

    pub fn count(&self, attention: Attention) -> usize {
        self.projects.iter().filter(|p| p.attention == attention && p.status != Status::Archived).count()
    }

    pub fn archived(&self) -> usize {
        self.projects.iter().filter(|p| p.status == Status::Archived).count()
    }

    /// Les projets en focus, dans leur ordre.
    pub fn focus(&self) -> Vec<usize> {
        let mut ids: Vec<usize> = (0..self.projects.len()).filter(|&i| self.projects[i].attention == Attention::Focus && self.projects[i].status != Status::Archived).collect();
        ids.sort_by_key(|&i| self.projects[i].order);
        ids
    }
}

/// Clé de comparaison d'un chemin : minuscules, `/` → `\`, sans séparateur final.
/// Le dossier parent d'un chemin, quels que soient ses séparateurs (`C:\a\b` comme `/a/b`) :
/// `Path` ne découpe que selon ceux du système, or les données viennent de Windows comme de Linux.
pub fn parent_path(path: &str) -> Option<&str> {
    let p = path.trim().trim_end_matches(['\\', '/']);
    let i = p.rfind(['\\', '/'])?;
    Some(if i == 0 { &p[..1] } else { &p[..i] })
}

/// Le dernier nom d'un chemin (`site` pour `C:\a\site` comme pour `/a/site`). Vide pour une
/// racine (`C:`, `/`).
pub fn file_name(path: &str) -> &str {
    let p = path.trim().trim_end_matches(['\\', '/']);
    let name = p.rsplit(['\\', '/']).next().unwrap_or(p);
    if name.ends_with(':') { "" } else { name }
}

pub fn path_key(path: &str) -> String {
    path.trim().replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

// ---- valeurs par défaut ------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn launcher(name: &str, icon: Icon, color: u32, kind: LauncherKind, command: &str, in_terminal: bool, group: Group, description: &str) -> Launcher {
    Launcher {
        name: name.into(),
        icon,
        color: rgb(color),
        kind,
        command: command.into(),
        in_terminal,
        shortcut: String::new(),
        description: description.into(),
        group,
    }
}

/// Une nouvelle conversation Claude Code dans Claude Desktop, sur le dossier du projet
/// (Claude Desktop demande de confirmer le dossier à chaque fois).
const CLAUDE_DESKTOP: &str = "claude://code/new?folder={dossier_url}";

pub fn gallery() -> Vec<Launcher> {
    use Group as G;
    use LauncherKind as K;
    vec![
        launcher("Terminal", Icon::SquareTerminal, 0x9aa3ad, K::App, r#"start "Terminal" /D "{dossier}" powershell -NoExit"#, false, G::Terminal, "PowerShell ouvert dans le dossier"),
        launcher("Windows Terminal", Icon::Terminal, 0x5b8def, K::App, r#"wt -d "{dossier}""#, false, G::Terminal, "Windows Terminal (s'il est installé)"),
        launcher("Invite de commandes", Icon::Terminal, 0x8a94a0, K::App, r#"start "cmd" /D "{dossier}" cmd /K"#, false, G::Terminal, "cmd.exe ouvert dans le dossier"),
        launcher("VS Code", Icon::Code, 0x3b9cff, K::App, r#"code "{dossier}""#, false, G::Editor, "Ouvre le dossier dans Visual Studio Code"),
        launcher("Cursor", Icon::MousePointer2, 0xd6d6d6, K::App, r#"cursor "{dossier}""#, false, G::Editor, "Ouvre le dossier dans Cursor"),
        launcher("JetBrains", Icon::Braces, 0xff5c8a, K::App, r#"idea64 "{dossier}""#, false, G::Editor, "IntelliJ, Rider, WebStorm…"),
        launcher("Claude Code", Icon::Sparkles, 0xe07a5f, K::Url, CLAUDE_DESKTOP, false, G::Ai, "Nouvelle conversation dans Claude Desktop, sur le dossier"),
        launcher("Claude (reprise)", Icon::History, 0xe07a5f, K::Command, "claude --continue", true, G::Ai, "Reprend la dernière conversation du projet"),
        launcher("Explorateur", Icon::FolderOpen, 0xf4c14d, K::Folder, r#"explorer "{dossier}""#, false, G::Tools, "Le dossier dans l'explorateur Windows"),
        launcher("Serveur de dev", Icon::Globe, 0x3fc1b0, K::Url, "http://localhost:{port}", false, G::Web, "Ouvre l'appli locale dans le navigateur"),
        launcher("Dépôt GitHub", Icon::GitBranch, 0xb7b7b7, K::Url, "{remote}", false, G::Git, "La page du dépôt distant"),
        launcher("Lazygit", Icon::GitPullRequest, 0x7fcf6f, K::Command, "lazygit", true, G::Git, "Git en mode terminal, dans le dossier"),
        launcher("npm run dev", Icon::Play, 0xcb3837, K::Command, "npm run dev", true, G::Tools, "Lance le serveur de développement"),
        launcher("cargo run", Icon::Hammer, 0xdea584, K::Command, "cargo run", true, G::Tools, "Compile et lance le projet Rust"),
        launcher("Docker Compose", Icon::Container, 0x2496ed, K::Command, "docker compose up", true, G::Tools, "Démarre les services du projet"),
        launcher("Obsidian", Icon::NotebookPen, 0x9f7aea, K::Url, "obsidian://open?vault={nom}", false, G::Tools, "Les notes du projet dans Obsidian"),
        launcher("Maquettes", Icon::PenTool, 0xf24e1e, K::Url, "https://www.figma.com/files/project/{nom}", false, G::Web, "Le dossier de maquettes du projet"),
    ]
}

fn from_gallery(name: &str, shortcut: &str) -> Launcher {
    let mut l = gallery().into_iter().find(|l| l.name == name).expect("lanceur de la galerie");
    l.shortcut = shortcut.into();
    l
}

/// Les lanceurs d'un fichier neuf : Claude Code, VS Code, Terminal, Explorateur.
pub fn default_launchers() -> Vec<Launcher> {
    vec![from_gallery("Claude Code", "Ctrl+1"), from_gallery("VS Code", "Ctrl+2"), from_gallery("Terminal", "Ctrl+3"), from_gallery("Explorateur", "")]
}

fn default_launcher_ids() -> Vec<usize> {
    vec![0, 1, 2, 3]
}

pub fn domains() -> Vec<Domain> {
    let d = |name: &str, icon, color| Domain { name: name.into(), icon, color: rgb(color) };
    vec![
        d("Outils de dev", Icon::Wrench, 0x6c9cf0),
        d("IA & agents", Icon::Sparkles, 0xb98cf0),
        d("Formation", Icon::GraduationCap, 0xf0a443),
        d("Web & SaaS", Icon::Globe, 0x3fc1b0),
        d("3D & jeux", Icon::Box, 0xef7f5f),
        d("Audio & voix", Icon::AudioWaveform, 0x7fcf6f),
        d("Perso & contenu", Icon::Newspaper, 0xd9b45a),
    ]
}

// ---- données de démonstration (tests et mode démo) ---------------------------------------------

const ROOT: &str = r"C:\Users\Djidjelli\Documents\Projet perso";
const CLIENTS: &str = r"D:\Clients";

/// Un projet de démo avec des textes plausibles déduits de son nom.
#[allow(clippy::too_many_arguments)]
fn project(name: &str, icon: Icon, domain: usize, kind: &str, stack: &[&str], status: Status, attention: Attention, idle_days: u32, outcome: &str, next: &str) -> Project {
    let root = if domain == 3 && idle_days.is_multiple_of(3) { CLIENTS } else { ROOT };
    let at = |d: u32| now().saturating_sub(u64::from(d) * 86_400);
    Project {
        name: name.into(),
        icon,
        description: format!("{kind} en {}.", stack.join(" et ")),
        outcome: outcome.into(),
        objective: format!("Mener {name} jusqu'à « {outcome} »."),
        last_point: if idle_days > 60 { "Mis de côté en attendant une idée plus claire.".into() } else { "Base en place, il reste à finir et polir.".into() },
        next_action: next.into(),
        steps: vec![
            Step { text: next.into(), done: false },
            Step { text: "Relire et nettoyer".into(), done: false },
            Step { text: "Écrire le README".into(), done: idle_days.is_multiple_of(2) },
        ],
        status,
        attention,
        domain,
        kind: kind.into(),
        stack: stack.iter().map(|s| s.to_string()).collect(),
        folders: vec![format!(r"{root}\{name}")],
        notes: vec!["Idées en vrac à trier.".into()],
        key_files: vec!["README.md".into()],
        idle_days,
        review: if attention == Attention::Reserve { "Quand il ressort".into() } else { "Chaque semaine".into() },
        confidence: 0.62 + (name.len() % 7) as f32 * 0.05,
        activity: vec![
            Activity { at: at(idle_days), kind: ActivityKind::Commit, text: format!("Dernier commit : « {next} » commencé") },
            Activity { at: at(idle_days + 1), kind: ActivityKind::Edit, text: format!("Prochaine action : « {next} »") },
            Activity { at: at(idle_days + 3), kind: ActivityKind::Ai, text: "L'IA a classé le projet et rédigé son résumé".into() },
            Activity { at: at(idle_days + 12), kind: ActivityKind::Note, text: "Note : idées en vrac à trier".into() },
            Activity { at: at(idle_days + 40), kind: ActivityKind::Scan, text: format!(r"Découvert dans {root}") },
        ],
        added_by: AddedBy::Demo,
        ..Default::default()
    }
}

pub fn demo() -> Data {
    use Attention::*;
    use Icon as I;
    use Status::*;

    let gallery = gallery();
    let by_name = |n: &str| gallery.iter().find(|l| l.name == n).cloned().expect("lanceur de la galerie");
    let mut launchers = default_launchers();
    launchers.extend([by_name("Serveur de dev"), by_name("cargo run"), by_name("npm run dev"), by_name("Obsidian")]);
    const DEV_SERVER: usize = 4;
    const CARGO: usize = 5;
    const NPM: usize = 6;
    const OBSIDIAN: usize = 7;

    let projects = vec![
        // Focus
        project("Atelier", I::Layers, 0, "Application desktop", &["Rust", "egui"], Active, Focus, 0, "Bibliothèque v2", "Comparer les maquettes."),
        project("Tempo", I::AudioWaveform, 3, "Outil de diagnostic", &["Next.js", "TypeScript"], Active, Focus, 1, "Diagnostic v3", "Relire le parcours."),
        project("ImmoFormation", I::House, 2, "Plateforme de formation", &["Laravel", "Vue"], Active, Focus, 2, "Module 1", "Finaliser le plan."),
        project("Carnet IA", I::Sparkles, 1, "Agent personnel", &["Python", "Claude"], Active, Focus, 0, "Synthèse mensuelle", "Brancher les notes de la semaine."),
        // Au fil de l'eau
        project("API locale", I::Code, 0, "API", &["Bun", "TypeScript"], Active, Flow, 3, "Endpoints v1", "Tester l'intégration."),
        project("Portfolio", I::Laptop, 6, "Site vitrine", &["Astro"], Active, Flow, 6, "Nouvelle version", "Mettre à jour les projets."),
        project("Studio 3D", I::Box, 4, "Éditeur 3D", &["Three.js", "React"], Active, Flow, 9, "Prototype jouable", "Poursuivre le modèle principal."),
        project("BunFlow", I::Workflow, 0, "Automatisation", &["Bun", "TypeScript"], Active, Flow, 4, "Moteur de workflows", "Ajouter les déclencheurs planifiés."),
        project("FactureX", I::ReceiptText, 3, "SaaS", &["Next.js", "Postgres"], Active, Flow, 12, "Factures PDF", "Brancher l'export PDF."),
        project("LeBonTerroir", I::ShoppingBasket, 3, "E-commerce", &["Laravel"], Active, Flow, 15, "Boutique en ligne", "Finir le tunnel de commande."),
        project("WhiperFlow", I::Mic, 5, "Transcription", &["Python", "Whisper"], Active, Flow, 7, "Dictée temps réel", "Réduire la latence."),
        project("powernote", I::NotebookPen, 6, "Prise de notes", &["Tauri", "React"], Active, Flow, 10, "Synchronisation", "Gérer les conflits d'édition."),
        project("MCP-HAR", I::Network, 1, "Serveur MCP", &["TypeScript", "MCP"], Active, Flow, 5, "Analyse de fichiers HAR", "Documenter les outils."),
        project("PowerCode", I::Zap, 0, "Extension d'éditeur", &["TypeScript"], Active, Flow, 18, "Version publique", "Préparer la publication."),
        project("RTK", I::Terminal, 0, "CLI", &["Rust"], Active, Flow, 2, "Sorties condensées", "Couvrir les commandes git."),
        project("InteractiveLearning", I::GraduationCap, 2, "Tutoriels interactifs", &["React"], Active, Flow, 21, "Parcours guidé", "Enregistrer le premier parcours."),
        // En réserve
        project("Arène 3D", I::Gamepad2, 4, "Jeu", &["Bevy", "Rust"], Paused, Reserve, 142, "Démo jouable", "Relire le code du combat."),
        project("TripoSR", I::Cuboid, 4, "Image → 3D", &["Python", "PyTorch"], Paused, Reserve, 96, "Pipeline d'export", "Tester les nouveaux poids."),
        project("xtts", I::AudioWaveform, 5, "Synthèse vocale", &["Python"], Paused, Reserve, 210, "Voix clonée", "Comparer les modèles."),
        project("Trading", I::ChartNoAxesColumn, 6, "Analyse de marché", &["Python", "Pandas"], Paused, Reserve, 75, "Tableau de bord", "Nettoyer les sources de données."),
        project("Youtube", I::Video, 6, "Chaîne vidéo", &["Montage"], Idea, Reserve, 120, "Trois vidéos prêtes", "Écrire le premier script."),
        project("TopFoot", I::Trophy, 3, "Application mobile", &["Flutter"], Paused, Reserve, 260, "Classements en direct", "Mettre à jour les dépendances."),
        project("voxel-archives", I::Shapes, 4, "Moteur voxel", &["Rust", "wgpu"], Paused, Reserve, 300, "Rendu de chunks", "Profiler le maillage."),
        project("superQuizAvatar", I::Gamepad2, 2, "Quiz animé", &["React", "Three.js"], Paused, Reserve, 88, "Quiz complet", "Animer l'avatar."),
        project("Fennectra", I::Compass, 3, "SaaS", &["Next.js"], Idea, Reserve, 64, "Page d'accueil", "Clarifier l'offre."),
        project("detourage", I::Scissors, 1, "Détourage d'images", &["Python", "ONNX"], Paused, Reserve, 150, "Détourage par lot", "Tester sur 100 images."),
        project("codex-proxy", I::Server, 1, "Proxy d'API", &["Go"], Paused, Reserve, 45, "Proxy stable", "Ajouter les journaux."),
        project("universal-email", I::Mail, 3, "Service e-mail", &["Node.js"], Paused, Reserve, 110, "Envoi multi-fournisseurs", "Écrire les tests."),
        project("RSS_WEBHOOK", I::Rss, 0, "Automatisation", &["Python"], Paused, Reserve, 190, "Flux vers webhooks", "Gérer les doublons."),
        project("Twilio", I::Phone, 3, "Téléphonie", &["Node.js"], Paused, Reserve, 230, "Standard vocal", "Revoir le scénario d'accueil."),
        project("moncv", I::FileText, 6, "CV en ligne", &["HTML"], Done, Reserve, 70, "CV à jour", "Ajouter les derniers projets."),
        project("blogs", I::Newspaper, 6, "Blog", &["Astro"], Idea, Reserve, 55, "Dix articles", "Choisir le premier sujet."),
        // Archivés
        project("BoilerplateAPP", I::Package, 0, "Modèle de projet", &["Next.js"], Archived, Reserve, 400, "Gabarit réutilisable", "—"),
        project("hack_NextJS", I::Code, 0, "Bac à sable", &["Next.js"], Archived, Reserve, 380, "Essais", "—"),
        project("quizSauv", I::Archive, 2, "Sauvegarde", &["React"], Archived, Reserve, 330, "Copie de sécurité", "—"),
    ];

    let mut data = Data { projects: Vec::new(), domains: domains(), launchers, default_launchers: default_launcher_ids(), gallery, config: Config::default(), next_id: 1, ontology: Default::default() };
    for p in projects {
        data.add_project(p);
    }
    let projects = &mut data.projects;

    // Les projets en focus, racontés plus en détail.
    {
        let p = &mut projects[0];
        p.order = 0;
        // Le vrai dossier de cette maquette : ses lanceurs fonctionnent pour de bon.
        p.folders = vec![r"C:\Users\Djidjelli\Documents\Projet perso\Cours_Rust\atelier".into()];
        p.description = "Gestionnaire de projets personnels en Rust, avec lanceurs et IA.".into();
        p.objective = "Une bibliothèque de projets qu'on a envie d'ouvrir chaque matin.".into();
        p.last_point = "La maquette du bureau est validée, le panneau de détail est en cours.".into();
        p.steps = vec![
            Step { text: "Comparer les maquettes".into(), done: false },
            Step { text: "Dessiner la galerie de lanceurs".into(), done: true },
            Step { text: "Écrire les tests de captures".into(), done: false },
        ];
        p.launchers = vec![CARGO];
        p.key_files = vec!["Cargo.toml".into(), "README.md".into(), "CLAUDE.md".into()];
        p.links = vec![("Maquettes".into(), "https://www.figma.com/files/atelier".into())];
        p.notes = vec!["Garder trois sections sur le bureau, pas plus.".into(), "Les lanceurs remplacent « Ouvrir l'espace ».".into()];
        p.confidence = 0.94;
    }
    {
        let p = &mut projects[1];
        p.order = 1;
        p.objective = "Simplifier le parcours de diagnostic.".into();
        p.last_point = "Le parcours est prêt à relire.".into();
        p.steps = vec![
            Step { text: "Relire le parcours".into(), done: false },
            Step { text: "Corriger les libellés".into(), done: false },
            Step { text: "Tester avec deux utilisateurs".into(), done: false },
        ];
        p.launchers = vec![DEV_SERVER, NPM];
        p.favorite = Some(1);
        p.key_files = vec!["package.json".into(), "next.config.ts".into(), ".env.example".into()];
        p.confidence = 0.81;
    }
    {
        let p = &mut projects[2];
        p.order = 2;
        p.description = "Plateforme de formation aux métiers de l'immobilier.".into();
        p.objective = "Préparer le premier module de formation.".into();
        p.last_point = "Le plan est presque terminé.".into();
        p.steps = vec![
            Step { text: "Finaliser le plan".into(), done: true },
            Step { text: "Rédiger la première leçon".into(), done: false },
            Step { text: "Préparer un exercice".into(), done: false },
        ];
        p.launchers = vec![OBSIDIAN, DEV_SERVER];
        p.links = vec![("Site de démo".into(), "https://immoformation.test".into())];
        p.key_files = vec!["composer.json".into(), "package.json".into(), "README.md".into()];
        p.confidence = 0.92;
    }
    {
        let p = &mut projects[3];
        p.order = 3;
        p.objective = "Un carnet qui résume seul ce que j'ai appris chaque mois.".into();
        p.last_point = "L'agent lit déjà les notes, la synthèse reste brouillonne.".into();
        p.steps = vec![
            Step { text: "Brancher les notes de la semaine".into(), done: false },
            Step { text: "Écrire le gabarit de synthèse".into(), done: true },
            Step { text: "Planifier l'envoi mensuel".into(), done: false },
        ];
        p.confidence = 0.88;
    }

    let recent = now().saturating_sub(300);
    data.config.roots = vec![
        ScanRoot { path: ROOT.into(), enabled: true, depth: 2, found: 31, last_scan: recent },
        ScanRoot { path: CLIENTS.into(), enabled: true, depth: 3, found: 4, last_scan: recent },
        ScanRoot { path: r"C:\Users\Djidjelli\Documents\Archives".into(), enabled: false, depth: 1, found: 0, last_scan: 0 },
    ];
    data.config.demo_data = true;
    data.ensure_ontology();
    data
}
