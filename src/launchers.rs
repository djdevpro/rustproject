//! La galerie de lanceurs (ouverte par « + Ajouter un lanceur ») et l'éditeur de lanceur
//! personnalisé, partagé avec la configuration.

use egui::{Align, Color32, CornerRadius, FontFamily, FontId, Frame, Id, Layout, Margin, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::{Action, GalleryState, GalleryTab, LauncherDraft, footer_buttons};
use crate::icons::{self, Icon};
use crate::model::{Data, Group, LauncherKind, Project, VARIABLES};
use crate::theme::{self, Tokens, tint};
use crate::widgets::{self, Kind};
use crate::i18n::{tf, tr};

fn modal_frame(t: &Tokens) -> Frame {
    Frame::new().fill(t.surface).stroke(Stroke::new(1.0, t.line_strong)).corner_radius(16).shadow(theme::shadow(t)).inner_margin(Margin::same(26))
}

/// En-tête de fenêtre : titre, sous-titre, bouton fermer. Renvoie `true` si on ferme.
fn modal_header(ui: &mut Ui, title: &str, subtitle: &str) -> bool {
    let t = Tokens::get(ui.ctx());
    let mut close = false;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            widgets::title(ui, title, 24.0);
            ui.label(RichText::new(subtitle).size(14.0).color(t.muted));
        });
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            close = widgets::icon_button(ui, Icon::X, tr("Fermer"), 34.0).clicked();
        });
    });
    close
}

/// La galerie. Renvoie `false` quand la fenêtre doit se fermer.
pub fn gallery(ctx: &egui::Context, data: &Data, g: &mut GalleryState, act: &mut Vec<Action>) -> bool {
    let t = Tokens::get(ctx);
    let mut open = true;
    let target_name = g.project.map(|p| data.projects[p].name.clone());
    let preview_project = g.project.or(data.focus().first().copied()).map(|p| &data.projects[p]);
    // Onglet « Sur mesure » : un fichier .svg déposé devient le logo du lanceur.
    if g.tab == GalleryTab::Custom {
        take_dropped_svg(ctx, &mut g.draft);
        drop_hint(ctx, tr("Dépose le fichier .svg : il devient le logo du lanceur"));
    }
    let modal = egui::Modal::new(Id::unique("gallery")).backdrop_color(theme::scrim(&t)).frame(modal_frame(&t)).show(ctx, |ui| {
        ui.set_width(820.0);
        let subtitle = match &target_name {
            Some(n) => tf("Un bouton de plus dans la colonne « Ouvrir avec » de {n}.", &[("n", &n)]),
            None => tr("Un bouton de plus sur tous les projets.").to_owned(),
        };
        if modal_header(ui, tr("Ajouter un lanceur"), &subtitle) {
            open = false;
        }
        ui.add_space(8.0);
        widgets::tabs(ui, "gallery-tabs", &mut g.tab, &[(GalleryTab::Gallery, tr("Galerie")), (GalleryTab::Custom, tr("Créer un lanceur"))]);
        ui.add_space(14.0);
        match g.tab {
            GalleryTab::Gallery => gallery_grid(ui, data, g, act),
            GalleryTab::Custom => {
                editor_form(ui, &mut g.draft, preview_project);
            }
        }
        ui.add_space(12.0);
        widgets::separator(ui);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if g.project.is_some() {
                ui.label(RichText::new(tr("Ajouter pour")).size(14.0).color(t.muted));
                let this = target_name.clone().unwrap_or_default();
                widgets::segmented(ui, "gallery-scope", &mut g.for_all, &[(false, Some(Icon::Folder), this.as_str()), (true, Some(Icon::Layers), tr("Tous les projets"))], None);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| match g.tab {
                GalleryTab::Gallery => {
                    if widgets::button(ui, None, tr("Terminé"), Kind::Primary).clicked() {
                        open = false;
                    }
                }
                GalleryTab::Custom => {
                    let valid = g.draft.is_valid();
                    let create = ui.add_enabled_ui(valid, |ui| widgets::button(ui, Some(Icon::Plus), tr("Créer le lanceur"), Kind::Primary)).inner;
                    if create.clicked() {
                        act.push(Action::AddLauncher { launcher: g.draft.to_launcher(), project: if g.for_all { None } else { g.project } });
                        g.draft = LauncherDraft::default();
                        g.tab = GalleryTab::Gallery;
                    }
                    if !valid {
                        ui.label(RichText::new(tr("Un nom et une commande suffisent.")).size(13.0).color(t.faint));
                    }
                }
            });
        });
    });
    open && !modal.should_close()
}

