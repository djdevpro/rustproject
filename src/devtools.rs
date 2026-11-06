//! Les outils de développement, comme ceux d'un navigateur : F12 (ou Ctrl + Maj + I) ouvre une
//! fenêtre à onglets.
//! - Inspecteur : les cadres et les infos de chaque élément au survol, les zones interactives,
//!   ce que capte le pointeur ; puis l'inspection d'egui (élément survolé, pointeur, couches).
//! - Performances : images par seconde, temps d'une image (courbe), pourquoi l'interface se
//!   redessine.
//! - Mémoire : l'état interne d'egui, les textures et les chargeurs d'images.
//! - Style : le style d'egui, modifiable en direct (jusqu'au redémarrage).
//! - État : l'état de l'application (fichier, projets, index, tâches en cours…), à copier pour
//!   un rapport de bug.

use std::collections::VecDeque;

use egui::{Color32, FontId, RichText, Sense, Stroke, Ui, vec2};

use crate::i18n::{tf, tr};
use crate::theme::{self, Tokens, tint};
use crate::widgets;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Inspect,
    Perf,
    Memory,
    Style,
    State,
}

/// L'état des outils : ouverts ou non, l'onglet, les derniers temps d'image (ms).
#[derive(Default)]
pub struct DevTools {
    pub open: bool,
    pub tab: Tab,
    frames: VecDeque<f32>,
}

/// F12, ou Ctrl + Maj + I : ouvrir ou fermer les outils.
pub fn toggled(ctx: &egui::Context) -> bool {
    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F12) || i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::I))
}

/// La fenêtre des outils. `state` : l'état de l'application, ligne à ligne (libellé, valeur).
pub fn show(ctx: &egui::Context, dev: &mut DevTools, state: &[(String, String)]) {
    let dt = ctx.input(|i| i.unstable_dt) * 1000.0;
    dev.frames.push_back(dt);
    while dev.frames.len() > 240 {
        dev.frames.pop_front();
    }
    let mut open = dev.open;
    egui::Window::new(tr("Outils de développement"))
        .id(egui::Id::unique("devtools"))
        .default_size([480.0, 600.0])
        .default_pos(ctx.viewport_rect().right_top() + vec2(-500.0, 70.0))
        .resizable(true)
        .collapsible(true)
        .open(&mut open)
        .show(ctx, |ui| {
            let tabs = [(Tab::Inspect, tr("Inspecteur")), (Tab::Perf, tr("Performances")), (Tab::Memory, tr("Mémoire")), (Tab::Style, tr("Style")), (Tab::State, tr("État"))];
            widgets::tabs(ui, "devtools-tabs", &mut dev.tab, &tabs);
            ui.add_space(8.0);
            egui::ScrollArea::vertical().id_salt("devtools-scroll").auto_shrink(false).show(ui, |ui| match dev.tab {
                Tab::Inspect => inspect(ui),
                Tab::Perf => perf(ui, &dev.frames),
                Tab::Memory => {
                    ctx.memory_ui(ui);
                    ui.separator();
                    ctx.texture_ui(ui);
                    ui.separator();
                    ctx.loaders_ui(ui);
                }
                Tab::Style => {
                    ui.label(RichText::new(tr("Les changements durent jusqu'au redémarrage.")).size(12.0).color(Tokens::get(ctx).faint));
                    ctx.style_ui(ui, ctx.theme());
                }
                Tab::State => app_state(ui, state),
            });
        });
    dev.open = open;
}

