//! Le panneau projet (à droite) : en-tête, rangée de lanceurs « Ouvrir avec », puis quatre
//! onglets : Résumé, Activité, Ressources, Modifier.

use egui::{Align, FontFamily, FontId, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, vec2};

use crate::app::{Action, CfgSection, DraftState, PanelTab, ProjectDraft, UiState, idle_text};
use crate::icons::{self, Icon};
use crate::model::{Attention, Data, Status};
use crate::scan;
use crate::theme::{self, Tokens};
use crate::widgets::{self, Kind};

/// Durée de l'éclat d'une tuile après un lancement, en secondes.
const FLASH_SECS: f64 = 0.8;

pub fn show(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>, p: usize) {
    let t = Tokens::get(ui.ctx());
    // Peu de marge à droite : la barre de défilement s'y loge, loin des boutons (voir `scroll_content`).
    let frame = Frame::new().fill(t.surface).stroke(Stroke::new(1.0, t.line)).corner_radius(14).inner_margin(Margin { left: 20, right: 6, top: 20, bottom: 16 });
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(ui.available_height());
        widgets::draggable(ScrollArea::vertical()).id_salt("panel-scroll").auto_shrink(false).show(ui, |ui| {
            widgets::scroll_content(ui, |ui| {
                header(ui, data, p, act);
                ui.add_space(16.0);
                launchers(ui, data, st, p, act);
                ui.add_space(10.0);
                let tabs = [(PanelTab::Summary, "Résumé"), (PanelTab::Activity, "Activité"), (PanelTab::Resources, "Ressources"), (PanelTab::Edit, "Modifier")];
                widgets::tabs(ui, "panel-tabs", &mut st.tab, &tabs);
                ui.add_space(16.0);
                match st.tab {
                    PanelTab::Summary => summary(ui, data, p, st, act),
                    PanelTab::Activity => activity(ui, data, p, st, act),
                    PanelTab::Resources => resources(ui, data, p, act),
                    PanelTab::Edit => edit(ui, data, p, st, act),
                }
            });
        });
    });
}

fn header(ui: &mut Ui, data: &Data, p: usize, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let project = &data.projects[p];
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        widgets::icon_tile(ui, project.icon, t.accent_ink, 52.0);
        // Place réservée à droite pour les deux boutons (focus, fermer).
        let text_w = (ui.available_width() - 84.0).max(80.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.set_width(text_w);
            ui.add(egui::Label::new(RichText::new(&project.name).font(FontId::new(26.0, crate::theme::bold())).color(t.ink_strong)).truncate());
            // Le résultat visé, ou à défaut la description en une phrase.
            let subtitle = if project.outcome.is_empty() { &project.description } else { &project.outcome };
            ui.add(egui::Label::new(RichText::new(subtitle).size(15.0).color(t.muted)).truncate()).on_hover_text(&project.description);
            ui.add_space(4.0);
            // L'état et l'attention se changent d'un clic sur leur pastille.
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                let statuses: Vec<(Status, &str)> = Status::ALL.iter().map(|s| (*s, s.label())).collect();
                if let Some(s) = widgets::chip_menu(ui, "Changer l'état", project.status, widgets::status_tone(project.status), &statuses) {
                    act.push(Action::SetStatus(p, s));
                }
                let attentions: Vec<(Attention, &str)> = Attention::ALL.iter().map(|a| (*a, a.label())).collect();
                if let Some(a) = widgets::chip_menu(ui, "Changer l'attention", project.attention, widgets::attention_tone(project.attention), &attentions) {
                    act.push(Action::SetAttention(p, a, false));
                }
            });
        });
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            if widgets::icon_button(ui, Icon::X, "Fermer le panneau", 32.0).clicked() {
                act.push(Action::Deselect);
            }
        });
    });
}

