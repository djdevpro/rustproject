//! Configuration › Ontologie : les facettes et leurs concepts, éditables à la main ; la
//! proposition par l'IA (Ollama, ou l'agent MCP) ; le classement des projets par Ollaya.
//!
//! Tout s'enregistre au fil de la frappe. Un concept retouché à la main devient « manuel » : une
//! nouvelle proposition de l'IA le garde tel quel.

use egui::{Align, FontId, Frame, Id, Layout, Margin, RichText, Stroke, Ui, vec2};

use crate::app::{Action, CfgSection, UiState};
use crate::icons::Icon;
use crate::model::{Data, ago};
use crate::ontology::{DOMAIN_FACET, Merge};
use crate::theme::{self, Tokens};
use crate::widgets::{self, Kind, Tone};
use crate::i18n::{tf, tr};

pub fn show(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        widgets::title(ui, tr("Ontologie"), 26.0);
        widgets::muted(ui, tr("Des facettes (axes de rangement) et leurs concepts. L'IA les propose en voyant tous tes projets ; tu les retouches ; Ollaya y range chaque projet."));
    });
    overview(ui, data, st, act);
    if st.onto_proposal.is_some() {
        proposal(ui, st, act);
    }
    classification(ui, data, st, act);
    let facets: Vec<u64> = data.ontology.facets.iter().map(|f| f.id).collect();
    for f in facets {
        facet_card(ui, data, f);
    }
    if widgets::button(ui, Some(Icon::Plus), tr("Ajouter une facette"), Kind::Secondary).clicked() {
        data.ontology.add_facet(tr("Nouvelle facette"), "");
    }
}

fn overview(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::settings_card(ui, tr("Vue d'ensemble"), None, |ui| {
        let linked = data.projects.iter().filter(|p| p.tags.iter().any(|x| x.weight >= crate::ontology::LINK_MIN)).count();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            let plural = |n: usize, word: &str| format!("{n} {word}{}", if n > 1 { "s" } else { "" });
            widgets::chip(ui, &plural(data.ontology.facets.len(), "facette"), Tone::Neutral);
            widgets::chip(ui, &plural(data.ontology.concepts.len(), "concept"), Tone::Neutral);
            let tone = if linked == data.projects.len() && linked > 0 { Tone::Good } else { Tone::Neutral };
            widgets::chip(ui, &tf("{linked}/{0} projets rattachés", &[("linked", &linked), ("0", &data.projects.len())]), tone);
            if data.ontology.generated_at > 0 {
                ui.label(RichText::new(tf("Proposée par {0} {1}", &[("0", &data.ontology.generated_by), ("1", &ago(data.ontology.generated_at))])).size(12.5).color(t.faint));
            }
        });
        let model = data.config.ollama_model.clone();
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!model.is_empty() && !st.onto_generating, |ui| {
                let resp = widgets::button(ui, Some(Icon::Sparkles), tr("Proposer avec l'IA"), Kind::Primary);
                if resp.clicked() {
                    act.push(Action::GenerateOntology);
                }
                resp.on_disabled_hover_text(tr("Choisis d'abord un modèle de langage Ollama (IA & modèles)."));
            });
            if st.onto_generating {
                ui.add(egui::Spinner::new().size(16.0).color(t.accent));
                ui.label(RichText::new(tf("{model} lit tes {0} projets…", &[("model", &model), ("0", &data.projects.len())])).size(13.0).color(t.muted));
            } else if model.is_empty() {
                if widgets::link(ui, tr("Choisir un modèle de langage"), Some(Icon::ArrowRight), 13.0).clicked() {
                    act.push(Action::GoConfig(CfgSection::Ai));
                }
            } else {
                ui.label(RichText::new(tf("Avec {model}, qui voit tous tes projets d'un coup. Rien n'est appliqué sans ton accord.", &[("model", &model)])).size(12.5).color(t.faint));
            }
        });
        if let Some(e) = &st.onto_error {
            ui.label(RichText::new(e).size(12.5).color(t.danger));
        }
        ui.horizontal(|ui| {
            crate::icons::show(ui, Icon::Lightbulb, 15.0, t.faint);
            ui.label(RichText::new(tr("Avec un agent MCP : « Propose une ontologie pour mes projets avec Atelier » (prompt concevoir_ontologie).")).size(12.5).color(t.muted));
        });
    });
}

