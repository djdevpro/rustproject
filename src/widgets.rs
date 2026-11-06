//! La boîte à widgets d'Atelier. Chaque widget suit le même schéma : réserver un rectangle,
//! lire l'interaction, peindre avec les tokens, et se décrire (`widget_info`) pour les lecteurs
//! d'écran et les tests.

use egui::{
    Align, Align2, Color32, CornerRadius, CursorIcon, FontFamily, FontId, Frame, Id, Margin, Rect, ResizeDirection, Response, Role, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2, ViewportCommand, WidgetInfo, pos2, vec2,
};

use crate::icons::{self, Icon};
use crate::model::{Attention, Launcher, Status};
use crate::theme::{self, R_CARD, R_CHIP, R_CTRL, Tokens, mix, tint};

fn tok(ui: &Ui) -> Tokens {
    Tokens::get(ui.ctx())
}

fn text_size(ui: &Ui, text: &str, font: &FontId) -> Vec2 {
    ui.painter().layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER).size()
}

// ---- textes ------------------------------------------------------------------------------------

/// Titre de section (« Mes projets du moment »).
pub fn title(ui: &mut Ui, text: &str, size: f32) -> Response {
    let t = tok(ui);
    ui.label(egui::RichText::new(text).font(FontId::new(size, theme::bold())).color(t.ink_strong))
}

/// Texte secondaire.
pub fn muted(ui: &mut Ui, text: &str) -> Response {
    let t = tok(ui);
    ui.label(egui::RichText::new(text).color(t.muted))
}

/// Petit label en capitales espacées (« OUVRIR AVEC »).
pub fn caps(ui: &mut Ui, text: &str) -> Response {
    let t = tok(ui);
    let format = egui::TextFormat { font_id: FontId::new(11.0, theme::semibold()), extra_letter_spacing: 1.2, color: t.faint, ..Default::default() };
    let galley = ui.painter().layout_job(egui::text::LayoutJob::single_section(text.to_uppercase(), format));
    let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, t.faint);
    resp
}

/// Label au-dessus d'un champ de formulaire.
pub fn field_label(ui: &mut Ui, text: &str) -> Response {
    let t = tok(ui);
    ui.label(egui::RichText::new(text).font(FontId::new(13.0, theme::semibold())).color(t.ink))
}

// ---- pastilles ---------------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Tone {
    Good,
    Neutral,
    /// Contour d'accent (« Focus »).
    Accent,
    /// Accent plein (« En pause »).
    Solid,
    Danger,
    Custom(Color32),
}

pub fn chip(ui: &mut Ui, text: &str, tone: Tone) -> Response {
    chip_sized(ui, text, tone, 12.5, 24.0)
}

/// Fond, encre et contour d'une pastille.
pub fn tone_colors(t: &Tokens, tone: Tone) -> (Color32, Color32, Stroke) {
    match tone {
        Tone::Good => (t.good_bg, t.good_fg, Stroke::NONE),
        Tone::Neutral => (t.neutral_bg, t.neutral_fg, Stroke::NONE),
        Tone::Accent => (t.accent_soft, t.accent_ink, Stroke::new(1.0, t.accent)),
        Tone::Solid => (t.accent, t.on_accent, Stroke::NONE),
        Tone::Danger => (tint(t.danger, 0.16), t.danger, Stroke::NONE),
        Tone::Custom(c) => (tint(c, 0.17), if t.dark { c } else { mix(c, Color32::BLACK, 0.35) }, Stroke::NONE),
    }
}

/// Peint une pastille dont le bord gauche est centré verticalement sur `left_center`.
/// Renvoie sa largeur (pour enchaîner).
pub fn paint_chip(p: &egui::Painter, t: &Tokens, left_center: egui::Pos2, text: &str, tone: Tone) -> f32 {
    let (bg, fg, stroke) = tone_colors(t, tone);
    let galley = p.layout_no_wrap(text.to_owned(), FontId::new(12.5, theme::semibold()), fg);
    let rect = Rect::from_min_size(pos2(left_center.x, left_center.y - 12.0), vec2(galley.size().x + 18.0, 24.0));
    p.rect(rect, CornerRadius::same(R_CHIP), bg, stroke, StrokeKind::Inside);
    p.galley(rect.center() - galley.size() / 2.0, galley, fg);
    rect.width()
}

/// Peint un texte d'une ligne en haut à gauche de `pos`, coupé par « … » au-delà de `max_w`.
pub fn paint_text(p: &egui::Painter, pos: egui::Pos2, text: &str, font: FontId, color: Color32, max_w: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = max_w.max(10.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = p.layout_job(job);
    p.galley(pos, galley.clone(), color);
    galley
}

pub fn chip_sized(ui: &mut Ui, text: &str, tone: Tone, size: f32, height: f32) -> Response {
    let t = tok(ui);
    let (bg, fg, stroke) = tone_colors(&t, tone);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::new(size, theme::semibold()), fg);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 18.0, height), Sense::hover());
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(R_CHIP), bg, stroke, StrokeKind::Inside);
    p.galley(rect.center() - galley.size() / 2.0, galley, fg);
    resp
}

