//! Le shell : état de l'interface, actions, barre de navigation, barre d'état, panneau projet,
//! fenêtres modales et notifications. Les écrans vivent dans `screens/`.
//!
//! Les écrans ne modifient pas les données en plein milieu du dessin : ils empilent des `Action`
//! que `apply_actions` exécute en fin d'image. Ça évite les conflits d'emprunt et garde toute la
//! logique (limite du focus, ajout de lanceur…) au même endroit.

use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Margin, RichText, Stroke, StrokeKind, Ui, vec2};

use crate::classify::{self, Proposal};
use crate::github::Repo;
use crate::icons::{self, Icon};
use crate::jobs::{self, Bus, Classifier, JobMsg};
use crate::model::{self, ActivityKind, AddedBy, Attention, Data, Group, Launcher, LauncherKind, Project, ScanRoot, Status, Step};
use crate::ollama::ModelInfo;
use crate::scan::{self, Found, ScanOptions, ScanResult};
use crate::screens;
use crate::search::{self, Hit, QueryCache};
use crate::store::{SemanticIndex, Store, text_hash};
use crate::theme::{self, Tokens, tint};
use crate::widgets::{self, Kind};
use crate::{launch, launchers, panel, update};
use crate::i18n::{tf, tr};

const TOAST_SECS: f64 = 4.0;
/// Au plus tant de dossiers envoyés à l'IA par analyse.
const MAX_ANALYSIS: usize = 150;
/// La version de cet exécutable : le tag de la release (posé par la CI), sinon Cargo.toml.
pub const VERSION: &str = crate::release::VERSION;
pub use crate::release::Phase as UpdatePhase;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Desk,
    Library,
    Revisit,
    Config,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelTab {
    Summary,
    Activity,
    Resources,
    Edit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibView {
    Map,
    Grid,
    List,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Recent,
    Name,
    Domain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CfgSection {
    Sources,
    Discovery,
    Ai,
    Ontology,
    Launchers,
    Attention,
    Appearance,
    Data,
}

impl CfgSection {
    pub const ALL: [Self; 8] = [Self::Sources, Self::Discovery, Self::Ai, Self::Ontology, Self::Launchers, Self::Attention, Self::Appearance, Self::Data];
    pub fn label(self) -> &'static str {
        match self {
            Self::Sources => tr("Dossiers sources"),
            Self::Discovery => tr("Découverte"),
            Self::Ai => tr("IA & modèles"),
            Self::Ontology => tr("Ontologie"),
            Self::Launchers => tr("Lanceurs"),
            Self::Attention => tr("États & attention"),
            Self::Appearance => tr("Apparence"),
            Self::Data => tr("Données"),
        }
    }
    pub fn icon(self) -> Icon {
        match self {
            Self::Sources => Icon::FolderTree,
            Self::Discovery => Icon::ScanSearch,
            Self::Ai => Icon::Sparkles,
            Self::Ontology => Icon::Network,
            Self::Launchers => Icon::Rocket,
            Self::Attention => Icon::Crosshair,
            Self::Appearance => Icon::Palette,
            Self::Data => Icon::Database,
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            Self::Sources => tr("Où chercher"),
            Self::Discovery => tr("Comment reconnaître un projet"),
            Self::Ai => tr("Ollama, Ollaya, recherche"),
            Self::Ontology => tr("Facettes, concepts, classement"),
            Self::Launchers => tr("Boutons « Ouvrir avec »"),
            Self::Attention => tr("Focus, réserve, revue"),
            Self::Appearance if theme::LIGHT_THEME => tr("Thème, accent, densité"),
            Self::Appearance => tr("Langue, accent, densité"),
            Self::Data => tr("Index, export, démo"),
        }
    }
}

/// Le brouillon d'un lanceur, dans l'éditeur.
#[derive(Clone, Debug)]
pub struct LauncherDraft {
    pub name: String,
    pub icon: Icon,
    pub color: Color32,
    pub kind: LauncherKind,
    pub command: String,
    pub in_terminal: bool,
    pub shortcut: String,
    pub description: String,
    /// Le logo SVG (vide : l'icône).
    pub svg: String,
    /// La zone « coller le code SVG », ouverte avec ce qui y est tapé.
    pub svg_paste: Option<String>,
    pub svg_error: Option<String>,
}

impl Default for LauncherDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            icon: Icon::Rocket,
            color: theme::rgb(0xf4a945),
            kind: LauncherKind::Command,
            command: String::new(),
            in_terminal: true,
            shortcut: String::new(),
            description: String::new(),
            svg: String::new(),
            svg_paste: None,
            svg_error: None,
        }
    }
}

impl LauncherDraft {
    pub fn from_launcher(l: &Launcher) -> Self {
        Self {
            name: l.name.clone(),
            icon: l.icon,
            color: l.color,
            kind: l.kind,
            command: l.command.clone(),
            in_terminal: l.in_terminal,
            shortcut: l.shortcut.clone(),
            description: l.description.clone(),
            svg: l.svg.clone(),
            svg_paste: None,
            svg_error: None,
        }
    }

