//! « Bibliothèque » : tous les projets, rangés par la taxonomie de l'IA. Trois vues : une carte
//! (chaque domaine est une île, chaque projet un point), une grille et une liste.

use egui::{Align, Align2, Color32, CornerRadius, FontFamily, FontId, Frame, Id, Layout, Margin, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::{Action, BulkOp, CfgSection, LibView, Sort, UiState, idle_text};
use crate::icons::{self, Icon};
use crate::model::{Attention, Data, Status};
use crate::theme::{self, Tokens, mix, tint};
use crate::widgets::{self, Tone};
use crate::i18n::{tf, tr};

/// Les projets visibles avec les filtres en cours, triés. Pendant une recherche (floue et
/// sémantique, voir `search.rs`), les plus pertinents d'abord.
fn visible(data: &Data, st: &UiState, ignore_domain: bool) -> Vec<usize> {
    let hits = st.lib_hits.as_ref();
    let mut ids: Vec<usize> = (0..data.projects.len())
        .filter(|&i| {
            let p = &data.projects[i];
            in_view(st.lib_archived, st.lib_attention, p)
                && (ignore_domain || st.lib_domain.is_none_or(|d| data.has_domain(i, d)))
                && st.lib_package.as_ref().is_none_or(|pk| p.packages.contains(pk))
                && hits.is_none_or(|h| h.iter().any(|x| x.index == i))
        })
        .collect();
    if let Some(h) = hits {
        ids.sort_by_key(|&i| h.iter().position(|x| x.index == i).unwrap_or(usize::MAX));
        return ids;
    }
    match st.lib_sort {
        Sort::Recent => ids.sort_by_key(|&i| data.projects[i].idle_days),
        Sort::Name => ids.sort_by_key(|&i| data.projects[i].name.to_lowercase()),
        Sort::Domain => ids.sort_by_key(|&i| (data.projects[i].domain, data.projects[i].idle_days)),
    }
    ids
}

/// Le filtre du bloc Attention : une attention (Focus, Au fil de l'eau, En réserve), les archivés
/// seuls, ou tout ce qui n'est pas archivé. Les archivés ne se mêlent jamais aux autres.
fn in_view(archived: bool, attention: Option<Attention>, p: &crate::model::Project) -> bool {
    if archived {
        p.status == Status::Archived
    } else {
        p.status != Status::Archived && attention.is_none_or(|a| p.attention == a)
    }
}

/// Une entrée de la colonne qui accepte un projet déposé : on la souligne pendant le glisser.
fn drop_highlight(ui: &Ui, t: &Tokens, resp: &egui::Response) {
    let over = resp.dnd_hover_payload::<DraggedProjects>().is_some();
    let stroke = if over { Stroke::new(2.0, t.accent) } else { Stroke::new(1.0, tint(t.accent, 0.45)) };
    if over {
        ui.painter().rect_filled(resp.rect, CornerRadius::same(8), tint(t.accent, 0.14));
    }
    ui.painter().rect_stroke(resp.rect.shrink(1.0), CornerRadius::same(8), stroke, StrokeKind::Inside);
}

/// Les projets qu'on fait glisser (depuis la carte, la grille ou la liste) vers une attention ou
/// un domaine : un seul, ou toute la sélection quand on tire l'un des projets sélectionnés.
#[derive(Clone)]
struct DraggedProjects(Vec<usize>);

/// Le groupe emporté en tirant le projet `i` : la sélection s'il en fait partie, sinon lui seul.
fn group_of(data: &Data, st: &UiState, i: usize) -> Vec<usize> {
    let id = data.projects[i].id;
    if st.lib_multi.len() > 1 && st.lib_multi.contains(&id) {
        st.lib_multi.items().iter().filter_map(|id| data.index_of(*id)).collect()
    } else {
        vec![i]
    }
}

/// Rend une carte de projet déplaçable (à appeler après le curseur de survol).
fn drag_source(resp: &egui::Response, group: Vec<usize>) {
    resp.dnd_set_drag_payload(DraggedProjects(group));
}

/// Un clic sur un projet de la grille ou de la liste : clic, Ctrl + clic, Maj + clic. Le panneau
/// montre le dernier projet cliqué (s'il est encore sélectionné).
fn click_project(ui: &Ui, data: &Data, st: &mut UiState, ids: &[usize], i: usize, act: &mut Vec<Action>) {
    let mods = ui.input(|inp| inp.modifiers);
    let order: Vec<u64> = ids.iter().map(|&k| data.projects[k].id).collect();
    let id = data.projects[i].id;
    // Ctrl ou Maj juste après avoir ouvert un projet : il fait partie de la sélection.
    if (mods.command || mods.shift)
        && st.lib_multi.is_empty()
        && let Some(open) = st.selected.and_then(|s| data.projects.get(s))
    {
        st.lib_multi.click(open.id, egui::Modifiers::NONE, &order);
    }
    st.lib_multi.click(id, mods, &order);
    if st.lib_multi.contains(&id) {
        act.push(Action::Select(i));
    }
}

pub fn show(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>) {
    egui::Panel::left("library-side").exact_size(250.0).resizable(false).show_separator_line(false).frame(Frame::new().inner_margin(Margin { left: 0, right: 18, top: 0, bottom: 0 })).show(ui, |ui| sidebar(ui, data, st, act));
    main(ui, data, st, act);
    drag_ghost(ui, data);
}

/// Pendant un glisser-déposer : le projet (ou le groupe) suit la souris.
fn drag_ghost(ui: &Ui, data: &Data) {
    let Some(dragged) = egui::DragAndDrop::payload::<DraggedProjects>(ui.ctx()) else { return };
    let (Some(p), Some(pos)) = (dragged.0.first().and_then(|&i| data.projects.get(i)), ui.ctx().pointer_interact_pos()) else { return };
    let t = Tokens::get(ui.ctx());
    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::unique("library-drag-ghost")));
    let font = FontId::new(13.5, theme::semibold());
    let label = if dragged.0.len() > 1 { tf("{0} et {1} autres", &[("0", &p.name), ("1", &(dragged.0.len() - 1))]) } else { p.name.clone() };
    let galley = painter.layout_no_wrap(label, font, t.ink_strong);
    let rect = Rect::from_min_size(pos + vec2(14.0, 10.0), vec2(galley.size().x + 44.0, 34.0));
    painter.rect(rect, CornerRadius::same(9), t.raised, Stroke::new(1.0, t.accent), StrokeKind::Inside);
    icons::paint(&painter, pos2(rect.left() + 19.0, rect.center().y), p.icon, 16.0, t.accent_ink);
    painter.galley(pos2(rect.left() + 34.0, rect.center().y - galley.size().y / 2.0), galley, t.ink_strong);
}