pub fn status_tone(s: Status) -> Tone {
    match s {
        Status::Idea => Tone::Custom(theme::rgb(0x7aa7ff)),
        Status::Active => Tone::Good,
        Status::Paused => Tone::Solid,
        Status::Done => Tone::Custom(theme::rgb(0x6fcf8e)),
        Status::Archived => Tone::Neutral,
    }
}

pub fn status_chip(ui: &mut Ui, s: Status) -> Response {
    chip(ui, s.label(), status_tone(s))
}

pub fn attention_chip(ui: &mut Ui, a: Attention) -> Response {
    chip(ui, a.label(), attention_tone(a))
}

pub fn attention_tone(a: Attention) -> Tone {
    if a == Attention::Focus { Tone::Accent } else { Tone::Neutral }
}

/// Une pastille qui se change d'un clic : une petite flèche dit qu'on peut choisir, et un menu
/// propose les valeurs. Renvoie la nouvelle valeur si on en a choisi une autre.
pub fn chip_menu<T: Copy + PartialEq>(ui: &mut Ui, name: &str, value: T, tone: Tone, options: &[(T, &str)]) -> Option<T> {
    let t = tok(ui);
    let label = options.iter().find(|(v, _)| *v == value).map(|(_, l)| *l).unwrap_or("");
    let (bg, fg, stroke) = tone_colors(&t, tone);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::new(12.5, theme::semibold()), fg);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 36.0, 24.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::ComboBox, true, name));
    let popup_id = egui::Popup::default_response_id(&resp);
    let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered() || open);
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(R_CHIP), bg, stroke, StrokeKind::Inside);
    p.rect_filled(rect, CornerRadius::same(R_CHIP), tint(fg, 0.12 * hover));
    p.galley(pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0), galley, fg);
    icons::paint(p, pos2(rect.right() - 13.0, rect.center().y + 0.5), Icon::ChevronDown, 13.0, fg);
    let resp = resp.on_hover_text(name).on_hover_cursor(CursorIcon::PointingHand);
    let mut picked = None;
    egui::Popup::menu(&resp).gap(4.0).show(|ui| {
        ui.set_min_width(rect.width().max(170.0));
        for (v, l) in options {
            if ui.selectable_label(*v == value, *l).clicked() {
                picked = Some(*v);
            }
        }
    });
    if picked.is_some() {
        egui::Popup::close_id(ui.ctx(), popup_id);
    }
    picked.filter(|v| *v != value)
}

/// Petit compteur arrondi à côté d'un titre (« 4 Focus », « 12 »).
pub fn count_pill(ui: &mut Ui, text: &str) -> Response {
    let t = tok(ui);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::new(14.0, theme::semibold()), t.ink);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 22.0, 28.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(14), t.neutral_bg);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, t.ink);
    resp
}

/// Un « tag » discret (stack technique).
pub fn tag(ui: &mut Ui, text: &str) -> Response {
    let t = tok(ui);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::new(11.5, FontFamily::Monospace), t.muted);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 12.0, 20.0), Sense::hover());
    ui.painter().rect(rect, CornerRadius::same(5), Color32::TRANSPARENT, Stroke::new(1.0, t.line_strong), StrokeKind::Inside);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, t.muted);
    resp
}

// ---- boutons -----------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Plein, couleur d'accent.
    Primary,
    /// Contour.
    Secondary,
    /// Texte seul, fond au survol.
    Ghost,
    Danger,
}

pub fn button(ui: &mut Ui, icon: Option<Icon>, text: &str, kind: Kind) -> Response {
    button_sized(ui, icon, text, kind, theme::CONTROL_H, None)
}

/// Bouton complet : icône optionnelle, hauteur, largeur imposée (sinon ajustée au texte).
pub fn button_sized(ui: &mut Ui, icon: Option<Icon>, text: &str, kind: Kind, height: f32, width: Option<f32>) -> Response {
    let t = tok(ui);
    let font = FontId::new(if height >= 44.0 { 16.0 } else { 14.0 }, theme::semibold());
    let text_w = text_size(ui, text, &font).x;
    let icon_size = if height >= 44.0 { 20.0 } else { 17.0 };
    let icon_w = if icon.is_some() { icon_size + if text.is_empty() { 0.0 } else { 8.0 } } else { 0.0 };
    let natural = text_w + icon_w + if text.is_empty() { 0.0 } else { 30.0 };
    let w = width.unwrap_or(natural.max(height));
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click());
    let enabled = ui.is_enabled();
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, text));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
    let (mut fill, mut fg, stroke) = match kind {
        Kind::Primary => (mix(t.accent, t.accent_hover, hover), t.on_accent, Stroke::NONE),
        Kind::Secondary => (mix(Color32::TRANSPARENT, t.raised, hover), t.ink_strong, Stroke::new(1.0, mix(t.line_strong, t.faint, hover))),
        Kind::Ghost => (tint(t.ink, 0.07 * hover), t.ink, Stroke::NONE),
        Kind::Danger => (tint(t.danger, 0.08 + 0.10 * hover), t.danger, Stroke::new(1.0, tint(t.danger, 0.45))),
    };
    if !enabled {
        // Désactivé : neutre et lisible, plutôt qu'un accent délavé.
        fill = t.raised;
        fg = t.faint;
    }
    let rect = if resp.is_pointer_button_down_on() { rect.translate(vec2(0.0, 1.0)) } else { rect };
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(R_CTRL), fill, stroke, StrokeKind::Inside);
    let galley = p.layout_no_wrap(text.to_owned(), font, fg);
    let content_w = icon_w + galley.size().x;
    let mut x = rect.center().x - content_w / 2.0;
    if let Some(icon) = icon {
        icons::paint(p, pos2(x + icon_size / 2.0, rect.center().y), icon, icon_size, fg);
        x += icon_w;
    }
    p.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, fg);
    if resp.has_focus() {
        p.rect_stroke(rect.expand(2.0), CornerRadius::same(R_CTRL + 2), Stroke::new(1.5, t.accent), StrokeKind::Outside);
    }
    if enabled { resp.on_hover_cursor(CursorIcon::PointingHand) } else { resp }
}

