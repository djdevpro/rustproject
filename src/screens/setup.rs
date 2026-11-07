//! L'assistant « Dossiers sources », pas à pas : 1. où chercher, 2. le scan, 3. vérifier ce qui a
//! été trouvé, 4. créer les fiches. Il s'ouvre au premier lancement, et depuis Configuration ›
//! Dossiers sources (au début, ou directement à l'étape « Vérifier » après un scan).

use egui::{Align, FontFamily, FontId, Frame, Id, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui, vec2};

use crate::app::{Action, UiState};
use crate::i18n::{tf, tr};
use crate::icons::{self, Icon};
use crate::model::{Data, path_key};
use crate::scan::{Found, ScanResult};
use crate::select::MultiSelect;
use crate::theme::{self, Tokens, tint};
use crate::widgets::{self, Kind, Tone};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Where,
    Scan,
    Review,
    Create,
}

impl Step {
    const ALL: [Self; 4] = [Self::Where, Self::Scan, Self::Review, Self::Create];

    fn label(self) -> &'static str {
        match self {
            Self::Where => tr("Où chercher"),
            Self::Scan => tr("Scan"),
            Self::Review => tr("Vérifier"),
            Self::Create => tr("Créer les fiches"),
        }
    }
}

/// Un dossier proposé à l'étape « Où chercher ».
#[derive(Clone, Debug)]
pub struct Folder {
    pub label: String,
    pub path: String,
    pub checked: bool,
    pub depth: u32,
    /// Projets probables qu'il contient (`None` : comptage en cours ou pas demandé).
    pub likely: Option<usize>,
    pub error: Option<String>,
}

impl Folder {
    pub fn new(label: String, path: String, checked: bool, depth: u32) -> Self {
        Self { label, path, checked, depth: depth.clamp(1, 3), likely: None, error: None }
    }
}

pub struct SetupState {
    pub step: Step,
    pub folders: Vec<Folder>,
    /// Le scan lancé par l'assistant n'est pas encore revenu.
    pub waiting: bool,
    /// « Vérifier » : les dossiers gardés (chemins), et les lignes sélectionnées à la souris.
    pub keep: Vec<String>,
    pub picked: MultiSelect<String>,
    /// Montrer aussi les dossiers sans marqueur (pas des projets probables).
    pub show_all: bool,
    /// Écarter des prochains scans les projets probables décochés.
    pub ignore_rest: bool,
    pub with_ai: bool,
    /// Fiches créées par les règles ; avec l'IA, le résultat vient de l'analyse en cours.
    pub created: Option<usize>,
    pub ai_started: bool,
}

impl SetupState {
    /// L'assistant au début : les dossiers sources actuels (cochés), puis les dossiers où l'on
    /// range souvent ses projets (rien n'est coché d'office).
    pub fn new(data: &Data) -> Self {
        let mut folders: Vec<Folder> = data.config.roots.iter().map(|r| Folder::new(short_name(&r.path), r.path.clone(), r.enabled, r.depth)).collect();
        for (label, path) in crate::model::suggested_roots() {
            if !folders.iter().any(|f| path_key(&f.path) == path_key(&path)) {
                folders.push(Folder::new(label, path, false, 3));
            }
        }
        Self { step: Step::Where, folders, waiting: false, keep: Vec::new(), picked: MultiSelect::default(), show_all: false, ignore_rest: true, with_ai: false, created: None, ai_started: false }
    }

    fn checked(&self) -> impl Iterator<Item = &Folder> {
        self.folders.iter().filter(|f| f.checked)
    }
}

/// Le dernier élément d'un chemin (« Documents »), ou le chemin entier.
pub fn short_name(path: &str) -> String {
    let p = path.trim().trim_end_matches(['\\', '/']);
    p.rsplit(['\\', '/']).next().filter(|s| !s.is_empty()).unwrap_or(p).to_owned()
}

