//! Les tags et les mots-clés d'un projet, dans le panneau : à lire (onglet « Résumé ») et à
//! modifier (onglet « Modifier »).
//!
//! - Les tags viennent de l'ontologie, rangés par facette (Domaine, Nature, Finalité…). Un projet
//!   peut avoir plusieurs domaines ; l'étoile désigne le principal (sa couleur et son île sur la
//!   carte). Un tag proposé par l'IA montre sa probabilité ; un clic le valide.
//! - On ajoute un tag en tapant : l'ontologie propose les concepts proches ; sinon on le crée,
//!   dans une facette existante ou une nouvelle, sous un parent si on veut.
//! - Les mots-clés sont libres : ce qu'on taperait pour retrouver le projet. Ils comptent dans la
//!   recherche et dans l'embedding (la recherche par le sens).

use egui::{Color32, FontId, Frame, Margin, RichText, Sense, Stroke, Ui, vec2};

use crate::app::{Action, TagNew};
use crate::i18n::{tf, tr};
use crate::icons::{self, Icon};
use crate::model::Data;
use crate::ontology::LINK_MIN;
use crate::theme::{self, Tokens, tint};
use crate::widgets::{self, Kind};

/// Ce qu'on a cliqué sur une pastille de tag.
enum Click {
    Star,
    Remove,
    Confirm,
}

/// Un petit bouton-icône (étoile, croix) dans une pastille.
fn mini(ui: &mut Ui, icon: Icon, color: Color32, name: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
    let c = if resp.hovered() { tint(color, 1.0) } else { tint(color, 0.75) };
    icons::paint(ui.painter(), rect.center(), icon, 13.0, c);
    resp.on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Une pastille de tag. `star` : `Some(principal)` pour un domaine ; `ai` : la probabilité d'un
/// tag proposé par l'IA (pas encore validé) ; `removable` : la croix.
fn chip(ui: &mut Ui, label: &str, color: Color32, star: Option<bool>, ai: Option<f32>, removable: bool, tip: &str) -> Option<Click> {
    let t = Tokens::get(ui.ctx());
    let mut out = None;
    // Sa largeur, calculée avant de la poser : une `Frame` dans une rangée qui passe à la ligne ne
    // va pas à la ligne d'elle-même, elle déborde (et le panneau avec). Le nom trop long est coupé.
    let font = egui::FontId::proportional(12.5);
    let pct = ai.map(|w| format!("{:.0} %", w * 100.0));
    let extras = 16.0 + 2.0
        + if star.is_some() { 20.0 } else { 0.0 }
        + pct.as_ref().map_or(0.0, |p| ui.painter().layout_no_wrap(p.clone(), egui::FontId::proportional(11.0), t.faint).size().x + 4.0)
        + if removable { 20.0 } else { 0.0 };
    let row = ui.max_rect().width();
    let label = widgets::elide_end(ui.painter(), label, &font, (row - extras).max(40.0));
    let width = extras + ui.painter().layout_no_wrap(label.clone(), font, t.ink_strong).size().x;
    if width > ui.available_size_before_wrap().x && ui.cursor().min.x > ui.max_rect().min.x + 1.0 {
        ui.end_row();
    }
    let label = label.as_str();
    let fill = if ai.is_some() { Color32::TRANSPARENT } else { theme::soft(&t, color, 0.16) };
    let stroke = Stroke::new(1.0, tint(color, if ai.is_some() { 0.6 } else { 0.32 }));
    let resp = Frame::new().fill(fill).stroke(stroke).corner_radius(12).inner_margin(Margin { left: 8, right: 6, top: 2, bottom: 2 }).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.horizontal(|ui| {
            if let Some(main) = star {
                let name = if main { tr("Domaine principal") } else { tr("En faire le domaine principal") };
                if mini(ui, Icon::Star, if main { t.accent } else { t.faint }, name) && !main {
                    out = Some(Click::Star);
                }
            }
            let text = RichText::new(label).size(12.5).color(t.ink_strong);
            if ai.is_some() {
                if ui.add(egui::Label::new(text).sense(Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    out = Some(Click::Confirm);
                }
            } else {
                ui.label(text);
            }
            if let Some(p) = &pct {
                ui.label(RichText::new(p).size(11.0).color(t.faint));
            }
            if removable && mini(ui, Icon::X, t.muted, &tf("Retirer {0}", &[("0", &label)])) {
                out = Some(Click::Remove);
            }
        });
    });
    resp.response.on_hover_text(tip);
    out
}