/// Bouton carré portant une icône, avec une bulle d'aide.
pub fn icon_button(ui: &mut Ui, icon: Icon, tip: &str, size: f32) -> Response {
    icon_button_tip(ui, icon, tip, tip, size)
}

/// Comme `icon_button`, avec une bulle différente du nom accessible (un état : « Atelier à
/// jour »…). Une seule bulle au survol.
pub fn icon_button_tip(ui: &mut Ui, icon: Icon, name: &str, tip: &str, size: f32) -> Response {
    let t = tok(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, name));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
    ui.painter().rect_filled(rect, CornerRadius::same(R_CTRL), tint(t.ink, 0.08 * hover));
    icons::paint(ui.painter(), rect.center(), icon, size * 0.56, mix(t.muted, t.ink_strong, hover));
    resp.on_hover_text(tip).on_hover_cursor(CursorIcon::PointingHand)
}

/// Le choix d'une icône, en un seul bouton qui montre l'icône choisie. Au clic, un petit menu :
/// une recherche (« maison », « house »…) et la grille des icônes. Renvoie `true` au changement.
pub fn icon_picker(ui: &mut Ui, name: &str, value: &mut Icon) -> bool {
    let t = tok(ui);
    let size = theme::CONTROL_H;
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::ComboBox, true, name));
    let popup_id = egui::Popup::default_response_id(&resp);
    let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered() || open);
    let stroke = if open { Stroke::new(1.0, t.accent) } else { Stroke::new(1.0, mix(t.line, t.line_strong, hover)) };
    ui.painter().rect(rect, CornerRadius::same(R_CTRL), t.input, stroke, StrokeKind::Inside);
    icons::paint(ui.painter(), rect.center(), *value, 19.0, t.accent_ink);
    let resp = resp.on_hover_text("Changer l'icône").on_hover_cursor(CursorIcon::PointingHand);

    let query_id = resp.id.with("icon-query");
    if !open {
        ui.data_mut(|d| d.remove::<String>(query_id));
    }
    let opening = resp.clicked() && !open;
    let mut picked = None;
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).width(296.0).gap(6.0).show(|ui| {
        ui.set_width(296.0);
        let mut query: String = ui.data(|d| d.get_temp(query_id)).unwrap_or_default();
        let search = icon_input(ui, &mut query, Icon::Search, "Rechercher une icône…", ui.available_width(), size);
        if opening {
            search.request_focus();
        }
        let found = icons::search(&query);
        // Entrée : la première icône trouvée.
        if search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            picked = found.first().copied();
        }
        ui.add_space(4.0);
        if found.is_empty() {
            ui.label(egui::RichText::new(format!("Aucune icône pour « {} ».", query.trim())).size(13.0).color(t.muted));
        }
        egui::ScrollArea::vertical().max_height(236.0).auto_shrink([false, true]).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                // Les premières seulement : au-delà, mieux vaut préciser la recherche.
                for &icon in found.iter().take(240) {
                    let selected = icons::same(icon, *value);
                    let (cell, r) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
                    let label = icons::name(icon);
                    r.widget_info(|| WidgetInfo::selected(Role::Button, true, selected, &label));
                    let fill = if selected { t.accent_soft } else if r.hovered() { t.raised } else { Color32::TRANSPARENT };
                    let stroke = if selected { Stroke::new(1.0, t.accent) } else { Stroke::NONE };
                    ui.painter().rect(cell, CornerRadius::same(7), fill, stroke, StrokeKind::Inside);
                    icons::paint(ui.painter(), cell.center(), icon, 18.0, if selected { t.accent_ink } else { mix(t.muted, t.ink_strong, r.hovered() as u8 as f32) });
                    if r.on_hover_text(&label).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        picked = Some(icon);
                    }
                }
            });
        });
        ui.data_mut(|d| d.insert_temp(query_id, query));
    });
    match picked {
        Some(icon) => {
            *value = icon;
            egui::Popup::close_id(ui.ctx(), popup_id);
            true
        }
        None => false,
    }
}