/// Les lanceurs en tuiles, côte à côte ; la rangée défile à l'horizontale (molette, barre, ou
/// en glissant à la souris).
fn launchers(ui: &mut Ui, data: &Data, st: &UiState, p: usize, act: &mut Vec<Action>) {
    let ids = data.launchers_of(p);
    let favorite = data.favorite_of(p);
    let project = &data.projects[p];
    ui.horizontal(|ui| {
        widgets::caps(ui, "Ouvrir avec");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::link(ui, "Gérer", Some(Icon::Settings2), 12.5).on_hover_text("Configuration › Lanceurs").clicked() {
                act.push(Action::GoConfig(CfgSection::Launchers));
            }
        });
    });
    widgets::draggable(ScrollArea::horizontal()).id_salt(("launcher-row", p)).auto_shrink([false, true]).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for &id in &ids {
                let l = &data.launchers[id];
                let is_default = data.default_launchers.contains(&id);
                let flash = match st.flash {
                    Some((fp, fl, at)) if fp == p && fl == id => (1.0 - (st.now - at) / FLASH_SECS).clamp(0.0, 1.0) as f32,
                    _ => 0.0,
                };
                if flash > 0.0 {
                    ui.ctx().request_repaint();
                }
                let resp = widgets::launcher_tile(ui, l, Some(id) == favorite, flash);
                if resp.clicked() {
                    act.push(Action::Launch(p, id));
                }
                let shortcut = if l.shortcut.is_empty() { String::new() } else { format!(" · {}", l.shortcut) };
                let resp = resp.on_hover_text(format!("{}{shortcut}\n{}\nClic droit : options", l.name, l.resolve(project)));
                resp.context_menu(|ui| {
                    ui.set_min_width(220.0);
                    if ui.button(format!("{}  Mettre en avant", icons::ch(Icon::Star))).clicked() {
                        act.push(Action::SetFavorite(p, id));
                    }
                    if !is_default {
                        if ui.button(format!("{}  Vers la gauche", icons::ch(Icon::ArrowLeft))).clicked() {
                            act.push(Action::MoveInProject(p, id, -1));
                        }
                        if ui.button(format!("{}  Vers la droite", icons::ch(Icon::ArrowRight))).clicked() {
                            act.push(Action::MoveInProject(p, id, 1));
                        }
                    }
                    if ui.button(format!("{}  Modifier le lanceur…", icons::ch(Icon::Pencil))).clicked() {
                        act.push(Action::OpenEditor(Some(id)));
                    }
                    ui.separator();
                    let label = if is_default { "Masquer pour ce projet" } else { "Retirer de ce projet" };
                    if ui.button(format!("{}  {label}", icons::ch(Icon::EyeOff))).clicked() {
                        act.push(Action::HideForProject(p, id));
                    }
                });
            }
            if widgets::add_tile(ui, "Ajouter un lanceur").clicked() {
                act.push(Action::OpenGallery(Some(p)));
            }
        });
        // De la place sous les tuiles pour la barre de défilement horizontale.
        ui.add_space(12.0);
    });
}