/// Un comptage de projets probables est revenu (même chemin et même profondeur).
pub fn counted(st: &mut UiState, path: &str, depth: u32, likely: usize, error: Option<String>) {
    if let Some(crate::app::Modal::Setup(s)) = &mut st.modal
        && let Some(f) = s.folders.iter_mut().find(|f| f.path == path && f.depth == depth)
    {
        f.likely = Some(likely);
        f.error = error;
    }
}

/// Les dossiers relevés à montrer à l'étape « Vérifier » : pas encore suivis, pas des briques ;
/// les projets probables seulement, sauf `show_all`. Par dossier source, les probables d'abord.
pub fn review_rows<'a>(data: &Data, scan: &'a ScanResult, show_all: bool) -> Vec<&'a Found> {
    let mut rows: Vec<&Found> = scan.dirs.iter().filter(|f| f.brick_of.is_none() && data.find_by_folder(&f.path).is_none() && (show_all || f.likely_project)).collect();
    rows.sort_by_key(|f| (f.root, !f.likely_project, f.path.to_lowercase()));
    rows
}

/// Fin d'un scan : si l'assistant l'attendait, il passe à « Vérifier » (projets probables cochés).
/// Renvoie `true` quand l'assistant prend la suite (pas de création automatique, pas de
/// notification).
pub fn scan_finished(st: &mut UiState, data: &Data) -> bool {
    let UiState { modal, scan, .. } = st;
    let (Some(crate::app::Modal::Setup(s)), Some(scan)) = (modal.as_mut(), scan.as_ref()) else { return false };
    if !s.waiting {
        // Ouvert sans avoir lancé ce scan : on ne crée rien dans son dos.
        return true;
    }
    s.waiting = false;
    start_review(s, data, scan);
    true
}

/// Prépare l'étape « Vérifier » : chaque projet probable est coché.
pub fn start_review(s: &mut SetupState, data: &Data, scan: &ScanResult) {
    s.step = Step::Review;
    s.keep = review_rows(data, scan, false).iter().map(|f| f.path.clone()).collect();
    s.picked.clear();
    s.created = None;
    s.ai_started = false;
}

/// La fenêtre de l'assistant. Renvoie `false` quand elle se ferme.
pub fn show(ctx: &egui::Context, data: &Data, st: &UiState, s: &mut SetupState, act: &mut Vec<Action>) -> bool {
    let t = Tokens::get(ctx);
    let mut open = true;
    let frame = Frame::new().fill(t.surface).stroke(Stroke::new(1.0, t.line_strong)).corner_radius(16).shadow(theme::shadow(&t)).inner_margin(Margin::same(26));
    let modal = egui::Modal::new(Id::unique("setup-wizard")).backdrop_color(theme::scrim(&t)).frame(frame).show(ctx, |ui| {
        let width = (ctx.viewport_rect().width() - 120.0).clamp(520.0, 760.0);
        ui.set_width(width);
        ui.horizontal(|ui| {
            stepper(ui, s.step);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::X, tr("Fermer l'assistant"), 32.0).clicked() {
                    open = false;
                }
            });
        });
        ui.add_space(18.0);
        let list_h = (ctx.viewport_rect().height() - 360.0).clamp(160.0, 380.0);
        match s.step {
            Step::Where => where_step(ui, s, list_h, act),
            Step::Scan => scan_step(ui, st, s),
            Step::Review => match &st.scan {
                Some(scan) => review_step(ui, data, scan, s, list_h, act),
                None => scan_step(ui, st, s),
            },
            Step::Create => create_step(ui, data, st, s, act, &mut open),
        }
    });
    // Échap ou un clic à côté ferment, sauf pendant le scan et la création (rien ne serait perdu,
    // mais l'assistant attend leur résultat).
    let busy = s.waiting || st.analysis.as_ref().is_some_and(|a| !a.finished);
    open && !(modal.should_close() && !busy)
}

