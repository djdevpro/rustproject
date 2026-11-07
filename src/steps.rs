//! La liste des étapes : les cinq plus récentes, avec édition sur place.

use egui::{Ui, vec2};

use crate::app::Action;
use crate::i18n::{tf, tr};
use crate::icons::Icon;
use crate::model::Project;
use crate::widgets::{self, Kind};

const PREVIEW_COUNT: usize = 5;

#[derive(Default)]
pub struct State {
    project: Option<u64>,
    all: bool,
    edit: Option<Edit>,
}

struct Edit {
    index: usize,
    original: String,
    text: String,
    focus: bool,
}

pub fn show(ui: &mut Ui, project: &Project, p: usize, state: &mut State, new_step: &mut String, act: &mut Vec<Action>) {
    if state.project != Some(project.id) {
        *state = State { project: Some(project.id), ..Default::default() };
    }
    // Un agent peut modifier ou retirer une étape pendant l'édition. Ne jamais appliquer
    // le brouillon à l'étape qui aurait pris sa place après une relecture du fichier.
    if state.edit.as_ref().is_some_and(|edit| project.steps.get(edit.index).is_none_or(|step| step.text != edit.original)) {
        state.edit = None;
    }
    let (done, total) = project.steps_done();
    widgets::info_row(ui, Icon::ListChecks, &tf("Prochaines étapes · {done}/{total}", &[("done", &done), ("total", &total)]), |ui| {
        ui.add_space(4.0);
        let width = ui.available_width();
        let spacing = ui.spacing().item_spacing.x;
        let count = if state.all { total } else { PREVIEW_COUNT };
        // L'ordre de stockage reste inchangé : le MCP utilise les index de `steps`.
        for (i, step) in project.steps.iter().enumerate().rev().take(count) {
            ui.push_id((project.id, i), |ui| {
                if state.edit.as_ref().is_some_and(|edit| edit.index == i) {
                    let edit = state.edit.as_mut().unwrap();
                    let response = ui.add(egui::TextEdit::multiline(&mut edit.text)
                        .hint_text(tr("Texte de l'étape"))
                        .desired_width(width)
                        .desired_rows(2));
                    if edit.focus {
                        response.request_focus();
                        edit.focus = false;
                    }
                    let cancel_key = (response.has_focus() || response.lost_focus()) && ui.input(|input| input.key_pressed(egui::Key::Escape));
                    let mut save = false;
                    let mut cancel = cancel_key;
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(!edit.text.trim().is_empty(), |ui| {
                            save = widgets::button(ui, Some(Icon::Check), tr("Enregistrer"), Kind::Primary).clicked();
                        });
                        cancel |= widgets::button(ui, None, tr("Annuler"), Kind::Secondary).clicked();
                    });
                    if cancel {
                        state.edit = None;
                    } else if save {
                        act.push(Action::EditStep(p, i, edit.original.clone(), edit.text.trim().to_owned()));
                        state.edit = None;
                    }
                } else {
                    ui.horizontal(|ui| {
                        let resp = ui.allocate_ui(vec2((width - 80.0 - 2.0 * spacing).max(40.0), 40.0), |ui| {
                            let mut checked = step.done;
                            widgets::checkbox(ui, &mut checked, &step.text, true)
                        });
                        if resp.inner.changed() {
                            act.push(Action::ToggleStep(p, i));
                        }
                        if widgets::icon_button(ui, Icon::Pencil, &tf("Modifier « {0} »", &[("0", &step.text)]), 40.0).clicked() {
                            state.edit = Some(Edit { index: i, original: step.text.clone(), text: step.text.clone(), focus: true });
                        }
                        if widgets::icon_button(ui, Icon::X, &tf("Retirer « {0} »", &[("0", &step.text)]), 40.0).clicked() {
                            act.push(Action::RemoveStep(p, i));
                        }
                    });
                }
            });
        }
        if total > PREVIEW_COUNT {
            let label = if state.all { tr("Voir moins") } else { tr("Voir plus") };
            let icon = if state.all { Icon::ChevronUp } else { Icon::ChevronDown };
            if widgets::button(ui, Some(icon), label, Kind::Ghost).clicked() {
                state.all = !state.all;
                state.edit = None;
            }
        }
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let resp = widgets::text_input(ui, new_step, tr("Ajouter une étape…"), width - 40.0 - spacing);
            let enter = resp.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            let add = widgets::icon_button(ui, Icon::Plus, tr("Ajouter l'étape"), 40.0).clicked();
            if (enter || add) && !new_step.trim().is_empty() {
                act.push(Action::AddStep(p, new_step.trim().to_owned()));
                new_step.clear();
                if enter {
                    resp.request_focus();
                }
            }
        });
    });
}