    pub fn to_launcher(&self) -> Launcher {
        Launcher {
            name: self.name.trim().to_owned(),
            icon: self.icon,
            color: self.color,
            kind: self.kind,
            command: self.command.trim().to_owned(),
            in_terminal: self.in_terminal && self.kind == LauncherKind::Command,
            shortcut: self.shortcut.trim().to_owned(),
            description: if self.description.trim().is_empty() { tr("Lanceur personnalisé").into() } else { self.description.trim().to_owned() },
            group: Group::Tools,
            svg: self.svg.clone(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.name.trim().is_empty() && !self.command.trim().is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GalleryTab {
    Gallery,
    Custom,
}

pub struct GalleryState {
    /// Le projet d'où l'on vient (`None` depuis la configuration : ajout aux lanceurs par défaut).
    pub project: Option<usize>,
    pub tab: GalleryTab,
    pub group: Option<Group>,
    pub search: String,
    pub for_all: bool,
    pub draft: LauncherDraft,
}

/// La fenêtre « Ajouter un projet » : un dossier, son analyse, la fiche proposée.
pub struct AddProjectState {
    pub path: String,
    pub analyzing: bool,
    pub proposal: Option<Proposal>,
    pub by: AddedBy,
    pub error: Option<String>,
    pub attention: Attention,
}

pub enum Modal {
    Gallery(GalleryState),
    Editor { launcher: Option<usize>, draft: LauncherDraft },
    AddProject(AddProjectState),
    Update,
    /// Confirmer la suppression d'un projet (index).
    ConfirmDelete(usize),
    /// L'assistant « Dossiers sources », pas à pas.
    Setup(Box<crate::screens::setup::SetupState>),
}

/// L'analyse en cours (ou la dernière) des dossiers relevés par le scan.
#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub total: usize,
    pub done: usize,
    pub current: String,
    pub added: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
    pub with_ai: bool,
    pub finished: bool,
    /// Dossiers choisis par l'utilisateur (assistant) : chacun devient une fiche, l'IA ne fait
    /// que la remplir.
    pub force: bool,
}

/// Le classement des projets dans l'ontologie par Ollaya.
#[derive(Clone, Debug, Default)]
pub struct Tagging {
    pub done: usize,
    pub total: usize,
    pub current: String,
    pub finished: bool,
    pub error: Option<String>,
    /// Partagé avec la tâche : `jobs::RUN`, `PAUSE` ou `STOP`.
    pub control: std::sync::Arc<std::sync::atomic::AtomicU8>,
}

impl Tagging {
    pub fn paused(&self) -> bool {
        self.control.load(std::sync::atomic::Ordering::Relaxed) == jobs::PAUSE
    }
}

/// Un tag en cours de création, dans l'onglet « Modifier ».
#[derive(Clone, Debug, Default)]
pub struct TagNew {
    pub name: String,
    /// La facette choisie (`None` : une nouvelle, nommée `new_facet`).
    pub facet: Option<u64>,
    pub new_facet: String,
    pub parent: Option<u64>,
}

/// Les champs de l'onglet « Modifier ».
#[derive(Clone, Debug)]
pub struct ProjectDraft {
    pub name: String,
    pub description: String,
    pub outcome: String,
    pub objective: String,
    pub next_action: String,
    pub status: Status,
    pub attention: Attention,
    pub domain: usize,
    pub icon: Icon,
    /// Un logo à la place de l'icône (SVG ou image).
    pub logo: crate::model::Logo,
    pub folders: Vec<String>,
    /// Adresse web du dépôt (GitHub…), vide s'il n'y en a pas.
    pub remote: String,
}

impl PartialEq for ProjectDraft {
    fn eq(&self, o: &Self) -> bool {
        self.name == o.name
            && self.description == o.description
            && self.outcome == o.outcome
            && self.objective == o.objective
            && self.next_action == o.next_action
            && self.status == o.status
            && self.attention == o.attention
            && self.domain == o.domain
            && icons::same(self.icon, o.icon)
            && self.logo == o.logo
            && self.folders == o.folders
            && self.remote == o.remote
    }
}

impl ProjectDraft {
    pub fn from_project(p: &Project) -> Self {
        Self {
            name: p.name.clone(),
            description: p.description.clone(),
            outcome: p.outcome.clone(),
            objective: p.objective.clone(),
            next_action: p.next_action.clone(),
            status: p.status,
            attention: p.attention,
            domain: p.domain,
            icon: p.icon,
            logo: p.logo.clone(),
            folders: p.folders.clone(),
            remote: p.remote.clone().unwrap_or_default(),
        }
    }

    /// Ce que le formulaire donne une fois enregistré : un nom vide garde l'ancien, les lignes de
    /// dossier vides (en cours de saisie) sont ignorées.
    pub fn normalized(&self, p: &Project) -> Self {
        let mut d = self.clone();
        if d.name.trim().is_empty() {
            d.name = p.name.clone();
        }
        d.folders.retain(|f| !f.trim().is_empty());
        d.remote = d.remote.trim().to_owned();
        d
    }

    /// Reporte dans le formulaire ce qui a changé ailleurs (bouton focus, agent MCP…) depuis
    /// `base` : ces champs-là prennent la valeur de `fresh`, les autres restent tels que tapés.
    pub fn merge_external(&mut self, base: &Self, fresh: &Self) {
        macro_rules! take {
            ($($f:ident),*) => { $(if base.$f != fresh.$f { self.$f = fresh.$f.clone(); })* };
        }
        take!(name, description, outcome, objective, next_action, status, attention, domain, logo, folders, remote);
        if !icons::same(base.icon, fresh.icon) {
            self.icon = fresh.icon;
        }
    }
}

/// L'onglet « Modifier » ouvert : le formulaire, et le projet tel qu'il était au dernier
/// enregistrement (pour repérer ce qui a changé ailleurs entre-temps).
#[derive(Clone, Debug)]
pub struct DraftState {
    pub project: usize,
    pub draft: ProjectDraft,
    pub base: ProjectDraft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Focus,
    Flow,
    Keep,
    Archive,
    Later,
}

impl Decision {
    pub fn label(self) -> &'static str {
        match self {
            Self::Focus => tr("Remis en focus"),
            Self::Flow => tr("Au fil de l'eau"),
            Self::Keep => tr("Gardé en réserve"),
            Self::Archive => tr("Archivé"),
            Self::Later => tr("Reporté"),
        }
    }
}

/// Ce qu'on fait d'un coup à plusieurs projets (sélection multiple de la Bibliothèque).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BulkOp {
    Attention(Attention),
    Archive,
    Domain(usize),
}

pub enum Action {
    Select(usize),
    /// Plusieurs projets (index) d'un coup : glisser-déposer groupé, barre de la sélection.
    Bulk(Vec<usize>, BulkOp),
    // ---- tags (ontologie) d'un projet
    /// Rattacher un projet à un concept (ou valider un tag proposé par l'IA).
    Tag(usize, u64),
    /// Retirer un tag (il ne revient pas au classement suivant).
    Untag(usize, u64),
    /// Le domaine principal d'un projet (couleur et île sur la carte).
    MainDomain(usize, usize),
    /// Créer un tag et le rattacher au projet : dans une facette existante, ou une nouvelle.
    NewTag { project: usize, facet: Option<u64>, new_facet: String, name: String, parent: Option<u64> },
    /// Les mots-clés d'un projet (la liste complète).
    SetKeywords(usize, Vec<String>),
    /// Faire proposer des mots-clés par le modèle de langage d'Ollama : à ces projets
    /// (identifiants), ou à ceux qui n'en ont pas.
    SuggestKeywords(Option<Vec<u64>>),
    /// Ouvrir une adresse dans le navigateur (page GitHub d'un projet…).
    OpenUrl(String),
    /// Ouvrir les outils de développement (comme F12).
    OpenDevTools,
    Deselect,
    Go(Screen),
    GoConfig(CfgSection),
    Library(Option<Attention>),
    Launch(usize, usize),
    Toast(Icon, String, Option<String>),
    ToggleStep(usize, usize),
    /// Projet, attention, et s'il faut une notification (oui pour un glisser-déposer, non quand
    /// le changement se voit déjà sur la pastille).
    SetAttention(usize, Attention, bool),
    SetStatus(usize, Status),
    OpenGallery(Option<usize>),
    OpenEditor(Option<usize>),
    OpenAddProject,
    AddLauncher { launcher: Launcher, project: Option<usize> },
    SaveLauncher(usize, Launcher),
    RemoveDefault(usize),
    MoveDefault(usize, i32),
    SetFavorite(usize, usize),
    MoveInProject(usize, usize, i32),
    HideForProject(usize, usize),
    MoveFocus(usize, i32),
    /// Enregistre l'onglet « Modifier » : à chaque changement, sans notification.
    SaveDraft(usize, ProjectDraft),
    Decide(usize, Decision),
    StartScan,
    ApplyTheme,
    /// Changer la langue de l'interface (`None` : celle du système).
    SetLanguage(Option<crate::i18n::Lang>),
    ApplyZoom,
    OpenUpdate,
    CheckUpdate,
    DownloadUpdate,
    Restart,
    // ---- projets : création, suppression, étapes, notes
    AnalyzeFolder(String),
    AddProposal(Proposal, AddedBy, Attention),
    AskDelete(usize),
    DeleteProject(usize),
    AddStep(usize, String),
    RemoveStep(usize, usize),
    AddNote(usize, String),
    // ---- découverte
    PickRoot,
    RemoveRoot(usize),
    /// Rétablir un dossier écarté.
    RestoreFolder(usize),
    AddAllLikely,
    AnalyzeCandidates,
    // ---- assistant « Dossiers sources »
    OpenSetup(crate::screens::setup::Step),
    /// Compter les projets probables d'un dossier proposé (chemin, profondeur).
    SetupCount(String, u32),
    SetupAddFolder,
    /// Les dossiers cochés deviennent les dossiers sources, puis scan.
    SetupScan,
    /// Créer les fiches des dossiers gardés, écarter ceux décochés.
    SetupCreate { keep: Vec<String>, ignore: Vec<String>, with_ai: bool },
    ListGithub,
    /// Les comptes GitHub connectés à la CLI gh.
    ListGithubAccounts,
    AddRepo(String),
    AddAllRepos,
    // ---- Ollama et recherche sémantique
    ListModels,
    PullModel(String),
    Reindex,
    // ---- Ollaya et ontologie
    ListOllaya,
    PullOllaya(String),
    /// Classe ces projets (identifiants) avec Ollaya ; `None` : ceux à (re)classer.
    TagProjects(Option<Vec<u64>>),
    /// `jobs::PAUSE`, `jobs::RUN` (reprendre) ou `jobs::STOP` pour le classement en cours.
    ControlTagging(u8),
    /// Demande une ontologie au modèle de langage d'Ollama.
    GenerateOntology,
    ApplyProposal(crate::ontology::Merge),
    DiscardProposal,
    // ---- divers
    CopyText(String),
    SetDemo(bool),
}

pub struct Toast {
    pub icon: Icon,
    pub title: String,
    pub detail: Option<String>,
    pub at: f64,
}

/// Tout l'état de l'interface (ce qui n'est pas une donnée de projet).
pub struct UiState {
    pub screen: Screen,
    /// La carte de la Bibliothèque (simulation et caméra).
    pub graph: crate::graph::GraphView,
    pub selected: Option<usize>,
    pub tab: PanelTab,
    pub search: String,
    pub reorder: bool,
    pub flow_expanded: bool,
    pub lib_view: LibView,
    pub lib_domain: Option<usize>,
    pub lib_attention: Option<Attention>,
    pub lib_sort: Sort,
    pub lib_search: String,
    pub lib_archived: bool,
    pub cfg: CfgSection,
    pub revisit: Option<usize>,
    pub decisions: Vec<(usize, Decision)>,
    pub modal: Option<Modal>,
    pub draft: Option<DraftState>,
    /// Le scan en cours : depuis quand.
    pub scan_started: Option<f64>,
    /// Dossiers déjà relevés par le scan en cours.
    pub scan_count: usize,
    /// Le résultat du dernier scan.
    pub scan: Option<ScanResult>,
    /// Un scan demandé pendant qu'un autre tournait : il repart à la fin de celui-ci.
    pub rescan: bool,
    pub analysis: Option<Analysis>,
    /// Projets sélectionnés ensemble dans la Bibliothèque (identifiants).
    pub lib_multi: crate::select::MultiSelect<u64>,
    /// Le filtre par paquet de la Bibliothèque.
    pub lib_package: Option<crate::packages::Package>,
    /// La colonne de la Bibliothèque : tous les domaines (sinon les 5 premiers), les gestionnaires
    /// de paquets dépliés en entier, la taxonomie ouverte.
    pub lib_domains_all: bool,
    pub lib_packages_all: Vec<crate::packages::Manager>,
    /// Les gestionnaires dépliés dans la colonne (repliés par défaut).
    pub lib_packages_open: Vec<crate::packages::Manager>,
    pub lib_taxonomy_open: bool,
    /// L'onglet « Modifier » : le texte tapé pour ajouter un tag, et le tag en cours de création.
    pub tag_query: String,
    pub tag_new: Option<TagNew>,
    /// Les outils de développement (F12).
    pub devtools: crate::devtools::DevTools,
    /// Le logo du projet dans l'onglet « Modifier » : la zone « coller le code SVG » ouverte,
    /// et le dernier fichier refusé.
    pub logo_paste: Option<String>,
    pub logo_error: Option<String>,
    pub keyword_input: String,
    /// Mots-clés proposés par Ollama : (faits, en tout).
    pub keywords_job: Option<(usize, usize)>,
    pub new_exclude: String,
    pub new_note: String,
    pub new_step: String,
    pub now: f64,
    /// Le dernier lancement (projet, lanceur, instant) : sa tuile brille un court instant.
    pub flash: Option<(usize, usize, f64)>,
    /// Les versions publiées sur GitHub : vérification, téléchargement, installation.
    pub updater: crate::release::Updater,
    // ---- Ollama
    pub models: Option<Result<Vec<ModelInfo>, String>>,
    pub models_loading: bool,
    /// Téléchargement en cours : (modèle, statut, fraction).
    pub pull: Option<(String, String, f32)>,
    pub pull_input: String,
    // ---- Ollaya et ontologie
    pub ollaya_models: Option<Result<Vec<ModelInfo>, String>>,
    pub ollaya_loading: bool,
    pub ollaya_pull: Option<(String, String, f32)>,
    pub ollaya_pull_input: String,
    /// Classement en cours (ou le dernier, terminé).
    pub tagging: Option<Tagging>,
    /// Une ontologie proposée par l'IA, pas encore appliquée : (modèle, proposition).
    pub onto_proposal: Option<(String, crate::ontology::Ontology)>,
    pub onto_generating: bool,
    pub onto_error: Option<String>,
    // ---- GitHub
    pub github: Option<Result<Vec<Repo>, String>>,
    pub github_loading: bool,
    /// Comptes connectés à gh, et l'actif (`None` : pas encore demandé).
    pub github_accounts: Option<Result<crate::github::Accounts, String>>,
    pub github_accounts_loading: bool,
    // ---- recherche
    /// Résultats de la recherche du bureau (`None` : pas de recherche).
    pub search_hits: Option<Vec<Hit>>,
    /// Résultats de la recherche de la bibliothèque.
    pub lib_hits: Option<Vec<Hit>>,
    /// Index sémantique : projets indexés, modèle, calcul en cours, dernière erreur.
    pub semantic_indexed: usize,
    pub semantic_busy: bool,
    pub semantic_error: Option<String>,
    /// La requête courante a-t-elle son embedding ?
    pub query_semantic: bool,
    // ---- enregistrement
    /// Chemin du fichier, ou `None` en démonstration.
    pub store_path: Option<String>,
}

impl UiState {
    /// Le projet passe-t-il la recherche du bureau ?
    pub fn passes(&self, i: usize) -> bool {
        self.search_hits.as_ref().is_none_or(|h| h.iter().any(|x| x.index == i))
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Screen::Desk,
            graph: Default::default(),
            selected: Some(2),
            tab: PanelTab::Summary,
            search: String::new(),
            reorder: false,
            flow_expanded: false,
            lib_view: LibView::Map,
            lib_domain: None,
            lib_attention: None,
            lib_sort: Sort::Recent,
            lib_search: String::new(),
            lib_archived: false,
            cfg: CfgSection::Launchers,
            revisit: None,
            decisions: Vec::new(),
            modal: None,
            draft: None,
            scan_started: None,
            scan_count: 0,
            scan: None,
            rescan: false,
            analysis: None,
            lib_multi: Default::default(),
            lib_package: None,
            lib_domains_all: false,
            lib_packages_all: Vec::new(),
            lib_packages_open: Vec::new(),
            lib_taxonomy_open: false,
            tag_query: String::new(),
            tag_new: None,
            devtools: Default::default(),
            logo_paste: None,
            logo_error: None,
            keyword_input: String::new(),
            keywords_job: None,
            new_exclude: String::new(),
            new_note: String::new(),
            new_step: String::new(),
            now: 0.0,
            flash: None,
            updater: Default::default(),
            models: None,
            models_loading: false,
            pull: None,
            pull_input: String::new(),
            ollaya_models: None,
            ollaya_loading: false,
            ollaya_pull: None,
            ollaya_pull_input: String::new(),
            tagging: None,
            onto_proposal: None,
            onto_generating: false,
            onto_error: None,
            github: None,
            github_loading: false,
            github_accounts: None,
            github_accounts_loading: false,
            search_hits: None,
            lib_hits: None,
            semantic_indexed: 0,
            semantic_busy: false,
            semantic_error: None,
            query_semantic: false,
            store_path: None,
        }
    }
}

/// « il y a 3 j », « aujourd'hui »…
pub fn idle_text(days: u32) -> String {
    match days {
        0 => tr("aujourd'hui").into(),
        1 => tr("hier").into(),
        d if d < 60 => tf("il y a {d} j", &[("d", &d)]),
        d => tf("il y a {0} mois", &[("0", &(d / 30))]),
    }
}

pub struct AtelierApp {
    pub data: Data,
    pub st: UiState,
    pub toasts: Vec<Toast>,
    actions: Vec<Action>,
    /// En test : rien n'est réellement lancé ni redémarré, on garde seulement une trace.
    pub dry_run: bool,
    /// Les lignes de commande lancées (ou qui l'auraient été en `dry_run`).
    pub launched: Vec<String>,
    pub restart_requested: bool,
    /// Les erreurs des programmes lancés arrivent ici, depuis leur fil d'exécution.
    errors_tx: Sender<(String, String)>,
    errors_rx: Receiver<(String, String)>,
    /// Le fichier `rust-project.json` (`None` en démonstration : rien n'est enregistré).
    pub store: Option<Store>,
    /// Le dernier contenu enregistré (on n'écrit que s'il change).
    saved_json: String,
    last_sync: f64,
    /// Les tâches de fond répondent ici.
    jobs_tx: Sender<JobMsg>,
    jobs_rx: Receiver<JobMsg>,
    ctx: egui::Context,
    pub semantic: SemanticIndex,
    queries: QueryCache,
    /// Dernière requête vue, et quand elle a changé (pour attendre la fin de la frappe).
    last_query: (String, f64),
    last_embed_check: f64,
}