fn proposal(ui: &mut Ui, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let Some((model, o)) = &st.onto_proposal else { return };
    Frame::new().fill(theme::tint(t.accent, 0.06)).stroke(Stroke::new(1.0, theme::tint(t.accent, 0.35))).corner_radius(theme::R_CARD).inner_margin(Margin::same(18)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.horizontal(|ui| {
            crate::icons::show(ui, Icon::Sparkles, 18.0, t.accent_ink);
            ui.label(RichText::new(tf("Proposition de {model}", &[("model", &model)])).font(FontId::new(17.0, theme::bold())).color(t.ink_strong));
            widgets::chip(ui, &tf("{0} facettes · {1} concepts", &[("0", &o.facets.len()), ("1", &o.concepts.len())]), Tone::Accent);
        });
        for f in &o.facets {
            ui.label(RichText::new(&f.name).font(FontId::new(14.5, theme::semibold())).color(t.ink));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                for (depth, c) in o.tree(f.id) {
                    let label = if depth > 0 { format!("↳ {}", c.name) } else { c.name.clone() };
                    let tip = if c.synonyms.is_empty() { c.definition.clone() } else { tf("{0}\nSynonymes : {1}", &[("0", &c.definition), ("1", &(c.synonyms.join(", ")))]) };
                    widgets::tag(ui, &label).on_hover_text(tip);
                }
            });
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if widgets::button(ui, Some(Icon::Check), tr("Remplacer (garde tes concepts manuels)"), Kind::Primary).clicked() {
                act.push(Action::ApplyProposal(Merge::Replace));
            }
            if widgets::button(ui, Some(Icon::Plus), tr("Ajouter seulement les nouveaux"), Kind::Secondary).clicked() {
                act.push(Action::ApplyProposal(Merge::AddNew));
            }
            if widgets::button(ui, Some(Icon::X), tr("Ignorer"), Kind::Ghost).clicked() {
                act.push(Action::DiscardProposal);
            }
        });
        ui.label(RichText::new(tr("Même nom, même concept : les rattachements des projets suivent. Après coup, reclasse les projets.")).size(12.5).color(t.faint));
    });
}