/// Les quatre étapes, la courante en avant.
fn stepper(ui: &mut Ui, current: Step) {
    let t = Tokens::get(ui.ctx());
    let at = Step::ALL.iter().position(|x| *x == current).unwrap_or(0);
    ui.spacing_mut().item_spacing.x = 8.0;
    for (i, step) in Step::ALL.iter().enumerate() {
        if i > 0 {
            let (r, _) = ui.allocate_exact_size(vec2(22.0, 2.0), Sense::hover());
            ui.painter().rect_filled(r, 1.0, if i <= at { t.accent } else { t.line_strong });
        }
        let (fill, fg) = match i.cmp(&at) {
            std::cmp::Ordering::Less => (t.accent, t.on_accent),
            std::cmp::Ordering::Equal => (t.accent_soft, t.ink_strong),
            std::cmp::Ordering::Greater => (tint(t.ink, 0.06), t.faint),
        };
        let (r, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 12.0, fill);
        if i < at {
            icons::paint(ui.painter(), r.center(), Icon::Check, 13.0, fg);
        } else {
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, (i + 1).to_string(), FontId::new(12.5, theme::bold()), fg);
        }
        let color = if i == at { t.ink_strong } else { t.muted };
        ui.label(RichText::new(step.label()).font(FontId::new(13.5, if i == at { theme::semibold() } else { theme::medium() })).color(color));
    }
}

fn heading(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.spacing_mut().item_spacing.y = 4.0;
    widgets::title(ui, title, 24.0);
    widgets::muted(ui, subtitle);
    ui.add_space(10.0);
}

/// Une case à cocher seule (le libellé est à côté, dans la ligne). Renvoie `true` au clic.
fn tick(ui: &mut Ui, on: bool, name: &str) -> bool {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, on, name));
    icons::paint(ui.painter(), rect.center(), if on { Icon::SquareCheck } else { Icon::Square }, 20.0, if on { t.accent_ink } else { t.faint });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Le pied de l'assistant : le bouton de gauche (retour…) et le bouton principal à droite.
fn footer(ui: &mut Ui, back: Option<&str>, primary: &str, enabled: bool) -> (bool, bool) {
    ui.add_space(14.0);
    widgets::separator(ui);
    ui.add_space(12.0);
    let (mut b, mut p) = (false, false);
    ui.horizontal(|ui| {
        if let Some(label) = back {
            b = widgets::button(ui, Some(Icon::ArrowLeft), label, Kind::Ghost).clicked();
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            p = ui.add_enabled_ui(enabled, |ui| widgets::button(ui, None, primary, Kind::Primary)).inner.clicked();
        });
    });
    (b, p)
}

// ---- 1. Où chercher ----------------------------------------------------------------------------

