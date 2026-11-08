//! Le panneau « En ce moment » (à droite du bureau, quand aucun projet n'est ouvert) : les
//! projets qui ont bougé, en direct. Pour chacun : son état et son attention (modifiables), sa
//! branche Git, sa prochaine étape à cocher, son README et ses derniers fichiers modifiés, qui
//! s'ouvrent d'un clic dans l'application par défaut du système. Le « fil combiné » mêle les
//! fichiers de tous les projets, du plus récent au plus ancien.
//!
//! Les cartes sont repliées au départ ; ouvrir l'une referme l'autre. La flèche dans les
//! détails ouvre la fiche du projet ; la fermer ramène ici.

use egui::{Align, Align2, CornerRadius, FontFamily, FontId, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui, pos2, vec2};

use crate::app::{Action, UiState};
use crate::i18n::{tf, tr};
use crate::icons::{self, Icon};
use crate::model::{Attention, Data, Status, ago};
use crate::recent::{FileHit, ProjectActivity, Window};
use crate::theme::{self, Tokens, mix, tint};
use crate::widgets::{self, Kind};

/// Fichiers montrés par projet avant « Voir les N fichiers ».
const PREVIEW: usize = 3;
/// Lignes au plus dans le fil combiné.
const FEED_MAX: usize = 80;

pub fn show(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let frame = Frame::new().fill(t.surface).stroke(Stroke::new(1.0, t.line)).corner_radius(14).inner_margin(Margin { left: 20, right: 6, top: 20, bottom: 16 });
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(ui.available_height());
        // L'en-tête reste en place ; seule la liste défile.
        ui.scope(|ui| {
            ui.set_width(ui.available_width() - 14.0);
            header(ui, st);
            ui.add_space(12.0);
            let w = ui.available_width();
            let windows = [(Window::Day, None, tr("24 h")), (Window::Week, None, tr("7 jours")), (Window::Month, None, tr("30 jours"))];
            if widgets::segmented(ui, "live-window", &mut st.live_window, &windows, Some(w)) {
                st.live.refresh();
            }
            ui.add_space(6.0);
            let views = [(false, Some(Icon::LayoutList), tr("Par projet")), (true, Some(Icon::History), tr("Fil combiné"))];
            widgets::segmented(ui, "live-view", &mut st.live_feed, &views, Some(w));
        });
        ui.add_space(12.0);
        widgets::draggable(ScrollArea::vertical()).id_salt("live-scroll").auto_shrink(false).show(ui, |ui| {
            widgets::scroll_content(ui, |ui| {
                let projects: Vec<ProjectActivity> = st.live.snapshot.as_ref().map(|s| s.projects.clone()).unwrap_or_default();
                // Les projets encore suivis (une fiche supprimée entre deux relevés disparaît).
                let rows: Vec<(usize, &ProjectActivity)> = projects.iter().filter_map(|pa| data.index_of(pa.id).map(|i| (i, pa))).collect();
                if st.live_expanded.is_some_and(|id| !rows.iter().any(|(_, pa)| pa.id == id)) {
                    st.live_expanded = None;
                }
                if st.live.snapshot.is_none() {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(16.0).color(t.accent));
                        widgets::muted(ui, tr("Lecture de tes dossiers…"));
                    });
                } else if rows.is_empty() {
                    empty(ui, st);
                } else if st.live_feed {
                    feed(ui, data, &rows, act);
                } else {
                    ui.spacing_mut().item_spacing.y = 10.0;
                    for &(i, pa) in &rows {
                        ui.push_id(pa.id, |ui| project_card(ui, data, st, i, pa, act));
                    }
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if widgets::link(ui, tr("Actualiser"), Some(Icon::RefreshCw), 12.5).on_hover_text(tr("Relire les dossiers maintenant (fait tout seul toutes les 30 s)")).clicked() {
                        st.live.refresh();
                    }
                });
                ui.add_space(2.0);
                ui.add(egui::Label::new(RichText::new(tr("Clic sur un fichier : l'ouvrir avec l'application par défaut. Clic droit : plus d'options.")).size(11.5).color(t.faint)).wrap());
            });
        });
    });
}