/// Lien texte en couleur d'accent, avec une icône optionnelle après le texte.
pub fn link(ui: &mut Ui, text: &str, icon: Option<Icon>, size: f32) -> Response {
    let t = tok(ui);
    let font = FontId::new(size, theme::semibold());
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, t.accent_ink);
    let extra = if icon.is_some() { size + 8.0 } else { 0.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + extra, galley.size().y.max(size + 4.0)), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::Link, true, text));
    let color = if resp.hovered() { t.accent_hover } else { t.accent_ink };
    let y = rect.center().y - galley.size().y / 2.0;
    ui.painter().galley(pos2(rect.left(), y), galley.clone(), color);
    if resp.hovered() {
        ui.painter().hline(rect.left()..=rect.left() + galley.size().x, y + galley.size().y, Stroke::new(1.0, color));
    }
    if let Some(icon) = icon {
        icons::paint(ui.painter(), pos2(rect.right() - size / 2.0, rect.center().y), icon, size, color);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Bouton à bordure pointillée (« + Ajouter un lanceur »).
pub fn dashed_button(ui: &mut Ui, icon: Icon, text: &str, size: Vec2) -> Response {
    let t = tok(ui);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, text));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
    let color = mix(t.faint, t.accent_ink, hover);
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(R_CTRL), tint(t.accent, 0.06 * hover));
    let r = rect.shrink(0.5);
    let points = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    p.extend(Shape::dashed_line(&points, Stroke::new(1.0, mix(t.line_strong, t.accent, hover)), 5.0, 4.0));
    let font = FontId::new(14.0, theme::semibold());
    let galley = p.layout_no_wrap(text.to_owned(), font, color);
    let w = 18.0 + 8.0 + galley.size().x;
    let x = rect.center().x - w / 2.0;
    icons::paint(p, pos2(x + 9.0, rect.center().y), icon, 17.0, color);
    p.galley(pos2(x + 26.0, rect.center().y - galley.size().y / 2.0), galley, color);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

// ---- navigation --------------------------------------------------------------------------------

/// Entrée de la barre de navigation : icône + label, fond doux d'accent quand elle est active.
pub fn nav_item(ui: &mut Ui, icon: Option<Icon>, label: &str, selected: bool) -> Response {
    let t = tok(ui);
    let font = FontId::new(15.5, theme::semibold());
    let text_w = text_size(ui, label, &font).x;
    let icon_w = if icon.is_some() { 28.0 } else { 0.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(text_w + icon_w + 30.0, 42.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(Role::Tab, true, selected, label));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered() && !selected);
    let (fill, fg) = if selected { (t.accent_soft, t.accent_ink) } else { (tint(t.ink, 0.06 * hover), mix(t.ink, t.ink_strong, hover)) };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(R_CTRL), fill);
    let mut x = rect.left() + 15.0;
    if let Some(icon) = icon {
        icons::paint(p, pos2(x + 10.0, rect.center().y), icon, 19.0, fg);
        x += icon_w;
    }
    let galley = p.layout_no_wrap(label.to_owned(), font, fg);
    p.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, fg);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Onglets soulignés, de largeur égale sur toute la place disponible. Renvoie `true` au changement.