fn gallery_grid(ui: &mut Ui, data: &Data, g: &mut GalleryState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        widgets::icon_input(ui, &mut g.search, Icon::Search, tr("Rechercher : terminal, éditeur, docker…"), 300.0, theme::CONTROL_H);
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing.x = 6.0;
        let chip = |ui: &mut Ui, selected: bool, label: &str| -> bool {
            let font = FontId::new(13.0, theme::semibold());
            let galley = ui.painter().layout_no_wrap(label.to_owned(), font, Color32::PLACEHOLDER);
            let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 22.0, 30.0), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, selected, label));
            let (fill, fg, stroke) = if selected {
                (t.accent_soft, t.accent_ink, Stroke::new(1.0, t.accent))
            } else if resp.hovered() {
                (t.raised, t.ink_strong, Stroke::new(1.0, t.line_strong))
            } else {
                (Color32::TRANSPARENT, t.muted, Stroke::new(1.0, t.line))
            };
            ui.painter().rect(rect, CornerRadius::same(15), fill, stroke, StrokeKind::Inside);
            ui.painter().galley(rect.center() - galley.size() / 2.0, galley, fg);
            resp.clicked()
        };
        if chip(ui, g.group.is_none(), tr("Tous")) {
            g.group = None;
        }
        for group in Group::ALL {
            if chip(ui, g.group == Some(group), group.label()) {
                g.group = Some(group);
            }
        }
    });
    ui.add_space(12.0);
    let q = g.search.trim().to_lowercase();
    let items: Vec<usize> = (0..data.gallery.len())
        .filter(|&i| {
            let l = &data.gallery[i];
            g.group.is_none_or(|gr| l.group == gr) && (q.is_empty() || l.name.to_lowercase().contains(&q) || l.description.to_lowercase().contains(&q) || l.command.to_lowercase().contains(&q))
        })
        .collect();
    // Déjà présent ? Dans les lanceurs par défaut, ou dans ceux du projet visé.
    let present: Vec<String> = match (g.project, g.for_all) {
        (Some(p), false) => data.launchers_of(p).iter().map(|&id| data.launchers[id].name.clone()).collect(),
        _ => data.default_launchers.iter().map(|&id| data.launchers[id].name.clone()).collect(),
    };
    widgets::draggable(ScrollArea::vertical()).id_salt("gallery-grid").max_height(400.0).auto_shrink([false, true]).show(ui, |ui| {
        if items.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                widgets::muted(ui, tr("Rien dans la galerie pour cette recherche."));
                if widgets::link(ui, tr("Créer un lanceur sur mesure"), Some(Icon::ArrowRight), 14.0).clicked() {
                    g.draft.name = g.search.trim().to_owned();
                    g.tab = GalleryTab::Custom;
                }
            });
            return;
        }
        let gap = 12.0;
        let cols = 3;
        let w = ((ui.available_width() - gap * (cols - 1) as f32) / cols as f32).floor();
        ui.spacing_mut().item_spacing = vec2(gap, gap);
        for row in items.chunks(cols) {
            ui.horizontal(|ui| {
                for &i in row {
                    let l = &data.gallery[i];
                    let added = present.contains(&l.name);
                    if tile(ui, l, added, w) {
                        act.push(Action::AddLauncher { launcher: l.clone(), project: if g.for_all { None } else { g.project } });
                    }
                }
            });
        }
    });
}