fn classification(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::settings_card(ui, tr("Classement par Ollaya"), Some(tr("Une question oui/non par concept, sa définition en critère. À partir de 50 %, le projet est rattaché. Les rattachements faits à la main restent.")), |ui| {
        let model = data.config.ollaya_model.clone();
        if model.is_empty() {
            ui.horizontal(|ui| {
                widgets::chip(ui, "inactif", Tone::Neutral);
                ui.label(RichText::new(tr("Choisis un modèle de décision Ollaya (decider:2b conseillé).")).size(13.0).color(t.muted));
            });
            if widgets::link(ui, tr("Régler Ollaya"), Some(Icon::ArrowRight), 13.0).clicked() {
                act.push(Action::GoConfig(CfgSection::Ai));
            }
            return;
        }
        let outdated = data.outdated_tags().len();
        let running = st.tagging.as_ref().is_some_and(|x| !x.finished);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            widgets::tag(ui, &model);
            if outdated == 0 {
                widgets::chip(ui, tr("tous les projets sont classés"), Tone::Good);
            } else {
                widgets::chip(ui, &tf("{outdated} projets à (re)classer", &[("outdated", &outdated)]), Tone::Neutral);
            }
        });
        if let Some(x) = &st.tagging {
            if !x.finished {
                let paused = x.paused();
                ui.horizontal(|ui| {
                    if paused {
                        crate::icons::show(ui, Icon::Pause, 16.0, t.muted);
                    } else {
                        ui.add(egui::Spinner::new().size(16.0).color(t.accent));
                    }
                    let what = if paused { tr("en pause").to_owned() } else { x.current.clone() };
                    let first = if x.done == 0 && !paused { tr(" · le premier appel charge le modèle") } else { "" };
                    ui.label(RichText::new(format!("{}/{} · {what}{first}", x.done, x.total)).size(13.0).color(t.ink));
                });
                widgets::progress(ui, x.done as f32 / x.total.max(1) as f32, if paused { t.muted } else { t.accent }, ui.available_width(), 6.0);
                ui.horizontal(|ui| {
                    let (icon, label, state) = if paused { (Icon::Play, tr("Reprendre"), crate::jobs::RUN) } else { (Icon::Pause, tr("Mettre en pause"), crate::jobs::PAUSE) };
                    if widgets::button(ui, Some(icon), label, Kind::Secondary).clicked() {
                        act.push(Action::ControlTagging(state));
                    }
                    if widgets::button(ui, Some(Icon::Square), tr("Arrêter"), Kind::Ghost).on_hover_text(tr("Les projets déjà classés le restent.")).clicked() {
                        act.push(Action::ControlTagging(crate::jobs::STOP));
                    }
                });
            }
            if let Some(e) = &x.error {
                ui.label(RichText::new(e).size(12.5).color(t.danger));
            }
        }
        if !running {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(outdated > 0 && !data.ontology.concepts.is_empty(), |ui| {
                    if widgets::button(ui, Some(Icon::Sparkles), &tf("Classer {outdated} projets", &[("outdated", &outdated)]), Kind::Primary).clicked() {
                        act.push(Action::TagProjects(None));
                    }
                });
                ui.add_enabled_ui(!data.projects.is_empty(), |ui| {
                    if widgets::button(ui, Some(Icon::RefreshCw), tr("Tout reclasser"), Kind::Secondary).clicked() {
                        act.push(Action::TagProjects(Some(data.projects.iter().map(|p| p.id).collect())));
                    }
                });
            });
        }
        ui.label(RichText::new(tr("Le classement ne part que sur ton clic. Après avoir changé l'ontologie, reclasse.")).size(12.5).color(t.faint));
    });
}

/// Une liste de mots séparés par des virgules. Pendant la frappe, le texte reste tel quel (pour
/// pouvoir taper la virgule suivante) ; la liste suit à chaque changement.
fn list_input(ui: &mut Ui, id: Id, list: &mut Vec<String>, hint: &str, width: f32) -> bool {
    let mut text = ui.data(|d| d.get_temp::<String>(id)).unwrap_or_else(|| list.join(", "));
    let resp = ui.push_id(id, |ui| widgets::text_input(ui, &mut text, hint, width)).inner;
    if resp.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    if resp.changed() {
        *list = text.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
        return true;
    }
    false
}