pub fn tabs<T: Copy + PartialEq>(ui: &mut Ui, id: &str, value: &mut T, options: &[(T, &str)]) -> bool {
    let t = tok(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::hover());
    let p = ui.painter().clone();
    p.hline(rect.x_range(), rect.bottom() - 0.5, t.hairline());
    let w = rect.width() / options.len() as f32;
    let mut changed = false;
    for (i, (option, label)) in options.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(rect.left() + w * i as f32, rect.top()), vec2(w, rect.height()));
        let resp = ui.interact(cell, Id::unique((id, i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let selected = *value == *option;
        resp.widget_info(|| WidgetInfo::selected(Role::Tab, true, selected, *label));
        if resp.clicked() && !selected {
            *value = *option;
            changed = true;
        }
        let k = ui.ctx().animate_bool(resp.id.with("sel"), selected);
        let fg = if selected { t.accent_ink } else if resp.hovered() { t.ink_strong } else { t.muted };
        let galley = p.layout_no_wrap(label.to_string(), FontId::new(15.0, theme::semibold()), fg);
        p.galley(cell.center() - galley.size() / 2.0 - vec2(0.0, 1.0), galley, fg);
        if k > 0.0 {
            let half = cell.width() * 0.5 * k;
            p.rect_filled(Rect::from_min_max(pos2(cell.center().x - half, cell.bottom() - 2.5), pos2(cell.center().x + half, cell.bottom())), CornerRadius::same(1), t.accent);
        }
    }
    changed
}

/// Contrôle segmenté : options avec icône facultative, l'option choisie sur fond doux d'accent.
pub fn segmented<T: Copy + PartialEq>(ui: &mut Ui, id: &str, value: &mut T, options: &[(T, Option<Icon>, &str)], width: Option<f32>) -> bool {
    let t = tok(ui);
    let font = FontId::new(13.5, theme::semibold());
    let naturals: Vec<f32> = options.iter().map(|(_, icon, l)| text_size(ui, l, &font).x + if icon.is_some() { 26.0 } else { 0.0 } + 26.0).collect();
    let total: f32 = width.unwrap_or_else(|| naturals.iter().sum());
    let scale = total / naturals.iter().sum::<f32>();
    let (rect, _) = ui.allocate_exact_size(vec2(total, theme::CONTROL_H), Sense::hover());
    let p = ui.painter().clone();
    p.rect(rect, CornerRadius::same(R_CTRL), t.input, Stroke::new(1.0, t.line_strong), StrokeKind::Inside);
    let mut changed = false;
    let mut x = rect.left();
    for (i, (option, icon, label)) in options.iter().enumerate() {
        let w = naturals[i] * scale;
        let cell = Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height()));
        x += w;
        let resp = ui.interact(cell, Id::unique((id, i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let selected = *value == *option;
        resp.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, selected, *label));
        if resp.clicked() && !selected {
            *value = *option;
            changed = true;
        }
        let fg = if selected { t.accent_ink } else if resp.hovered() { t.ink_strong } else { t.muted };
        if selected {
            p.rect(cell.shrink(2.0), CornerRadius::same(R_CTRL - 2), t.accent_soft, Stroke::new(1.0, t.accent), StrokeKind::Inside);
        }
        let galley = p.layout_no_wrap(label.to_string(), font.clone(), fg);
        let content = galley.size().x + if icon.is_some() { 24.0 } else { 0.0 };
        let mut cx = cell.center().x - content / 2.0;
        if let Some(icon) = icon {
            icons::paint(&p, pos2(cx + 8.0, cell.center().y), *icon, 16.0, fg);
            cx += 24.0;
        }
        p.galley(pos2(cx, cell.center().y - galley.size().y / 2.0), galley, fg);
    }
    changed
}

// ---- interrupteurs, cases, champs --------------------------------------------------------------

/// Interrupteur arrondi.
pub fn switch(ui: &mut Ui, on: &mut bool, a11y_label: &str) -> Response {
    let t = tok(ui);
    let (rect, mut resp) = ui.allocate_exact_size(vec2(40.0, 22.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let state = *on;
    resp.widget_info(|| WidgetInfo::selected(Role::Switch, true, state, a11y_label));
    let k = ui.ctx().animate_bool(resp.id.with("on"), *on);
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(11), mix(t.input, t.accent, k), Stroke::new(1.0, mix(t.line_strong, t.accent, k)), StrokeKind::Inside);
    let x = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), k);
    p.circle_filled(pos2(x, rect.center().y), 7.5, mix(t.muted, t.on_accent, k));
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Ligne de réglage : titre et description à gauche, interrupteur à droite.
pub fn toggle_row(ui: &mut Ui, on: &mut bool, title: &str, description: Option<&str>) -> Response {
    let t = tok(ui);
    ui.horizontal(|ui| {
        let width = ui.available_width();
        ui.vertical(|ui| {
            ui.set_width(width - 60.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(egui::RichText::new(title).font(FontId::new(14.0, theme::semibold())).color(t.ink_strong));
            if let Some(d) = description {
                ui.label(egui::RichText::new(d).size(12.5).color(t.muted));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| switch(ui, on, title)).inner
    })
    .inner
}

/// Case à cocher arrondie ; le texte est barré quand elle est cochée si `strike`.
pub fn checkbox(ui: &mut Ui, checked: &mut bool, text: &str, strike: bool) -> Response {
    let t = tok(ui);
    let font = FontId::new(14.5, FontFamily::Proportional);
    let galley = ui.painter().layout(text.to_owned(), font, Color32::PLACEHOLDER, ui.available_width() - 32.0);
    let size = vec2(ui.available_width(), galley.size().y.max(22.0));
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    let state = *checked;
    resp.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, state, text));
    let k = ui.ctx().animate_bool(resp.id.with("on"), *checked);
    let boxr = Rect::from_min_size(pos2(rect.left(), rect.top() + (rect.height() - 20.0) / 2.0), vec2(20.0, 20.0));
    let p = ui.painter();
    let stroke = if resp.hovered() { t.accent } else { mix(t.faint, t.accent, k) };
    p.rect(boxr, CornerRadius::same(5), mix(Color32::TRANSPARENT, t.accent, k), Stroke::new(1.5, stroke), StrokeKind::Inside);
    if k > 0.3 {
        icons::paint(p, boxr.center(), Icon::Check, 15.0, tint(t.on_accent, k));
    }
    let color = if *checked && strike { t.faint } else { t.ink };
    let pos = pos2(boxr.right() + 12.0, rect.center().y - galley.size().y / 2.0);
    if *checked && strike {
        let w = galley.size().x;
        p.galley(pos, galley, color);
        p.hline(pos.x..=pos.x + w, rect.center().y + 1.0, Stroke::new(1.0, t.faint));
    } else {
        p.galley(pos, galley, color);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Champ texte d'une ligne.
pub fn text_input(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> Response {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(width)
            .min_size(vec2(width, theme::CONTROL_H))
            .align(Align2::LEFT_CENTER)
            .margin(Margin::symmetric(12, 8)),
    )
}

/// Champ texte en police mono (commandes, chemins).
pub fn mono_input(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> Response {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .font(egui::TextStyle::Monospace)
            .desired_width(width)
            .min_size(vec2(width, theme::CONTROL_H))
            .align(Align2::LEFT_CENTER)
            .margin(Margin::symmetric(12, 8)),
    )
}

/// Champ avec une icône peinte à gauche (recherche…).
pub fn icon_input(ui: &mut Ui, value: &mut String, icon: Icon, hint: &str, width: f32, height: f32) -> Response {
    let t = tok(ui);
    let resp = ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(width)
            .min_size(vec2(width, height))
            .align(Align2::LEFT_CENTER)
            .font(FontId::proportional(if height > 40.0 { 16.0 } else { 14.0 }))
            .margin(Margin { left: 42, right: 12, top: 8, bottom: 8 }),
    );
    let color = if resp.has_focus() { t.accent_ink } else { t.muted };
    icons::paint(ui.painter(), pos2(resp.rect.left() + 21.0, resp.rect.center().y), icon, 19.0, color);
    resp
}

/// Chemin de dossier : champ mono + bouton « parcourir » (simulé).
pub fn path_input(ui: &mut Ui, value: &mut String, width: f32) -> Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let mut resp = mono_input(ui, value, r"C:\chemin\du\dossier", width - 42.0);
        // « Parcourir… » ouvre le sélecteur de dossier de Windows, à partir du chemin saisi.
        if icon_button(ui, Icon::FolderOpen, "Parcourir…", theme::CONTROL_H).clicked() {
            let mut dialog = rfd::FileDialog::new().set_title("Choisir un dossier");
            let current = std::path::Path::new(value.trim());
            if let Some(dir) = [Some(current), current.parent()].into_iter().flatten().find(|d| d.is_dir()) {
                dialog = dialog.set_directory(dir);
            }
            if let Some(dir) = dialog.pick_folder() {
                *value = dir.display().to_string();
                resp.mark_changed();
            }
        }
        resp
    })
    .inner
}