fn where_step(ui: &mut Ui, s: &mut SetupState, list_h: f32, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    heading(ui, tr("Où sont tes projets ?"), tr("Coche les dossiers qui contiennent tes projets. Rust Project n'y lit que les noms de dossiers, les README et les manifestes."));
    let checked: Vec<(String, String)> = s.checked().map(|f| (path_key(&f.path), f.label.clone())).collect();
    widgets::draggable(ScrollArea::vertical()).id_salt("setup-where").max_height(list_h).auto_shrink([false, true]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        for (i, f) in s.folders.iter_mut().enumerate() {
            let key = path_key(&f.path);
            // Un dossier coché dans un autre dossier coché : déjà parcouru avec lui.
            let inside = checked.iter().find(|(k, _)| key != *k && key.starts_with(&format!("{k}\\"))).map(|(_, l)| l.clone());
            let fill = if f.checked { tint(t.accent, 0.07) } else { t.bg };
            let stroke = if f.checked { Stroke::new(1.0, tint(t.accent, 0.5)) } else { Stroke::new(1.0, t.line) };
            Frame::new().fill(fill).stroke(stroke).corner_radius(10).inner_margin(Margin::symmetric(12, 10)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if tick(ui, f.checked, &tf("Chercher dans {0}", &[("0", &f.label)])) {
                        f.checked = !f.checked;
                    }
                    icons::show(ui, Icon::Folder, 18.0, if f.checked { t.accent_ink } else { t.muted });
                    let right = if f.checked { 250.0 } else { 150.0 };
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - right).max(120.0));
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(RichText::new(&f.label).font(FontId::new(14.5, theme::semibold())).color(t.ink_strong));
                        let font = FontId::new(11.5, FontFamily::Monospace);
                        let shown = widgets::elide_middle(ui.painter(), &f.path, &font, ui.available_width());
                        ui.label(RichText::new(shown).font(font).color(t.faint)).on_hover_text(&f.path);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if f.checked {
                            let mut depth = f.depth;
                            if widgets::segmented(ui, &format!("setup-depth-{i}"), &mut depth, &[(1, None, "1"), (2, None, "2"), (3, None, "3")], Some(96.0)) {
                                f.depth = depth;
                                f.likely = None;
                                act.push(Action::SetupCount(f.path.clone(), depth));
                            }
                        }
                        match (&inside, f.likely, &f.error) {
                            (Some(parent), _, _) if f.checked => {
                                widgets::chip_sized(ui, &tf("déjà dans {0}", &[("0", parent)]), Tone::Accent, 11.5, 22.0);
                            }
                            (_, _, Some(e)) => {
                                widgets::chip_sized(ui, tr("illisible"), Tone::Danger, 11.5, 22.0).on_hover_text(e);
                            }
                            (_, Some(0), _) => {
                                widgets::chip_sized(ui, tr("aucun projet"), Tone::Neutral, 11.5, 22.0);
                            }
                            (_, Some(n), _) => {
                                let text = if n == 1 { tr("1 projet probable").to_owned() } else { tf("{0} projets probables", &[("0", &n)]) };
                                widgets::chip_sized(ui, &text, Tone::Good, 11.5, 22.0);
                            }
                            (_, None, _) => {
                                ui.add(egui::Spinner::new().size(14.0).color(t.faint));
                            }
                        }
                    });
                });
            });
        }
    });
    ui.add_space(8.0);
    if widgets::dashed_button(ui, Icon::FolderPlus, tr("Ajouter un autre dossier…"), vec2(ui.available_width(), 40.0)).clicked() {
        act.push(Action::SetupAddFolder);
    }
    let n = s.checked().count();
    let primary = match n {
        0 => tr("Coche au moins un dossier").to_owned(),
        1 => tr("Scanner ce dossier").to_owned(),
        _ => tf("Scanner ces {0} dossiers", &[("0", &n)]),
    };
    let (_, go) = footer(ui, None, &primary, n > 0);
    if go {
        act.push(Action::SetupScan);
    }
}

// ---- 2. Scan -----------------------------------------------------------------------------------

fn scan_step(ui: &mut Ui, st: &UiState, s: &mut SetupState) {
    let t = Tokens::get(ui.ctx());
    heading(ui, tr("Rust Project parcourt tes dossiers"), tr("Il relève les dossiers et repère les projets à leurs marqueurs (.git, package.json, Cargo.toml…). Les briques (frontend, api…) sont rangées sous leur projet."));
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new().size(18.0).color(t.accent));
        ui.label(RichText::new(tf("Parcours des dossiers… {0} relevés", &[("0", &st.scan_count)])).size(15.0).color(t.ink));
    });
    ui.add_space(10.0);
    for f in s.checked() {
        ui.horizontal(|ui| {
            icons::show(ui, Icon::Folder, 16.0, t.muted);
            ui.label(RichText::new(&f.path).font(FontId::new(12.5, FontFamily::Monospace)).color(t.muted));
            widgets::chip_sized(ui, &tf("{0} niveaux", &[("0", &f.depth)]), Tone::Neutral, 11.0, 20.0);
        });
    }
    let (back, _) = footer(ui, Some(tr("Retour")), tr("Scan en cours…"), false);
    if back {
        s.step = Step::Where;
        s.waiting = false;
    }
}

// ---- 3. Vérifier -------------------------------------------------------------------------------