/// Une tuile de la galerie. Renvoie `true` si l'on clique « Ajouter ».
fn tile(ui: &mut Ui, l: &crate::model::Launcher, added: bool, width: f32) -> bool {
    let t = Tokens::get(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 128.0), Sense::hover());
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), ui.rect_contains_pointer(rect));
    let p = ui.painter().clone();
    p.rect(rect, CornerRadius::same(12), theme::mix(t.surface, t.raised, hover), Stroke::new(1.0, theme::mix(t.line, t.line_strong, hover)), StrokeKind::Inside);
    let icon_rect = Rect::from_min_size(rect.min + vec2(14.0, 14.0), vec2(38.0, 38.0));
    p.rect_filled(icon_rect, CornerRadius::same(9), tint(l.color, 0.18));
    icons::paint(&p, icon_rect.center(), l.icon, 21.0, l.color);
    let name = p.layout_no_wrap(l.name.clone(), FontId::new(15.5, theme::semibold()), t.ink_strong);
    p.galley(pos2(icon_rect.right() + 12.0, icon_rect.top() + 1.0), name, t.ink_strong);
    let kind = p.layout_no_wrap(l.kind.label().to_owned(), FontId::new(12.0, FontFamily::Proportional), t.faint);
    p.galley(pos2(icon_rect.right() + 12.0, icon_rect.top() + 21.0), kind, t.faint);
    let mut job = egui::text::LayoutJob::simple(l.description.clone(), FontId::proportional(13.0), t.muted, width - 28.0);
    job.wrap.max_rows = 2;
    job.wrap.overflow_character = Some('…');
    p.galley(pos2(rect.left() + 14.0, icon_rect.bottom() + 10.0), p.layout_job(job), t.muted);
    let mut cmd = egui::text::LayoutJob::simple_singleline(l.command.clone(), FontId::new(11.0, FontFamily::Monospace), t.faint);
    cmd.wrap.max_width = width - 110.0;
    cmd.wrap.max_rows = 1;
    cmd.wrap.overflow_character = Some('…');
    p.galley(pos2(rect.left() + 14.0, rect.bottom() - 24.0), p.layout_job(cmd), t.faint);
    // Bouton d'ajout, en bas à droite.
    let button = Rect::from_min_size(rect.right_bottom() - vec2(88.0, 40.0), vec2(76.0, 30.0));
    if added {
        p.text(button.center(), egui::Align2::CENTER_CENTER, tf("{0} Ajouté", &[("0", &(icons::ch(Icon::Check)))]), FontId::new(13.0, theme::semibold()), t.good_fg);
        false
    } else {
        let b = ui.interact(button, Id::unique(("gallery-add", &l.name)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        b.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tf("Ajouter {0}", &[("0", &l.name)])));
        let fill = if b.hovered() { t.accent_hover } else { t.accent };
        p.rect_filled(button, CornerRadius::same(7), fill);
        p.text(button.center(), egui::Align2::CENTER_CENTER, tf("{0} Ajouter", &[("0", &(icons::ch(Icon::Plus)))]), FontId::new(13.0, theme::semibold()), t.on_accent);
        b.clicked()
    }
}

/// Le plus gros logo SVG accepté (le code du fichier).
const SVG_MAX: usize = 300_000;

/// Vérifie le code d'un logo SVG : du SVG, pas trop lourd. Renvoie le code nettoyé.
pub fn check_svg(text: &str) -> Result<String, String> {
    let text = text.trim().trim_start_matches('\u{feff}');
    if !text.contains("<svg") {
        return Err(tr("Ce n'est pas du SVG (il faut une balise <svg>).").into());
    }
    if text.len() > SVG_MAX {
        return Err(tf("Logo trop lourd : {0} Ko au plus.", &[("0", &(SVG_MAX / 1000))]));
    }
    Ok(text.to_owned())
}