/// L'inspecteur : les aides visuelles d'egui, puis son inspection.
fn inspect(ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    ui.label(RichText::new(tr("Survole l'interface : chaque élément montre son cadre, sa taille et son rôle.")).size(12.5).color(t.muted));
    let debug = ui.ctx().global_style().debug;
    let mut options = [
        (debug.debug_on_hover, tr("Cadres et infos au survol")),
        (debug.show_interactive_widgets, tr("Zones cliquables")),
        (debug.show_widget_hits, tr("Ce que capte le pointeur")),
        (debug.show_expand_width, tr("Éléments qui s'élargissent")),
        (debug.show_resize, tr("Zones redimensionnables")),
    ];
    let mut changed = false;
    for (on, label) in options.iter_mut() {
        changed |= ui.checkbox(on, *label).changed();
    }
    if changed {
        let [hover, interactive, hits, expand, resize] = options.map(|(on, _)| on);
        ui.ctx().all_styles_mut(|s| {
            s.debug.debug_on_hover = hover;
            s.debug.show_interactive_widgets = interactive;
            s.debug.show_widget_hits = hits;
            s.debug.show_expand_width = expand;
            s.debug.show_resize = resize;
        });
    }
    ui.add_space(6.0);
    ui.separator();
    ui.ctx().clone().inspection_ui(ui);
}

/// Les performances : images par seconde, temps d'une image, ce qui redemande un rendu.
fn perf(ui: &mut Ui, frames: &VecDeque<f32>) {
    let t = Tokens::get(ui.ctx());
    let n = frames.len().max(1) as f32;
    let avg = frames.iter().sum::<f32>() / n;
    let max = frames.iter().copied().fold(0.0, f32::max);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        for (value, label) in [(format!("{:.0}", 1000.0 / avg.max(0.001)), tr("images/s")), (format!("{avg:.1} ms"), tr("par image (moyenne)")), (format!("{max:.1} ms"), tr("la plus lente"))] {
            ui.vertical(|ui| {
                ui.label(RichText::new(value).font(FontId::new(20.0, theme::bold())).color(t.ink_strong));
                ui.label(RichText::new(label).size(12.0).color(t.muted));
            });
        }
    });
    // La courbe des dernières images ; la ligne marque 16,7 ms (60 images/s).
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 90.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 6.0, t.input);
    let top = max.max(20.0);
    let y = |ms: f32| rect.bottom() - (ms / top).min(1.0) * rect.height();
    p.hline(rect.x_range(), y(16.7), Stroke::new(1.0, tint(t.good_fg, 0.5)));
    let step = rect.width() / 240.0;
    let points: Vec<egui::Pos2> = frames.iter().enumerate().map(|(k, ms)| egui::pos2(rect.right() - (frames.len() - k) as f32 * step, y(*ms))).collect();
    p.add(egui::Shape::line(points, Stroke::new(1.5, t.accent)));
    ui.label(RichText::new(tr("La ligne verte : 60 images par seconde. L'interface ne se redessine que si quelque chose change.")).size(12.0).color(t.faint));
    ui.add_space(6.0);
    widgets::caps(ui, tr("Pourquoi elle se redessine"));
    let causes = ui.ctx().repaint_causes();
    if causes.is_empty() {
        ui.label(RichText::new(tr("Rien : elle attend une action.")).size(12.5).color(t.muted));
    }
    for c in causes.iter().take(8) {
        ui.label(RichText::new(c.to_string()).font(FontId::monospace(11.5)).color(t.muted));
    }
}

/// L'état de l'application, et un bouton pour le copier.
fn app_state(ui: &mut Ui, state: &[(String, String)]) {
    let t = Tokens::get(ui.ctx());
    if widgets::button(ui, Some(crate::icons::Icon::Copy), tr("Copier l'état"), widgets::Kind::Secondary).clicked() {
        let text = state.iter().map(|(k, v)| format!("{k} : {v}")).collect::<Vec<_>>().join("\n");
        ui.ctx().copy_text(text);
    }
    ui.add_space(6.0);
    egui::Grid::new("devtools-state").num_columns(2).spacing([16.0, 6.0]).striped(true).show(ui, |ui| {
        for (k, v) in state {
            ui.label(RichText::new(k).size(12.5).color(t.muted));
            ui.label(RichText::new(v).font(FontId::monospace(12.0)).color(if v.is_empty() { Color32::TRANSPARENT } else { t.ink }));
            ui.end_row();
        }
    });
    ui.add_space(6.0);
    ui.label(RichText::new(tf("{0} lignes", &[("0", &state.len())])).size(11.5).color(t.faint));
}