/// Une entrée de la colonne de filtres.
fn side_item(ui: &mut Ui, icon: Icon, color: Color32, label: &str, count: usize, selected: bool) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, label));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
    let p = ui.painter();
    let fill = if selected { t.accent_soft } else { tint(t.ink, 0.05 * hover) };
    p.rect_filled(rect, CornerRadius::same(8), fill);
    icons::paint(p, pos2(rect.left() + 20.0, rect.center().y), icon, 17.0, color);
    let fg = if selected { t.ink_strong } else { t.ink };
    widgets::paint_text(p, pos2(rect.left() + 40.0, rect.center().y - 9.0), label, FontId::new(14.0, theme::medium()), fg, rect.width() - 80.0);
    p.text(pos2(rect.right() - 12.0, rect.center().y), Align2::RIGHT_CENTER, count.to_string(), FontId::new(13.0, theme::semibold()), if selected { t.ink_strong } else { t.faint });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn sidebar(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::draggable(ScrollArea::vertical()).id_salt("library-side").auto_shrink(false).show(ui, |ui| widgets::scroll_content(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        // L'attention d'abord : c'est elle qui range le bureau. On y dépose aussi les projets.
        let dragging = egui::DragAndDrop::has_payload_of_type::<DraggedProjects>(ui.ctx());
        widgets::caps(ui, if dragging { tr("Attention · dépose le projet ici") } else { tr("Attention") });
        ui.add_space(6.0);
        for a in Attention::ALL {
            let color = if a == Attention::Focus { t.accent_ink } else { t.muted };
            let resp = side_item(ui, a.icon(), color, a.label(), data.count(a), st.lib_attention == Some(a));
            if dragging {
                drop_highlight(ui, &t, &resp);
            }
            if let Some(dropped) = resp.dnd_release_payload::<DraggedProjects>() {
                // Un projet archivé qu'on range dans une attention reprend du service.
                act.push(Action::Bulk(dropped.0.clone(), BulkOp::Attention(a)));
            } else if resp.clicked() {
                st.lib_attention = if st.lib_attention == Some(a) { None } else { Some(a) };
                st.lib_archived = false;
            }
        }
        // Les archivés : une entrée comme les autres (elle montre les archivés seuls), et on y
        // dépose un projet pour l'archiver.
        let resp = side_item(ui, Icon::ArchiveX, t.muted, tr("Archivés"), data.archived(), st.lib_archived);
        if dragging {
            drop_highlight(ui, &t, &resp);
        }
        if let Some(dropped) = resp.dnd_release_payload::<DraggedProjects>() {
            let todo: Vec<usize> = dropped.0.iter().copied().filter(|&i| data.projects.get(i).is_some_and(|p| p.status != Status::Archived)).collect();
            if !todo.is_empty() {
                act.push(Action::Bulk(todo, BulkOp::Archive));
            }
        } else if resp.clicked() {
            st.lib_archived = !st.lib_archived;
            st.lib_attention = None;
        }
        ui.add_space(16.0);
        widgets::caps(ui, if dragging { tr("Domaines · dépose le projet ici") } else { tr("Domaines") });
        ui.add_space(6.0);
        let (archived, attention) = (st.lib_archived, st.lib_attention);
        // Un projet compte dans chacun de ses domaines (le principal et ceux de ses tags).
        let in_scope = |d: Option<usize>| (0..data.projects.len()).filter(|&i| in_view(archived, attention, &data.projects[i]) && d.is_none_or(|d| data.has_domain(i, d))).count();
        if side_item(ui, Icon::LayoutGrid, t.muted, tr("Tous les domaines"), in_scope(None), st.lib_domain.is_none()).clicked() {
            st.lib_domain = None;
        }
        // Les plus fournis d'abord ; les 5 premiers seulement, sauf « Tout voir » (et pendant un
        // glisser-déposer, où chaque domaine est une cible).
        let mut order: Vec<(usize, usize)> = (0..data.domains.len()).map(|d| (d, in_scope(Some(d)))).collect();
        order.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let show_all = st.lib_domains_all || dragging;
        for (k, &(i, count)) in order.iter().enumerate() {
            if !show_all && k >= SHOWN && st.lib_domain != Some(i) {
                continue;
            }
            let d = &data.domains[i];
            let resp = side_item(ui, d.icon, d.color, &d.name, count, st.lib_domain == Some(i));
            if dragging {
                drop_highlight(ui, &t, &resp);
            }
            if let Some(dropped) = resp.dnd_release_payload::<DraggedProjects>() {
                act.push(Action::Bulk(dropped.0.clone(), BulkOp::Domain(i)));
            } else if resp.clicked() {
                st.lib_domain = if st.lib_domain == Some(i) { None } else { Some(i) };
            }
        }
        if order.len() > SHOWN && !dragging {
            more_link(ui, &mut st.lib_domains_all, order.len());
        }
        ui.add_space(16.0);
        packages_block(ui, data, st);
        ui.add_space(16.0);
        docker_block(ui, data, st);
        ui.add_space(16.0);
        taxonomy_block(ui, data, st, act);
    }));
}