/// Lit un fichier .svg déposé ou choisi.
fn read_svg(path: &std::path::Path) -> Result<String, String> {
    let is_svg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if !is_svg {
        return Err(tr("Seuls les fichiers .svg sont acceptés.").into());
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    check_svg(&text)
}

/// Le logo SVG d'un lanceur : choisir un fichier, coller le code, ou déposer un fichier sur la
/// fenêtre (voir `editor`). Le logo remplace l'icône ; on revient à l'icône en le retirant.
fn svg_logo(ui: &mut Ui, d: &mut LauncherDraft, width: f32) {
    let t = Tokens::get(ui.ctx());
    widgets::labeled(ui, tr("Logo (SVG)"), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if !d.svg.is_empty() {
                let l = d.to_launcher();
                widgets::launcher_icon_tile(ui, &l, 34.0);
                if widgets::button_sized(ui, Some(Icon::X), tr("Retirer le logo"), Kind::Secondary, 32.0, None).clicked() {
                    d.svg.clear();
                }
            }
            if widgets::button_sized(ui, Some(Icon::FileUp), tr("Choisir un fichier SVG…"), Kind::Secondary, 32.0, None).clicked()
                && let Some(path) = rfd::FileDialog::new().set_title(tr("Logo du lanceur (SVG)")).add_filter("SVG", &["svg"]).pick_file()
            {
                match read_svg(&path) {
                    Ok(svg) => (d.svg, d.svg_error) = (svg, None),
                    Err(e) => d.svg_error = Some(e),
                }
            }
            if d.svg_paste.is_none() && widgets::button_sized(ui, Some(Icon::ClipboardPaste), tr("Coller le code SVG"), Kind::Secondary, 32.0, None).clicked() {
                d.svg_paste = Some(String::new());
            }
        });
        if let Some(code) = d.svg_paste.as_mut() {
            widgets::named(ui.add(egui::TextEdit::multiline(code).code_editor().desired_rows(4).desired_width(width).hint_text("<svg viewBox=\"0 0 24 24\">…</svg>")), egui::Role::MultilineTextInput, tr("Code SVG du logo"));
            let mut close = false;
            ui.horizontal(|ui| {
                if widgets::button_sized(ui, Some(Icon::Check), tr("Utiliser ce logo"), Kind::Primary, 30.0, None).clicked() {
                    match check_svg(code) {
                        Ok(svg) => {
                            (d.svg, d.svg_error) = (svg, None);
                            close = true;
                        }
                        Err(e) => d.svg_error = Some(e),
                    }
                }
                if widgets::button_sized(ui, None, tr("Annuler"), Kind::Ghost, 30.0, None).clicked() {
                    close = true;
                }
            });
            if close {
                d.svg_paste = None;
            }
        }
        if let Some(e) = &d.svg_error {
            ui.label(RichText::new(e).size(12.5).color(t.danger));
        }
        ui.label(RichText::new(tr("Ou dépose un fichier .svg sur cette fenêtre. Le logo remplace l'icône.")).size(12.0).color(t.faint));
    });
}

/// Un fichier .svg déposé sur la fenêtre devient le logo du lanceur en cours d'édition.
fn take_dropped_svg(ctx: &egui::Context, d: &mut LauncherDraft) {
    let dropped = ctx.input(|i| i.raw.dropped_files.clone());
    for file in dropped {
        let path = file.path();
        let result = if path.as_os_str().is_empty() {
            file.bytes().and_then(|bytes| check_svg(&String::from_utf8_lossy(&bytes)))
        } else {
            read_svg(path)
        };
        match result {
            Ok(svg) => (d.svg, d.svg_error) = (svg, None),
            Err(e) => d.svg_error = Some(e),
        }
    }
}

/// Pendant qu'on survole la fenêtre avec un fichier : un cadre invite à le déposer.
pub fn drop_hint(ctx: &egui::Context, text: &str) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let t = Tokens::get(ctx);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::unique("svg-drop-hint")));
    let rect = ctx.viewport_rect().shrink(24.0);
    painter.rect(rect, CornerRadius::same(16), tint(t.accent, 0.10), Stroke::new(2.0, t.accent), StrokeKind::Inside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, text, FontId::new(18.0, theme::semibold()), t.ink_strong);
}