/// Donne un nom accessible à un widget standard qui n'en a pas (slider, compteur…).
pub fn named(resp: Response, role: Role, name: &str) -> Response {
    resp.widget_info(|| WidgetInfo::labeled(role, true, name));
    resp
}

/// Liste déroulante. `name` sert d'identifiant et de nom accessible (lu par les lecteurs d'écran).
pub fn combo<T: Copy + PartialEq>(ui: &mut Ui, name: &str, value: &mut T, options: &[(T, &str)], width: f32) -> bool {
    let current = options.iter().find(|(v, _)| *v == *value).map(|(_, l)| *l).unwrap_or("");
    let mut changed = false;
    let inner = egui::ComboBox::from_id_salt(name).selected_text(current).width(width).show_ui(ui, |ui| {
        for (v, label) in options {
            if ui.selectable_value(value, *v, *label).changed() {
                changed = true;
            }
        }
    });
    inner.response.widget_info(|| WidgetInfo::labeled(Role::ComboBox, true, name));
    changed
}

/// Label au-dessus d'un contrôle.
pub fn labeled<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        field_label(ui, label);
        add(ui)
    })
    .inner
}

// ---- conteneurs et petits éléments -------------------------------------------------------------

/// Le cadre d'une carte.
pub fn card(t: &Tokens) -> Frame {
    Frame::new().fill(t.surface).stroke(Stroke::new(1.0, t.line)).corner_radius(R_CARD).inner_margin(Margin::same(18))
}

/// Une carte de réglages : titre, description, contenu.
pub fn settings_card(ui: &mut Ui, title_text: &str, description: Option<&str>, add: impl FnOnce(&mut Ui)) {
    let t = tok(ui);
    card(&t).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 10.0;
        ui.label(egui::RichText::new(title_text).font(FontId::new(17.0, theme::bold())).color(t.ink_strong));
        if let Some(d) = description {
            ui.add_space(-6.0);
            ui.label(egui::RichText::new(d).size(13.0).color(t.muted));
        }
        ui.add_space(2.0);
        add(ui);
    });
}

/// Titre de section avec compteur facultatif.
pub fn section_header(ui: &mut Ui, text: &str, count: Option<&str>) {
    ui.horizontal(|ui| {
        title(ui, text, 25.0);
        if let Some(c) = count {
            ui.add_space(4.0);
            count_pill(ui, c);
        }
    });
}

/// Carré arrondi teinté portant une icône.
pub fn icon_tile(ui: &mut Ui, icon: Icon, color: Color32, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same((size * 0.24) as u8), tint(color, 0.16));
    icons::paint(ui.painter(), rect.center(), icon, size * 0.56, color);
    resp
}