fn review_step(ui: &mut Ui, data: &Data, scan: &ScanResult, s: &mut SetupState, list_h: f32, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let likely = review_rows(data, scan, false);
    let all = review_rows(data, scan, true);
    let tracked = scan.dirs.iter().filter(|f| data.find_by_folder(&f.path).is_some()).count();
    let bricks = scan.dirs.iter().filter(|f| f.brick_of.is_some()).count();
    let subtitle = if likely.is_empty() {
        tr("Rien de nouveau : chaque projet trouvé a déjà sa fiche.").to_owned()
    } else if likely.len() == 1 {
        tr("1 projet probable n'a pas encore de fiche. Décoche-le si ce n'est pas un projet.").to_owned()
    } else {
        tf("{0} projets probables n'ont pas encore de fiche. Décoche ce qui n'est pas un projet ; Ctrl + clic et Maj + clic sélectionnent plusieurs lignes.", &[("0", &likely.len())])
    };
    heading(ui, tr("Vérifie ce qui a été trouvé"), &subtitle);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        widgets::chip_sized(ui, &tf("{0} dossiers relevés", &[("0", &scan.dirs.len())]), Tone::Neutral, 11.5, 22.0);
        widgets::chip_sized(ui, &tf("{0} déjà suivis", &[("0", &tracked)]), Tone::Good, 11.5, 22.0);
        if bricks > 0 {
            widgets::chip_sized(ui, &tf("{0} briques rangées sous leur projet", &[("0", &bricks)]), Tone::Neutral, 11.5, 22.0);
        }
    });
    ui.add_space(8.0);
    let rows = if s.show_all { &all } else { &likely };
    let order: Vec<String> = rows.iter().map(|f| f.path.clone()).collect();
    // La barre d'outils : tout cocher, et ce qu'on fait des lignes sélectionnées.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        if !rows.is_empty() {
            if widgets::link(ui, tr("Tout cocher"), None, 13.0).clicked() {
                for p in &order {
                    if !s.keep.contains(p) {
                        s.keep.push(p.clone());
                    }
                }
            }
            if widgets::link(ui, tr("Tout décocher"), None, 13.0).clicked() {
                s.keep.retain(|p| !order.contains(p));
            }
        }
        if all.len() > likely.len() {
            let mut show = s.show_all;
            let others = all.len() - likely.len();
            let label = if others == 1 { tr("Montrer aussi le dossier sans marqueur").to_owned() } else { tf("Montrer aussi les {0} dossiers sans marqueur", &[("0", &others)]) };
            if widgets::checkbox(ui, &mut show, &label, false).changed() {
                s.show_all = show;
            }
        }
    });
    if s.picked.len() > 1 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(RichText::new(tf("{0} lignes sélectionnées :", &[("0", &s.picked.len())])).size(13.0).color(t.ink));
            if widgets::button_sized(ui, Some(Icon::SquareCheck), tr("Cocher"), Kind::Secondary, 28.0, None).clicked() {
                for p in s.picked.items().to_vec() {
                    if !s.keep.contains(&p) {
                        s.keep.push(p);
                    }
                }
            }
            if widgets::button_sized(ui, Some(Icon::Square), tr("Décocher"), Kind::Secondary, 28.0, None).clicked() {
                let picked = s.picked.items().to_vec();
                s.keep.retain(|p| !picked.contains(p));
            }
        });
    }
    ui.add_space(4.0);
    widgets::draggable(ScrollArea::vertical()).id_salt("setup-review").max_height(list_h).auto_shrink([false, true]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut root = usize::MAX;
        for f in rows.iter() {
            if f.root != root {
                root = f.root;
                root_header(ui, data.config.roots.get(root).map(|r| r.path.as_str()).unwrap_or_default());
            }
            review_row(ui, f, s, &order);
        }
    });
    let k = s.keep.iter().filter(|p| all.iter().any(|f| f.path == **p)).count();
    let primary = match k {
        0 => tr("Terminer").to_owned(),
        1 => tr("Suivant : créer 1 fiche").to_owned(),
        _ => tf("Suivant : créer {0} fiches", &[("0", &k)]),
    };
    // « Faire avec l'IA » : les dossiers cochés partent vers l'outil de code choisi (Claude Code
    // par défaut), qui crée les fiches par le serveur MCP.
    if k > 0
        && let Some(l) = data.ai_launcher() {
            let name = &data.launchers[l].name;
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.vertical(|ui| {
                    if widgets::button(ui, Some(Icon::Sparkles), tr("Faire avec l'IA"), Kind::Secondary).clicked() {
                        let checked: Vec<String> = s.keep.iter().filter(|p| all.iter().any(|f| f.path == **p)).cloned().collect();
                        act.push(Action::SetupWithAi(checked));
                    }
                    widgets::muted(ui, &tf("Avec {0} · modifiable dans Configuration › IA & modèles", &[("0", name)]));
                });
            });
        }
    let (back, go) = footer(ui, Some(tr("Retour")), &primary, true);
    if back {
        s.step = Step::Where;
    }
    if go {
        s.step = Step::Create;
    }
}