/// Combien d'entrées d'une liste de la colonne avant « Tout voir ».
const SHOWN: usize = 5;

/// « Tout voir (17) » / « Réduire », sous une liste de la colonne.
fn more_link(ui: &mut Ui, all: &mut bool, total: usize) {
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        let label = if *all { tr("Réduire").to_owned() } else { tf("Tout voir ({0})", &[("0", &total)]) };
        if widgets::link(ui, &label, Some(if *all { Icon::ChevronUp } else { Icon::ChevronDown }), 12.5).clicked() {
            *all = !*all;
        }
    });
}

/// Les paquets des projets affichés, par gestionnaire (npm, Cargo, pip, Composer) : les plus
/// utilisés d'abord, 5 par gestionnaire, « Tout voir » pour la suite. Un clic filtre.
fn packages_block(ui: &mut Ui, data: &Data, st: &mut UiState) {
    use crate::packages::{Manager, Package};
    let t = Tokens::get(ui.ctx());
    widgets::caps(ui, tr("Paquets"));
    ui.add_space(6.0);
    let (archived, attention) = (st.lib_archived, st.lib_attention);
    let mut counts: std::collections::BTreeMap<Manager, std::collections::HashMap<&str, usize>> = Default::default();
    for p in data.projects.iter().filter(|p| in_view(archived, attention, p)) {
        // Les images Docker ont leur propre rubrique.
        for pk in p.packages.iter().filter(|pk| pk.manager != Manager::Docker) {
            *counts.entry(pk.manager).or_default().entry(pk.name.as_str()).or_default() += 1;
        }
    }
    if counts.is_empty() {
        ui.label(RichText::new(tr("Aucun paquet relevé : ils viennent des manifestes (package.json, Cargo.toml, composer.json, requirements.txt).")).size(12.0).color(t.faint));
        return;
    }
    for m in Manager::ALL {
        let Some(map) = counts.get(&m) else { continue };
        let mut list: Vec<(&str, usize)> = map.iter().map(|(n, c)| (*n, *c)).collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        // Chaque gestionnaire est une ligne repliable (repliée par défaut) : tous restent visibles
        // d'un coup d'œil. Celui du paquet filtré s'ouvre de lui-même.
        let filtered = st.lib_package.as_ref().is_some_and(|p| p.manager == m);
        let open = st.lib_packages_open.contains(&m) || filtered;
        let (rect, head) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
        head.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, open, m.label()));
        if head.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(8), tint(t.ink, 0.05));
        }
        ui.painter().circle_filled(pos2(rect.left() + 14.0, rect.center().y), 3.5, m.color());
        ui.painter().text(pos2(rect.left() + 26.0, rect.center().y), Align2::LEFT_CENTER, m.label(), FontId::new(13.0, theme::semibold()), t.ink);
        let count = if list.len() == 1 { tr("1 paquet").to_owned() } else { tf("{0} paquets", &[("0", &list.len())]) };
        ui.painter().text(pos2(rect.right() - 30.0, rect.center().y), Align2::RIGHT_CENTER, count, FontId::proportional(11.5), t.faint);
        icons::paint(ui.painter(), pos2(rect.right() - 14.0, rect.center().y), if open { Icon::ChevronUp } else { Icon::ChevronDown }, 14.0, t.faint);
        if head.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            if st.lib_packages_open.contains(&m) {
                st.lib_packages_open.retain(|x| *x != m);
            } else {
                st.lib_packages_open.push(m);
            }
        }
        if !open {
            continue;
        }
        let mut all = st.lib_packages_all.contains(&m);
        for (k, (name, count)) in list.iter().enumerate() {
            let selected = st.lib_package.as_ref().is_some_and(|p| p.manager == m && p.name == *name);
            if !all && k >= SHOWN && !selected {
                continue;
            }
            if side_item(ui, Icon::Package, m.color(), name, *count, selected).clicked() {
                st.lib_package = if selected { None } else { Some(Package { manager: m, name: (*name).to_owned() }) };
            }
        }
        if list.len() > SHOWN {
            let before = all;
            more_link(ui, &mut all, list.len());
            if all != before {
                if all {
                    st.lib_packages_all.push(m);
                } else {
                    st.lib_packages_all.retain(|x| *x != m);
                }
            }
        }
        ui.add_space(6.0);
    }
}