/// L'onglet « Résumé » : les tags par facette, puis les mots-clés.
pub fn summary(ui: &mut Ui, data: &Data, p: usize) {
    let t = Tokens::get(ui.ctx());
    let project = &data.projects[p];
    let domain_facet = data.ontology.domain_facet();
    for facet in &data.ontology.facets {
        let mut names: Vec<(String, String)> = Vec::new();
        if Some(facet.id) == domain_facet {
            for (k, d) in data.domains_of(p).into_iter().enumerate() {
                let name = &data.domains[d].name;
                // L'étoile de la police d'icônes (la police à chasse fixe n'en a pas).
                let label = if k == 0 { format!("{} {name}", icons::ch(Icon::Star)) } else { name.clone() };
                names.push((label, if k == 0 { tr("Domaine principal").to_owned() } else { name.clone() }));
            }
        }
        for tag in project.tags.iter().filter(|t| t.weight >= LINK_MIN) {
            let Some(c) = data.ontology.concept(tag.concept).filter(|c| c.facet == facet.id) else { continue };
            // Les domaines de premier niveau sont déjà là.
            if Some(facet.id) == domain_facet && c.parent.is_none() {
                continue;
            }
            names.push((c.name.clone(), format!("{} · {:.0} %", data.ontology.path(c.id), tag.weight * 100.0)));
        }
        if names.is_empty() {
            continue;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            ui.label(RichText::new(&facet.name).size(12.0).color(t.faint));
            for (name, tip) in &names {
                widgets::tag(ui, name).on_hover_text(tip);
            }
        });
    }
    if !project.keywords.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            ui.label(RichText::new(tr("Mots-clés")).size(12.0).color(t.faint));
            for k in &project.keywords {
                widgets::tag(ui, k);
            }
        });
    }
}