/// L'éditeur seul (depuis la configuration : création ou modification).
pub fn editor(ctx: &egui::Context, data: &Data, launcher: Option<usize>, draft: &mut LauncherDraft, act: &mut Vec<Action>) -> bool {
    let t = Tokens::get(ctx);
    let mut open = true;
    take_dropped_svg(ctx, draft);
    drop_hint(ctx, tr("Dépose le fichier .svg : il devient le logo du lanceur"));
    let preview = data.focus().first().map(|&p| &data.projects[p]);
    let modal = egui::Modal::new(Id::unique("launcher-editor")).backdrop_color(theme::scrim(&t)).frame(modal_frame(&t)).show(ctx, |ui| {
        ui.set_width(820.0);
        let (title, subtitle) = match launcher {
            Some(id) => (tf("Modifier « {0} »", &[("0", &data.launchers[id].name)]), tr("Les projets qui l'utilisent verront le changement.")),
            None => (tr("Nouveau lanceur").to_owned(), tr("Il s'ajoutera aux lanceurs par défaut, sur tous les projets.")),
        };
        if modal_header(ui, &title, subtitle) {
            open = false;
        }
        ui.add_space(16.0);
        editor_form(ui, draft, preview);
        ui.add_space(12.0);
        widgets::separator(ui);
        ui.add_space(4.0);
        let (cancel, ok) = footer_buttons(ui, if launcher.is_some() { tr("Enregistrer") } else { tr("Créer le lanceur") }, draft.is_valid());
        if cancel {
            open = false;
        }
        if ok {
            match launcher {
                Some(id) => act.push(Action::SaveLauncher(id, draft.to_launcher())),
                None => act.push(Action::AddLauncher { launcher: draft.to_launcher(), project: None }),
            }
            open = false;
        }
    });
    open && !modal.should_close()
}