/// Les images Docker des projets affichés (Dockerfile, docker-compose) : les plus utilisées
/// d'abord, 5 puis « Tout voir ». Un clic filtre. Le bloc n'apparaît que s'il y a des images.
fn docker_block(ui: &mut Ui, data: &Data, st: &mut UiState) {
    use crate::packages::{Manager, Package};
    let (archived, attention) = (st.lib_archived, st.lib_attention);
    let mut counts: std::collections::HashMap<&str, usize> = Default::default();
    for p in data.projects.iter().filter(|p| in_view(archived, attention, p)) {
        for pk in p.packages.iter().filter(|pk| pk.manager == Manager::Docker) {
            *counts.entry(pk.name.as_str()).or_default() += 1;
        }
    }
    if counts.is_empty() {
        return;
    }
    let mut list: Vec<(&str, usize)> = counts.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    widgets::caps(ui, tr("Images Docker"));
    ui.add_space(6.0);
    let mut all = st.lib_packages_all.contains(&Manager::Docker);
    for (k, (name, count)) in list.iter().enumerate() {
        let selected = st.lib_package.as_ref().is_some_and(|p| p.manager == Manager::Docker && p.name == *name);
        if !all && k >= SHOWN && !selected {
            continue;
        }
        if side_item(ui, Icon::Container, Manager::Docker.color(), name, *count, selected).clicked() {
            st.lib_package = if selected { None } else { Some(Package { manager: Manager::Docker, name: (*name).to_owned() }) };
        }
    }
    if list.len() > SHOWN {
        let before = all;
        more_link(ui, &mut all, list.len());
        if all != before {
            if all {
                st.lib_packages_all.push(Manager::Docker);
            } else {
                st.lib_packages_all.retain(|x| *x != Manager::Docker);
            }
        }
    }
}

/// La taxonomie, repliée : un clic l'ouvre (combien de domaines et de tags, et où les régler).
fn taxonomy_block(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let open = st.lib_taxonomy_open;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, open, tr("Taxonomie")));
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), tint(t.ink, 0.05));
    }
    icons::paint(ui.painter(), pos2(rect.left() + 20.0, rect.center().y), Icon::Network, 16.0, t.muted);
    ui.painter().text(pos2(rect.left() + 40.0, rect.center().y), Align2::LEFT_CENTER, tr("Taxonomie"), FontId::new(13.5, theme::semibold()), t.ink);
    icons::paint(ui.painter(), pos2(rect.right() - 16.0, rect.center().y), if open { Icon::ChevronUp } else { Icon::ChevronDown }, 15.0, t.faint);
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        st.lib_taxonomy_open = !open;
    }
    if !open {
        return;
    }
    Frame::new().fill(tint(t.accent, 0.06)).stroke(Stroke::new(1.0, tint(t.accent, 0.25))).corner_radius(10).inner_margin(Margin::same(14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.label(RichText::new(tf("{0} domaines · {1} tags dans l'ontologie", &[("0", &data.domains.len()), ("1", &data.ontology.concepts.len())])).size(12.5).color(t.muted));
        if widgets::link(ui, tr("Ontologie"), Some(Icon::ArrowRight), 13.0).clicked() {
            act.push(Action::GoConfig(CfgSection::Ontology));
        }
        if widgets::link(ui, tr("Régler l'IA"), Some(Icon::ArrowRight), 13.0).clicked() {
            act.push(Action::GoConfig(CfgSection::Ai));
        }
    });
}

/// Un filtre actif, montré en pastille retirable dans la barre d'outils.
#[derive(Clone, Copy)]
enum Filter {
    Domain,
    Attention,
    Archived,
    Package,
}