fn summary(ui: &mut Ui, data: &Data, p: usize, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let project = &data.projects[p];
    let gap = |ui: &mut Ui| {
        ui.add_space(10.0);
        widgets::separator(ui);
        ui.add_space(12.0);
    };
    if !project.description.is_empty() {
        widgets::info_row(ui, Icon::Info, "En une phrase", |ui| {
            ui.label(RichText::new(&project.description).size(14.5).color(t.ink));
        });
        gap(ui);
    }
    if !project.objective.is_empty() {
        widgets::info_row(ui, Icon::Crosshair, "Objectif", |ui| {
            widgets::muted(ui, &project.objective);
        });
        gap(ui);
    }
    if !project.last_point.is_empty() {
        widgets::info_row(ui, Icon::FileText, "Dernier point", |ui| {
            widgets::muted(ui, &project.last_point);
        });
        gap(ui);
    }
    widgets::info_row(ui, Icon::Zap, "Prochaine action", |ui| {
        if project.next_action.is_empty() {
            widgets::muted(ui, "Pas encore définie (onglet « Modifier »).");
        } else {
            ui.label(RichText::new(&project.next_action).size(14.5).color(t.ink));
        }
    });
    gap(ui);
    let (done, total) = project.steps_done();
    widgets::info_row(ui, Icon::ListChecks, &format!("Prochaines étapes · {done}/{total}"), |ui| {
        ui.add_space(4.0);
        let width = ui.available_width();
        // L'espacement hérité de `info_row` (14 points), pas celui par défaut : sinon la rangée
        // déborde et le panneau, élargi à gauche, voit sa bordure rognée.
        let spacing = ui.spacing().item_spacing.x;
        for (i, step) in project.steps.iter().enumerate() {
            // Case à cocher, et une croix discrète pour retirer l'étape.
            ui.horizontal(|ui| {
                let resp = ui.allocate_ui(vec2(width - 24.0 - spacing, 24.0), |ui| {
                    let mut checked = step.done;
                    widgets::checkbox(ui, &mut checked, &step.text, true)
                });
                if resp.inner.changed() {
                    act.push(Action::ToggleStep(p, i));
                }
                if widgets::icon_button(ui, Icon::X, &format!("Retirer « {} »", step.text), 24.0).clicked() {
                    act.push(Action::RemoveStep(p, i));
                }
            });
        }
        // Ajouter une étape : on tape, puis Entrée.
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let resp = widgets::text_input(ui, &mut st.new_step, "Ajouter une étape…", width - 34.0 - spacing);
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let add = widgets::icon_button(ui, Icon::Plus, "Ajouter l'étape", 34.0).clicked();
            if (enter || add) && !st.new_step.trim().is_empty() {
                act.push(Action::AddStep(p, st.new_step.trim().to_owned()));
                st.new_step.clear();
                if enter {
                    // On garde la main dans le champ pour enchaîner les étapes.
                    resp.request_focus();
                }
            }
        });
    });
    gap(ui);
    widgets::info_row(ui, Icon::Calendar, "Prochaine revue", |ui| {
        widgets::muted(ui, &project.review);
    });
    // Les concepts de l'ontologie auxquels Ollaya (ou l'utilisateur) l'a rattaché.
    let concepts = data.concepts_of(p);
    if !concepts.is_empty() {
        gap(ui);
        widgets::info_row(ui, Icon::Network, "Concepts", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                for (c, w) in concepts {
                    let facet = data.ontology.facet(c.facet).map(|f| f.name.as_str()).unwrap_or("");
                    widgets::tag(ui, &c.name).on_hover_text(format!("{facet} · {} · {:.0} %", data.ontology.path(c.id), w * 100.0));
                }
            });
        });
    }
}

fn activity(ui: &mut Ui, data: &Data, p: usize, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let project = &data.projects[p];
    ui.horizontal(|ui| {
        let w = ui.available_width() - 92.0;
        let resp = widgets::text_input(ui, &mut st.new_note, "Noter un point d'étape…", w);
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if (widgets::button(ui, None, "Noter", Kind::Secondary).clicked() || enter) && !st.new_note.trim().is_empty() {
            act.push(Action::AddNote(p, st.new_note.trim().to_owned()));
            st.new_note.clear();
        }
    });
    ui.add_space(14.0);
    if project.activity.is_empty() {
        widgets::muted(ui, "Rien pour l'instant : les étapes, notes et modifications apparaîtront ici.");
    }
    // Frise : une ligne verticale, un point par évènement.
    for (i, a) in project.activity.iter().enumerate() {
        let last = i + 1 == project.activity.len();
        ui.horizontal_top(|ui| {
            let (rail, _) = ui.allocate_exact_size(vec2(30.0, 58.0), egui::Sense::hover());
            let c = rail.center_top() + vec2(0.0, 14.0);
            if !last {
                ui.painter().vline(c.x, (c.y + 14.0)..=rail.bottom() + 8.0, Stroke::new(1.5, t.line_strong));
            }
            ui.painter().circle_filled(c, 13.0, t.raised);
            icons::paint(ui.painter(), c, a.kind.icon(), 14.0, t.accent_ink);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add_space(4.0);
                ui.label(RichText::new(&a.text).size(14.0).color(t.ink));
                ui.label(RichText::new(crate::model::ago(a.at)).size(12.5).color(t.faint));
            });
        });
    }
    ui.add_space(6.0);
    widgets::muted(ui, &format!("Dernière activité dans le dossier {} · {}.", idle_text(project.idle_days), project.added_by.label()));
}