/// L'onglet « Modifier » : les tags par facette (retirer, valider, domaine principal), puis le
/// champ pour en ajouter ou en créer.
pub fn editor(ui: &mut Ui, data: &Data, p: usize, query: &mut String, new: &mut Option<TagNew>, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let project = &data.projects[p];
    let domain_facet = data.ontology.domain_facet();
    let domains = data.domains_of(p);
    let w = ui.available_width();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        widgets::field_label(ui, tr("Tags"));
        ui.label(RichText::new(tr("Plusieurs domaines possibles : l'étoile désigne le principal, celui de la carte.")).size(12.0).color(t.faint));
        for facet in &data.ontology.facets {
            let is_domain = Some(facet.id) == domain_facet;
            let tags: Vec<_> = project.tags.iter().filter(|t| t.weight >= LINK_MIN && data.ontology.concept(t.concept).is_some_and(|c| c.facet == facet.id && !(is_domain && c.parent.is_none()))).collect();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                let (r, _) = ui.allocate_exact_size(vec2(78.0, 22.0), Sense::hover());
                widgets::paint_text(ui.painter(), r.left_top() + vec2(0.0, 4.0), &facet.name, FontId::new(12.0, theme::semibold()), t.muted, 76.0);
                if is_domain {
                    for (k, &d) in domains.iter().enumerate() {
                        let dm = &data.domains[d];
                        let concept = data.domain_concept(d);
                        let tag = concept.and_then(|c| project.tags.iter().find(|t| t.concept == c));
                        let ai = tag.filter(|t| !t.manual).map(|t| t.weight);
                        let tip = if k == 0 { tr("Domaine principal : sa couleur et son île sur la carte.") } else { tr("Domaine en plus") };
                        match chip(ui, &dm.name, dm.color, Some(k == 0), ai, domains.len() > 1 && concept.is_some(), tip) {
                            Some(Click::Star) => act.push(Action::MainDomain(p, d)),
                            Some(Click::Remove) => act.extend(concept.map(|c| Action::Untag(p, c))),
                            Some(Click::Confirm) => act.extend(concept.map(|c| Action::Tag(p, c))),
                            None => {}
                        }
                    }
                }
                for tag in &tags {
                    let Some(c) = data.ontology.concept(tag.concept) else { continue };
                    let color = if is_domain { data.domain_of_concept(c.id).and_then(|d| data.domains.get(d)).map(|d| d.color).unwrap_or(t.accent) } else { t.info };
                    let tip = if tag.manual { data.ontology.path(c.id) } else { tf("{0} · proposé par l'IA, clic pour valider", &[("0", &data.ontology.path(c.id))]) };
                    match chip(ui, &c.name, color, None, (!tag.manual).then_some(tag.weight), true, &tip) {
                        Some(Click::Remove) => act.push(Action::Untag(p, c.id)),
                        Some(Click::Confirm) => act.push(Action::Tag(p, c.id)),
                        _ => {}
                    }
                }
                if tags.is_empty() && !is_domain {
                    ui.label(RichText::new("—").size(12.5).color(t.faint));
                }
            });
        }
        // Ajouter : les concepts proches de ce qu'on tape, ou en créer un.
        let resp = widgets::icon_input(ui, query, Icon::Tag, tr("Ajouter un tag : tape un mot…"), w, theme::CONTROL_H);
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let q = query.trim().to_owned();
        if !q.is_empty() && new.is_none() {
            let hits = data.suggest_concepts(p, &q, 6);
            if enter && let Some(c) = hits.first() {
                act.push(Action::Tag(p, c.id));
                query.clear();
            }
            Frame::new().fill(t.raised).stroke(Stroke::new(1.0, t.line)).corner_radius(8).inner_margin(Margin::same(4)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                for c in &hits {
                    let facet = data.ontology.facet(c.facet).map(|f| f.name.as_str()).unwrap_or("");
                    let label = format!("{}   ·   {facet} › {}", c.name, data.ontology.path(c.id));
                    if ui.add(egui::Button::new(RichText::new(label).size(13.0).color(t.ink)).frame(false).min_size(vec2(ui.available_width(), 28.0))).clicked() {
                        act.push(Action::Tag(p, c.id));
                        query.clear();
                    }
                }
                let create = tf("Créer le tag « {0} »…", &[("0", &q)]);
                if ui.add(egui::Button::new(RichText::new(format!("{}  {create}", icons::ch(Icon::Plus))).size(13.0).color(t.accent_ink)).frame(false).min_size(vec2(ui.available_width(), 28.0))).clicked() {
                    // Par défaut : la facette du concept le plus proche, sinon « Domaine ».
                    let facet = hits.first().map(|c| c.facet).or(domain_facet).or(data.ontology.facets.first().map(|f| f.id));
                    *new = Some(TagNew { name: q.clone(), facet, ..Default::default() });
                }
            });
        }
        if let Some(n) = new.as_mut()
            && creator(ui, data, p, n, act)
        {
            *new = None;
            query.clear();
        }
    });
}