/// Une ligne de l'étape « Vérifier » : la case (garder ou non), le nom, les marqueurs, le chemin.
/// Un clic sur la ligne la sélectionne (Ctrl, Maj : plusieurs) ; la case d'une ligne sélectionnée
/// coche ou décoche toute la sélection.
fn review_row(ui: &mut Ui, f: &Found, s: &mut SetupState, order: &[String]) {
    let t = Tokens::get(ui.ctx());
    let kept = s.keep.contains(&f.path);
    let picked = s.picked.contains(&f.path);
    let (rect, row) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::click());
    row.widget_info(|| egui::WidgetInfo::selected(egui::Role::Row, true, picked, &f.name));
    // La case, par-dessus la ligne : elle a son propre clic.
    let tick_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 20.0, rect.center().y), vec2(24.0, 24.0));
    let tick = ui.interact(tick_rect, row.id.with("tick"), Sense::click());
    let name = tf("Garder {0}", &[("0", &f.name)]);
    tick.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, kept, &name));
    let painter = ui.painter();
    if picked {
        painter.rect_filled(rect, 6.0, tint(t.accent, 0.12));
    } else if row.hovered() {
        painter.rect_filled(rect, 6.0, tint(t.ink, 0.04));
    }
    painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, tint(t.line, 0.6)));
    icons::paint(painter, tick_rect.center(), if kept { Icon::SquareCheck } else { Icon::Square }, 20.0, if kept { t.accent_ink } else { t.faint });
    icons::paint(painter, egui::pos2(rect.left() + 52.0, rect.center().y), if f.likely_project { Icon::FolderGit2 } else { Icon::Folder }, 17.0, if f.likely_project { t.good_fg } else { t.faint });
    let x = rect.left() + 72.0;
    let width = rect.right() - x - 10.0;
    let font = FontId::new(14.0, theme::semibold());
    let galley = painter.layout_no_wrap(f.name.clone(), font.clone(), t.ink);
    let name_w = galley.size().x.min(width * 0.5);
    widgets::paint_text(painter, egui::pos2(x, rect.center().y - 19.0), &f.name, font, if kept { t.ink_strong } else { t.muted }, name_w);
    // Les marqueurs (et « copie ? ») à la suite du nom.
    let mut cx = x + name_w + 8.0;
    let chip_y = rect.center().y - 10.0;
    if crate::scan::copy_like(&f.name) {
        cx += widgets::paint_chip(painter, &t, egui::pos2(cx, chip_y), tr("copie ?"), Tone::Accent) + 6.0;
    }
    for m in f.markers.iter().take(3) {
        if cx > rect.right() - 90.0 {
            break;
        }
        cx += widgets::paint_chip(painter, &t, egui::pos2(cx, chip_y), m, Tone::Neutral) + 6.0;
    }
    let mono = FontId::new(11.5, FontFamily::Monospace);
    let shown = widgets::elide_middle(painter, &f.path, &mono, width);
    painter.text(egui::pos2(x, rect.center().y + 11.0), egui::Align2::LEFT_CENTER, shown, mono, t.faint);
    let mods = ui.input(|i| i.modifiers);
    if tick.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        let group: Vec<String> = if picked && s.picked.len() > 1 { s.picked.items().to_vec() } else { vec![f.path.clone()] };
        if kept {
            s.keep.retain(|p| !group.contains(p));
        } else {
            for p in group {
                if !s.keep.contains(&p) {
                    s.keep.push(p);
                }
            }
        }
    } else if row.on_hover_text(&f.path).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        s.picked.click(f.path.clone(), mods, order);
    }
}