fn resources(ui: &mut Ui, data: &Data, p: usize, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let project = &data.projects[p];
    let explorer = data.launchers.iter().position(|l| l.name == "Explorateur");

    widgets::caps(ui, "Dossiers");
    for folder in &project.folders {
        let actions = if explorer.is_some() { 2 } else { 1 };
        row(ui, Icon::Folder, folder, true, actions, |ui| {
            if let Some(e) = explorer
                && widgets::icon_button(ui, Icon::FolderOpen, "Ouvrir dans l'explorateur", 28.0).clicked()
            {
                act.push(Action::Launch(p, e));
            }
            if widgets::icon_button(ui, Icon::Copy, "Copier le chemin", 28.0).clicked() {
                ui.ctx().copy_text(folder.clone());
                act.push(Action::Toast(Icon::Copy, "Chemin copié".into(), Some(folder.clone())));
            }
        });
    }
    ui.add_space(12.0);
    widgets::caps(ui, "Fichiers clés · relevés au scan");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for f in &project.key_files {
            widgets::tag(ui, f);
        }
    });
    ui.add_space(14.0);
    widgets::caps(ui, "Liens");
    if project.links.is_empty() {
        widgets::muted(ui, "Aucun lien pour l'instant.");
    }
    for (name, url) in &project.links {
        row(ui, Icon::Link, &format!("{name} — {url}"), false, 0, |_| {});
    }
    ui.add_space(14.0);
    widgets::caps(ui, "Notes");
    for note in &project.notes {
        Frame::new().fill(t.raised).corner_radius(8).inner_margin(Margin::symmetric(12, 10)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(note).size(14.0).color(t.ink));
        });
    }
}

/// Ligne de ressource : icône, texte coupé par « … » s'il est trop long (au milieu pour un
/// chemin, pour garder sa fin), puis `actions` boutons à droite.
fn row(ui: &mut Ui, icon: Icon, text: &str, path: bool, actions: usize, add_actions: impl FnOnce(&mut Ui)) {
    let t = Tokens::get(ui.ctx());
    Frame::new().stroke(Stroke::new(1.0, t.line)).corner_radius(8).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            icons::show(ui, icon, 17.0, t.muted);
            let font = if path { FontId::new(12.0, FontFamily::Monospace) } else { FontId::proportional(13.5) };
            // Largeur restante : moins l'icône, les boutons (28 + espace chacun) et une marge.
            let max_w = (ui.available_width() - actions as f32 * 34.0 - 8.0).max(40.0);
            let shown = if path { widgets::elide_middle(ui.painter(), text, &font, max_w) } else { widgets::elide_end(ui.painter(), text, &font, max_w) };
            let label = ui.add(egui::Label::new(RichText::new(shown).font(font).color(t.ink)).selectable(false));
            if label.hovered() {
                label.on_hover_text(text);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                add_actions(ui);
            });
        });
    });
}