/// Le formulaire de création d'un tag : son nom, sa facette (ou une nouvelle), son parent.
/// Renvoie `true` si on l'abandonne.
fn creator(ui: &mut Ui, data: &Data, p: usize, n: &mut TagNew, act: &mut Vec<Action>) -> bool {
    let t = Tokens::get(ui.ctx());
    let mut cancel = false;
    Frame::new().fill(tint(t.accent, 0.06)).stroke(Stroke::new(1.0, tint(t.accent, 0.35))).corner_radius(10).inner_margin(Margin::same(12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 8.0;
        let w = ui.available_width();
        widgets::labeled(ui, tr("Nouveau tag"), |ui| widgets::text_input(ui, &mut n.name, tr("Nom du tag"), w));
        // La facette : une existante, ou une nouvelle (dernière option).
        const NEW: u64 = u64::MAX;
        let mut facets: Vec<(u64, &str)> = data.ontology.facets.iter().map(|f| (f.id, f.name.as_str())).collect();
        facets.push((NEW, tr("Nouvelle facette…")));
        let mut facet = n.facet.unwrap_or(NEW);
        widgets::labeled(ui, tr("Facette"), |ui| {
            if widgets::combo(ui, tr("Facette du tag"), &mut facet, &facets, w - 4.0) {
                n.facet = (facet != NEW).then_some(facet);
                n.parent = None;
            }
        });
        if n.facet.is_none() {
            widgets::labeled(ui, tr("Nom de la facette"), |ui| widgets::text_input(ui, &mut n.new_facet, tr("ex. Client, Stade, Public"), w));
        } else {
            // Le parent : un concept plus général de la même facette (facultatif).
            let tree = n.facet.map(|f| data.ontology.tree(f)).unwrap_or_default();
            let names: Vec<String> = tree.iter().map(|(depth, c)| format!("{}{}", "   ".repeat(*depth), c.name)).collect();
            let mut parents: Vec<(u64, &str)> = vec![(0, tr("Aucun (premier niveau)"))];
            parents.extend(tree.iter().zip(names.iter()).map(|((_, c), name)| (c.id, name.as_str())));
            let mut parent = n.parent.unwrap_or(0);
            widgets::labeled(ui, tr("Sous le tag"), |ui| {
                if widgets::combo(ui, tr("Tag parent"), &mut parent, &parents, w - 4.0) {
                    n.parent = (parent != 0).then_some(parent);
                }
            });
            if data.ontology.domain_facet() == n.facet && n.parent.is_none() {
                ui.label(RichText::new(tr("Au premier niveau de « Domaine », le tag crée un nouveau domaine.")).size(12.0).color(t.faint));
            }
        }
        ui.horizontal(|ui| {
            let ready = !n.name.trim().is_empty() && (n.facet.is_some() || !n.new_facet.trim().is_empty());
            if ui.add_enabled_ui(ready, |ui| widgets::button(ui, Some(Icon::Plus), tr("Créer et ajouter"), Kind::Primary)).inner.clicked() {
                act.push(Action::NewTag { project: p, facet: n.facet, new_facet: n.new_facet.clone(), name: n.name.clone(), parent: n.parent });
            }
            if widgets::button(ui, None, tr("Annuler"), Kind::Secondary).clicked() {
                cancel = true;
            }
        });
    });
    cancel
}

/// Les mots-clés, dans l'onglet « Modifier » : une pastille par mot, une croix pour le retirer,
/// un champ pour en ajouter (Entrée ; plusieurs d'un coup, séparés par des virgules).
pub fn keywords_editor(ui: &mut Ui, data: &Data, p: usize, input: &mut String, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let project = &data.projects[p];
    let w = ui.available_width();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.horizontal(|ui| {
            widgets::field_label(ui, tr("Mots-clés"));
            let llm = data.config.ollama_on && !data.config.ollama_model.trim().is_empty();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if llm && widgets::link(ui, tr("Proposer avec Ollama"), Some(Icon::Sparkles), 12.5).clicked() {
                    act.push(Action::SuggestKeywords(Some(vec![project.id])));
                }
            });
        });
        ui.label(RichText::new(tr("Les mots qu'on taperait pour le retrouver : sujet, public, usage. Ils comptent dans la recherche par le sens.")).size(12.0).color(t.faint));
        if !project.keywords.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                for (k, word) in project.keywords.iter().enumerate() {
                    if chip(ui, word, t.muted, None, None, true, word).is_some() {
                        let mut words = project.keywords.clone();
                        words.remove(k);
                        act.push(Action::SetKeywords(p, words));
                    }
                }
            });
        }
        let resp = widgets::text_input(ui, input, tr("Ajouter un mot-clé, puis Entrée"), w);
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !input.trim().is_empty() {
            let mut words = project.keywords.clone();
            words.extend(input.split([',', ';']).map(|s| s.trim().to_owned()));
            act.push(Action::SetKeywords(p, crate::classify::clean_keywords(words)));
            input.clear();
            resp.request_focus();
        }
    });
}