/// Le formulaire d'un lanceur, avec un aperçu en direct du bouton et de la commande résolue.
pub fn editor_form(ui: &mut Ui, d: &mut LauncherDraft, preview: Option<&Project>) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal_top(|ui| {
        let form_w = ui.available_width() - 300.0;
        ui.vertical(|ui| {
            ui.set_width(form_w);
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.horizontal(|ui| {
                let name_w = form_w - 150.0;
                ui.vertical(|ui| {
                    ui.set_width(name_w);
                    widgets::labeled(ui, tr("Nom du bouton"), |ui| widgets::text_input(ui, &mut d.name, tr("ex. Storybook"), name_w - 8.0));
                });
                ui.vertical(|ui| widgets::labeled(ui, tr("Raccourci"), |ui| widgets::text_input(ui, &mut d.shortcut, tr("Ctrl+4"), 136.0)));
            });
            widgets::labeled(ui, tr("Type"), |ui| {
                let options: Vec<(LauncherKind, Option<Icon>, &str)> = LauncherKind::ALL.iter().map(|k| (*k, Some(k.icon()), k.label())).collect();
                widgets::segmented(ui, "launcher-kind", &mut d.kind, &options, Some(form_w));
            });
            let (label, hint) = match d.kind {
                LauncherKind::App => ("Application et arguments", r#"code "{dossier}""#),
                LauncherKind::Command => (tr("Commande"), tr("npm run storybook")),
                LauncherKind::Url => (tr("Adresse"), "http://localhost:{port}"),
                LauncherKind::Folder => (tr("Dossier"), r"{dossier}\docs"),
            };
            widgets::labeled(ui, label, |ui| {
                widgets::mono_input(ui, &mut d.command, hint, form_w);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    ui.label(RichText::new(tr("Insérer :")).size(12.5).color(t.faint));
                    for (var, help) in VARIABLES {
                        if widgets::tag(ui, var).interact(Sense::click()).on_hover_text(help).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            if !d.command.is_empty() && !d.command.ends_with(' ') {
                                d.command.push(' ');
                            }
                            d.command.push_str(var);
                        }
                    }
                });
            });
            if d.kind == LauncherKind::Command {
                widgets::toggle_row(ui, &mut d.in_terminal, tr("Ouvrir dans un terminal"), Some(tr("La commande tourne dans un nouvel onglet, placé dans le dossier du projet.")));
            }
            widgets::labeled(ui, tr("Icône"), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    for icon in LAUNCHER_ICONS {
                        let selected = icons::same(icon, d.icon);
                        let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
                        let fill = if selected { tint(d.color, 0.22) } else if resp.hovered() { t.raised } else { t.input };
                        let stroke = if selected { Stroke::new(1.5, d.color) } else { Stroke::new(1.0, t.line) };
                        ui.painter().rect(rect, CornerRadius::same(8), fill, stroke, StrokeKind::Inside);
                        icons::paint(ui.painter(), rect.center(), icon, 18.0, if selected { d.color } else { t.muted });
                        if resp.clicked() {
                            d.icon = icon;
                        }
                    }
                });
            });
            svg_logo(ui, d, form_w);
            widgets::labeled(ui, tr("Couleur"), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (hex, name) in COLORS {
                        let c = theme::rgb(hex);
                        if widgets::swatch(ui, c, c == d.color, tr(name)).clicked() {
                            d.color = c;
                        }
                    }
                });
            });
        });
        ui.add_space(16.0);
        // Aperçu en direct. (Un `Frame` hérite de la direction de son parent : ici une rangée
        // horizontale, d'où le `vertical` explicite.)
        Frame::new().fill(t.bg).stroke(Stroke::new(1.0, t.line)).corner_radius(12).inner_margin(Margin::same(16)).show(ui, |ui| ui.vertical(|ui| {
            ui.set_width(250.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            widgets::caps(ui, tr("Aperçu"));
            let mut l = d.to_launcher();
            if l.name.is_empty() {
                l.name = tr("Mon lanceur").into();
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                widgets::launcher_pill(ui, &l, false, 0.0);
                widgets::launcher_pill(ui, &l, true, 0.0);
            });
            ui.label(RichText::new(tr("À droite, en favori : il prend la couleur d'accent.")).size(12.0).color(t.faint));
            ui.add_space(6.0);
            if let Some(project) = preview {
                widgets::caps(ui, &tf("Pour {0}", &[("0", &project.name)]));
                let resolved = if l.command.is_empty() { "—".to_owned() } else { l.resolve(project) };
                Frame::new().fill(t.input).corner_radius(8).inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(resolved).font(FontId::new(11.5, FontFamily::Monospace)).color(t.ink));
                });
                if l.in_terminal {
                    ui.label(RichText::new(tf("{0} dans un terminal ouvert sur le dossier", &[("0", &(icons::ch(Icon::SquareTerminal)))])).size(12.0).color(t.muted));
                }
            }
        }));
    });
}

const LAUNCHER_ICONS: [Icon; 24] = [
    Icon::Rocket,
    Icon::SquareTerminal,
    Icon::Terminal,
    Icon::Code,
    Icon::Braces,
    Icon::Sparkles,
    Icon::Bot,
    Icon::Globe,
    Icon::GitBranch,
    Icon::Play,
    Icon::Hammer,
    Icon::Container,
    Icon::Database,
    Icon::Server,
    Icon::FolderOpen,
    Icon::NotebookPen,
    Icon::BookOpen,
    Icon::PenTool,
    Icon::Palette,
    Icon::Webhook,
    Icon::Bug,
    Icon::FlaskConical,
    Icon::Cloud,
    Icon::Zap,
];

const COLORS: [(u32, &str); 10] = [
    (0xf4a945, "Ambre"),
    (0xe07a5f, "Corail"),
    (0xff5c8a, "Rose"),
    (0xb98cf0, "Violet"),
    (0x5b8def, "Bleu"),
    (0x3b9cff, "Azur"),
    (0x3fc1b0, "Turquoise"),
    (0x7fcf6f, "Vert"),
    (0xdea584, "Sable"),
    (0x9aa3ad, "Gris"),
];