impl AtelierApp {
    /// L'application réelle : le fichier `rust-project.json` (créé s'il n'existe pas).
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_store(cc, Store::open(Store::default_path()))
    }

    /// Le démarrage réel, sur un fichier de données donné (celui de l'utilisateur, ou un fichier de test).
    pub fn new_with_store(cc: &eframe::CreationContext<'_>, mut store: Store) -> Self {
        let (data, error) = match store.load() {
            Ok(Some(d)) => (d, None),
            Ok(None) => (Data::fresh(), None),
            // Fichier illisible : on ne l'écrase pas, on part d'une base vide sans enregistrer.
            Err(e) => (Data::fresh(), Some(e)),
        };
        let readable = error.is_none();
        // La langue de l'interface : celle choisie, sinon celle du système. Pas en test : la langue
        // est globale, et les tests (en parallèle) cherchent les libellés en français.
        if !cfg!(test) {
            crate::i18n::set(data.config.language.unwrap_or_else(crate::i18n::detect));
        }
        let mut app = Self::with_data(cc, data, readable.then_some(store));
        // L'adresse du dépôt, lue dans `.git/config`, pour les projets qui ne l'ont pas encore.
        app.data.fill_remotes();
        // Les paquets de chaque projet, relus dans ses manifestes (en arrière-plan).
        app.refresh_packages();
        if let Some(e) = error {
            app.toast(Icon::TriangleAlert, tr("Fichier de données illisible"), Some(e));
        }
        crate::release::cleanup_old_exes();
        if std::env::args().any(|a| a == launch::AFTER_UPDATE) {
            app.toast(Icon::PackageCheck, tf("Rust Project {VERSION} est installé", &[("VERSION", &VERSION)]), None);
        }
        // Une seule question à GitHub au démarrage : y a-t-il une version plus récente ?
        if app.data.config.update_auto {
            app.st.updater.check(&cc.egui_ctx, app.data.config.update_beta);
        }
        // Premier lancement ou scan au démarrage : on relève les dossiers tout de suite.
        if app.store.is_some() && !app.data.config.roots.is_empty() && (app.data.projects.is_empty() || app.data.config.freq == model::ScanFreq::Startup) {
            app.actions.push(Action::StartScan);
        }
        // Tout premier lancement : l'assistant guide le choix des dossiers sources.
        if app.store.is_some() && app.data.config.roots.is_empty() && app.data.projects.is_empty() {
            app.actions.push(Action::OpenSetup(crate::screens::setup::Step::Where));
        }
        app
    }

    /// Un fichier de projets fictifs (`--fixture`) : lu une fois, jamais réécrit.
    pub fn from_fixture(cc: &eframe::CreationContext<'_>, path: &str) -> Self {
        let (data, error) = match Store::open(path).load() {
            Ok(Some(d)) => (d, None),
            Ok(None) => (Data::fresh(), Some(tf("Fixture introuvable : {path}", &[("path", &path)]))),
            Err(e) => (Data::fresh(), Some(e)),
        };
        if !cfg!(test) {
            crate::i18n::set(data.config.language.unwrap_or_else(crate::i18n::detect));
        }
        let mut app = Self::with_data(cc, data, None);
        if let Some(e) = error {
            app.toast(Icon::TriangleAlert, tr("Fixture illisible"), Some(e));
        }
        app
    }

    /// Les données de démonstration, sans fichier (tests, mode démo).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn demo(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::with_data(cc, model::demo(), None);
        app.st.selected = Some(2);
        app
    }

    pub fn with_data(cc: &eframe::CreationContext<'_>, data: Data, store: Option<Store>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        // Les logos SVG des lanceurs.
        egui_extras::install_image_loaders(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, data.config.theme, data.config.accent, data.config.compact);
        let (errors_tx, errors_rx) = channel();
        let (jobs_tx, jobs_rx) = channel();
        let mut st = UiState { selected: None, ..UiState::default() };
        st.store_path = store.as_ref().map(|s| s.path().display().to_string());
        let semantic = store.as_ref().map(|s| SemanticIndex::load(&s.embeddings_path())).unwrap_or_default();
        let saved_json = serde_json::to_string_pretty(&data).unwrap_or_default();
        let mut app = Self {
            data,
            st,
            toasts: Vec::new(),
            actions: Vec::new(),
            dry_run: false,
            launched: Vec::new(),
            restart_requested: false,
            errors_tx,
            errors_rx,
            store,
            saved_json,
            last_sync: 0.0,
            jobs_tx,
            jobs_rx,
            ctx: cc.egui_ctx.clone(),
            semantic,
            queries: QueryCache::default(),
            last_query: (String::new(), 0.0),
            last_embed_check: -10.0,
        };
        // Un fichier neuf est écrit tout de suite (on sait ainsi où il est).
        if let Some(store) = &mut app.store
            && !store.path().exists()
        {
            let _ = store.save_text(&app.saved_json);
        }
        app.st.semantic_indexed = app.indexed_count();
        app
    }

    fn bus(&self) -> Bus {
        Bus { tx: self.jobs_tx.clone(), ctx: self.ctx.clone() }
    }

    /// Le classement à utiliser : Ollama si un modèle est choisi, sinon les règles locales.
    fn classifier(&self) -> Classifier {
        let c = &self.data.config;
        let ollama = (c.ollama_on && !c.ollama_model.trim().is_empty()).then(|| (c.ollama_url.clone(), c.ollama_model.clone()));
        Classifier { ollama, domains: self.data.domains.iter().map(|d| d.name.clone()).collect(), depth: c.read_depth }
    }

    fn indexed_count(&self) -> usize {
        if self.semantic.model != self.data.config.ollama_embed_model {
            return 0;
        }
        self.data.projects.iter().filter(|p| self.semantic.vectors.contains_key(&p.id)).count()
    }

    /// Les dossiers du dernier scan à soumettre au classement : non suivis ; avec l'IA, aussi les
    /// dossiers de premier niveau sans marqueur (l'IA tranche), sans projet probable en dessous.
    pub fn candidates(&self, with_ai: bool) -> Vec<Found> {
        let Some(scan) = &self.st.scan else { return Vec::new() };
        let likely_paths: Vec<String> = scan.likely().map(|f| model::path_key(&f.path)).collect();
        scan.dirs
            .iter()
            .filter(|f| self.data.find_by_folder(&f.path).is_none())
            // Une brique (frontend, api…) appartient à son projet : jamais une fiche à part.
            .filter(|f| f.brick_of.is_none())
            .filter(|f| {
                if f.likely_project {
                    return true;
                }
                let key = model::path_key(&f.path) + "\\";
                with_ai && f.depth == 1 && f.files > 0 && !likely_paths.iter().any(|l| l.starts_with(&key))
            })
            .take(MAX_ANALYSIS)
            .cloned()
            .collect()
    }

    fn start_analysis(&mut self, candidates: Vec<Found>) {
        let classifier = self.classifier();
        let with_ai = classifier.ollama.is_some();
        if candidates.is_empty() {
            self.toast(Icon::CircleCheck, tr("Rien de nouveau à classer"), Some(tr("Tous les dossiers relevés sont déjà suivis.").into()));
            return;
        }
        self.st.analysis = Some(Analysis { total: candidates.len(), with_ai, ..Default::default() });
        jobs::spawn_analysis(self.bus(), candidates, classifier);
    }

    /// Bascule vers les données de démonstration (rien n'est enregistré) ou revient au fichier.
    fn set_demo(&mut self, on: bool) {
        if on {
            self.data = model::demo();
            self.store = None;
            self.st.store_path = None;
        } else {
            let mut store = Store::open(Store::default_path());
            self.data = store.load().ok().flatten().unwrap_or_else(Data::fresh);
            self.data.config.demo_data = false;
            self.st.store_path = Some(store.path().display().to_string());
            self.saved_json = serde_json::to_string_pretty(&self.data).unwrap_or_default();
            self.store = Some(store);
        }
        self.st.selected = None;
        self.st.draft = None;
        self.st.scan = None;
    }

    /// Lit les messages des tâches de fond.
    fn process_jobs(&mut self) {
        while let Ok(msg) = self.jobs_rx.try_recv() {
            match msg {
                JobMsg::ScanProgress(n) => self.st.scan_count = n,
                JobMsg::ScanDone(result) => self.scan_done(result),
                JobMsg::AnalysisProgress { done, total, current } => {
                    if let Some(a) = &mut self.st.analysis {
                        (a.done, a.total, a.current) = (done, total, current);
                    }
                }
                JobMsg::Classified { mut proposal, by, warning } => {
                    // Choisi dans l'assistant : une fiche, même si l'IA hésite.
                    if self.st.analysis.as_ref().is_some_and(|a| a.force) {
                        proposal.is_project = true;
                    }
                    let added = proposal.is_project && classify::apply(&mut self.data, &proposal, by).is_some();
                    if let Some(a) = &mut self.st.analysis {
                        if added { a.added += 1 } else { a.skipped += 1 }
                        a.warnings.extend(warning);
                    }
                }
                JobMsg::AnalysisDone => {
                    let wizard = matches!(self.st.modal, Some(Modal::Setup(_)));
                    if let Some(a) = &mut self.st.analysis {
                        a.finished = true;
                        // L'assistant montre lui-même le bilan.
                        if wizard {
                            continue;
                        }
                        let (added, skipped, warnings) = (a.added, a.skipped, a.warnings.len());
                        let how = if a.with_ai { tr("par Ollama") } else { tr("par les règles locales") };
                        let detail = if warnings > 0 { tf("{skipped} dossiers écartés · {warnings} réponses de l'IA illisibles (règles utilisées)", &[("skipped", &skipped), ("warnings", &warnings)]) } else { tf("{skipped} dossiers écartés", &[("skipped", &skipped)]) };
                        self.toast(Icon::Sparkles, tf("{added} projets créés {how}", &[("added", &added), ("how", &how)]), Some(detail));
                    }
                }
                JobMsg::OneAnalyzed(result) => {
                    if let Some(Modal::AddProject(a)) = &mut self.st.modal {
                        a.analyzing = false;
                        match result {
                            Ok((p, by)) => {
                                a.proposal = Some(p);
                                a.by = by;
                                a.error = None;
                            }
                            Err(e) => a.error = Some(e),
                        }
                    }
                }
                JobMsg::Models(result) => {
                    self.st.models_loading = false;
                    if let Ok(models) = &result {
                        // Choix par défaut : le premier modèle de langage et le premier modèle d'embeddings.
                        let c = &mut self.data.config;
                        if c.ollama_model.is_empty()
                            && let Some(m) = models.iter().find(|m| !m.is_embedding())
                        {
                            c.ollama_model = m.name.clone();
                        }
                        if c.ollama_embed_model.is_empty()
                            && let Some(m) = models.iter().find(|m| m.is_embedding())
                        {
                            c.ollama_embed_model = m.name.clone();
                        }
                    }
                    self.st.models = Some(result);
                }
                JobMsg::PullProgress { status, fraction } => {
                    if let Some(p) = &mut self.st.pull {
                        (p.1, p.2) = (status, fraction);
                    }
                }
                JobMsg::PullDone(result) => {
                    self.st.pull = None;
                    match result {
                        Ok(model) => self.toast(Icon::PackageCheck, tf("{model} téléchargé", &[("model", &model)]), Some(tr("Il apparaît dans la liste des modèles.").into())),
                        Err(e) => self.toast(Icon::TriangleAlert, tr("Téléchargement impossible"), Some(e)),
                    }
                    self.actions.push(Action::ListModels);
                }
                JobMsg::Github(result) => {
                    self.st.github_loading = false;
                    if let Err(e) = &result {
                        self.toast(Icon::TriangleAlert, tr("GitHub injoignable"), Some(e.clone()));
                    }
                    self.st.github = Some(result);
                }
                JobMsg::Packages(list) => {
                    for (id, packages) in list {
                        if let Some(p) = self.data.index_of(id).and_then(|i| self.data.projects.get_mut(i))
                            && p.packages != packages
                        {
                            p.packages = packages;
                        }
                    }
                }
                JobMsg::Keywords { id, result } => {
                    if let Some(job) = &mut self.st.keywords_job {
                        job.0 += 1;
                    }
                    match result {
                        // Ceux de l'IA s'ajoutent à ceux qui existent déjà.
                        Ok(words) => {
                            if let Some(p) = self.data.index_of(id).and_then(|i| self.data.projects.get_mut(i)) {
                                let mut all = p.keywords.clone();
                                all.extend(words);
                                p.keywords = crate::classify::clean_keywords(all);
                            }
                        }
                        Err(e) => self.toast(Icon::TriangleAlert, tr("Mots-clés impossibles"), Some(e)),
                    }
                }
                JobMsg::KeywordsDone => {
                    if let Some((done, _)) = self.st.keywords_job.take() {
                        self.toast(Icon::Sparkles, tf("Mots-clés proposés pour {0} projets", &[("0", &done)]), Some(tr("Ils comptent maintenant dans la recherche par le sens.").into()));
                    }
                }
                JobMsg::GithubAccounts(result) => {
                    self.st.github_accounts_loading = false;
                    self.st.github_accounts = Some(result);
                }
                JobMsg::SetupCount { path, depth, likely, error } => screens::setup::counted(&mut self.st, &path, depth, likely, error),
                JobMsg::Embedded { model, result } => {
                    self.st.semantic_busy = false;
                    match result {
                        Ok(items) if model == self.data.config.ollama_embed_model => {
                            if self.semantic.model != model {
                                self.semantic = SemanticIndex { model, ..Default::default() };
                            }
                            for (id, hash, v) in items {
                                self.semantic.vectors.insert(id, (hash, v));
                            }
                            self.st.semantic_error = None;
                            if let Some(store) = &self.store {
                                let _ = self.semantic.save(&store.embeddings_path());
                            }
                        }
                        Ok(_) => {}
                        Err(e) => self.st.semantic_error = Some(e),
                    }
                    self.st.semantic_indexed = self.indexed_count();
                }
                JobMsg::OllayaModels(result) => {
                    self.st.ollaya_loading = false;
                    // Par défaut : decider:2b s'il est là (le plus juste en français), sinon le premier.
                    if let Ok(models) = &result
                        && self.data.config.ollaya_model.is_empty()
                        && let Some(m) = models.iter().find(|m| m.name.starts_with("decider:2b")).or(models.iter().find(|m| m.family != "router")).or(models.first())
                    {
                        self.data.config.ollaya_model = m.name.clone();
                    }
                    self.st.ollaya_models = Some(result);
                }
                JobMsg::OllayaPullProgress { status, fraction } => {
                    if let Some(p) = &mut self.st.ollaya_pull {
                        (p.1, p.2) = (status, fraction);
                    }
                }
                JobMsg::OllayaPullDone(result) => {
                    self.st.ollaya_pull = None;
                    match result {
                        Ok(model) => self.toast(Icon::PackageCheck, tf("{model} téléchargé", &[("model", &model)]), Some(tr("Il apparaît dans la liste des modèles d'Ollaya.").into())),
                        Err(e) => self.toast(Icon::TriangleAlert, tr("Téléchargement impossible"), Some(e)),
                    }
                    self.actions.push(Action::ListOllaya);
                }
                JobMsg::TaggingProgress { done, total, current } => {
                    if let Some(t) = &mut self.st.tagging {
                        (t.done, t.total, t.current) = (done, total, current);
                    }
                }
                JobMsg::Tagged { id, fingerprint, result } => match result {
                    Ok(scores) => {
                        if let Some(i) = self.data.index_of(id) {
                            self.data.apply_scores(i, &scores, fingerprint);
                        }
                        if let Some(t) = &mut self.st.tagging {
                            t.done += 1;
                        }
                    }
                    Err(e) => {
                        if let Some(t) = &mut self.st.tagging {
                            t.error = Some(e);
                        }
                    }
                },
                JobMsg::TaggingDone => {
                    if let Some(t) = &mut self.st.tagging {
                        t.finished = true;
                        t.current.clear();
                        // Une seule notification, et seulement si quelque chose a coincé.
                        if let Some(e) = t.error.clone() {
                            self.toast(Icon::TriangleAlert, tr("Classement interrompu"), Some(e));
                        }
                    }
                }
                JobMsg::OntologyProposed { model, result } => {
                    self.st.onto_generating = false;
                    match result {
                        Ok(o) => {
                            self.st.onto_error = None;
                            self.st.onto_proposal = Some((model, o));
                        }
                        Err(e) => self.st.onto_error = Some(e),
                    }
                }
                JobMsg::QueryVec { query, result } => {
                    self.queries.pending = None;
                    match result {
                        Ok(v) => {
                            self.queries.vectors.insert(query, v);
                        }
                        Err(e) => self.st.semantic_error = Some(e),
                    }
                }
            }
        }
    }

    /// Fin d'un scan : chiffres par dossier source, puis création automatique selon le mode.
    fn scan_done(&mut self, result: ScanResult) {
        self.st.scan_started = None;
        let now = model::now();
        for (i, n) in &result.per_root {
            if let Some(r) = self.data.config.roots.get_mut(*i) {
                r.found = *n;
                r.last_scan = now;
            }
        }
        // Les dossiers sources ont changé pendant le scan : ce résultat est déjà dépassé.
        if std::mem::take(&mut self.st.rescan) {
            self.st.scan = Some(result);
            self.actions.push(Action::StartScan);
            return;
        }
        // L'activité des projets suivis suit celle de leur dossier.
        for f in &result.dirs {
            if let Some(i) = self.data.find_by_folder(&f.path) {
                self.data.projects[i].idle_days = f.idle_days;
            }
        }
        // Les projets suivis sans adresse de dépôt la prennent de leur dossier Git.
        self.data.fill_remotes();
        self.refresh_packages();
        let errors = result.errors.clone();
        let likely = result.likely().count();
        let total = result.dirs.len();
        self.st.scan = Some(result);
        let fresh = self.candidates(false).len();
        // L'assistant « Dossiers sources » attendait ce scan : c'est lui qui montre la suite.
        if screens::setup::scan_finished(&mut self.st, &self.data) {
            return;
        }
        // Une seule notification par scan : le résultat, ou ce qui attend.
        let mut detail = tf("{likely} projets probables, dont {fresh} pas encore suivis", &[("likely", &likely), ("fresh", &fresh)]);
        match errors.as_slice() {
            [] => {}
            [e] => detail.push_str(&tf(" · {e}", &[("e", &e)])),
            many => detail.push_str(&tf(" · {0} dossiers sources illisibles", &[("0", &many.len())])),
        }
        let c = &self.data.config;
        let (auto, llm) = (c.auto_add, c.ollama_on && !c.ollama_model.trim().is_empty());
        if auto && (llm || (!c.mcp_on && !c.ollama_on)) {
            // Le modèle de langage d'Ollama classe ; sans aucune IA, les règles locales.
            let candidates = self.candidates(llm);
            if candidates.is_empty() {
                self.toast(Icon::FolderSearch, tf("Scan terminé : {total} dossiers", &[("total", &total)]), Some(detail));
            } else {
                // Le classement suit : sa notification dira ce qui a été créé.
                self.start_analysis(candidates);
            }
        } else if auto && !c.mcp_on && fresh > 0 {
            // Ollama sans modèle de langage : on attend, plutôt que de tout créer par les règles.
            self.toast(Icon::Sparkles, tf("{fresh} projets probables attendent l'IA", &[("fresh", &fresh)]), Some(tr("Choisis un modèle dans Configuration › IA, ou crée les fiches avec les règles.").into()));
        } else {
            // L'agent MCP classe lui-même (outil `scanner_dossiers`) : rien d'automatique ici.
            self.toast(Icon::FolderSearch, tf("Scan terminé : {total} dossiers", &[("total", &total)]), Some(detail));
        }
    }

    /// Enregistre si quelque chose a changé ; recharge si un autre processus (MCP) a écrit.
    fn sync_store(&mut self) {
        if self.st.now - self.last_sync < 0.75 {
            return;
        }
        self.last_sync = self.st.now;
        let Some(store) = &mut self.store else { return };
        if store.changed_on_disk() {
            match store.load() {
                Ok(Some(mut data)) => {
                    data.config.demo_data = false;
                    self.saved_json = serde_json::to_string_pretty(&data).unwrap_or_default();
                    let before = self.data.projects.len();
                    let selected_id = self.st.selected.and_then(|i| self.data.projects.get(i)).map(|p| p.id);
                    self.data = data;
                    self.st.selected = selected_id.and_then(|id| self.data.index_of(id));
                    self.st.draft = None;
                    let after = self.data.projects.len();
                    let detail = match after.cmp(&before) {
                        std::cmp::Ordering::Greater => tf("{0} projets ajoutés", &[("0", &(after - before))]),
                        std::cmp::Ordering::Less => tf("{0} projets retirés", &[("0", &(before - after))]),
                        std::cmp::Ordering::Equal => tr("Fiches mises à jour").to_owned(),
                    };
                    self.toast(Icon::Bot, tr("Modifié par l'agent IA"), Some(detail));
                }
                Ok(None) => {}
                Err(e) => self.toast(Icon::TriangleAlert, tr("Rechargement impossible"), Some(e)),
            }
            return;
        }
        let json = serde_json::to_string_pretty(&self.data).unwrap_or_default();
        if json != self.saved_json {
            match store.save_text(&json) {
                Ok(()) => self.saved_json = json,
                Err(e) => self.toast(Icon::TriangleAlert, tr("Enregistrement impossible"), Some(e)),
            }
        }
    }

    /// Les résultats de recherche du bureau et de la bibliothèque (flou + sémantique).
    fn update_search(&mut self) {
        let embed_model = self.data.config.ollama_embed_model.clone();
        let semantic_on = self.data.config.ollama_on && !embed_model.is_empty() && self.semantic.model == embed_model;
        let query_for = |q: &str| q.trim().to_owned();
        let (desk_q, lib_q) = (query_for(&self.st.search), query_for(&self.st.lib_search));
        let active = if self.st.screen == Screen::Library { lib_q.clone() } else { desk_q.clone() };
        // Embedding de la requête : demandé quand la frappe s'arrête (350 ms).
        if active != self.last_query.0 {
            self.last_query = (active.clone(), self.st.now);
        }
        if semantic_on && active.chars().count() >= 3 && !self.queries.vectors.contains_key(&active) && self.queries.pending.is_none() {
            if self.st.now - self.last_query.1 > 0.35 {
                self.queries.pending = Some(active.clone());
                jobs::spawn_query_embed(self.bus(), self.data.config.ollama_url.clone(), embed_model.clone(), active.clone());
            } else {
                self.ctx.request_repaint_after(std::time::Duration::from_millis(400));
            }
        }
        let hits = |q: &str, app: &Self| -> Option<Vec<Hit>> {
            if q.is_empty() {
                return None;
            }
            let vec = if semantic_on { app.queries.vectors.get(q).map(Vec::as_slice) } else { None };
            Some(search::rank(&app.data, q, vec, &app.semantic))
        };
        self.st.search_hits = hits(&desk_q, self);
        self.st.lib_hits = hits(&lib_q, self);
        self.st.query_semantic = semantic_on && self.queries.vectors.contains_key(&active);
    }

    /// Met à jour l'index sémantique (projets nouveaux ou modifiés), au plus toutes les 3 s.
    fn schedule_embeddings(&mut self) {
        let c = &self.data.config;
        if !c.ollama_on || c.ollama_embed_model.is_empty() || self.st.semantic_busy || self.st.now - self.last_embed_check < 3.0 || self.dry_run {
            return;
        }
        self.last_embed_check = self.st.now;
        if self.st.semantic_error.is_some() && self.st.now - self.last_embed_check < 30.0 {
            // Après une erreur, on retente plus tard.
            self.last_embed_check = self.st.now + 27.0;
        }
        let model = c.ollama_embed_model.clone();
        let stale = self.semantic.model != model;
        let projects = self.data.projects.iter().map(|p| (p.id, search::embed_text(&self.data, p)));
        // Les concepts de l'ontologie aussi : « logement » rejoint « Immobilier » par le sens.
        let concepts = self.data.ontology.concepts.iter().map(|c| (search::CONCEPT_KEY | c.id, self.data.ontology.concept_text(c)));
        let items: Vec<(u64, u64, String)> = projects
            .chain(concepts)
            .filter_map(|(key, text)| {
                let hash = text_hash(&text);
                let current = !stale && self.semantic.vectors.get(&key).is_some_and(|(h, _)| *h == hash);
                (!current).then_some((key, hash, text))
            })
            .collect();
        if items.is_empty() {
            return;
        }
        self.st.semantic_busy = true;
        jobs::spawn_embed(self.bus(), c.ollama_url.clone(), model, items);
    }

    fn toast(&mut self, icon: Icon, title: impl Into<String>, detail: Option<String>) {
        let title = title.into();
        // Jamais deux fois la même notification à l'écran : la plus récente remplace l'autre.
        self.toasts.retain(|t| t.title != title);
        self.toasts.push(Toast { icon, title, detail, at: self.st.now });
        if self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    // ---- barre du haut -----------------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut Ui, on_edge: bool) {
        let t = Tokens::get(ui.ctx());
        if !on_edge {
            widgets::title_bar_drag(ui, ui.max_rect());
        }
        let screen = self.st.screen;
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add_space(24.0);
            ui.label(RichText::new(tr("Rust Project")).font(FontId::new(30.0, theme::bold())).color(t.ink_strong));
            ui.add_space(14.0);
            widgets::vsep(ui, 30.0);
            ui.add_space(14.0);
            ui.spacing_mut().item_spacing.x = 6.0;
            // « À revisiter » n'est plus dans la barre : on y arrive depuis « À redécouvrir ».
            for (s, icon, label) in [(Screen::Desk, Icon::LayoutGrid, tr("Mon bureau")), (Screen::Library, Icon::Layers, tr("Bibliothèque"))] {
                if widgets::nav_item(ui, Some(icon), label, screen == s || (s == Screen::Desk && screen == Screen::Revisit)).clicked() {
                    self.actions.push(Action::Go(s));
                }
            }
            ui.add_space(6.0);
            widgets::vsep(ui, 26.0);
            ui.add_space(6.0);
            if widgets::nav_item(ui, Some(Icon::Settings), tr("Configuration"), screen == Screen::Config).clicked() {
                self.actions.push(Action::Go(Screen::Config));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                widgets::window_buttons(ui, theme::TOPBAR_H);
                ui.add_space(12.0);
                // Mise à jour : une pastille d'accent quand une version attend.
                let latest = self.st.updater.release.as_ref().map(|r| r.version.clone()).unwrap_or_default();
                let (tip, pending) = match self.st.updater.phase {
                    UpdatePhase::Available => (tf("Mise à jour disponible : {latest}", &[("latest", &latest)]), true),
                    UpdatePhase::Ready => (tr("Mise à jour prête : redémarrer pour l'appliquer").to_owned(), true),
                    UpdatePhase::Downloading => (tr("Téléchargement de la mise à jour…").to_owned(), false),
                    UpdatePhase::Checking => (tr("Recherche d'une nouvelle version…").to_owned(), false),
                    UpdatePhase::Failed => (tr("Vérification des mises à jour impossible").to_owned(), false),
                    _ => (tf("Rust Project {VERSION}", &[("VERSION", &VERSION)]), false),
                };
                let resp = widgets::icon_button_tip(ui, Icon::CircleArrowUp, tr("Mise à jour"), &tip, 34.0);
                if pending {
                    ui.painter().circle_filled(resp.rect.right_top() + vec2(-7.0, 7.0), 4.5, t.accent);
                }
                if resp.clicked() {
                    self.actions.push(Action::OpenUpdate);
                }
            });
        });
    }

    // ---- barre d'état ------------------------------------------------------------------------

    fn status_bar(&mut self, ui: &mut Ui) {
        let t = Tokens::get(ui.ctx());
        let d = &self.data;
        let total = d.projects.len();
        let archived = d.archived();
        let text = tf("{total} au total (dont {archived} archivés) · {0} non archivés : Focus {1} · Au fil de l'eau {2} · En réserve {3}", &[("total", &total), ("archived", &archived), ("0", &(total - archived)), ("1", &(d.count(Attention::Focus))), ("2", &(d.count(Attention::Flow))), ("3", &(d.count(Attention::Reserve)))]);
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add_space(24.0);
            if widgets::link(ui, &tf("Explorer les {total} projets", &[("total", &total)]), Some(Icon::ArrowRight), 14.5).clicked() {
                self.actions.push(Action::Library(None));
            }
            ui.add_space(12.0);
            widgets::vsep(ui, 18.0);
            ui.add_space(12.0);
            // Coupé par « … » plutôt que de chevaucher « Données de démonstration ».
            let font = FontId::proportional(13.0);
            let demo = self.st.store_path.is_none();
            let shown = widgets::elide_end(ui.painter(), &text, &font, ui.available_width() - if demo { 470.0 } else { 290.0 });
            ui.label(RichText::new(shown).font(font).color(t.muted)).on_hover_text(&text);
            // À droite : l'état du scan des dossiers, et le bouton pour le relancer.
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(18.0);
                ui.spacing_mut().item_spacing.x = 0.0;
                let scanning = self.st.scan_started.is_some();
                if widgets::icon_button(ui, Icon::RefreshCw, tr("Scanner les dossiers maintenant"), 28.0).clicked() && !scanning {
                    self.actions.push(Action::StartScan);
                }
                ui.add_space(6.0);
                let last = self.data.config.roots.iter().map(|r| r.last_scan).max().unwrap_or(0);
                let (text, color) = if scanning {
                    (tf("Scan en cours… {0} dossiers", &[("0", &self.st.scan_count)]), t.accent_ink)
                } else if last == 0 {
                    (tr("Aucun scan pour l'instant").to_owned(), t.muted)
                } else {
                    (tf("Dossiers scannés {0}", &[("0", &model::ago(last))]), t.muted)
                };
                let label = ui.label(RichText::new(text).size(13.0).color(color));
                if let Some(path) = &self.st.store_path {
                    label.on_hover_text(tf("Données enregistrées dans {0}", &[("0", path)]));
                }
                ui.add_space(8.0);
                let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.0, if scanning { t.accent } else { t.good_fg });
                if demo {
                    ui.add_space(14.0);
                    widgets::vsep(ui, 18.0);
                    ui.add_space(14.0);
                    ui.label(RichText::new(tr("Données de démonstration")).size(13.0).color(t.faint));
                    ui.add_space(6.0);
                    icons::show(ui, Icon::FlaskConical, 15.0, t.faint);
                }
            });
        });
    }

    // ---- actions -----------------------------------------------------------------------------

    /// Lance le classement Ollaya : ces projets, ou ceux classés avec une autre ontologie.
    fn start_tagging(&mut self, ids: Option<Vec<u64>>) {
        let c = &self.data.config;
        if c.ollaya_model.is_empty() || self.data.ontology.concepts.is_empty() || self.st.tagging.as_ref().is_some_and(|t| !t.finished) || self.dry_run {
            return;
        }
        let todo: Vec<usize> = match ids {
            Some(ids) => ids.iter().filter_map(|id| self.data.index_of(*id)).collect(),
            None => self.data.outdated_tags(),
        };
        if todo.is_empty() {
            return;
        }
        let projects = todo.iter().map(|&i| {
            let p = &self.data.projects[i];
            (p.id, p.name.clone(), crate::ontology::project_state(&self.data, p))
        });
        let tagging = Tagging { total: todo.len(), ..Default::default() };
        let control = tagging.control.clone();
        self.st.tagging = Some(tagging);
        jobs::spawn_tagging(self.bus(), c.ollaya_url.clone(), c.ollaya_model.clone(), self.data.ontology.clone(), projects.collect(), control);
    }

    /// Met en pause, reprend ou arrête le classement en cours (entre deux projets).
    fn control_tagging(&mut self, state: u8) {
        if let Some(t) = self.st.tagging.as_ref().filter(|t| !t.finished) {
            t.control.store(state, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn set_attention(&mut self, p: usize, a: Attention) -> bool {
        self.change_attention(p, a, true)
    }

    /// Change l'attention d'un projet. `notify` : une notification de confirmation (pas depuis
    /// l'onglet « Modifier », où le changement se voit déjà). Le refus (focus complet) est
    /// toujours signalé.
    fn change_attention(&mut self, p: usize, a: Attention, notify: bool) -> bool {
        let limit = self.data.config.focus_limit;
        let focus = self.data.count(Attention::Focus);
        if self.data.config.limit_focus && a == Attention::Focus && self.data.projects[p].attention != Attention::Focus && focus >= limit {
            self.toast(Icon::Crosshair, tf("Focus complet ({focus}/{limit})", &[("focus", &focus), ("limit", &limit)]), Some(tr("Passe d'abord un projet au fil de l'eau pour libérer une place.").into()));
            return false;
        }
        let next_order = self.data.projects.iter().map(|p| p.order).max().unwrap_or(0) + 1;
        let project = &mut self.data.projects[p];
        if project.attention != a && a == Attention::Focus {
            project.order = next_order;
        }
        project.attention = a;
        if a != Attention::Reserve && matches!(project.status, Status::Paused | Status::Idea) {
            project.status = Status::Active;
        }
        let name = project.name.clone();
        if notify {
            self.toast(a.icon(), format!("{name} → {}", a.label()), None);
        }
        true
    }

    /// Relit en arrière-plan les paquets de chaque projet (dépendances de ses manifestes).
    fn refresh_packages(&mut self) {
        let projects = self.data.projects.iter().map(|p| (p.id, p.folders.clone())).collect();
        jobs::spawn_packages(self.bus(), projects);
    }

    /// L'état de l'application, pour l'onglet « État » des outils de développement.
    fn debug_state(&self) -> Vec<(String, String)> {
        let d = &self.data;
        let yes_no = |b: bool| if b { tr("en cours").to_owned() } else { "—".to_owned() };
        let tags: usize = d.projects.iter().map(|p| p.tags.iter().filter(|t| t.weight >= crate::ontology::LINK_MIN).count()).sum();
        let packages: usize = d.projects.iter().map(|p| p.packages.len()).sum();
        let mut out = vec![
            (tr("Version").to_owned(), VERSION.to_owned()),
            (tr("Fichier de données").to_owned(), self.st.store_path.clone().unwrap_or_else(|| tr("démonstration (rien n'est enregistré)").to_owned())),
            (tr("Projets").to_owned(), d.projects.len().to_string()),
            (tr("Domaines").to_owned(), d.domains.len().to_string()),
            (tr("Tags (concepts) · rattachements").to_owned(), format!("{} · {tags}", d.ontology.concepts.len())),
            (tr("Fiches avec mots-clés").to_owned(), d.projects.iter().filter(|p| !p.keywords.is_empty()).count().to_string()),
            (tr("Paquets relevés").to_owned(), packages.to_string()),
            (tr("Index sémantique").to_owned(), format!("{}/{} · {}", self.st.semantic_indexed, d.projects.len(), if self.semantic.model.is_empty() { "—" } else { self.semantic.model.as_str() })),
            (tr("Ollama · agent MCP").to_owned(), format!("{} · {}", d.config.ollama_on, d.config.mcp_on)),
            (tr("Modèle de langage · d'embeddings").to_owned(), format!("{} · {}", if d.config.ollama_model.is_empty() { "—" } else { &d.config.ollama_model }, if d.config.ollama_embed_model.is_empty() { "—" } else { &d.config.ollama_embed_model })),
            (tr("Dossiers sources").to_owned(), d.config.roots.iter().map(|r| format!("{} ({})", r.path, r.depth)).collect::<Vec<_>>().join(" · ")),
            (tr("Dernier scan").to_owned(), self.st.scan.as_ref().map(|s| tf("{0} dossiers", &[("0", &s.dirs.len())])).unwrap_or_else(|| "—".into())),
            (tr("Scan").to_owned(), yes_no(self.st.scan_started.is_some())),
            (tr("Classement par l'IA").to_owned(), yes_no(self.st.analysis.as_ref().is_some_and(|a| !a.finished))),
            (tr("Classement Ollaya").to_owned(), yes_no(self.st.tagging.as_ref().is_some_and(|t| !t.finished))),
            (tr("Calcul des embeddings").to_owned(), yes_no(self.st.semantic_busy)),
            (tr("Mots-clés par Ollama").to_owned(), self.st.keywords_job.map(|(a, b)| format!("{a}/{b}")).unwrap_or_else(|| "—".into())),
            (tr("Carte : nœuds · liens").to_owned(), format!("{} · {}", self.st.graph.sim.nodes.len(), self.st.graph.sim.edges.len())),
            (tr("Écran · projet ouvert").to_owned(), format!("{:?} · {}", self.st.screen, self.st.selected.and_then(|i| d.projects.get(i)).map(|p| p.name.as_str()).unwrap_or("—"))),
        ];
        if let Some(e) = &self.st.semantic_error {
            out.push((tr("Dernière erreur d'embeddings").to_owned(), e.clone()));
        }
        for (k, toast) in self.toasts.iter().rev().enumerate() {
            out.push((tf("Notification {0}", &[("0", &(k + 1))]), format!("{} {}", toast.title, toast.detail.clone().unwrap_or_default())));
        }
        for (k, line) in self.launched.iter().rev().take(5).enumerate() {
            out.push((tf("Lancement {0}", &[("0", &(k + 1))]), line.clone()));
        }
        out
    }

    /// Écarte un dossier des scans (et de ce que le dernier scan a relevé, lui et ses sous-dossiers).
    fn ignore_folder(&mut self, path: &str) {
        let key = model::path_key(path);
        if !self.data.config.ignored.iter().any(|f| model::path_key(f) == key) {
            self.data.config.ignored.push(path.to_owned());
        }
        if let Some(scan) = &mut self.st.scan {
            let below = format!("{key}\\");
            scan.dirs.retain(|d| {
                let k = model::path_key(&d.path);
                k != key && !k.starts_with(&below)
            });
        }
    }

    /// Ouvre l'assistant « Dossiers sources » : au début, ou à l'étape « Vérifier » (après le
    /// dernier scan, ou après un nouveau scan s'il n'y en a pas encore).
    fn open_setup(&mut self, step: crate::screens::setup::Step) {
        use crate::screens::setup::{SetupState, Step};
        let mut s = SetupState::new(&self.data);
        if step != Step::Where {
            match &self.st.scan {
                Some(scan) => crate::screens::setup::start_review(&mut s, &self.data, scan),
                None => {
                    s.step = Step::Scan;
                    s.waiting = true;
                    self.actions.push(Action::StartScan);
                }
            }
        }
        let counts: Vec<(String, u32)> = if s.step == Step::Where { s.folders.iter().map(|f| (f.path.clone(), f.depth)).collect() } else { Vec::new() };
        self.st.modal = Some(Modal::Setup(Box::new(s)));
        for (path, depth) in counts {
            self.count_setup_folder(path, depth);
        }
    }

    /// Compte en arrière-plan les projets probables d'un dossier proposé par l'assistant.
    fn count_setup_folder(&mut self, path: String, depth: u32) {
        if !self.dry_run {
            jobs::spawn_setup_count(self.bus(), ScanOptions::from_config(&self.data.config), path, depth);
        }
    }

    /// Une même action sur plusieurs projets, et une seule notification pour le tout.
    fn bulk(&mut self, projects: Vec<usize>, op: BulkOp) {
        let projects: Vec<usize> = projects.into_iter().filter(|&p| p < self.data.projects.len()).collect();
        let Some(&first) = projects.first() else { return };
        let name = self.data.projects[first].name.clone();
        let n = projects.len();
        match op {
            BulkOp::Attention(a) => {
                let (mut moved, mut last) = (0, name);
                for &p in &projects {
                    let archived = self.data.projects[p].status == Status::Archived;
                    // Un projet archivé qu'on range dans une attention reprend du service.
                    if self.data.projects[p].attention == a || self.change_attention(p, a, false) {
                        if archived {
                            self.data.projects[p].status = Status::Active;
                        }
                        moved += 1;
                        last = self.data.projects[p].name.clone();
                    }
                }
                let detail = (moved < n).then(|| tf("{0} laissés de côté : le focus est complet.", &[("0", &(n - moved))]));
                match moved {
                    0 => {}
                    1 => self.toast(a.icon(), format!("{last} → {}", a.label()), detail),
                    _ => self.toast(a.icon(), tf("{0} projets → {1}", &[("0", &moved), ("1", &a.label())]), detail),
                }
            }
            BulkOp::Archive => {
                for &p in &projects {
                    self.data.projects[p].status = Status::Archived;
                }
                let title = if n == 1 { tf("« {0} » archivé", &[("0", &name)]) } else { tf("{0} projets archivés", &[("0", &n)]) };
                let detail = if n == 1 { tr("Retrouve-le dans « Archivés ».") } else { tr("Retrouve-les dans « Archivés ».") };
                self.toast(Icon::ArchiveX, title, Some(detail.into()));
            }
            BulkOp::Domain(d) => {
                // Un domaine de plus : le domaine principal (la couleur sur la carte) ne change pas.
                let Some(domain) = self.data.domains.get(d).map(|x| x.name.clone()) else { return };
                for &p in &projects {
                    self.data.add_domain(p, d);
                }
                let title = if n == 1 { format!("{name} + {domain}") } else { tf("{0} projets + {1}", &[("0", &n), ("1", &domain)]) };
                self.toast(Icon::Tag, title, Some(tr("Un domaine de plus : le domaine principal ne change pas.").into()));
            }
        }
    }

    fn apply_actions(&mut self, ctx: &egui::Context) {
        for action in std::mem::take(&mut self.actions) {
            match action {
                Action::Select(p) => {
                    if self.st.selected != Some(p) {
                        self.st.draft = None;
                        self.st.tag_query.clear();
                        self.st.tag_new = None;
                    }
                    self.st.selected = Some(p);
                }
                Action::Deselect => self.st.selected = None,
                Action::Bulk(projects, op) => self.bulk(projects, op),
                Action::OpenDevTools => self.st.devtools.open = true,
                Action::Tag(p, concept) => {
                    self.data.tag(p, concept);
                    let name = self.data.ontology.path(concept);
                    if let Some(project) = self.data.projects.get_mut(p) {
                        project.log(ActivityKind::Edit, tf("Tag ajouté : {0}", &[("0", &name)]));
                    }
                }
                Action::Untag(p, concept) => {
                    self.data.untag(p, concept);
                    let name = self.data.ontology.path(concept);
                    if let Some(project) = self.data.projects.get_mut(p) {
                        project.log(ActivityKind::Edit, tf("Tag retiré : {0}", &[("0", &name)]));
                    }
                }
                Action::MainDomain(p, d) => self.data.set_main_domain(p, d, true),
                Action::SetKeywords(p, words) => {
                    if let Some(project) = self.data.projects.get_mut(p) {
                        project.keywords = crate::classify::clean_keywords(words);
                    }
                }
                Action::SuggestKeywords(ids) => {
                    let c = &self.data.config;
                    if self.dry_run || !c.ollama_on || c.ollama_model.trim().is_empty() || self.st.keywords_job.is_some() {
                        continue;
                    }
                    let todo: Vec<(u64, String)> = self
                        .data
                        .projects
                        .iter()
                        .filter(|p| ids.as_ref().map_or(p.keywords.is_empty(), |ids| ids.contains(&p.id)))
                        .map(|p| (p.id, crate::search::embed_text(&self.data, p)))
                        .collect();
                    if todo.is_empty() {
                        self.toast(Icon::Sparkles, tr("Chaque projet a déjà ses mots-clés"), None);
                        continue;
                    }
                    self.st.keywords_job = Some((0, todo.len()));
                    jobs::spawn_keywords(self.bus(), c.ollama_url.clone(), c.ollama_model.clone(), todo);
                }
                Action::NewTag { project, facet, new_facet, name, parent } => {
                    let name = name.trim().to_owned();
                    let facet = match facet {
                        Some(f) => Some(f),
                        None if !new_facet.trim().is_empty() => {
                            Some(self.data.ontology.facet_named(new_facet.trim()).unwrap_or_else(|| self.data.ontology.add_facet(new_facet.trim(), "")))
                        }
                        None => None,
                    };
                    if let Some(facet) = facet.filter(|_| !name.is_empty()) {
                        let concept = self.data.create_concept(facet, &name, parent);
                        self.data.tag(project, concept);
                        self.st.tag_new = None;
                        self.st.tag_query.clear();
                        let path = self.data.ontology.path(concept);
                        self.toast(Icon::Tag, tf("Tag « {0} » créé", &[("0", &path)]), self.data.ontology.facet(facet).map(|f| f.name.clone()));
                    }
                }
                Action::OpenUrl(url) => {
                    let plan = launch::open_url(&url);
                    self.launched.push(plan.line.clone());
                    if !self.dry_run {
                        let tx = self.errors_tx.clone();
                        let repaint = ctx.clone();
                        let result = launch::spawn(&plan, move |err| {
                            let _ = tx.send((tr("Le navigateur").to_owned(), err));
                            repaint.request_repaint();
                        });
                        if let Err(why) = result {
                            self.toast(Icon::TriangleAlert, tr("Impossible d'ouvrir l'adresse"), Some(why));
                        }
                    }
                }
                Action::Go(s) => self.st.screen = s,
                Action::GoConfig(section) => {
                    self.st.screen = Screen::Config;
                    self.st.cfg = section;
                }
                Action::Library(attention) => {
                    self.st.screen = Screen::Library;
                    self.st.lib_attention = attention;
                    self.st.lib_domain = None;
                }
                Action::Launch(p, l) => {
                    // Lancement direct : pas de notification si tout va bien, juste l'éclat de
                    // la tuile. Une notification seulement en cas d'échec.
                    let launcher = &self.data.launchers[l];
                    let (name, icon) = (launcher.name.clone(), launcher.icon);
                    match launch::plan(launcher, &self.data.projects[p]) {
                        Err(why) => self.toast(Icon::TriangleAlert, tf("{name} n'a pas pu être lancé", &[("name", &name)]), Some(why)),
                        Ok(plan) => {
                            self.launched.push(plan.line.clone());
                            self.st.flash = Some((p, l, self.st.now));
                            if !self.dry_run {
                                let tx = self.errors_tx.clone();
                                let repaint = ctx.clone();
                                let who = name.clone();
                                let result = launch::spawn(&plan, move |err| {
                                    let _ = tx.send((who, err));
                                    repaint.request_repaint();
                                });
                                if let Err(why) = result {
                                    self.toast(icon, tf("{name} n'a pas pu être lancé", &[("name", &name)]), Some(why));
                                }
                            }
                        }
                    }
                }
                Action::Toast(icon, title, detail) => self.toast(icon, title, detail),
                Action::ToggleStep(p, s) => {
                    if let Some(step) = self.data.projects.get_mut(p).and_then(|x| x.steps.get_mut(s)) {
                        step.done = !step.done;
                        let text = tf("Étape {0} : {1}", &[("0", &if step.done { "faite" } else { "rouverte" }), ("1", &step.text)]);
                        self.data.projects[p].log(ActivityKind::Edit, text);
                    }
                }
                Action::SetAttention(p, a, notify) => {
                    if self.data.projects.get(p).is_some_and(|x| x.attention != a) {
                        self.change_attention(p, a, notify);
                    }
                }
                Action::SetStatus(p, s) => {
                    if let Some(project) = self.data.projects.get_mut(p)
                        && project.status != s
                    {
                        project.status = s;
                        project.log(ActivityKind::Edit, tf("État : {0}", &[("0", &s.label())]));
                    }
                }
                Action::OpenGallery(project) => {
                    self.st.modal = Some(Modal::Gallery(GalleryState {
                        project,
                        tab: GalleryTab::Gallery,
                        group: None,
                        search: String::new(),
                        for_all: project.is_none(),
                        draft: LauncherDraft::default(),
                    }));
                }
                Action::OpenEditor(id) => {
                    let draft = id.map(|i| LauncherDraft::from_launcher(&self.data.launchers[i])).unwrap_or_default();
                    self.st.modal = Some(Modal::Editor { launcher: id, draft });
                }
                Action::OpenAddProject => {
                    let start = self.data.config.roots.first().map(|r| format!("{}\\", r.path.trim_end_matches('\\'))).unwrap_or_default();
                    self.st.modal = Some(Modal::AddProject(AddProjectState {
                        path: start,
                        analyzing: false,
                        proposal: None,
                        by: AddedBy::Manual,
                        error: None,
                        attention: Attention::Flow,
                    }));
                }
                Action::AddLauncher { launcher, project } => {
                    let id = match self.data.launchers.iter().position(|l| l.name == launcher.name) {
                        Some(id) => id,
                        None => {
                            self.data.launchers.push(launcher.clone());
                            self.data.launchers.len() - 1
                        }
                    };
                    let target = match project {
                        Some(p) => {
                            let project = &mut self.data.projects[p];
                            project.hidden.retain(|h| *h != id);
                            if !project.launchers.contains(&id) && !self.data.default_launchers.contains(&id) {
                                project.launchers.push(id);
                            }
                            tf("Pour {0}", &[("0", &project.name)])
                        }
                        None => {
                            if !self.data.default_launchers.contains(&id) {
                                self.data.default_launchers.push(id);
                            }
                            tr("Sur tous les projets").to_owned()
                        }
                    };
                    self.toast(launcher.icon, tf("{0} ajouté", &[("0", &launcher.name)]), Some(target));
                }
                Action::SaveLauncher(id, launcher) => {
                    let name = launcher.name.clone();
                    self.data.launchers[id] = launcher;
                    self.toast(Icon::Check, tf("« {name} » enregistré", &[("name", &name)]), None);
                }
                Action::RemoveDefault(id) => {
                    self.data.default_launchers.retain(|l| *l != id);
                    let name = self.data.launchers[id].name.clone();
                    self.toast(Icon::Minus, tf("« {name} » n'est plus affiché par défaut", &[("name", &name)]), None);
                }
                Action::MoveDefault(id, delta) => {
                    let list = &mut self.data.default_launchers;
                    if let Some(i) = list.iter().position(|l| *l == id) {
                        let j = (i as i32 + delta).clamp(0, list.len() as i32 - 1) as usize;
                        list.swap(i, j);
                    }
                }
                Action::SetFavorite(p, l) => {
                    self.data.projects[p].favorite = Some(l);
                    let name = self.data.launchers[l].name.clone();
                    self.toast(Icon::Star, tf("{name} mis en avant", &[("name", &name)]), Some(tf("Pour {0}", &[("0", &self.data.projects[p].name)])));
                }
                Action::MoveInProject(p, l, delta) => {
                    let list = &mut self.data.projects[p].launchers;
                    if let Some(i) = list.iter().position(|x| *x == l) {
                        let j = (i as i32 + delta).clamp(0, list.len() as i32 - 1) as usize;
                        list.swap(i, j);
                    } else {
                        self.toast(Icon::Info, tr("Lanceur par défaut"), Some(tr("Son ordre se règle dans Configuration › Lanceurs.").into()));
                    }
                }
                Action::HideForProject(p, l) => {
                    let project = &mut self.data.projects[p];
                    if project.launchers.contains(&l) {
                        project.launchers.retain(|x| *x != l);
                    } else {
                        project.hidden.push(l);
                    }
                    let (name, pname) = (self.data.launchers[l].name.clone(), project.name.clone());
                    self.toast(Icon::EyeOff, tf("{name} masqué", &[("name", &name)]), Some(tf("Pour {pname}", &[("pname", &pname)])));
                }
                Action::MoveFocus(p, delta) => {
                    let ids = self.data.focus();
                    if let Some(i) = ids.iter().position(|x| *x == p) {
                        let j = (i as i32 + delta).clamp(0, ids.len() as i32 - 1) as usize;
                        if i != j {
                            let (a, b) = (self.data.projects[ids[i]].order, self.data.projects[ids[j]].order);
                            self.data.projects[ids[i]].order = b;
                            self.data.projects[ids[j]].order = a;
                        }
                    }
                }
                Action::SaveDraft(p, d) => {
                    let Some(project) = self.data.projects.get_mut(p) else { continue };
                    let attention_changed = project.attention != d.attention;
                    let folders_changed = project.folders != d.folders;
                    project.name = d.name;
                    project.description = d.description;
                    project.outcome = d.outcome;
                    project.objective = d.objective;
                    project.next_action = d.next_action;
                    project.status = d.status;
                    project.domain = d.domain;
                    project.icon = d.icon;
                    project.logo = d.logo;
                    project.folders = d.folders;
                    project.remote = (!d.remote.is_empty()).then_some(d.remote);
                    // De nouveaux dossiers : leur dépôt Git, si l'adresse manque encore.
                    if folders_changed && project.remote.is_none() {
                        project.remote = scan::remote_of(&project.folders);
                    }
                    // Une ligne de journal par séance d'édition, pas une par touche frappée.
                    const EDITED: &str = "Fiche modifiée";
                    match project.activity.first_mut() {
                        Some(a) if a.kind == ActivityKind::Edit && a.text == EDITED && model::now().saturating_sub(a.at) < 600 => a.at = model::now(),
                        _ => project.log(ActivityKind::Edit, EDITED),
                    }
                    if attention_changed {
                        self.change_attention(p, d.attention, false);
                    }
                    // Le formulaire montre tout de suite ce qui a été complété (adresse du dépôt…).
                    ctx.request_repaint();
                }
                Action::Decide(p, decision) => {
                    let applied = match decision {
                        Decision::Focus => self.set_attention(p, Attention::Focus),
                        Decision::Flow => self.set_attention(p, Attention::Flow),
                        Decision::Archive => {
                            self.data.projects[p].status = Status::Archived;
                            let name = self.data.projects[p].name.clone();
                            self.toast(Icon::Archive, tf("{name} archivé", &[("name", &name)]), Some(tr("Il reste consultable dans la bibliothèque.").into()));
                            true
                        }
                        Decision::Keep | Decision::Later => true,
                    };
                    if applied {
                        self.st.decisions.push((p, decision));
                        self.st.revisit = None;
                    }
                }
                Action::StartScan => {
                    if self.st.scan_started.is_some() {
                        self.st.rescan = true;
                        continue;
                    }
                    let opts = ScanOptions::from_config(&self.data.config);
                    if opts.roots.is_empty() {
                        self.toast(Icon::FolderSearch, tr("Aucun dossier source"), Some(tr("Ajoute un dossier dans Configuration › Dossiers sources.").into()));
                        continue;
                    }
                    self.st.scan_started = Some(self.st.now);
                    self.st.scan_count = 0;
                    jobs::spawn_scan(self.bus(), opts);
                }
                Action::AnalyzeFolder(path) => {
                    let opts = ScanOptions::from_config(&self.data.config);
                    jobs::spawn_analyze_one(self.bus(), path, opts, self.classifier());
                }
                Action::AddProposal(proposal, by, attention) => match classify::apply(&mut self.data, &proposal, by) {
                    Some(i) => {
                        self.data.projects[i].attention = attention;
                        if attention == Attention::Focus {
                            self.data.projects[i].order = self.data.projects.iter().map(|p| p.order).max().unwrap_or(0) + 1;
                        }
                        let name = self.data.projects[i].name.clone();
                        self.st.selected = Some(i);
                        self.toast(Icon::CirclePlus, tf("{name} ajouté", &[("name", &name)]), Some(proposal.description.clone()));
                    }
                    None => self.toast(Icon::Info, tr("Ce dossier est déjà suivi"), proposal.folder.clone()),
                },
                Action::AskDelete(p) => self.st.modal = Some(Modal::ConfirmDelete(p)),
                Action::DeleteProject(p) => {
                    if p < self.data.projects.len() {
                        let removed = self.data.projects.remove(p);
                        self.semantic.vectors.remove(&removed.id);
                        // Les index suivants ont décalé : on désélectionne et on oublie ce qui en dépend.
                        self.st.selected = None;
                        self.st.draft = None;
                        self.st.flash = None;
                        self.st.revisit = None;
                        self.st.decisions.clear();
                        self.toast(Icon::Trash2, tf("{0} supprimé", &[("0", &removed.name)]), Some(tr("La fiche seulement : le dossier n'est pas touché.").into()));
                    }
                }
                Action::AddStep(p, text) => {
                    let text = text.trim().to_owned();
                    if !text.is_empty()
                        && let Some(project) = self.data.projects.get_mut(p)
                    {
                        project.steps.push(Step { text: text.clone(), done: false });
                        project.log(ActivityKind::Edit, tf("Étape ajoutée : {text}", &[("text", &text)]));
                    }
                }
                Action::RemoveStep(p, s) => {
                    if let Some(project) = self.data.projects.get_mut(p)
                        && s < project.steps.len()
                    {
                        let step = project.steps.remove(s);
                        project.log(ActivityKind::Edit, tf("Étape retirée : {0}", &[("0", &step.text)]));
                    }
                }
                Action::AddNote(p, text) => {
                    if let Some(project) = self.data.projects.get_mut(p) {
                        project.notes.insert(0, text.clone());
                        project.log(ActivityKind::Note, tf("Note : {text}", &[("text", &text)]));
                    }
                }
                Action::PickRoot => {
                    let start = self.data.config.roots.first().map(|r| r.path.clone()).unwrap_or_else(model::default_root);
                    if !self.dry_run
                        && let Some(dir) = rfd::FileDialog::new().set_title(tr("Dossier qui contient des projets")).set_directory(start).pick_folder()
                    {
                        let path = dir.display().to_string();
                        if self.data.config.roots.iter().any(|r| model::path_key(&r.path) == model::path_key(&path)) {
                            self.toast(Icon::Info, tr("Ce dossier est déjà un dossier source"), Some(path));
                        } else {
                            self.data.config.roots.push(ScanRoot { path: path.clone(), ..Default::default() });
                            self.toast(Icon::FolderPlus, tr("Dossier source ajouté"), Some(path));
                            // On relève tout de suite ce qu'il contient.
                            self.actions.push(Action::StartScan);
                        }
                    }
                }
                Action::RemoveRoot(i) => {
                    if i < self.data.config.roots.len() {
                        let r = self.data.config.roots.remove(i);
                        self.toast(Icon::Trash2, tr("Dossier source retiré"), Some(tf("{0} — ses projets restent suivis.", &[("0", &r.path)])));
                        // Les chiffres du scan ne valent plus : on relève ce qui reste.
                        self.st.scan = None;
                        if !self.data.config.roots.is_empty() {
                            self.actions.push(Action::StartScan);
                        }
                    }
                }
                Action::OpenSetup(step) => self.open_setup(step),
                Action::SetupCount(path, depth) => self.count_setup_folder(path, depth),
                Action::SetupAddFolder => {
                    let picked = if self.dry_run { None } else { rfd::FileDialog::new().set_title(tr("Dossier qui contient des projets")).set_directory(model::default_root()).pick_folder() };
                    if let Some(dir) = picked {
                        let path = dir.display().to_string();
                        let mut count = None;
                        if let Some(Modal::Setup(s)) = &mut self.st.modal {
                            match s.folders.iter_mut().find(|f| model::path_key(&f.path) == model::path_key(&path)) {
                                Some(f) => f.checked = true,
                                None => {
                                    s.folders.push(crate::screens::setup::Folder::new(crate::screens::setup::short_name(&path), path.clone(), true, 3));
                                    count = Some(path);
                                }
                            }
                        }
                        if let Some(path) = count {
                            self.count_setup_folder(path, 3);
                        }
                    }
                }
                Action::SetupScan => {
                    if let Some(Modal::Setup(s)) = &mut self.st.modal {
                        let old = std::mem::take(&mut self.data.config.roots);
                        self.data.config.roots = s
                            .folders
                            .iter()
                            .filter(|f| f.checked)
                            .map(|f| {
                                let prev = old.iter().find(|r| model::path_key(&r.path) == model::path_key(&f.path) && r.depth == f.depth);
                                ScanRoot { path: f.path.clone(), enabled: true, depth: f.depth, found: prev.map(|r| r.found).unwrap_or(0), last_scan: prev.map(|r| r.last_scan).unwrap_or(0) }
                            })
                            .collect();
                        s.step = crate::screens::setup::Step::Scan;
                        s.waiting = true;
                        self.st.scan = None;
                        self.actions.push(Action::StartScan);
                    }
                }
                Action::SetupCreate { keep, ignore, with_ai } => {
                    let found: Vec<Found> = self.st.scan.as_ref().map(|s| s.dirs.iter().filter(|d| keep.contains(&d.path)).cloned().collect()).unwrap_or_default();
                    for path in &ignore {
                        self.ignore_folder(path);
                    }
                    if with_ai && !found.is_empty() {
                        let classifier = self.classifier();
                        self.st.analysis = Some(Analysis { total: found.len(), with_ai: true, force: true, ..Default::default() });
                        jobs::spawn_analysis(self.bus(), found, classifier);
                    } else {
                        let mut added = 0;
                        for f in &found {
                            let mut p = classify::by_rules(f);
                            p.is_project = true;
                            if classify::apply(&mut self.data, &p, AddedBy::Rules).is_some() {
                                added += 1;
                            }
                        }
                        if let Some(Modal::Setup(s)) = &mut self.st.modal {
                            s.created = Some(added);
                        }
                    }
                }
                Action::RestoreFolder(i) => {
                    if i < self.data.config.ignored.len() {
                        let path = self.data.config.ignored.remove(i);
                        self.toast(Icon::Eye, tr("Dossier rétabli"), Some(tf("{path} — il réapparaîtra au prochain scan.", &[("path", &path)])));
                    }
                }
                Action::AddAllLikely => {
                    let found: Vec<Found> = self.candidates(false);
                    let mut added = 0;
                    for f in &found {
                        if classify::apply(&mut self.data, &classify::by_rules(f), AddedBy::Rules).is_some() {
                            added += 1;
                        }
                    }
                    self.toast(Icon::CirclePlus, tf("{added} projets ajoutés", &[("added", &added)]), Some(tr("Reconnus par leurs marqueurs (.git, Cargo.toml…).").into()));
                }
                Action::AnalyzeCandidates => {
                    let with_ai = self.classifier().ollama.is_some();
                    let candidates = self.candidates(with_ai);
                    self.start_analysis(candidates);
                }
                Action::ListGithub => {
                    self.st.github_loading = true;
                    let projects = self.data.projects.iter().map(|p| (p.id, p.folders.clone())).collect();
                    jobs::spawn_github(self.bus(), self.data.config.github_account.clone(), projects);
                }
                Action::ListGithubAccounts => {
                    if !self.dry_run && !self.st.github_accounts_loading {
                        self.st.github_accounts_loading = true;
                        jobs::spawn_github_accounts(self.bus());
                    }
                }
                Action::AddRepo(full_name) => {
                    if let Some(Ok(repos)) = &self.st.github
                        && let Some(repo) = repos.iter().find(|r| r.full_name == full_name)
                    {
                        let mut p = crate::github::proposal(repo);
                        p.is_project = true;
                        if let Some(i) = classify::apply(&mut self.data, &p, AddedBy::Manual) {
                            self.toast(Icon::GitBranch, tf("{0} ajouté", &[("0", &self.data.projects[i].name)]), Some(repo.html_url.clone()));
                        }
                    }
                }
                Action::AddAllRepos => {
                    let mut added = 0;
                    if let Some(Ok(repos)) = self.st.github.clone() {
                        let todo: Vec<&Repo> = repos.iter().filter(|r| !r.fork && crate::github::local_match(&self.data, r).is_none()).collect();
                        for repo in todo {
                            if classify::apply(&mut self.data, &crate::github::proposal(repo), AddedBy::Rules).is_some() {
                                added += 1;
                            }
                        }
                    }
                    self.toast(Icon::GitBranch, tf("{added} dépôts ajoutés", &[("added", &added)]), Some(tr("Sans dossier local : ils sont marqués « GitHub ».").into()));
                }
                Action::ListModels => {
                    self.st.models_loading = true;
                    jobs::spawn_models(self.bus(), self.data.config.ollama_url.clone());
                }
                Action::PullModel(name) => {
                    let name = name.trim().to_owned();
                    if !name.is_empty() && self.st.pull.is_none() {
                        self.st.pull = Some((name.clone(), tr("démarrage…").into(), -1.0));
                        jobs::spawn_pull(self.bus(), self.data.config.ollama_url.clone(), name);
                    }
                }
                Action::ListOllaya => {
                    self.st.ollaya_loading = true;
                    jobs::spawn_ollaya_models(self.bus(), self.data.config.ollaya_url.clone());
                }
                Action::PullOllaya(name) => {
                    let name = name.trim().to_owned();
                    if !name.is_empty() && self.st.ollaya_pull.is_none() {
                        self.st.ollaya_pull = Some((name.clone(), tr("démarrage…").into(), -1.0));
                        jobs::spawn_ollaya_pull(self.bus(), self.data.config.ollaya_url.clone(), name);
                    }
                }
                Action::TagProjects(ids) => self.start_tagging(ids),
                Action::ControlTagging(state) => self.control_tagging(state),
                Action::GenerateOntology => {
                    let c = &self.data.config;
                    if !c.ollama_model.is_empty() && !self.st.onto_generating {
                        self.st.onto_generating = true;
                        self.st.onto_error = None;
                        jobs::spawn_generate_ontology(self.bus(), c.ollama_url.clone(), c.ollama_model.clone(), crate::ontology::generation_prompt(&self.data));
                    }
                }
                Action::ApplyProposal(mode) => {
                    if let Some((model, proposal)) = self.st.onto_proposal.take() {
                        self.data.merge_ontology(&proposal, mode, &model);
                    }
                }
                Action::DiscardProposal => self.st.onto_proposal = None,
                Action::Reindex => {
                    self.semantic = SemanticIndex { model: self.data.config.ollama_embed_model.clone(), ..Default::default() };
                    self.st.semantic_indexed = 0;
                    self.st.semantic_error = None;
                    self.last_embed_check = -10.0;
                }
                Action::CopyText(text) => {
                    ctx.copy_text(text);
                    self.toast(Icon::Copy, tr("Copié dans le presse-papiers"), None);
                }
                Action::SetDemo(on) => self.set_demo(on),
                Action::SetLanguage(lang) => {
                    self.data.config.language = lang;
                    crate::i18n::set(lang.unwrap_or_else(crate::i18n::detect));
                    theme::install_fonts(ctx);
                }
                Action::ApplyTheme => {
                    let c = &self.data.config;
                    theme::apply(ctx, c.theme, c.accent, c.compact);
                }
                Action::ApplyZoom => ctx.set_zoom_factor(self.data.config.zoom),
                Action::OpenUpdate => {
                    self.st.modal = Some(Modal::Update);
                    // La vérification part dès l'ouverture : pas besoin de cliquer « Vérifier
                    // maintenant ». Une version déjà trouvée (ou téléchargée) reste affichée.
                    if !self.dry_run && matches!(self.st.updater.phase, UpdatePhase::Idle | UpdatePhase::UpToDate | UpdatePhase::Failed) {
                        self.st.updater.check(ctx, self.data.config.update_beta);
                    }
                }
                Action::CheckUpdate => {
                    if !self.dry_run {
                        self.st.updater.check(ctx, self.data.config.update_beta);
                    }
                }
                Action::DownloadUpdate => {
                    if !self.dry_run {
                        let dir = Store::default_path().parent().map(|p| p.join("updates")).unwrap_or_else(std::env::temp_dir);
                        self.st.updater.download(ctx, dir);
                    }
                }
                Action::Restart => {
                    // On met la nouvelle version en place (si elle est téléchargée), on relance
                    // l'exécutable, puis on ferme celui-ci.
                    self.restart_requested = true;
                    if !self.dry_run {
                        if let (UpdatePhase::Ready, Some(path)) = (self.st.updater.phase, self.st.updater.downloaded.clone())
                            && let Err(why) = crate::release::install(&path)
                        {
                            self.toast(Icon::TriangleAlert, tr("Installation impossible"), Some(why));
                            continue;
                        }
                        match launch::restart() {
                            Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                            Err(why) => self.toast(Icon::TriangleAlert, tr("Redémarrage impossible"), Some(why)),
                        }
                    }
                }
            }
        }
    }

    // ---- fenêtres modales et notifications ---------------------------------------------------

    fn modal(&mut self, ctx: &egui::Context) {
        let Some(mut modal) = self.st.modal.take() else { return };
        let keep = match &mut modal {
            Modal::Gallery(g) => launchers::gallery(ctx, &self.data, g, &mut self.actions),
            Modal::Editor { launcher, draft } => launchers::editor(ctx, &self.data, *launcher, draft, &mut self.actions),
            Modal::AddProject(a) => screens::desk::add_project(ctx, &self.data, a, &mut self.actions),
            Modal::Update => update::modal(ctx, &self.data, &self.st, &mut self.actions),
            Modal::ConfirmDelete(p) => confirm_delete(ctx, &self.data, *p, &mut self.actions),
            Modal::Setup(s) => screens::setup::show(ctx, &self.data, &self.st, s, &mut self.actions),
        };
        if keep {
            self.st.modal = Some(modal);
        }
    }

    fn show_toasts(&mut self, ctx: &egui::Context) {
        let now = self.st.now;
        self.toasts.retain(|t| now - t.at < TOAST_SECS);
        if self.toasts.is_empty() {
            return;
        }
        let t = Tokens::get(ctx);
        egui::Area::new(Id::unique("toasts"))
            .order(egui::Order::Foreground)
            .anchor(Align2::RIGHT_BOTTOM, vec2(-20.0, -(theme::STATUS_H + 14.0)))
            .interactable(false)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for toast in &self.toasts {
                    let age = now - toast.at;
                    ui.scope(|ui| {
                        ui.set_opacity(((age / 0.15).min(1.0) * ((TOAST_SECS - age) / 0.4).min(1.0)) as f32);
                        Frame::new()
                            .fill(t.raised)
                            .stroke(Stroke::new(1.0, t.line_strong))
                            .corner_radius(10)
                            .inner_margin(Margin::symmetric(14, 12))
                            .shadow(theme::shadow(&t))
                            .show(ui, |ui| {
                                ui.set_max_width(440.0);
                                ui.horizontal(|ui| {
                                    widgets::icon_tile(ui, toast.icon, t.accent_ink, 34.0);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.label(RichText::new(&toast.title).font(FontId::new(14.5, theme::semibold())).color(t.ink_strong));
                                        if let Some(detail) = &toast.detail {
                                            // En mono seulement pour une commande ou un chemin.
                                            let code = detail.contains(['"', '\\', '/', '{']);
                                            let font = if code { FontId::monospace(12.0) } else { FontId::proportional(13.0) };
                                            ui.label(RichText::new(detail).font(font).color(t.muted));
                                        }
                                    });
                                });
                            });
                    });
                }
            });
        ctx.request_repaint();
    }
}