fn facet_card(ui: &mut Ui, data: &mut Data, facet: u64) {
    let t = Tokens::get(ui.ctx());
    widgets::card(&t).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 8.0;
        let w = ui.available_width();
        let Some(fi) = data.ontology.facets.iter().position(|f| f.id == facet) else { return };
        let is_domain = data.ontology.facets[fi].name == DOMAIN_FACET;
        let mut delete_facet = false;
        ui.horizontal(|ui| {
            let f = &mut data.ontology.facets[fi];
            ui.push_id(("facet-name", facet), |ui| {
                ui.add(egui::TextEdit::singleline(&mut f.name).font(FontId::new(17.0, theme::bold())).desired_width(w - 160.0).margin(Margin::symmetric(8, 4)).hint_text(tr("Nom de la facette")));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if !is_domain && widgets::icon_button(ui, Icon::Trash2, tr("Supprimer la facette et ses concepts"), 30.0).clicked() {
                    delete_facet = true;
                }
                let n = data.ontology.concepts.iter().filter(|c| c.facet == facet).count();
                ui.label(RichText::new(tf("{n} concepts", &[("n", &n)])).size(12.5).color(t.faint));
            });
        });
        ui.push_id(("facet-desc", facet), |ui| widgets::text_input(ui, &mut data.ontology.facets[fi].description, tr("À quoi sert cette facette (un axe de rangement)"), w));
        if is_domain {
            ui.label(RichText::new(tr("Le meilleur concept de premier niveau devient le domaine du projet (carte, filtres de la bibliothèque).")).size(12.0).color(t.faint));
        }
        ui.add_space(2.0);
        let rows: Vec<(usize, u64)> = data.ontology.tree(facet).into_iter().map(|(d, c)| (d, c.id)).collect();
        let mut delete = None;
        for (depth, id) in rows {
            let Some(ci) = data.ontology.concepts.iter().position(|c| c.id == id) else { continue };
            let mut c = data.ontology.concepts[ci].clone();
            let old_name = c.name.clone();
            let indent = depth as f32 * 22.0;
            let cw = w - indent;
            let mut changed = false;
            // Les parents possibles : les concepts de la facette, sauf lui et ses descendants.
            let mut excluded = vec![id];
            loop {
                let more: Vec<u64> = data.ontology.concepts.iter().filter(|x| x.parent.is_some_and(|p| excluded.contains(&p)) && !excluded.contains(&x.id)).map(|x| x.id).collect();
                if more.is_empty() {
                    break;
                }
                excluded.extend(more);
            }
            let names: Vec<(Option<u64>, String)> =
                std::iter::once((None, tr("— premier niveau —").to_owned())).chain(data.ontology.concepts.iter().filter(|x| x.facet == facet && !excluded.contains(&x.id)).map(|x| (Some(x.id), tf("sous « {0} »", &[("0", &x.name)])))).collect();
            let options: Vec<(Option<u64>, &str)> = names.iter().map(|(v, l)| (*v, l.as_str())).collect();
            let count = data.concept_count(id);
            ui.push_id(("concept", id), |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    changed |= widgets::text_input(ui, &mut c.name, tr("Nom du concept"), (cw * 0.38).max(120.0)).changed();
                    changed |= widgets::combo(ui, tr("Concept parent"), &mut c.parent, &options, (cw * 0.30).max(110.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, Icon::Trash2, &tf("Supprimer « {0} »", &[("0", &c.name)]), 30.0).clicked() {
                            delete = Some(id);
                        }
                        if c.manual {
                            widgets::chip(ui, "manuel", Tone::Accent).on_hover_text(tr("Retouché à la main : gardé tel quel par les propositions de l'IA."));
                        }
                        widgets::chip(ui, &if count > 1 { tf("{count} projets", &[("count", &count)]) } else { tf("{count} projet", &[("count", &count)]) }, if count > 0 { Tone::Good } else { Tone::Neutral });
                    });
                });
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    changed |= widgets::text_input(ui, &mut c.definition, tr("Définition : une phrase qui permet de décider si un projet en relève"), cw).changed();
                });
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    changed |= list_input(ui, Id::unique(("synonyms", id)), &mut c.synonyms, tr("Synonymes, séparés par des virgules (servent à la recherche)"), cw);
                });
            });
            if changed {
                c.manual = true;
                // Un concept de domaine renommé : son domaine suit (carte, filtres).
                if is_domain
                    && c.name != old_name
                    && !c.name.trim().is_empty()
                    && let Some(d) = data.domains.iter_mut().find(|d| d.name == old_name)
                {
                    d.name = c.name.clone();
                }
                data.ontology.concepts[ci] = c;
            }
            widgets::separator(ui);
        }
        if let Some(id) = delete {
            data.remove_concept(id);
        }
        if widgets::link(ui, tr("Ajouter un concept"), Some(Icon::Plus), 13.5).clicked() {
            let id = data.ontology.add_concept(facet, tr("Nouveau concept"), "");
            if let Some(c) = data.ontology.concepts.iter_mut().find(|c| c.id == id) {
                c.manual = true;
            }
        }
        if delete_facet {
            data.remove_facet(facet);
        }
    });
}