/// L'en-tête d'un groupe de l'étape « Vérifier » : le dossier source, son nom puis son chemin.
fn root_header(ui: &mut Ui, path: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        icons::show(ui, Icon::FolderTree, 15.0, t.muted);
        ui.label(RichText::new(short_name(path)).font(FontId::new(13.0, theme::semibold())).color(t.ink));
        let mono = FontId::new(11.5, FontFamily::Monospace);
        let shown = widgets::elide_middle(ui.painter(), path, &mono, ui.available_width());
        ui.label(RichText::new(shown).font(mono).color(t.faint)).on_hover_text(path);
    });
    ui.add_space(4.0);
}

// ---- 4. Créer les fiches -----------------------------------------------------------------------

fn create_step(ui: &mut Ui, data: &Data, st: &UiState, s: &mut SetupState, act: &mut Vec<Action>, open: &mut bool) {
    let t = Tokens::get(ui.ctx());
    let Some(scan) = &st.scan else { return };
    let all = review_rows(data, scan, true);
    let keep: Vec<String> = s.keep.iter().filter(|p| all.iter().any(|f| f.path == **p)).cloned().collect();
    let dropped: Vec<String> = review_rows(data, scan, false).iter().map(|f| f.path.clone()).filter(|p| !s.keep.contains(p)).collect();
    // Après la création : le bilan, et où aller ensuite.
    let analysis = st.analysis.as_ref().filter(|_| s.ai_started);
    if s.created.is_some() || analysis.is_some() {
        match (s.created, analysis) {
            (_, Some(a)) if !a.finished => {
                heading(ui, tr("L'IA crée les fiches"), &tf("{0}/{1} · {2}", &[("0", &a.done), ("1", &a.total), ("2", &a.current)]));
                widgets::progress(ui, a.done as f32 / a.total.max(1) as f32, t.accent, ui.available_width(), 6.0);
                return;
            }
            (_, Some(a)) => heading(ui, &created_title(a.added), tr("Par l'IA locale. Tu les retrouves dans la bibliothèque, au fil de l'eau ou en réserve.")),
            (Some(n), None) => heading(ui, &created_title(n), tr("Tu les retrouves dans la bibliothèque. Les descriptions viennent des manifestes et des README.")),
            (None, None) => {}
        }
        if data.config.github_enabled {
            ui.horizontal(|ui| {
                icons::show(ui, Icon::GitBranch, 16.0, t.muted);
                widgets::muted(ui, tr("Tes dépôts GitHub sans dossier local s'ajoutent depuis Configuration › Dossiers sources."));
            });
        }
        ui.add_space(14.0);
        widgets::separator(ui);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if data.config.github_enabled && widgets::button(ui, Some(Icon::GitBranch), tr("Voir mes dépôts GitHub"), Kind::Ghost).clicked() {
                act.push(Action::GoConfig(crate::app::CfgSection::Sources));
                *open = false;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button(ui, Some(Icon::Layers), tr("Voir la bibliothèque"), Kind::Primary).clicked() {
                    act.push(Action::Library(None));
                    *open = false;
                }
            });
        });
        return;
    }
    if keep.is_empty() {
        heading(ui, tr("Rien à créer"), tr("Aucun dossier n'est coché. Tu pourras relancer l'assistant à tout moment depuis Configuration › Dossiers sources."));
    } else if keep.len() == 1 {
        heading(ui, tr("Créer 1 fiche"), tr("Choisis qui la remplit. Tu pourras tout modifier ensuite."));
    } else {
        heading(ui, &tf("Créer {0} fiches", &[("0", &keep.len())]), tr("Choisis qui les remplit. Tu pourras tout modifier ensuite, fiche par fiche."));
    }
    let ollama = data.config.ollama_on && !data.config.ollama_model.trim().is_empty();
    if !ollama {
        s.with_ai = false;
    }
    if !keep.is_empty() {
        let rules = tr("Les règles locales");
        if choice(ui, !s.with_ai, Icon::ListChecks, rules, tr("Rapide et sans IA : le nom, les technos et la description sont lus dans les manifestes et le README."), true) {
            s.with_ai = false;
        }
        ui.add_space(6.0);
        let ai = if ollama { tf("L'IA locale ({0})", &[("0", &data.config.ollama_model)]) } else { tr("L'IA locale (Ollama)").to_owned() };
        let ai_help = if ollama { tr("Plus lent : l'IA écrit la description et choisit le domaine de chaque projet.") } else { tr("Choisis d'abord un modèle dans Configuration › IA & modèles.") };
        if choice(ui, s.with_ai, Icon::Sparkles, &ai, ai_help, ollama) {
            s.with_ai = true;
        }
        if data.config.mcp_on {
            ui.add_space(6.0);
            widgets::muted(ui, tr("Ton agent MCP peut aussi s'en charger : demande-lui de ranger tes nouveaux projets."));
        }
    }
    if !dropped.is_empty() {
        ui.add_space(10.0);
        let mut ignore = s.ignore_rest;
        let label = if dropped.len() == 1 { tr("Écarter des prochains scans le projet probable décoché").to_owned() } else { tf("Écarter des prochains scans les {0} projets probables décochés", &[("0", &dropped.len())]) };
        if widgets::checkbox(ui, &mut ignore, &label, false).changed() {
            s.ignore_rest = ignore;
        }
    }
    let primary = match keep.len() {
        0 => tr("Terminer").to_owned(),
        1 => tr("Créer la fiche").to_owned(),
        n => tf("Créer les {0} fiches", &[("0", &n)]),
    };
    let (back, go) = footer(ui, Some(tr("Retour")), &primary, true);
    if back {
        s.step = Step::Review;
    }
    if go {
        let ignore = if s.ignore_rest { dropped } else { Vec::new() };
        if keep.is_empty() {
            if !ignore.is_empty() {
                act.push(Action::SetupCreate { keep, ignore, with_ai: false });
            }
            *open = false;
        } else {
            if s.with_ai {
                s.ai_started = true;
            }
            act.push(Action::SetupCreate { keep, ignore, with_ai: s.with_ai });
        }
    }
}