/// Ligne « icône + titre + contenu » du résumé d'un projet.
pub fn info_row(ui: &mut Ui, icon: Icon, heading: &str, add: impl FnOnce(&mut Ui)) {
    let t = tok(ui);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        ui.add_space(2.0);
        icons::show(ui, icon, 22.0, t.muted);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.label(egui::RichText::new(heading).font(FontId::new(16.5, theme::bold())).color(t.ink_strong));
            add(ui);
        });
    });
}

/// Barre de progression arrondie.
pub fn progress(ui: &mut Ui, fraction: f32, color: Color32, width: f32, height: f32) -> Response {
    let t = tok(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let r = CornerRadius::same((height / 2.0) as u8);
    ui.painter().rect_filled(rect, r, t.neutral_bg);
    let done = Rect::from_min_size(rect.min, vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()));
    if done.width() > 0.5 {
        ui.painter().rect_filled(done, r, color);
    }
    resp
}

/// Pastille de couleur sélectionnable.
pub fn swatch(ui: &mut Ui, color: Color32, selected: bool, label: &str) -> Response {
    let t = tok(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, selected, label));
    let p = ui.painter();
    if selected {
        p.circle_stroke(rect.center(), 14.0, Stroke::new(2.0, t.ink_strong));
    } else if resp.hovered() {
        p.circle_stroke(rect.center(), 14.0, Stroke::new(1.0, t.line_strong));
    }
    p.circle_filled(rect.center(), 10.5, color);
    resp.on_hover_text(label).on_hover_cursor(CursorIcon::PointingHand)
}

/// Filet horizontal sur toute la largeur.
pub fn separator(ui: &mut Ui) {
    let t = tok(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.center().y, t.hairline());
}

/// Filet vertical.
pub fn vsep(ui: &mut Ui, height: f32) {
    let t = tok(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, height), Sense::hover());
    ui.painter().vline(rect.center().x, rect.y_range(), t.hairline());
}

// ---- lanceurs ----------------------------------------------------------------------------------

pub const TILE: Vec2 = vec2(70.0, 68.0);

/// Un lanceur en tuile compacte : icône au-dessus, petit libellé dessous. Le favori est plein,
/// couleur d'accent. `flash` (0 → 1) fait briller la tuile juste après un lancement.
pub fn launcher_tile(ui: &mut Ui, l: &Launcher, favorite: bool, flash: f32) -> Response {
    let t = tok(ui);
    let (rect, resp) = ui.allocate_exact_size(TILE, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &l.name));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
    let rect = if resp.is_pointer_button_down_on() { rect.translate(vec2(0.0, 1.0)) } else { rect };
    let p = ui.painter();
    let (fill, stroke, fg) = if favorite {
        (mix(t.accent, t.accent_hover, hover), Stroke::NONE, t.on_accent)
    } else {
        (tint(t.ink, 0.06 * hover), Stroke::new(1.0, mix(t.line, t.line_strong, hover)), mix(t.muted, t.ink_strong, hover))
    };
    p.rect(rect, CornerRadius::same(R_CTRL + 2), fill, stroke, StrokeKind::Inside);
    let tile = Rect::from_center_size(pos2(rect.center().x, rect.top() + 24.0), vec2(30.0, 30.0));
    let (tile_bg, icon_color) = if favorite { (tint(t.on_accent, 0.14), t.on_accent) } else { (tint(l.color, 0.18), l.color) };
    p.rect_filled(tile, CornerRadius::same(7), tile_bg);
    icons::paint(p, tile.center(), l.icon, 16.0, icon_color);
    let font = FontId::new(11.0, theme::medium());
    let label = elide_end(p, &l.name, &font, rect.width() - 8.0);
    p.text(pos2(rect.center().x, rect.bottom() - 13.0), Align2::CENTER_CENTER, label, font, fg);
    if flash > 0.0 {
        p.rect_stroke(rect.expand(2.0 + 4.0 * (1.0 - flash)), CornerRadius::same(R_CTRL + 4), Stroke::new(2.0, tint(t.accent, flash)), StrokeKind::Outside);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// La tuile « + » au bout de la rangée de lanceurs.
pub fn add_tile(ui: &mut Ui, label: &str) -> Response {
    let t = tok(ui);
    let (rect, resp) = ui.allocate_exact_size(TILE, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
    let color = mix(t.faint, t.accent_ink, hover);
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(R_CTRL + 2), tint(t.accent, 0.06 * hover));
    let r = rect.shrink(0.5);
    let points = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    p.extend(Shape::dashed_line(&points, Stroke::new(1.0, mix(t.line_strong, t.accent, hover)), 4.0, 3.0));
    icons::paint(p, pos2(rect.center().x, rect.top() + 24.0), Icon::Plus, 20.0, color);
    p.text(pos2(rect.center().x, rect.bottom() - 13.0), Align2::CENTER_CENTER, "Ajouter", FontId::new(11.0, theme::medium()), color);
    resp.on_hover_text(label).on_hover_cursor(CursorIcon::PointingHand)
}

// ---- textes trop longs -------------------------------------------------------------------------

fn width_of(p: &egui::Painter, text: &str, font: &FontId) -> f32 {
    p.layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER).size().x
}

