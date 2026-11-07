//! La carte de la Bibliothèque : un graphe vivant. La simulation est dans `graph.rs` ; ici, le
//! dessin et les gestes.
//!
//! - Les domaines sont des moyeux lumineux, les projets gravitent autour, reliés à leur domaine.
//!   Liens pointillés : projets rangés ensemble (gris) et technos communes (ambre).
//! - Survoler un nœud allume ses voisins (le reste s'estompe) et fait courir des particules sur
//!   ses liens. Un clic sur un domaine le filtre (la caméra glisse jusqu'à lui), sur un projet
//!   l'ouvre.
//! - Glisser le fond déplace la vue, la molette zoome sous la souris, double-clic : recentrer.
//!   Un nœud se prend à la main et entraîne ses voisins ; sorti de la carte, il devient un
//!   glisser-déposer vers les filtres d'attention.

use std::time::Duration;

use egui::{Align, Align2, Color32, CornerRadius, FontId, Id, Layout, Pos2, Rect, Response, RichText, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

use crate::app::{Action, UiState, idle_text};
use crate::graph::EdgeKind;
use crate::icons::{self, Icon};
use crate::model::{Attention, Data, Status};
use crate::theme::{self, Tokens, mix, tint};
use crate::widgets;
use crate::i18n::{tf, tr};

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Le léger flottement d'un nœud au repos (unités du monde).
fn sway(k: usize, now: f32) -> Vec2 {
    let k = k as f32;
    vec2((now * 0.8 + k * 1.7).sin(), (now * 0.63 + k * 2.3).cos()) * 1.6
}

/// `drag_out` est appelé quand on emporte un projet hors de la carte (glisser-déposer).
pub fn show(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>, visible: &[usize], drag_out: &dyn Fn(&Response, usize)) {
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    let size = vec2(ui.available_width(), (ui.available_height() - 4.0).max(380.0));
    let (area, bg) = ui.allocate_exact_size(size, Sense::click_and_drag());
    bg.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Canvas, true, tr("Carte des projets")));
    let painter = ui.painter_at(area);
    painter.rect(area, CornerRadius::same(theme::R_CARD), t.surface, Stroke::new(1.0, t.line), StrokeKind::Inside);

    let now = ui.input(|i| i.time) as f32;
    let dt = ui.input(|i| i.stable_dt).clamp(1.0 / 240.0, 0.1);
    let domain_filter = st.lib_domain;
    let selected = st.selected;
    let mut toggle_domain = None;
    let g = &mut st.graph;

    // ---- simulation
    if g.sim.sync(data, visible) {
        g.dragging = None;
        g.last_hover = None;
    }
    if g.instant {
        if g.sim.warm() {
            g.sim.settle();
        }
        g.snap = true;
    } else if g.sim.warm() {
        for _ in 0..((dt * 60.0).round() as usize).clamp(1, 3) {
            g.sim.tick();
        }
    }
    dot_grid(&painter, &t, area, g.center, g.zoom);
    if g.sim.nodes.is_empty() {
        painter.text(area.center(), Align2::CENTER_CENTER, tr("Aucun projet avec ces filtres"), FontId::proportional(15.0), t.muted);
        return;
    }

    // ---- caméra : elle suit le graphe tant qu'on ne l'a pas déplacée, et glisse vers un domaine filtré.
    if g.filter_seen != Some(domain_filter) {
        if g.filter_seen.is_some() {
            g.auto_fit = true;
        }
        g.filter_seen = Some(domain_filter);
    }
    let fit_domain = domain_filter.filter(|d| g.sim.nodes.iter().any(|n| n.domain == *d));
    let mut camera_moving = false;
    if g.auto_fit
        && let Some(target) = g.sim.fit(area.size(), fit_domain)
    {
        if g.snap {
            (g.center, g.zoom) = target;
        } else {
            camera_moving = g.ease_to(target, dt);
        }
    }
    g.snap = false;
    let (origin, center, zoom) = (area.center(), g.center, g.zoom);
    let to_screen = |w: Vec2| origin + (w - center) * zoom;
    let to_world = |p: Pos2| (p - origin) / zoom + center;

    // ---- positions à l'écran (avec un flottement très léger quand la fenêtre a le focus)
    let ambient = !g.instant && ui.input(|i| i.viewport().focused.unwrap_or(true));
    let pos: Vec<Pos2> = g
        .sim
        .nodes
        .iter()
        .enumerate()
        .map(|(k, n)| {
            let drift = if ambient && g.dragging != Some(k) { sway(k, now) * if n.is_hub() { 0.5 } else { 1.0 } } else { Vec2::ZERO };
            to_screen(n.pos + drift)
        })
        .collect();
    let radii: Vec<f32> = g.sim.nodes.iter().map(|n| if n.is_hub() { (n.radius * zoom).max(12.0) } else { (n.radius * zoom).max(3.5) }).collect();
    let screen_radius = |k: usize| radii[k];

    // ---- gestes sur les nœuds (les projets d'abord, les domaines par-dessus)
    let mut hovered = None;
    let order: Vec<usize> = (0..g.sim.nodes.len()).filter(|&k| !g.sim.nodes[k].is_hub()).chain((0..g.sim.nodes.len()).filter(|&k| g.sim.nodes[k].is_hub())).collect();
    for k in order {
        let (key, index, hub) = (g.sim.nodes[k].key, g.sim.nodes[k].index, g.sim.nodes[k].is_hub());
        let hit = Rect::from_center_size(pos[k], Vec2::splat((screen_radius(k) * 2.0 + 8.0).max(16.0))).intersect(area);
        if !hit.is_positive() {
            continue;
        }
        let resp = ui.interact(hit, Id::unique(("graph-node", key)), Sense::click_and_drag());
        let name = if hub { tf("Domaine {0}", &[("0", &data.domains[index].name)]) } else { data.projects[index].name.clone() };
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
        if resp.hovered() {
            hovered = Some(k);
        }
        if resp.drag_started() {
            g.dragging = Some(k);
            g.sim.nodes[k].fixed = true;
            g.sim.alpha_target = 0.25;
            g.sim.reheat(0.3);
        }
        if g.dragging == Some(k)
            && resp.dragged()
            && let Some(p) = resp.interact_pointer_pos()
        {
            if area.contains(p) {
                g.sim.nodes[k].pos = to_world(p);
            } else if !hub {
                // Emporté hors de la carte : il devient un glisser-déposer (vers une attention).
                drag_out(&resp, index);
            }
        }
        if resp.drag_stopped() {
            g.sim.nodes[k].fixed = false;
            g.dragging = None;
            g.sim.alpha_target = 0.0;
        }
        let resp = if g.dragging.is_some() {
            resp.on_hover_cursor(egui::CursorIcon::Grabbing)
        } else if hub {
            resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            let p = &data.projects[index];
            resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_ui(|ui| {
                ui.label(RichText::new(&p.name).font(FontId::new(15.0, theme::bold())));
                if !p.description.is_empty() {
                    ui.label(RichText::new(&p.description).size(13.0).color(t.ink));
                }
                ui.label(RichText::new(format!("{} · {}", p.kind, p.stack.join(", "))).size(12.5).color(t.muted));
                ui.label(RichText::new(format!("{} · {} · {}", p.status.label(), p.attention.label(), idle_text(p.idle_days))).size(12.5).color(t.muted));
            })
        };
        if resp.clicked() {
            if hub {
                toggle_domain = Some(index);
            } else {
                act.push(Action::Select(index));
            }
        }
    }

    // ---- gestes sur le fond : déplacer, zoomer, recentrer
    if bg.dragged() && g.dragging.is_none() {
        g.center -= bg.drag_delta() / zoom;
        g.auto_fit = false;
    }
    if bg.double_clicked() {
        g.auto_fit = true;
    }
    let grabbing = bg.dragged();
    let bg = bg.on_hover_cursor(if grabbing { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
    if let Some(p) = ui.input(|i| i.pointer.hover_pos()).filter(|p| area.contains(*p)) {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let factor = (scroll * 0.0018).exp() * pinch;
        if (factor - 1.0).abs() > 1e-4 {
            g.zoom_at(p - origin, factor);
            g.auto_fit = false;
            ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
        }
    }
    let _ = bg;

    // ---- mise en lumière : le nœud survolé et ses voisins ; le domaine filtré
    if let Some(h) = hovered.or(g.dragging) {
        g.last_hover = Some(h);
    }
    let hl = ctx.animate_bool_with_time(Id::unique("graph-highlight"), hovered.is_some() || g.dragging.is_some(), 0.18);
    let spot = g.last_hover.filter(|&f| hl > 0.0 && f < g.sim.nodes.len());
    if domain_filter.is_some() {
        g.last_domain = domain_filter;
    }
    let df = ctx.animate_bool_with_time(Id::unique("graph-domain-focus"), domain_filter.is_some(), 0.3);
    let nodes = &g.sim.nodes;
    let neighbors = &g.sim.neighbors;
    let lit = |k: usize| spot.is_some_and(|f| f == k || neighbors[f].contains(&k));
    let fade = |k: usize| {
        let mut a = 1.0;
        if g.last_domain.is_some_and(|d| nodes[k].domain != d && !nodes[k].also.contains(&d)) {
            a *= 1.0 - 0.82 * df;
        }
        if spot.is_some() && !lit(k) {
            a *= 1.0 - 0.72 * hl;
        }
        a
    };

    // ---- liens : par défaut, seuls les rayons des grappes, très légers. Les autres liens
    // (autres domaines, rangés ensemble, technos, tags) n'apparaissent que pour le point survolé,
    // le projet ouvert (cliqué) ou le centre de grappe survolé. Les allumés par-dessus, avec des
    // particules sur ceux du point survolé.
    let opened = nodes.iter().position(|n| !n.is_hub() && Some(n.index) == selected);
    for pass_lit in [false, true] {
        for (e_idx, e) in g.sim.edges.iter().enumerate() {
            let hover_lit = spot.is_some_and(|f| e.a == f || e.b == f);
            let open_lit = opened.is_some_and(|s| e.a == s || e.b == s);
            let is_lit = hover_lit || open_lit;
            if is_lit != pass_lit {
                continue;
            }
            let (a, b) = (pos[e.a], pos[e.b]);
            let alpha = fade(e.a).min(fade(e.b));
            let lit_amount = if hover_lit { hl.max(if open_lit { 0.75 } else { 0.0 }) } else if open_lit { 0.75 } else { 0.0 };
            let (base, width, dim, bright) = match e.kind {
                EdgeKind::Domain => (data.domains[nodes[e.a].domain].color, 1.0, 0.10, 0.85),
                EdgeKind::AlsoDomain => (data.domains[nodes[e.a].domain].color, 1.0, 0.0, 0.75),
                EdgeKind::Family => (t.ink, 1.2, 0.0, 0.8),
                EdgeKind::Stack => (t.accent, 1.0, 0.0, 0.7),
                EdgeKind::Tags => (t.info, 1.0, 0.0, 0.7),
            };
            let strength = (dim + (bright - dim) * lit_amount) * alpha;
            if strength < 0.01 {
                continue;
            }
            let stroke = Stroke::new(width + 0.7 * lit_amount, tint(base, strength));
            if matches!(e.kind, EdgeKind::Domain | EdgeKind::AlsoDomain) {
                painter.line_segment([a, b], stroke);
            } else {
                painter.extend(Shape::dashed_line(&[a, b], stroke, 5.0, 4.0));
            }
            if hover_lit && hl > 0.05 && !g.instant {
                // Une particule court sur le lien, du domaine (ou du nœud survolé) vers l'autre bout.
                let (from, to) = if matches!(e.kind, EdgeKind::Domain | EdgeKind::AlsoDomain) || spot == Some(e.a) { (a, b) } else { (b, a) };
                for phase in [0.0, 0.5] {
                    let f = (now * 0.5 + e_idx as f32 * 0.37 + phase).fract();
                    painter.circle_filled(from + (to - from) * f, 2.2, tint(base, 0.9 * hl * (1.0 - (f - 0.5).abs())));
                }
            }
        }
    }

    // ---- projets
    for (k, n) in nodes.iter().enumerate().filter(|(_, n)| !n.is_hub()) {
        let p = &data.projects[n.index];
        let color = data.domains[n.domain].color;
        let grow = ctx.animate_bool_with_time(Id::unique(("graph-grow", n.key)), hovered == Some(k) || g.dragging == Some(k), 0.15);
        let is_selected = selected == Some(n.index);
        let a = fade(k);
        let status_alpha = match (p.status, p.attention) {
            (Status::Archived, _) => 0.3,
            (_, Attention::Reserve) => 0.45,
            _ => 1.0,
        };
        let r = screen_radius(k) * (1.0 + 0.35 * grow);
        let c = pos[k];
        let glow = grow.max(if is_selected { 0.8 } else { 0.0 });
        if glow > 0.0 {
            painter.circle_filled(c, r * 2.5, tint(color, 0.07 * glow * a));
            painter.circle_filled(c, r * 1.7, tint(color, 0.14 * glow * a));
        }
        painter.circle_filled(c, r, tint(color, status_alpha * a));
        painter.circle_stroke(c, r - 0.5, Stroke::new(1.0, tint(Color32::WHITE, 0.16 * status_alpha * a)));
        if p.attention == Attention::Focus && p.status != Status::Archived {
            painter.circle_stroke(c, r + 3.5, Stroke::new(2.0, tint(t.accent, a)));
        }
        if is_selected {
            painter.circle_stroke(c, r + 7.0, Stroke::new(2.0, tint(t.ink_strong, a.max(0.5))));
        }
    }

    // ---- domaines
    for (k, n) in nodes.iter().enumerate().filter(|(_, n)| n.is_hub()) {
        let dm = &data.domains[n.index];
        let grow = ctx.animate_bool_with_time(Id::unique(("graph-grow", n.key)), hovered == Some(k) || g.dragging == Some(k), 0.15);
        let a = fade(k);
        let r = screen_radius(k) * (1.0 + 0.12 * grow);
        let c = pos[k];
        painter.circle_filled(c, r * 2.2, tint(dm.color, (0.045 + 0.04 * grow) * a));
        painter.circle_filled(c, r * 1.55, tint(dm.color, (0.08 + 0.06 * grow) * a));
        painter.circle_filled(c, r, tint(mix(t.surface, dm.color, 0.22), a));
        painter.circle_stroke(c, r, Stroke::new(1.5, tint(dm.color, 0.9 * a)));
        if domain_filter == Some(n.index) {
            painter.circle_stroke(c, r + 5.0, Stroke::new(2.0, tint(dm.color, a)));
        }
        icons::paint(&painter, c, dm.icon, (r * 0.95).clamp(11.0, 28.0), tint(dm.color, a));
    }

    // ---- étiquettes : les domaines toujours ; les projets par priorité, sans se chevaucher
    // (survolé, sélectionné, voisins allumés, focus, puis les plus récents). Une étiquette qui
    // en recouvrirait une autre n'est pas affichée : le zoom en révèle davantage.
    let near = smoothstep(0.55, 0.7, zoom);
    let mut placed: Vec<Rect> = Vec::new();
    let mut pills = Vec::new();
    for (k, n) in nodes.iter().enumerate().filter(|(_, n)| n.is_hub()) {
        let dm = &data.domains[n.index];
        let a = fade(k).max(0.25);
        let count = nodes.iter().filter(|m| !m.is_hub() && m.domain == n.index).count();
        let name = painter.layout_no_wrap(dm.name.clone(), FontId::new(13.0, theme::semibold()), tint(dm.color, a));
        let num = painter.layout_no_wrap(count.to_string(), FontId::new(12.0, theme::semibold()), tint(t.faint, a));
        let w = name.size().x + num.size().x + 26.0;
        let pill = Rect::from_min_size(pos[k] + vec2(-w / 2.0, screen_radius(k) + 8.0), vec2(w, 22.0));
        placed.push(pill.expand(2.0));
        placed.push(Rect::from_center_size(pos[k], Vec2::splat(screen_radius(k) * 2.0)));
        pills.push((pill, name, num, dm.color, a));
    }
    let mut labels = Vec::new();
    for (k, n) in nodes.iter().enumerate().filter(|(_, n)| !n.is_hub()) {
        let p = &data.projects[n.index];
        let is_hover = hovered == Some(k) || g.dragging == Some(k);
        let is_selected = selected == Some(n.index);
        let focus = p.attention == Attention::Focus && p.status != Status::Archived;
        let is_lit = lit(k) && hl > 0.3;
        let priority = if is_hover { 4 } else if is_selected { 3 } else if is_lit { 2 } else if focus { 1 } else { 0 };
        let mut alpha: f32 = if p.attention == Attention::Reserve { near * 0.65 } else { near };
        if priority >= 1 {
            alpha = 1.0;
        }
        let alpha = if is_hover || is_selected { 1.0 } else { alpha * fade(k) };
        if alpha < 0.05 {
            continue;
        }
        labels.push((priority, n.radius, k, alpha, is_hover, is_selected, focus));
    }
    labels.sort_by(|x, y| y.0.cmp(&x.0).then(y.1.total_cmp(&x.1)).then(x.2.cmp(&y.2)));
    for (priority, _, k, alpha, is_hover, is_selected, focus) in labels {
        let p = &data.projects[nodes[k].index];
        // Après l'anneau de focus ou de sélection.
        let ring = if is_selected { 9.0 } else if focus { 6.0 } else { 0.0 };
        let font = if is_hover { FontId::new(13.5, theme::semibold()) } else { FontId::new(12.0, theme::medium()) };
        let color = if p.attention == Attention::Reserve { t.muted } else { t.ink };
        let galley = painter.layout_no_wrap(p.name.clone(), font, tint(color, alpha));
        let at = pos[k] + vec2(screen_radius(k) + ring + 7.0, -galley.size().y / 2.0);
        let rect = Rect::from_min_size(at, galley.size()).expand(2.0);
        if priority < 3 && (placed.iter().any(|r| r.intersects(rect)) || !area.shrink(4.0).contains_rect(rect)) {
            continue;
        }
        placed.push(rect);
        if is_hover {
            painter.rect_filled(rect.expand2(vec2(3.0, 0.0)), CornerRadius::same(6), tint(t.raised, 0.92));
        }
        painter.galley(at, galley, tint(color, alpha));
    }
    for (pill, name, num, color, a) in pills {
        painter.rect(pill, CornerRadius::same(11), tint(t.surface, 0.88 * a), Stroke::new(1.0, tint(color, 0.35 * a)), StrokeKind::Inside);
        painter.galley(pos2(pill.left() + 10.0, pill.center().y - name.size().y / 2.0), name, tint(color, a));
        painter.galley(pos2(pill.right() - 10.0 - num.size().x, pill.center().y - num.size().y / 2.0), num, tint(t.faint, a));
    }

    // ---- habillage : aide, légende, commandes
    let hint = painter.layout_no_wrap(tr("Survole ou clique un point pour voir ses liens · glisse pour te déplacer · molette pour zoomer").into(), FontId::proportional(12.0), t.faint);
    if area.left() + 16.0 + hint.size().x < area.right() - 16.0 - 4.0 * 36.0 - 150.0 {
        painter.galley(area.left_top() + vec2(16.0, 16.0), hint, t.faint);
    }
    let projects = nodes.iter().filter(|n| !n.is_hub()).count();
    legend(&painter, &t, area);
    let bar = Rect::from_min_size(pos2(area.right() - 16.0 - 4.0 * 36.0, area.top() + 10.0), vec2(4.0 * 36.0, 34.0));
    painter.text(pos2(bar.left() - 10.0, bar.center().y), Align2::RIGHT_CENTER, tf("{projects} projets · {0} liens", &[("projects", &projects), ("0", &g.sim.edges.len())]), FontId::proportional(12.0), t.faint);
    let mut tools = ui.new_child(UiBuilder::new().max_rect(bar).layout(Layout::right_to_left(Align::Center)));
    tools.spacing_mut().item_spacing.x = 4.0;
    if widgets::icon_button(&mut tools, Icon::Maximize, tr("Recentrer la carte"), 32.0).clicked() {
        g.auto_fit = true;
    }
    if widgets::icon_button(&mut tools, Icon::Wind, tr("Secouer le graphe"), 32.0).clicked() {
        g.sim.shake();
    }
    if widgets::icon_button(&mut tools, Icon::Minus, tr("Zoom arrière"), 32.0).clicked() {
        g.zoom_at(Vec2::ZERO, 0.8);
        g.auto_fit = false;
    }
    if widgets::icon_button(&mut tools, Icon::Plus, tr("Zoom avant"), 32.0).clicked() {
        g.zoom_at(Vec2::ZERO, 1.25);
        g.auto_fit = false;
    }

    // ---- images suivantes : à plein régime tant que ça bouge, puis au ralenti pour le flottement
    if !g.instant {
        let animating = g.sim.warm() || camera_moving || g.dragging.is_some() || (0.0..1.0).contains(&hl) && hl > 0.0 || (df > 0.0 && df < 1.0);
        if animating {
            ctx.request_repaint();
        } else if ambient || hl > 0.0 {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
    }
    if let Some(d) = toggle_domain {
        st.lib_domain = if st.lib_domain == Some(d) { None } else { Some(d) };
    }
}

/// Une trame de points qui suit la caméra (on sent le déplacement et le zoom).
fn dot_grid(painter: &egui::Painter, t: &Tokens, area: Rect, center: Vec2, zoom: f32) {
    let mut step = 28.0 * zoom;
    while step < 14.0 {
        step *= 2.0;
    }
    let offset = (-center * zoom + area.center().to_vec2()).to_pos2();
    let start = pos2(area.left() + (offset.x - area.left()).rem_euclid(step), area.top() + (offset.y - area.top()).rem_euclid(step));
    let mut y = start.y;
    while y < area.bottom() {
        let mut x = start.x;
        while x < area.right() {
            painter.circle_filled(pos2(x, y), 0.9, tint(t.ink, 0.07));
            x += step;
        }
        y += step;
    }
}

fn legend(p: &egui::Painter, t: &Tokens, area: Rect) {
    let y = area.bottom() - 20.0;
    let mut x = area.left() + 20.0;
    let mut item = |draw: &dyn Fn(Pos2), label: &str| {
        draw(pos2(x + 8.0, y));
        let g = p.layout_no_wrap(label.to_owned(), FontId::proportional(12.0), t.faint);
        p.galley(pos2(x + 22.0, y - g.size().y / 2.0), g.clone(), t.faint);
        x += 34.0 + g.size().x;
    };
    item(&|c| { p.circle_filled(c, 7.0, t.muted); }, tr("actif cette semaine"));
    item(&|c| { p.circle_filled(c, 4.5, t.muted); }, tr("plus ancien"));
    item(&|c| { p.circle_stroke(c, 6.5, Stroke::new(2.0, t.accent)); }, "focus");
    item(&|c| { p.circle_filled(c, 5.5, tint(t.muted, 0.42)); }, tr("en réserve"));
    item(&|c| { p.extend(Shape::dashed_line(&[c - vec2(8.0, 0.0), c + vec2(8.0, 0.0)], Stroke::new(1.2, tint(t.ink, 0.5)), 4.0, 3.0)); }, tr("rangés ensemble"));
    item(&|c| { p.extend(Shape::dashed_line(&[c - vec2(8.0, 0.0), c + vec2(8.0, 0.0)], Stroke::new(1.2, tint(t.accent, 0.8)), 4.0, 3.0)); }, tr("technos communes (au survol)"));
    item(&|c| { p.extend(Shape::dashed_line(&[c - vec2(8.0, 0.0), c + vec2(8.0, 0.0)], Stroke::new(1.2, tint(t.info, 0.8)), 4.0, 3.0)); }, tr("tags communs (au survol)"));
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    use crate::app::{AtelierApp, LibView, Screen};

    fn library() -> Harness<'static, AtelierApp> {
        let mut h = Harness::builder().with_size(egui::vec2(1480.0, 920.0)).with_max_steps(60).build_eframe(|cc| AtelierApp::demo(cc));
        h.state_mut().dry_run = true;
        h.state_mut().st.graph.instant = true;
        h.state_mut().st.screen = Screen::Library;
        h.state_mut().st.lib_view = LibView::Map;
        h.state_mut().st.selected = None;
        h.run();
        h
    }

    #[test]
    fn carte_survol_filtre_et_ouverture() {
        let mut h = library();
        assert!(!h.state().st.graph.sim.nodes.is_empty(), "le graphe est construit");
        assert!(h.state().st.graph.sim.alpha <= crate::graph::ALPHA_MIN, "et posé");
        // Survoler un domaine allume ses projets.
        h.get_by_label("Domaine Outils de dev").hover();
        h.run();
        // Un clic sur un domaine le filtre : la caméra se rapproche.
        let zoom_before = h.state().st.graph.zoom;
        h.get_by_label("Domaine Outils de dev").click_accesskit();
        h.run();
        let outils = h.state().data.domains.iter().position(|d| d.name == "Outils de dev");
        assert_eq!(h.state().st.lib_domain, outils);
        assert!(h.state().st.graph.zoom > zoom_before, "zoom sur le domaine");
        // Un clic sur un projet l'ouvre.
        h.get_by_label("BunFlow").click_accesskit();
        h.run();
        let bunflow = h.state().data.projects.iter().position(|p| p.name == "BunFlow");
        assert_eq!(h.state().st.selected, bunflow);
        // Recentrer : la caméra cadre de nouveau, le zoom manuel est oublié.
        h.state_mut().st.graph.auto_fit = false;
        h.get_by_label("Recentrer la carte").click_accesskit();
        h.run();
        assert!(h.state().st.graph.auto_fit);
    }
}