/// L'onglet « Modifier » : chaque changement est enregistré tout de suite, sans bouton ni
/// notification.
fn edit(ui: &mut Ui, data: &Data, p: usize, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let project = &data.projects[p];
    let fresh = ProjectDraft::from_project(project);
    match &mut st.draft {
        // Ce qui a changé ailleurs (bouton focus, agent MCP…) arrive dans le formulaire.
        Some(s) if s.project == p => {
            if s.base != fresh {
                s.draft.merge_external(&s.base, &fresh);
                s.base = fresh.clone();
            }
        }
        _ => st.draft = Some(DraftState { project: p, draft: fresh.clone(), base: fresh.clone() }),
    }
    let Some(s) = st.draft.as_mut() else { return };
    let d = &mut s.draft;
    let w = ui.available_width();
    ui.spacing_mut().item_spacing.y = 12.0;
    widgets::labeled(ui, "Nom du projet", |ui| widgets::text_input(ui, &mut d.name, "Nom", w));
    widgets::labeled(ui, "Description (une phrase)", |ui| widgets::text_input(ui, &mut d.description, "Ce qu'est le projet, en une phrase", w));
    widgets::labeled(ui, "Résultat recherché", |ui| widgets::text_input(ui, &mut d.outcome, "Ce que tu veux avoir entre les mains", w));
    ui.horizontal(|ui| {
        let half = (w - 12.0) / 2.0;
        let statuses: Vec<(Status, &str)> = Status::ALL.iter().map(|s| (*s, s.label())).collect();
        let attentions: Vec<(Attention, &str)> = Attention::ALL.iter().map(|a| (*a, a.label())).collect();
        ui.vertical(|ui| {
            ui.set_width(half);
            widgets::labeled(ui, "État", |ui| widgets::combo(ui, "État du projet", &mut d.status, &statuses, half - 4.0));
        });
        ui.vertical(|ui| {
            ui.set_width(half);
            widgets::labeled(ui, "Attention", |ui| widgets::combo(ui, "Niveau d'attention", &mut d.attention, &attentions, half - 4.0));
        });
    });
    widgets::labeled(ui, "Prochaine action", |ui| widgets::text_input(ui, &mut d.next_action, "La toute prochaine chose à faire", w));
    widgets::labeled(ui, "Objectif", |ui| {
        widgets::named(ui.add(egui::TextEdit::multiline(&mut d.objective).desired_rows(3).desired_width(w).margin(Margin::symmetric(12, 8))), egui::Role::MultilineTextInput, "Objectif")
    });
    let domains: Vec<(usize, &str)> = data.domains.iter().enumerate().map(|(i, dm)| (i, dm.name.as_str())).collect();
    widgets::labeled(ui, "Domaine (proposé par l'IA)", |ui| widgets::combo(ui, "Domaine du projet", &mut d.domain, &domains, w - 4.0));
    // L'icône (un bouton qui ouvre le sélecteur), à gauche des dossiers.
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        widgets::labeled(ui, "Icône", |ui| widgets::icon_picker(ui, "Icône du projet", &mut d.icon));
        ui.vertical(|ui| {
            let fw = ui.available_width();
            widgets::labeled(ui, "Dossiers du projet", |ui| {
                let mut remove = None;
                for (i, folder) in d.folders.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        widgets::path_input(ui, folder, fw - 40.0);
                        if widgets::icon_button(ui, Icon::Trash2, "Retirer ce dossier", 34.0).clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    d.folders.remove(i);
                }
                if widgets::link(ui, "Ajouter un dossier", Some(Icon::Plus), 13.5).clicked() {
                    d.folders.push(String::new());
                }
            });
        });
    });
    widgets::labeled(ui, "Dépôt GitHub", |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            widgets::mono_input(ui, &mut d.remote, "https://github.com/…", w - 42.0);
            if widgets::icon_button(ui, Icon::GitBranch, "Détecter depuis le dépôt Git du dossier", theme::CONTROL_H).clicked() {
                match scan::remote_of(&d.folders) {
                    Some(url) => d.remote = url,
                    None => act.push(Action::Toast(Icon::GitBranch, "Aucun dépôt distant".into(), Some("Les dossiers du projet n'ont pas de remote Git.".into()))),
                }
            }
        });
    });
    // Enregistrement automatique : ce qui diffère du projet part tout de suite.
    let applied = s.draft.normalized(project);
    if applied != fresh {
        s.base = applied.clone();
        act.push(Action::SaveDraft(p, applied));
    }
    ui.add_space(6.0);
    widgets::separator(ui);
    ui.add_space(4.0);
    if widgets::button(ui, Some(Icon::Trash2), "Supprimer le projet", Kind::Danger).clicked() {
        act.push(Action::AskDelete(p));
    }
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Chaque changement est enregistré tout de suite.").italics().size(12.5).color(t.faint));
    });
}