fn header(ui: &mut Ui, st: &mut UiState) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let text_w = (ui.available_width() - 44.0).max(80.0);
        ui.vertical(|ui| {
            ui.set_width(text_w);
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.horizontal(|ui| {
                // Le témoin « en direct » : vert, il palpite pendant un relevé.
                let (r, _) = ui.allocate_exact_size(vec2(14.0, 26.0), Sense::hover());
                let busy = st.live.busy();
                let pulse = if busy {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
                    ((st.now * 4.0).sin() * 0.5 + 0.5) as f32
                } else {
                    0.0
                };
                ui.painter().circle_filled(r.center(), 7.0 + 2.0 * pulse, tint(t.good_fg, 0.18 + 0.12 * pulse));
                ui.painter().circle_filled(r.center(), 4.0, t.good_fg);
                ui.label(RichText::new(tr("En ce moment")).font(FontId::new(23.0, theme::bold())).color(t.ink_strong));
            });
            let line = match &st.live.snapshot {
                None => tr("Ce qui bouge dans tes projets, en direct.").to_owned(),
                Some(s) => {
                    let n = s.projects.len();
                    let what = if n == 1 { tf("1 projet · {0} fichiers modifiés", &[("0", &s.files())]) } else { tf("{n} projets · {f} fichiers modifiés", &[("n", &n), ("f", &s.files())]) };
                    tf("{what} · relevé {when}", &[("what", &what), ("when", &ago(s.at))])
                }
            };
            ui.label(RichText::new(line).size(13.0).color(t.muted));
        });
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            if widgets::icon_button(ui, Icon::X, tr("Masquer « En ce moment »"), 32.0).clicked() {
                st.live_hidden = true;
            }
        });
    });
}

fn empty(ui: &mut Ui, st: &mut UiState) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        widgets::icon_tile(ui, Icon::Coffee, t.muted, 48.0);
        ui.add_space(6.0);
        widgets::muted(ui, tr("Rien n'a bougé dans tes projets sur cette période."));
        ui.add_space(8.0);
        let wider = match st.live_window {
            Window::Day => Some((Window::Week, tr("Voir les 7 derniers jours"))),
            Window::Week => Some((Window::Month, tr("Voir les 30 derniers jours"))),
            Window::Month => None,
        };
        if let Some((w, label)) = wider
            && widgets::button(ui, Some(Icon::CalendarRange), label, Kind::Secondary).clicked()
        {
            st.live_window = w;
            st.live.refresh();
        }
    });
}