impl eframe::App for AtelierApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.st.now = ctx.input(|i| i.time);
        // Erreurs remontées par les programmes lancés.
        while let Ok((who, err)) = self.errors_rx.try_recv() {
            self.toast(Icon::TriangleAlert, tf("{who} a échoué", &[("who", &who)]), Some(err));
        }
        // Mise à jour : les réponses de GitHub et la progression du téléchargement.
        self.st.updater.poll();
        self.process_jobs();
        self.sync_store();
        self.schedule_embeddings();
        self.update_search();
        // Un index de projet périmé (suppression par l'agent MCP…) ne doit rien casser.
        if self.st.selected.is_some_and(|i| i >= self.data.projects.len()) {
            self.st.selected = None;
            self.st.draft = None;
        }
        if self.st.scan_started.is_some() || self.st.analysis.as_ref().is_some_and(|a| !a.finished) || self.st.pull.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        // Le fichier est relu régulièrement (changements faits par l'agent MCP).
        if self.store.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        let edge = widgets::edge_under_pointer(&ctx);
        let t = Tokens::get(&ctx);

        egui::Panel::top("topbar").exact_size(theme::TOPBAR_H).frame(Frame::new().fill(t.bg)).show(ui, |ui| self.top_bar(ui, edge.is_some()));
        egui::Panel::bottom("statusbar").exact_size(theme::STATUS_H).frame(Frame::new().fill(t.bg)).show(ui, |ui| self.status_bar(ui));

        // Le panneau projet accompagne le bureau et la bibliothèque ; « À revisiter » a sa propre fiche.
        if let Some(p) = self.st.selected.filter(|_| matches!(self.st.screen, Screen::Desk | Screen::Library)) {
            egui::Panel::right("detail")
                .exact_size(theme::PANEL_W)
                .resizable(false)
                .show_separator_line(false)
                .frame(Frame::new().fill(t.bg).inner_margin(Margin { left: 0, right: 16, top: 16, bottom: 14 }))
                .show(ui, |ui| {
                    // Une zone fixe : un contenu trop large y est coupé au lieu d'élargir le
                    // panneau (egui décalerait alors la zone centrale par-dessus).
                    let rect = ui.max_rect();
                    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
                    inner.shrink_clip_rect(rect);
                    panel::show(&mut inner, &self.data, &mut self.st, &mut self.actions, p);
                    ui.advance_cursor_after_rect(rect);
                });
        }

        let margin = Margin { left: 24, right: 20, top: 18, bottom: 10 };
        egui::CentralPanel::default().frame(Frame::new().fill(t.bg).inner_margin(margin)).show(ui, |ui| match self.st.screen {
            Screen::Desk => screens::desk::show(ui, &self.data, &mut self.st, &mut self.actions),
            Screen::Library => screens::library::show(ui, &self.data, &mut self.st, &mut self.actions),
            Screen::Revisit => screens::revisit::show(ui, &self.data, &mut self.st, &mut self.actions),
            Screen::Config => screens::config::show(ui, &mut self.data, &mut self.st, &mut self.actions),
        });

        self.modal(&ctx);
        self.apply_actions(&ctx);
        self.show_toasts(&ctx);
        // Les outils de développement : F12 (ou Ctrl + Maj + I).
        if crate::devtools::toggled(&ctx) {
            self.st.devtools.open = !self.st.devtools.open;
        }
        if self.st.devtools.open {
            let state = self.debug_state();
            crate::devtools::show(&ctx, &mut self.st.devtools, &state);
        }

        // Un filet autour de la fenêtre sans bordure.
        if !ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::unique("window-border")));
            painter.rect_stroke(ctx.viewport_rect(), CornerRadius::ZERO, Stroke::new(1.0, tint(t.line_strong, 0.9)), StrokeKind::Inside);
        }
        widgets::resize_from_edge(&ctx, edge);
    }
}