/// « 1 fiche créée », « 12 fiches créées ».
fn created_title(n: usize) -> String {
    if n == 1 { tr("1 fiche créée").to_owned() } else { tf("{0} fiches créées", &[("0", &n)]) }
}

/// Un choix exclusif, en carte : icône, titre, explication. Renvoie `true` au clic.
fn choice(ui: &mut Ui, on: bool, icon: Icon, title: &str, help: &str, enabled: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let fill = if on { tint(t.accent, 0.08) } else { t.bg };
    let stroke = if on { Stroke::new(1.5, t.accent) } else { Stroke::new(1.0, t.line) };
    let inner = Frame::new().fill(fill).stroke(stroke).corner_radius(10).inner_margin(Margin::symmetric(14, 12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            icons::show(ui, if on { Icon::CircleDot } else { Icon::Circle }, 18.0, if on { t.accent_ink } else { t.faint });
            icons::show(ui, icon, 18.0, if enabled { t.ink } else { t.faint });
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(title).font(FontId::new(14.5, theme::semibold())).color(if enabled { t.ink_strong } else { t.faint }));
                ui.label(RichText::new(help).size(12.5).color(t.muted));
            });
        });
    });
    let resp = ui.interact(inner.response.rect, Id::unique(("setup-choice", title)), if enabled { Sense::click() } else { Sense::hover() });
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, enabled, on, title));
    enabled && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}