fn main(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let ids = visible(data, st, false);
    let mut filters: Vec<(Filter, String, Color32)> = Vec::new();
    if let Some(d) = st.lib_domain.and_then(|d| data.domains.get(d)) {
        filters.push((Filter::Domain, d.name.clone(), d.color));
    }
    if let Some(a) = st.lib_attention {
        filters.push((Filter::Attention, a.label().into(), t.accent_ink));
    }
    if st.lib_archived {
        filters.push((Filter::Archived, tr("Archivés").into(), t.muted));
    }
    if let Some(pk) = &st.lib_package {
        filters.push((Filter::Package, format!("{} · {}", pk.manager.label(), pk.name), pk.manager.color()));
    }

    // ---- Ligne 1 : le titre et le compte à gauche, la vue à droite. Toujours sur une ligne :
    // quand la place manque, le choix de la vue passe en icônes.
    ui.horizontal(|ui| {
        widgets::title(ui, tr("Bibliothèque"), 28.0);
        widgets::count_pill(ui, &ids.len().to_string());
        let pool = if st.lib_archived { data.archived() } else { data.projects.len() - data.archived() };
        if ids.len() != pool {
            ui.label(RichText::new(tf("sur {pool}", &[("pool", &pool)])).size(13.5).color(t.faint));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let compact = ui.available_width() < 330.0;
            view_switch(ui, &mut st.lib_view, compact);
        });
    });
    ui.add_space(10.0);

    // ---- Ligne 2 : la barre d'outils. La recherche prend la place libre, les filtres actifs la
    // suivent, le tri se range à droite (pas sur la carte, où l'ordre ne compte pas).
    let show_sort = st.lib_view != LibView::Map;
    let sort_w = if show_sort { 210.0 } else { 0.0 };
    let font = FontId::new(13.0, theme::semibold());
    let chips_w: f32 = filters.iter().map(|(_, l, _)| ui.painter().layout_no_wrap(l.clone(), font.clone(), t.ink).size().x + 46.0).sum::<f32>() + if filters.len() >= 2 { 90.0 } else { 0.0 };
    let avail = ui.available_width();
    let inline = filters.is_empty() || avail - sort_w - chips_w - 24.0 >= 240.0;
    let (mut removed, mut clear_all) = (None, false);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let search_w = (avail - sort_w - if inline { chips_w } else { 0.0 } - 24.0).max(160.0);
        let hint = if search_w >= 380.0 { tr("Rechercher : nom, type, techno (rust, next.js…)") } else { tr("Rechercher un projet…") };
        widgets::icon_input(ui, &mut st.lib_search, Icon::Search, hint, search_w, theme::CONTROL_H);
        if inline {
            filter_chips(ui, &filters, &mut removed, &mut clear_all);
        }
        if show_sort {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| sort_menu(ui, &mut st.lib_sort));
        }
    });
    if !inline {
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| filter_chips(ui, &filters, &mut removed, &mut clear_all));
    }
    match removed {
        Some(Filter::Domain) => st.lib_domain = None,
        Some(Filter::Attention) => st.lib_attention = None,
        Some(Filter::Archived) => st.lib_archived = false,
        Some(Filter::Package) => st.lib_package = None,
        None => {}
    }
    if clear_all {
        (st.lib_domain, st.lib_attention, st.lib_archived) = (None, None, false);
        st.lib_package = None;
        st.lib_search.clear();
    }
    ui.add_space(10.0);
    // La sélection multiple ne garde que les projets affichés (un filtre a pu en masquer).
    let shown: Vec<u64> = ids.iter().map(|&i| data.projects[i].id).collect();
    st.lib_multi.retain(|id| shown.contains(id));
    if st.lib_view != LibView::Map {
        selection_keys(ui, st, &shown);
        if st.lib_multi.len() > 1 {
            selection_bar(ui, data, st, act);
            ui.add_space(10.0);
        }
    }
    match st.lib_view {
        LibView::Map => map(ui, data, st, act),
        LibView::Grid => grid(ui, data, &ids, st, act),
        LibView::List => list(ui, data, &ids, st, act),
    }
}

/// Ctrl + A sélectionne tout ce qui est affiché, Échap vide la sélection (hors saisie de texte).
fn selection_keys(ui: &Ui, st: &mut UiState, shown: &[u64]) {
    if st.modal.is_some() || ui.ctx().memory(|m| m.focused().is_some()) {
        return;
    }
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A)) {
        st.lib_multi.set_all(shown);
    }
    if !st.lib_multi.is_empty() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        st.lib_multi.clear();
    }
}