/// Confirmer la suppression d'un projet. Renvoie `false` quand la fenêtre se ferme.
fn confirm_delete(ctx: &egui::Context, data: &Data, p: usize, act: &mut Vec<Action>) -> bool {
    let Some(project) = data.projects.get(p) else { return false };
    let t = Tokens::get(ctx);
    let mut open = true;
    let frame = Frame::new().fill(t.surface).stroke(Stroke::new(1.0, t.line_strong)).corner_radius(16).shadow(theme::shadow(&t)).inner_margin(Margin::same(24));
    let modal = egui::Modal::new(Id::unique("confirm-delete")).backdrop_color(theme::scrim(&t)).frame(frame).show(ctx, |ui| {
        ui.set_width(440.0);
        ui.horizontal(|ui| {
            widgets::icon_tile(ui, Icon::Trash2, t.danger, 40.0);
            widgets::title(ui, &tf("Supprimer « {0} » ?", &[("0", &project.name)]), 20.0);
        });
        ui.add_space(8.0);
        widgets::muted(ui, tr("La fiche, ses étapes et ses notes disparaissent de Rust Project. Le dossier sur le disque n'est pas touché ; un prochain scan pourra le proposer à nouveau."));
        ui.add_space(14.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::button(ui, Some(Icon::Trash2), tr("Supprimer"), Kind::Danger).clicked() {
                act.push(Action::DeleteProject(p));
                open = false;
            }
            if widgets::button(ui, None, tr("Annuler"), Kind::Secondary).clicked() {
                open = false;
            }
        });
    });
    open && !modal.should_close()
}

/// Un bouton « primaire » de largeur pleine, utilisé dans plusieurs fenêtres.
pub fn footer_buttons(ui: &mut Ui, primary: &str, primary_enabled: bool) -> (bool, bool) {
    let mut cancel = false;
    let mut ok = false;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ok = ui.add_enabled_ui(primary_enabled, |ui| widgets::button(ui, None, primary, Kind::Primary)).inner.clicked();
        cancel = widgets::button(ui, None, tr("Annuler"), Kind::Secondary).clicked();
    });
    (cancel, ok)
}