/// Coupe la fin d'un texte par « … » pour qu'il tienne dans `max_w`.
pub fn elide_end(p: &egui::Painter, text: &str, font: &FontId, max_w: f32) -> String {
    if width_of(p, text, font) <= max_w {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let s: String = chars[..mid].iter().chain(['…'].iter()).collect();
        if width_of(p, &s, font) <= max_w { lo = mid } else { hi = mid - 1 }
    }
    chars[..lo].iter().chain(['…'].iter()).collect()
}

/// Coupe le milieu d'un texte (idéal pour un chemin : on garde le début et surtout la fin).
pub fn elide_middle(p: &egui::Painter, text: &str, font: &FontId, max_w: f32) -> String {
    if width_of(p, text, font) <= max_w {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let build = |k: usize| {
        let head = k * 2 / 5;
        let tail = k - head;
        let mut s: String = chars[..head].iter().collect();
        s.push('…');
        s.extend(chars[chars.len() - tail..].iter());
        s
    };
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if width_of(p, &build(mid), font) <= max_w { lo = mid } else { hi = mid - 1 }
    }
    build(lo)
}

/// Une zone défilante qu'on peut aussi faire glisser à la souris : maintenir le clic et tirer.
/// (egui 0.37 ne l'active par défaut que sur écran tactile.) Le curseur devient une main fermée.
pub fn draggable(area: egui::ScrollArea) -> egui::ScrollArea {
    use egui::containers::scroll_area::{DragScroll, ScrollSource};
    area.scroll_source(ScrollSource { drag: DragScroll::Always, ..Default::default() }).on_drag_cursor(CursorIcon::Grabbing)
}

/// Contenu d'une zone défilante, avec une marge à droite pour que la barre de défilement ne
/// colle pas aux boutons.
pub fn scroll_content<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new().inner_margin(Margin { left: 0, right: 16, top: 0, bottom: 0 }).show(ui, |ui| ui.vertical(add).inner).inner
}

// ---- fenêtre sans bordure ----------------------------------------------------------------------

/// Glisser la zone déplace la fenêtre, double-cliquer la maximise. À appeler avant les boutons.
pub fn title_bar_drag(ui: &mut Ui, rect: Rect) {
    let resp = ui.interact(rect, Id::unique("title-bar-drag"), Sense::click_and_drag());
    if resp.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if resp.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

/// Réduire / agrandir / fermer. `ui` doit être en disposition droite → gauche.
pub fn window_buttons(ui: &mut Ui, height: f32) {
    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let t = tok(ui);
    let button = |ui: &mut Ui, icon: Icon, label: &str, danger: bool| {
        let (rect, resp) = ui.allocate_exact_size(vec2(46.0, height), Sense::click());
        resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
        let (fill, fg) = match (resp.hovered(), danger) {
            (true, true) => (theme::rgb(0xe81123), Color32::WHITE),
            (true, false) => (tint(t.ink, 0.08), t.ink_strong),
            _ => (Color32::TRANSPARENT, t.muted),
        };
        ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
        icons::paint(ui.painter(), rect.center(), icon, 16.0, fg);
        resp.clicked()
    };
    if button(ui, Icon::X, "Fermer la fenêtre", true) {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
    if button(ui, if maximized { Icon::Copy } else { Icon::Square }, if maximized { "Restaurer" } else { "Agrandir" }, false) {
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    if button(ui, Icon::Minus, "Réduire", false) {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

/// Le bord de fenêtre sous le pointeur (fenêtre sans bordure → redimensionnement maison).
pub fn edge_under_pointer(ctx: &egui::Context) -> Option<(ResizeDirection, CursorIcon)> {
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return None;
    }
    let pos = ctx.pointer_hover_pos()?;
    let r = ctx.viewport_rect();
    const BAND: f32 = 5.0;
    let (n, s) = (pos.y - r.top() < BAND, r.bottom() - pos.y < BAND);
    let (w, e) = (pos.x - r.left() < BAND, r.right() - pos.x < BAND);
    use ResizeDirection as D;
    Some(match (n, s, w, e) {
        (true, _, true, _) => (D::NorthWest, CursorIcon::ResizeNorthWest),
        (true, _, _, true) => (D::NorthEast, CursorIcon::ResizeNorthEast),
        (_, true, true, _) => (D::SouthWest, CursorIcon::ResizeSouthWest),
        (_, true, _, true) => (D::SouthEast, CursorIcon::ResizeSouthEast),
        (true, ..) => (D::North, CursorIcon::ResizeNorth),
        (_, true, ..) => (D::South, CursorIcon::ResizeSouth),
        (_, _, true, _) => (D::West, CursorIcon::ResizeWest),
        (_, _, _, true) => (D::East, CursorIcon::ResizeEast),
        _ => return None,
    })
}

/// À appeler en fin d'image : curseur de redimensionnement et passage de relais au système.
pub fn resize_from_edge(ctx: &egui::Context, edge: Option<(ResizeDirection, CursorIcon)>) {
    let Some((direction, cursor)) = edge else { return };
    ctx.set_cursor_icon(cursor);
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}