/// La barre de la sélection multiple : combien, et ce qu'on peut faire du groupe.
fn selection_bar(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let group: Vec<usize> = st.lib_multi.items().iter().filter_map(|id| data.index_of(*id)).collect();
    Frame::new().fill(tint(t.accent, 0.08)).stroke(Stroke::new(1.0, tint(t.accent, 0.35))).corner_radius(10).inner_margin(Margin::symmetric(14, 8)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            icons::show(ui, Icon::SquareCheck, 17.0, t.accent_ink);
            ui.label(RichText::new(tf("{0} projets sélectionnés", &[("0", &group.len())])).font(FontId::new(14.0, theme::semibold())).color(t.ink_strong));
            if ui.available_width() > 620.0 {
                ui.label(RichText::new(tr("Glisse-les vers une attention ou un domaine")).size(12.5).color(t.muted));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::X, tr("Vider la sélection (Échap)"), 30.0).clicked() {
                    st.lib_multi.clear();
                }
                let domains: Vec<(usize, &str)> = data.domains.iter().enumerate().map(|(i, d)| (i, d.name.as_str())).collect();
                if let Some(d) = widgets::menu_button(ui, Some(Icon::Tag), tr("Ajouter un domaine"), widgets::Kind::Secondary, &domains) {
                    act.push(Action::Bulk(group.clone(), BulkOp::Domain(d)));
                }
                let mut targets: Vec<(Option<Attention>, &str)> = Attention::ALL.iter().map(|a| (Some(*a), a.label())).collect();
                targets.push((None, tr("Archiver")));
                if let Some(target) = widgets::menu_button(ui, Some(Icon::Crosshair), tr("Attention"), widgets::Kind::Secondary, &targets) {
                    act.push(Action::Bulk(group.clone(), target.map(BulkOp::Attention).unwrap_or(BulkOp::Archive)));
                }
            });
        });
    });
}

/// Les pastilles des filtres actifs, et « Tout effacer » à partir de deux.
fn filter_chips(ui: &mut Ui, filters: &[(Filter, String, Color32)], removed: &mut Option<Filter>, clear_all: &mut bool) {
    ui.spacing_mut().item_spacing.x = 6.0;
    for (f, label, color) in filters {
        if filter_chip(ui, label, *color) {
            *removed = Some(*f);
        }
    }
    if filters.len() >= 2 && widgets::link(ui, tr("Tout effacer"), None, 12.5).clicked() {
        *clear_all = true;
    }
}

/// Le tri, compact : « Tri : Activité récente ▾ ».
fn sort_menu(ui: &mut Ui, sort: &mut Sort) {
    let sorts = [(Sort::Recent, tr("Activité récente")), (Sort::Name, tr("Nom")), (Sort::Domain, tr("Domaine"))];
    let current = sorts.iter().find(|s| s.0 == *sort).map(|s| s.1).unwrap_or("");
    let inner = egui::ComboBox::from_id_salt("lib-sort").selected_text(tf("Tri : {current}", &[("current", &current)])).width(200.0).show_ui(ui, |ui| {
        for (v, label) in sorts {
            ui.selectable_value(sort, v, label);
        }
    });
    inner.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, tr("Trier par")));
}

/// Carte, Grille, Liste : avec leurs noms, ou en icônes seules quand la place manque (le nom
/// reste en bulle et pour l'accessibilité).
fn view_switch(ui: &mut Ui, view: &mut LibView, compact: bool) {
    let views = [(LibView::Map, Icon::Map, tr("Carte")), (LibView::Grid, Icon::LayoutGrid, tr("Grille")), (LibView::List, Icon::List, tr("Liste"))];
    if !compact {
        widgets::segmented(ui, "lib-view", view, &views.map(|(v, i, l)| (v, Some(i), l)), None);
        return;
    }
    let t = Tokens::get(ui.ctx());
    let cell_w = 40.0;
    let (rect, _) = ui.allocate_exact_size(vec2(cell_w * views.len() as f32, theme::CONTROL_H), Sense::hover());
    let painter = ui.painter().clone();
    painter.rect(rect, CornerRadius::same(theme::R_CTRL), t.input, Stroke::new(1.0, t.line_strong), StrokeKind::Inside);
    for (k, (v, icon, label)) in views.into_iter().enumerate() {
        let cell = Rect::from_min_size(rect.min + vec2(k as f32 * cell_w, 0.0), vec2(cell_w, rect.height()));
        let resp = ui.interact(cell, Id::unique(("lib-view-compact", k)), Sense::click()).on_hover_text(label).on_hover_cursor(egui::CursorIcon::PointingHand);
        let selected = *view == v;
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, selected, label));
        if resp.clicked() {
            *view = v;
        }
        if selected {
            painter.rect(cell.shrink(2.0), CornerRadius::same(theme::R_CTRL - 2), t.accent_soft, Stroke::new(1.0, t.accent), StrokeKind::Inside);
        }
        let fg = if selected || resp.hovered() { t.ink_strong } else { t.muted };
        icons::paint(&painter, cell.center(), icon, 17.0, fg);
    }
}

/// Pastille de filtre avec une croix. Renvoie `true` si on la retire.
fn filter_chip(ui: &mut Ui, label: &str, color: Color32) -> bool {
    let t = Tokens::get(ui.ctx());
    let galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::new(13.0, theme::semibold()), color);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 40.0, 30.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tf("Retirer le filtre {label}", &[("label", &label)])));
    ui.painter().rect(rect, CornerRadius::same(15), tint(color, 0.14), Stroke::new(1.0, tint(color, if resp.hovered() { 0.8 } else { 0.4 })), StrokeKind::Inside);
    ui.painter().galley(pos2(rect.left() + 12.0, rect.center().y - galley.size().y / 2.0), galley, color);
    icons::paint(ui.painter(), pos2(rect.right() - 15.0, rect.center().y), Icon::X, 14.0, if resp.hovered() { t.ink_strong } else { color });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