/// La carte d'un projet qui a bougé.
fn project_card(ui: &mut Ui, data: &Data, st: &mut UiState, i: usize, pa: &ProjectActivity, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let p = &data.projects[i];
    let open = st.live_open.contains(&pa.id);
    let expanded = st.live_expanded == Some(pa.id);
    let border = if expanded { t.accent } else { t.line };
    Frame::new().fill(mix(t.surface, t.bg, 0.45)).stroke(Stroke::new(1.0, border)).corner_radius(12).inner_margin(Margin::symmetric(12, 12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 6.0;
        if project_header(ui, p, pa.last, expanded).clicked() {
            st.live_expanded = if expanded { None } else { Some(pa.id) };
            ui.ctx().request_repaint();
        }
        if st.live_expanded != Some(pa.id) {
            return;
        }
        // État et attention, modifiables ; la branche et ce qui n'est pas commité.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            let statuses: Vec<(Status, &str)> = Status::ALL.iter().map(|s| (*s, s.label())).collect();
            if let Some(s) = widgets::chip_menu(ui, &tf("Changer l'état de {0}", &[("0", &p.name)]), p.status, widgets::status_tone(p.status), &statuses) {
                act.push(Action::SetStatus(i, s));
            }
            let attentions: Vec<(Attention, &str)> = Attention::ALL.iter().map(|a| (*a, a.label())).collect();
            if let Some(a) = widgets::chip_menu(ui, &tf("Changer l'attention de {0}", &[("0", &p.name)]), p.attention, widgets::attention_tone(p.attention), &attentions) {
                act.push(Action::SetAttention(i, a, false));
            }
            if let Some(g) = &pa.git {
                ui.add_space(2.0);
                icons::show(ui, Icon::GitBranch, 14.0, t.faint);
                let mut text = g.branch.clone();
                match g.dirty {
                    Some(0) => text.push_str(&format!(" · {}", tr("tout est commité"))),
                    Some(n) => text.push_str(&format!(" · {}", tf("{0} non commités", &[("0", &n)]))),
                    None => {}
                }
                let color = if g.dirty.is_some_and(|n| n > 0) { t.accent_ink } else { t.faint };
                ui.label(RichText::new(text).size(12.0).color(color));
            }
        });
        // La prochaine étape, à cocher d'ici.
        if let Some((si, step)) = p.steps.iter().enumerate().find(|(_, s)| !s.done) {
            ui.add_space(2.0);
            ui.label(RichText::new(tr("Prochaine étape")).size(11.5).color(t.faint));
            let mut done = false;
            if widgets::checkbox(ui, &mut done, &step.text, true).changed() {
                act.push(Action::ToggleStep(i, si));
            }
        }
        ui.add_space(2.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        if let Some(readme) = &pa.readme {
            let name = std::path::Path::new(readme).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "README".into());
            file_row(ui, Icon::BookOpen, &name, readme, None, None, act);
        }
        let shown = if open { pa.files.len() } else { pa.files.len().min(PREVIEW) };
        for f in pa.files.iter().take(shown) {
            file_row(ui, file_icon(&f.rel), &f.rel, &f.path, Some(f.mtime), None, act);
        }
        if open && pa.count > pa.files.len() {
            ui.add_space(4.0);
            ui.label(RichText::new(tf("… et {0} autres, plus anciens", &[("0", &(pa.count - pa.files.len()))])).size(12.0).color(t.faint));
        }
        if pa.count > PREVIEW {
            ui.add_space(6.0);
            let (label, icon) = if open { (tr("Réduire").to_owned(), Icon::ChevronUp) } else { (tf("Voir les {0} fichiers", &[("0", &pa.count)]), Icon::ChevronDown) };
            if widgets::link(ui, &label, Some(icon), 12.5).clicked() {
                if open {
                    st.live_open.retain(|id| *id != pa.id);
                } else {
                    st.live_open.push(pa.id);
                }
            }
        }
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::ArrowRight, &tf("Ouvrir le projet {0}", &[("0", &p.name)]), 40.0).clicked() {
                    act.push(Action::Select(i));
                }
            });
        });
    });
}

/// Toute la ligne est cliquable, y compris l'icône et la date. Le nom reste accessible en
/// entier quand il doit être tronqué à l'écran.
fn project_header(ui: &mut Ui, project: &crate::model::Project, last: u64, expanded: bool) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::DisclosureTriangle, true, &project.name));
    ui.ctx().accesskit_node_builder(resp.id, |node| node.set_expanded(expanded));
    if resp.clicked() {
        resp.request_focus();
    }
    let cy = rect.center().y;
    crate::logo::paint(ui, project, pos2(rect.left() + 10.0, cy), 20.0, t.accent_ink);
    icons::paint(ui.painter(), pos2(rect.right() - 10.0, cy), if expanded { Icon::ChevronDown } else { Icon::ChevronRight }, 16.0, t.muted);
    let when = ui.painter().layout_no_wrap(ago(last), FontId::proportional(12.0), t.faint);
    let when_x = rect.right() - 30.0 - when.size().x;
    ui.painter().galley(pos2(when_x, cy - when.size().y / 2.0), when, t.faint);
    let name_x = rect.left() + 28.0;
    let font = FontId::new(16.5, theme::bold());
    let name = widgets::elide_end(ui.painter(), &project.name, &font, (when_x - name_x - 10.0).max(0.0));
    ui.painter().text(pos2(name_x, cy), Align2::LEFT_CENTER, name, font, t.ink_strong);
    resp.on_hover_text(&project.name).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Le fil combiné : les fichiers de tous les projets, par tranche de temps.