// ---- carte -------------------------------------------------------------------------------------

/// La carte : un graphe vivant (voir `screens/map.rs` et `graph.rs`). Le filtre de domaine
/// n'enlève pas de nœuds : il met le domaine en lumière et la caméra glisse jusqu'à lui.
fn map(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>) {
    let all = visible(data, st, true);
    // Le nœud est déjà tenu quand il quitte la carte : on pose la charge utile directement
    // (`dnd_set_drag_payload` n'agit qu'à l'image où le glisser commence).
    super::map::show(ui, data, st, act, &all, &|resp, i| egui::DragAndDrop::set_payload(&resp.ctx, DraggedProjects(vec![i])));
}

// ---- grille et liste ---------------------------------------------------------------------------

/// Un projet de la sélection multiple : une coche dans le coin.
fn paint_check(painter: &egui::Painter, t: &Tokens, center: egui::Pos2) {
    painter.circle_filled(center, 9.0, t.accent);
    icons::paint(painter, center, Icon::Check, 12.0, t.on_accent);
}

fn grid(ui: &mut Ui, data: &Data, ids: &[usize], st: &mut UiState, act: &mut Vec<Action>) {
    let selected = st.selected;
    let multi = st.lib_multi.len() > 1;
    let t = Tokens::get(ui.ctx());
    widgets::draggable(ScrollArea::vertical()).id_salt("library-grid").auto_shrink(false).show(ui, |ui| {
        widgets::scroll_content(ui, |ui| {
            let gap = 12.0;
            let width = ui.available_width();
            let cols = (((width + gap) / (250.0 + gap)).floor() as usize).max(1);
            let w = ((width - gap * (cols - 1) as f32) / cols as f32).floor();
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            for row in ids.chunks(cols) {
                ui.horizontal(|ui| {
                    for &i in row {
                        let p = &data.projects[i];
                        let dm = &data.domains[p.domain];
                        let (rect, resp) = ui.allocate_exact_size(vec2(w, 150.0), Sense::click_and_drag());
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &p.name));
                        let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
                        let painter = ui.painter().clone();
                        let picked = multi && st.lib_multi.contains(&p.id);
                        let stroke = if selected == Some(i) || picked { Stroke::new(2.0, t.accent) } else { Stroke::new(1.0, mix(t.line, t.line_strong, hover)) };
                        painter.rect(rect, CornerRadius::same(theme::R_CARD), mix(t.surface, t.raised, hover * 0.6), stroke, StrokeKind::Inside);
                        let tile = Rect::from_min_size(rect.min + vec2(14.0, 14.0), vec2(36.0, 36.0));
                        painter.rect_filled(tile, CornerRadius::same(9), tint(dm.color, 0.16));
                        icons::paint(&painter, tile.center(), p.icon, 20.0, dm.color);
                        widgets::paint_text(&painter, pos2(tile.right() + 12.0, tile.top()), &p.name, FontId::new(16.0, theme::bold()), t.ink_strong, w - 80.0);
                        widgets::paint_text(&painter, pos2(tile.right() + 12.0, tile.top() + 21.0), &p.kind, FontId::proportional(12.5), t.muted, w - 80.0);
                        let mut x = rect.left() + 14.0;
                        let cy = rect.top() + 76.0;
                        x += widgets::paint_chip(&painter, &t, pos2(x, cy), p.status.label(), widgets::status_tone(p.status)) + 6.0;
                        let tone = if p.attention == Attention::Focus { Tone::Accent } else { Tone::Neutral };
                        widgets::paint_chip(&painter, &t, pos2(x, cy), p.attention.label(), tone);
                        let stack = p.stack.join(" · ");
                        widgets::paint_text(&painter, pos2(rect.left() + 14.0, rect.top() + 100.0), &stack, FontId::new(11.5, FontFamily::Monospace), t.faint, w - 28.0);
                        let g = painter.layout_no_wrap(idle_text(p.idle_days), FontId::proportional(12.0), t.faint);
                        painter.galley(pos2(rect.left() + 14.0, rect.bottom() - 26.0), g, t.faint);
                        painter.text(pos2(rect.right() - 14.0, rect.bottom() - 19.0), Align2::RIGHT_CENTER, icons::ch(Icon::ChevronRight), FontId::new(16.0, theme::icons()), mix(t.faint, t.accent_ink, hover));
                        if picked {
                            paint_check(&painter, &t, pos2(rect.right() - 18.0, rect.top() + 18.0));
                        }
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        drag_source(&resp, group_of(data, st, i));
                        if resp.clicked() {
                            click_project(ui, data, st, ids, i, act);
                        }
                    }
                });
            }
        });
    });
}

/// Les colonnes de la liste, en fraction de la largeur. Moins de colonnes quand la place manque :
/// une colonne à 0 n'est pas affichée.
fn list_columns(width: f32) -> [(&'static str, f32); 7] {
    if width >= 900.0 {
        [(tr("Projet"), 0.24), (tr("Domaine"), 0.16), (tr("Type"), 0.17), (tr("Techno"), 0.14), (tr("État"), 0.11), (tr("Attention"), 0.12), (tr("Activité"), 0.06)]
    } else if width >= 620.0 {
        [(tr("Projet"), 0.32), (tr("Domaine"), 0.22), (tr("Type"), 0.0), (tr("Techno"), 0.0), (tr("État"), 0.16), (tr("Attention"), 0.20), (tr("Activité"), 0.10)]
    } else {
        [(tr("Projet"), 0.50), (tr("Domaine"), 0.0), (tr("Type"), 0.0), (tr("Techno"), 0.0), (tr("État"), 0.30), (tr("Attention"), 0.0), (tr("Activité"), 0.20)]
    }
}

fn list(ui: &mut Ui, data: &Data, ids: &[usize], st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let selected = st.selected;
    let multi = st.lib_multi.len() > 1;
    // La même largeur pour l'en-tête et les lignes (qui laissent 16 points à la barre de défilement).
    let width = ui.available_width() - 16.0;
    let columns = list_columns(width);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
    let mut x = rect.left() + 14.0;
    for (k, (name, frac)) in columns.iter().enumerate() {
        if *frac > 0.0 && k + 1 < columns.len() {
            ui.painter().text(pos2(x, rect.center().y), Align2::LEFT_CENTER, name.to_uppercase(), FontId::new(11.0, theme::semibold()), t.faint);
        }
        x += (width - 28.0) * frac;
    }
    ui.painter().text(pos2(rect.right() - 14.0, rect.center().y), Align2::RIGHT_CENTER, tr("ACTIVITÉ"), FontId::new(11.0, theme::semibold()), t.faint);
    ui.painter().hline(rect.x_range(), rect.bottom(), t.hairline());
    widgets::draggable(ScrollArea::vertical()).id_salt("library-list").auto_shrink(false).show(ui, |ui| {
        widgets::scroll_content(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for &i in ids {
                let p = &data.projects[i];
                let dm = &data.domains[p.domain];
                let (rect, resp) = ui.allocate_exact_size(vec2(width, 46.0), Sense::click_and_drag());
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &p.name));
                let painter = ui.painter().clone();
                let picked = multi && st.lib_multi.contains(&p.id);
                if selected == Some(i) || picked {
                    painter.rect_filled(rect, CornerRadius::same(6), tint(t.accent, 0.12));
                } else if resp.hovered() {
                    painter.rect_filled(rect, CornerRadius::same(6), tint(t.ink, 0.04));
                }
                painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, tint(t.line, 0.6)));
                let cy = rect.center().y;
                let col = |k: usize| rect.left() + 14.0 + (width - 28.0) * columns[..k].iter().map(|c| c.1).sum::<f32>();
                let cw = |k: usize| (width - 28.0) * columns[k].1 - 10.0;
                let shown = |k: usize| columns[k].1 > 0.0;
                icons::paint(&painter, pos2(col(0) + 9.0, cy), p.icon, 17.0, dm.color);
                widgets::paint_text(&painter, pos2(col(0) + 26.0, cy - 9.0), &p.name, FontId::new(14.5, theme::semibold()), t.ink_strong, cw(0) - 26.0);
                if shown(1) {
                    painter.circle_filled(pos2(col(1) + 5.0, cy), 4.0, dm.color);
                    widgets::paint_text(&painter, pos2(col(1) + 16.0, cy - 8.0), &dm.name, FontId::proportional(13.0), t.ink, cw(1) - 16.0);
                }
                if shown(2) {
                    widgets::paint_text(&painter, pos2(col(2), cy - 8.0), &p.kind, FontId::proportional(13.0), t.muted, cw(2));
                }
                if shown(3) {
                    widgets::paint_text(&painter, pos2(col(3), cy - 8.0), &p.stack.join(", "), FontId::new(12.0, FontFamily::Monospace), t.muted, cw(3));
                }
                if shown(4) {
                    widgets::paint_chip(&painter, &t, pos2(col(4), cy), p.status.label(), widgets::status_tone(p.status));
                }
                if shown(5) {
                    let tone = if p.attention == Attention::Focus { Tone::Accent } else { Tone::Neutral };
                    widgets::paint_chip(&painter, &t, pos2(col(5), cy), p.attention.label(), tone);
                }
                painter.text(pos2(rect.right() - 14.0, cy), Align2::RIGHT_CENTER, idle_text(p.idle_days), FontId::proportional(12.5), t.faint);
                if picked {
                    painter.rect_filled(egui::Rect::from_min_size(rect.min + vec2(0.0, 8.0), vec2(3.0, rect.height() - 16.0)), CornerRadius::same(1), t.accent);
                }
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                drag_source(&resp, group_of(data, st, i));
                if resp.clicked() {
                    click_project(ui, data, st, ids, i, act);
                }
            }
        });
    });
}