fn feed(ui: &mut Ui, data: &Data, rows: &[(usize, &ProjectActivity)], act: &mut Vec<Action>) {
    let mut all: Vec<(usize, &FileHit)> = rows.iter().flat_map(|&(i, pa)| pa.files.iter().map(move |f| (i, f))).collect();
    all.sort_by_key(|(_, f)| std::cmp::Reverse(f.mtime));
    let now = crate::model::now();
    let mut bucket = None;
    ui.spacing_mut().item_spacing.y = 0.0;
    for (i, f) in all.into_iter().take(FEED_MAX) {
        let age = now.saturating_sub(f.mtime);
        let b = match age {
            0..3600 => 0,
            3600..86_400 => 1,
            86_400..604_800 => 2,
            _ => 3,
        };
        if bucket != Some(b) {
            if bucket.is_some() {
                ui.add_space(10.0);
            }
            bucket = Some(b);
            let title = [tr("Dernière heure"), tr("Dernières 24 h"), tr("Cette semaine"), tr("Plus tôt")][b];
            widgets::caps(ui, title);
            ui.add_space(4.0);
        }
        let p = &data.projects[i];
        file_row(ui, file_icon(&f.rel), &f.rel, &f.path, Some(f.mtime), Some((i, &p.name)), act);
    }
}

/// Une icône selon le genre de fichier.
fn file_icon(rel: &str) -> Icon {
    let ext = rel.rsplit('.').next().unwrap_or_default().to_lowercase();
    match ext.as_str() {
        "md" | "txt" | "rst" | "adoc" | "org" => Icon::FileText,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" | "bmp" => Icon::FileImage,
        "json" | "toml" | "yaml" | "yml" | "lock" | "ini" | "env" | "xml" => Icon::FileCog,
        _ => Icon::FileCode,
    }
}

/// Une ligne de fichier : clic pour l'ouvrir (application par défaut), clic droit pour le
/// montrer dans l'explorateur, copier son chemin ou (fil combiné) ouvrir son projet. `project` :
/// le projet (index, nom), écrit sous le chemin.
fn file_row(ui: &mut Ui, icon: Icon, label: &str, path: &str, when: Option<u64>, project: Option<(usize, &str)>, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let h = if project.is_some() { 42.0 } else { 28.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
    {
        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::same(6), tint(t.ink, 0.07 * hover));
        let cy = if project.is_some() { rect.top() + 14.0 } else { rect.center().y };
        icons::paint(p, pos2(rect.left() + 13.0, cy), icon, 15.0, mix(t.muted, t.accent_ink, hover));
        let mut right = rect.right() - 8.0;
        if let Some(ts) = when {
            let g = p.layout_no_wrap(ago(ts), FontId::proportional(11.5), t.faint);
            right -= g.size().x;
            p.galley(pos2(right, cy - g.size().y / 2.0), g, t.faint);
            right -= 10.0;
        }
        let font = FontId::new(12.5, FontFamily::Monospace);
        let x = rect.left() + 30.0;
        let shown = widgets::elide_middle(p, label, &font, (right - x).max(30.0));
        p.text(pos2(x, cy), Align2::LEFT_CENTER, shown, font, mix(t.ink, t.ink_strong, hover));
        if let Some((_, name)) = project {
            let font = FontId::new(11.5, theme::semibold());
            let shown = widgets::elide_end(p, name, &font, rect.right() - 8.0 - x);
            p.text(pos2(x, cy + 17.0), Align2::LEFT_CENTER, shown, font, t.accent_ink);
        }
    }
    let resp = resp.on_hover_text(path).on_hover_cursor(egui::CursorIcon::PointingHand);
    if resp.clicked() {
        act.push(Action::OpenFile(path.to_owned()));
    }
    resp.context_menu(|ui| {
        ui.set_min_width(240.0);
        if ui.button(format!("{}  {}", icons::ch(Icon::ExternalLink), tr("Ouvrir avec l'application par défaut"))).clicked() {
            act.push(Action::OpenFile(path.to_owned()));
        }
        if ui.button(format!("{}  {}", icons::ch(Icon::FolderOpen), tr("Afficher dans l'explorateur"))).clicked() {
            act.push(Action::RevealFile(path.to_owned()));
        }
        if ui.button(format!("{}  {}", icons::ch(Icon::Copy), tr("Copier le chemin"))).clicked() {
            act.push(Action::CopyText(path.to_owned()));
        }
        if let Some((i, name)) = project {
            ui.separator();
            if ui.button(format!("{}  {}", icons::ch(Icon::ArrowRight), tf("Ouvrir le projet {0}", &[("0", &name)]))).clicked() {
                act.push(Action::Select(i));
            }
        }
    });
}
