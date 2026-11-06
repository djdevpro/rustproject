//! « Mon bureau » : les projets du moment (focus, en nombre limité), ceux qui avancent au fil de
//! l'eau, et un projet de la réserve à redécouvrir.

use egui::{Align, CornerRadius, FontId, Id, Layout, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::{Action, AddProjectState, CfgSection, Screen, UiState, footer_buttons, idle_text};
use crate::icons::{self, Icon};
use crate::model::{Attention, Data, Status};
use crate::theme::{self, Tokens, mix, tint};
use crate::widgets::{self, Kind, Tone};
use crate::i18n::{tf, tr};

const FLOW_PREVIEW: usize = 6;

/// Le projet de la réserve proposé aujourd'hui : le plus ancien pas encore passé en revue.
pub fn rediscover_queue(data: &Data, st: &UiState) -> Vec<usize> {
    let mut ids: Vec<usize> = (0..data.projects.len())
        .filter(|&i| {
            let p = &data.projects[i];
            p.attention == Attention::Reserve && p.status != Status::Archived && !st.decisions.iter().any(|(d, _)| *d == i)
        })
        .collect();
    ids.sort_by_key(|&i| std::cmp::Reverse(data.projects[i].idle_days));
    ids
}

pub fn show(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    // Barre d'outils : recherche et ajout.
    ui.horizontal(|ui| {
        let w = ui.available_width() - 214.0;
        widgets::icon_input(ui, &mut st.search, Icon::Search, tr("Rechercher un projet…"), w, 46.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::button_sized(ui, Some(Icon::Plus), tr("Ajouter un projet"), Kind::Primary, 46.0, Some(200.0)).clicked() {
                act.push(Action::OpenAddProject);
            }
        });
    });
    ui.add_space(14.0);
    if data.projects.is_empty() {
        first_run(ui, st, !data.config.roots.is_empty(), act);
        return;
    }
    // Une recherche : la liste des résultats (tous les projets, réserve comprise) remplace les sections.
    if let Some(hits) = st.search_hits.clone() {
        search_results(ui, data, st, &hits, act);
        return;
    }

    widgets::draggable(ScrollArea::vertical()).id_salt("desk-scroll").auto_shrink(false).show(ui, |ui| {
      widgets::scroll_content(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        let width = ui.available_width();

        // ---- Mes projets du moment : une rangée qui défile à l'horizontale.
        let focus: Vec<usize> = data.focus().into_iter().filter(|&i| st.passes(i)).collect();
        ui.horizontal(|ui| {
            widgets::section_header(ui, tr("Mes projets du moment"), None);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (icon, label) = if st.reorder { (Icon::Check, tr("Terminer")) } else { (Icon::ArrowLeftRight, tr("Réorganiser")) };
                if widgets::button(ui, Some(icon), label, Kind::Ghost).clicked() {
                    st.reorder = !st.reorder;
                }
            });
        });
        // Quatre cartes tiennent dans la largeur (moins si la fenêtre est étroite) ; au-delà, on
        // fait défiler : molette, barre, ou en glissant à la souris.
        let gap = theme::GAP;
        let slots = (((width + gap) / (230.0 + gap)).floor() as usize).clamp(1, 4);
        let card_w = ((width - gap * (slots - 1) as f32) / slots as f32).floor();
        if focus.is_empty() {
            widgets::card(&t).show(ui, |ui| {
                ui.set_width(ui.available_width());
                let text = tr("Aucun projet en focus. Choisis-en un au fil de l'eau ou dans la bibliothèque.");
                widgets::muted(ui, text);
            });
        } else {
            widgets::draggable(ScrollArea::horizontal()).id_salt("focus-row").auto_shrink([false, true]).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &i in &focus {
                        focus_card(ui, data, i, st.selected == Some(i), st.reorder, card_w, act);
                    }
                });
                if focus.len() > slots {
                    // De la place pour la barre de défilement sous les cartes.
                    ui.add_space(12.0);
                }
            });
        }

        // ---- Au fil de l'eau
        ui.add_space(14.0);
        let flow: Vec<usize> = (0..data.projects.len())
            .filter(|&i| data.projects[i].attention == Attention::Flow && data.projects[i].status != Status::Archived && st.passes(i))
            .collect();
        ui.horizontal(|ui| {
            widgets::section_header(ui, tr("Au fil de l'eau"), Some(&data.count(Attention::Flow).to_string()));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::link(ui, tr("Tout voir dans la bibliothèque"), Some(Icon::ArrowRight), 14.0).clicked() {
                    act.push(Action::Library(Some(Attention::Flow)));
                }
            });
        });
        let shown = if st.flow_expanded { flow.len() } else { flow.len().min(FLOW_PREVIEW) };
        widgets::card(&t).inner_margin(egui::Margin::symmetric(0, 4)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            for (n, &i) in flow.iter().take(shown).enumerate() {
                if n > 0 {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().hline((r.left() + 16.0)..=(r.right() - 16.0), r.center().y, t.hairline());
                }
                flow_row(ui, data, i, st.selected == Some(i), act);
            }
            if flow.is_empty() {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| widgets::muted(ui, tr("Rien au fil de l'eau pour cette recherche.")));
                ui.add_space(12.0);
            }
            if flow.len() > shown || (st.flow_expanded && flow.len() > FLOW_PREVIEW) {
                let label = if st.flow_expanded { tr("Réduire").to_owned() } else { tf("Afficher les {0} autres", &[("0", &(flow.len() - shown))]) };
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    if widgets::button(ui, Some(if st.flow_expanded { Icon::ChevronUp } else { Icon::ChevronDown }), &label, Kind::Ghost).clicked() {
                        st.flow_expanded = !st.flow_expanded;
                    }
                });
                ui.add_space(4.0);
            }
        });

        // ---- À redécouvrir
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            widgets::section_header(ui, tr("À redécouvrir"), None);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::link(ui, tr("Ouvrir la revue"), Some(Icon::ArrowRight), 14.0).clicked() {
                    act.push(Action::Go(Screen::Revisit));
                }
            });
        });
        match rediscover_queue(data, st).first() {
            Some(&i) => rediscover_card(ui, data, i, act),
            None => {
                widgets::card(&t).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        icons::show(ui, Icon::CircleCheck, 22.0, t.good_fg);
                        let text = if data.count(Attention::Reserve) == 0 { tr("Aucun projet en réserve : rien à redécouvrir pour l'instant.") } else { tr("Revue du jour terminée. Rendez-vous demain pour un autre projet.") };
                        widgets::muted(ui, text);
                    });
                });
            }
        }
        ui.add_space(8.0);
      });
    });
}

/// Premier lancement : aucun projet encore. Le scan démarre tout seul ; on explique et on guide.
fn first_run(ui: &mut Ui, st: &UiState, has_roots: bool, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(40.0);
    widgets::card(&t).inner_margin(egui::Margin::same(32)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical_centered(|ui| {
            widgets::icon_tile(ui, Icon::FolderSearch, t.accent_ink, 64.0);
            ui.add_space(8.0);
            if !has_roots && st.scan_started.is_none() {
                // Rien n'est choisi à la place de l'utilisateur : c'est lui qui désigne ses dossiers.
                widgets::title(ui, tr("Où sont tes projets ?"), 26.0);
                ui.add_space(4.0);
                widgets::muted(ui, tr("Choisis le ou les dossiers qui contiennent tes projets. Rust Project les parcourt sur 1 à 3 niveaux, puis l'IA (ou les règles) en fait des fiches."));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 420.0).max(0.0) / 2.0);
                    if widgets::button(ui, Some(Icon::FolderPlus), tr("Choisir un dossier source"), Kind::Primary).clicked() {
                        act.push(Action::OpenSetup(crate::screens::setup::Step::Where));
                    }
                    if widgets::button(ui, Some(Icon::Plus), tr("Ajouter un projet"), Kind::Secondary).clicked() {
                        act.push(Action::OpenAddProject);
                    }
                });
                return;
            }
            widgets::title(ui, tr("Tes projets arrivent"), 26.0);
            ui.add_space(4.0);
            if let Some(started) = st.scan_started {
                let _ = started;
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 320.0).max(0.0) / 2.0);
                    ui.add(egui::Spinner::new().size(16.0).color(t.accent));
                    ui.label(RichText::new(tf("Scan en cours… {0} dossiers relevés", &[("0", &st.scan_count)])).size(15.0).color(t.muted));
                });
            } else if let Some(a) = st.analysis.as_ref().filter(|a| !a.finished) {
                ui.label(RichText::new(tf("Classement {0}/{1} · {2}", &[("0", &a.done), ("1", &a.total), ("2", &a.current)])).size(15.0).color(t.muted));
            } else if let Some(scan) = &st.scan {
                // Le scan est fait : c'est à l'utilisateur de choisir qui trie (IA ou règles).
                let likely = scan.likely().count();
                ui.label(RichText::new(tf("{0} dossiers relevés, dont {likely} projets probables. Qui fait le tri ?", &[("0", &scan.dirs.len()), ("likely", &likely)])).size(15.0).color(t.muted));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 640.0).max(0.0) / 2.0);
                    if widgets::button(ui, Some(Icon::Cpu), tr("Choisir un modèle Ollama"), Kind::Primary).clicked() {
                        act.push(Action::GoConfig(CfgSection::Ai));
                    }
                    if widgets::button(ui, Some(Icon::Network), tr("Passer par un agent MCP"), Kind::Secondary).clicked() {
                        act.push(Action::GoConfig(CfgSection::Ai));
                    }
                    if widgets::button(ui, Some(Icon::ListPlus), &tf("Créer les {likely} fiches (règles)", &[("likely", &likely)]), Kind::Secondary).on_hover_text(tr("Sans IA : chaque dossier avec un marqueur devient un projet.")).clicked() {
                        act.push(Action::AddAllLikely);
                    }
                });
                return;
            } else {
                widgets::muted(ui, tr("Rust Project parcourt tes dossiers sources (3 niveaux) et crée une fiche pour chaque projet reconnu."));
            }
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 560.0).max(0.0) / 2.0);
                if widgets::button(ui, Some(Icon::RefreshCw), tr("Scanner mes dossiers"), Kind::Primary).clicked() {
                    act.push(Action::StartScan);
                }
                if widgets::button(ui, Some(Icon::FolderTree), tr("Dossiers sources"), Kind::Secondary).clicked() {
                    act.push(Action::GoConfig(CfgSection::Sources));
                }
                if widgets::button(ui, Some(Icon::Plus), tr("Ajouter un projet"), Kind::Secondary).clicked() {
                    act.push(Action::OpenAddProject);
                }
            });
        });
    });
}

/// Les résultats d'une recherche, du plus pertinent au moins pertinent.
fn search_results(ui: &mut Ui, data: &Data, st: &UiState, hits: &[crate::search::Hit], act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        widgets::section_header(ui, tr("Résultats"), Some(&hits.len().to_string()));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (text, tone) = if st.query_semantic {
                (tr("Recherche sémantique active"), Tone::Good)
            } else if data.config.ollama_embed_model.is_empty() {
                (tr("Recherche floue · choisis un modèle d'embeddings pour chercher par le sens"), Tone::Neutral)
            } else {
                (tr("Recherche floue · le sens arrive…"), Tone::Neutral)
            };
            widgets::chip(ui, text, tone);
        });
    });
    if hits.is_empty() {
        widgets::card(&t).show(ui, |ui| {
            ui.set_width(ui.available_width());
            widgets::muted(ui, tr("Aucun projet ne correspond. Essaie un autre mot : la recherche tolère les fautes de frappe."));
        });
        return;
    }
    widgets::draggable(ScrollArea::vertical()).id_salt("desk-results").auto_shrink(false).show(ui, |ui| {
        widgets::scroll_content(ui, |ui| {
            widgets::card(&t).inner_margin(egui::Margin::symmetric(0, 4)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                for (n, hit) in hits.iter().enumerate() {
                    if n > 0 {
                        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                        ui.painter().hline((r.left() + 16.0)..=(r.right() - 16.0), r.center().y, t.hairline());
                    }
                    let before = ui.cursor().min;
                    flow_row(ui, data, hit.index, st.selected == Some(hit.index), act);
                    // « ≈ sens proche » : trouvé par le sens, pas par les mots ; « via Immobilier » :
                    // par un concept de l'ontologie auquel le projet est rattaché.
                    let why = match hit.via.and_then(|c| data.ontology.concept(c)) {
                        Some(c) => Some(tf("via {0}", &[("0", &c.name)])),
                        None => hit.semantic_only.then(|| tr("≈ sens proche").to_owned()),
                    };
                    if let Some(why) = why {
                        let pos = pos2(before.x + 22.0 + 42.0, before.y + 47.0);
                        ui.painter().text(pos, egui::Align2::LEFT_CENTER, why, FontId::new(11.0, theme::semibold()), t.accent_ink);
                    }
                }
            });
        });
    });
}

/// Une carte de projet en focus (cliquable en entier).
fn focus_card(ui: &mut Ui, data: &Data, i: usize, selected: bool, reorder: bool, width: f32, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let p = &data.projects[i];
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 206.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &p.name));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
    let painter = ui.painter().clone();
    let stroke = if selected { Stroke::new(2.0, t.accent) } else { Stroke::new(1.0, mix(t.line, t.line_strong, hover)) };
    painter.rect(rect, CornerRadius::same(theme::R_CARD), mix(t.surface, t.raised, hover * 0.6), stroke, StrokeKind::Inside);
    let x0 = rect.left() + 18.0;
    let inner_w = width - 36.0;
    let mut y = rect.top() + 18.0;
    icons::paint(&painter, pos2(x0 + 13.0, y + 13.0), p.icon, 26.0, t.accent_ink);
    widgets::paint_text(&painter, pos2(x0 + 38.0, y + 1.0), &p.name, FontId::new(19.5, theme::bold()), t.ink_strong, inner_w - 38.0 - if reorder { 56.0 } else { 0.0 });
    y += 38.0;
    widgets::paint_text(&painter, pos2(x0, y), &tf("Résultat visé : {0}", &[("0", &p.outcome)]), FontId::proportional(13.5), t.muted, inner_w);
    y += 30.0;
    let w = widgets::paint_chip(&painter, &t, pos2(x0, y + 12.0), p.status.label(), widgets::status_tone(p.status));
    if p.idle_days > 0 {
        let g = painter.layout_no_wrap(idle_text(p.idle_days), FontId::proportional(12.5), t.faint);
        painter.galley(pos2(x0 + w + 10.0, y + 12.0 - g.size().y / 2.0), g, t.faint);
    }
    y += 38.0;
    // Avancement des étapes.
    let (done, total) = p.steps_done();
    let bar = Rect::from_min_size(pos2(x0, y + 4.0), vec2(inner_w - 44.0, 6.0));
    painter.rect_filled(bar, CornerRadius::same(3), t.neutral_bg);
    if total > 0 && done > 0 {
        painter.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * done as f32 / total as f32, bar.height())), CornerRadius::same(3), t.accent);
    }
    let g = painter.layout_no_wrap(format!("{done}/{total}"), FontId::new(12.5, theme::semibold()), t.muted);
    painter.galley(pos2(rect.right() - 18.0 - g.size().x, y + 7.0 - g.size().y / 2.0), g, t.muted);
    y += 22.0;
    icons::paint(&painter, pos2(x0 + 8.0, y + 9.0), Icon::Zap, 14.0, t.faint);
    widgets::paint_text(&painter, pos2(x0 + 22.0, y), &p.next_action, FontId::proportional(13.0), t.ink, inner_w - 22.0);
    // « → Détails », en bas.
    let by = rect.bottom() - 28.0;
    let color = mix(t.muted, t.accent_ink, hover.max(if selected { 1.0 } else { 0.0 }));
    icons::paint(&painter, pos2(x0 + 8.0, by + 9.0), Icon::ArrowRight, 16.0, color);
    let g = painter.layout_no_wrap(tr("Détails").to_owned(), FontId::new(14.0, theme::semibold()), color);
    painter.galley(pos2(x0 + 24.0, by + 9.0 - g.size().y / 2.0), g, color);
    // Mode « Réorganiser » : flèches pour déplacer la carte.
    if reorder {
        for (k, (icon, delta, tip)) in [(Icon::ChevronLeft, -1, tr("Vers la gauche")), (Icon::ChevronRight, 1, tr("Vers la droite"))].into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(rect.right() - 64.0 + 30.0 * k as f32, rect.top() + 14.0), vec2(28.0, 28.0));
            let b = ui.interact(r, Id::unique(("focus-move", i, k)), Sense::click()).on_hover_text(tip);
            painter.rect(r, CornerRadius::same(7), if b.hovered() { t.raised } else { t.input }, Stroke::new(1.0, t.line_strong), StrokeKind::Inside);
            icons::paint(&painter, r.center(), icon, 16.0, t.ink);
            if b.clicked() {
                act.push(Action::MoveFocus(i, delta));
            }
        }
    }
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        act.push(Action::Select(i));
    }
}

/// Une ligne « au fil de l'eau ».
fn flow_row(ui: &mut Ui, data: &Data, i: usize, selected: bool, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let p = &data.projects[i];
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 58.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &p.name));
    let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
    let painter = ui.painter().clone();
    let inner = rect.shrink2(vec2(6.0, 3.0));
    if selected {
        painter.rect(inner, CornerRadius::same(8), tint(t.accent, 0.12), Stroke::new(1.0, tint(t.accent, 0.6)), StrokeKind::Inside);
        painter.rect_filled(Rect::from_min_size(inner.min, vec2(4.0, inner.height())), CornerRadius { nw: 8, sw: 8, ne: 0, se: 0 }, t.accent);
    } else {
        painter.rect_filled(inner, CornerRadius::same(8), tint(t.ink, 0.04 * hover));
    }
    let cy = rect.center().y;
    let x0 = rect.left() + 22.0;
    icons::paint(&painter, pos2(x0 + 12.0, cy), p.icon, 22.0, if selected { t.accent_ink } else { t.muted });
    // Adaptation à la largeur : on retire d'abord ce qui est redondant (la pastille « Au fil de
    // l'eau », toute la section l'est), puis l'état, puis la date ; la prochaine action se coupe.
    let row_w = rect.width();
    let name_max = if row_w < 760.0 { 150.0 } else { 190.0 };
    let name = widgets::paint_text(&painter, pos2(x0 + 42.0, cy - 11.0), &p.name, FontId::new(16.5, theme::bold()), t.ink_strong, name_max);
    let mut x = x0 + 42.0 + name.size().x + 16.0;
    if row_w >= 900.0 {
        x = x.max(x0 + 230.0);
    }
    if row_w >= 640.0 {
        x += widgets::paint_chip(&painter, &t, pos2(x, cy), p.status.label(), widgets::status_tone(p.status)) + 6.0;
    }
    if row_w >= 980.0 {
        x += widgets::paint_chip(&painter, &t, pos2(x, cy), p.attention.label(), if p.attention == Attention::Focus { Tone::Accent } else { Tone::Neutral }) + 6.0;
    }
    x += 12.0;
    painter.vline(x, (cy - 12.0)..=(cy + 12.0), t.hairline());
    x += 18.0;
    // À droite : dernière activité et bouton Détails.
    let button = Rect::from_min_size(pos2(rect.right() - 118.0, cy - 16.0), vec2(100.0, 32.0));
    let mut idle_x = button.left();
    if row_w >= 720.0 {
        let idle = painter.layout_no_wrap(idle_text(p.idle_days), FontId::proportional(12.5), t.faint);
        idle_x = button.left() - 16.0 - idle.size().x;
        painter.galley(pos2(idle_x, cy - idle.size().y / 2.0), idle, t.faint);
    }
    let max_w = idle_x - 16.0 - x;
    if max_w > 60.0 {
        let mut job = egui::text::LayoutJob::default();
        let f = |family, color| egui::TextFormat { font_id: FontId::new(13.5, family), color, ..Default::default() };
        if p.next_action.is_empty() {
            // Fiche neuve : pas encore de prochaine action, on montre sa description.
            job.append(&p.description, 0.0, f(egui::FontFamily::Proportional, t.muted));
        } else {
            job.append(tr("Prochaine action : "), 0.0, f(theme::semibold(), t.ink));
            job.append(&p.next_action, 0.0, f(egui::FontFamily::Proportional, t.muted));
        }
        job.wrap.max_width = max_w;
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        job.wrap.overflow_character = Some('…');
        let g = painter.layout_job(job);
        painter.galley(pos2(x, cy - g.size().y / 2.0), g, t.ink);
    }
    let b = ui.interact(button, Id::unique(("flow-details", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let bh = ui.ctx().animate_bool(b.id.with("hover"), b.hovered());
    painter.rect(button, CornerRadius::same(8), tint(t.ink, 0.06 * bh), Stroke::new(1.0, mix(t.line_strong, t.faint, bh)), StrokeKind::Inside);
    icons::paint(&painter, pos2(button.left() + 20.0, cy), Icon::ArrowRight, 15.0, t.ink);
    let g = painter.layout_no_wrap(tr("Détails").to_owned(), FontId::new(13.5, theme::semibold()), t.ink);
    painter.galley(pos2(button.left() + 34.0, cy - g.size().y / 2.0), g, t.ink);
    if b.clicked() || resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        act.push(Action::Select(i));
    }
}

/// La carte « À redécouvrir ». Sur une ligne quand la place le permet ; sinon les boutons passent
/// sur une seconde ligne. Les textes se coupent par « … » au lieu de passer sous les boutons.
fn rediscover_card(ui: &mut Ui, data: &Data, i: usize, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let p = &data.projects[i];
    widgets::card(&t).inner_margin(egui::Margin::symmetric(22, 16)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let one_line = ui.available_width() >= 1000.0;
        let buttons = |ui: &mut Ui, act: &mut Vec<Action>| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button(ui, Some(Icon::RotateCcw), tr("Passer en revue"), Kind::Primary).clicked() {
                    act.push(Action::Go(Screen::Revisit));
                }
                if widgets::button(ui, None, tr("Voir le résumé"), Kind::Secondary).clicked() {
                    act.push(Action::Select(i));
                }
            });
        };
        // Le texte « revue du jour », coupé à `width`.
        let blurb = |ui: &mut Ui, width: f32| {
            ui.allocate_ui(vec2(width.max(60.0), 40.0), |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add(egui::Label::new(RichText::new(tr("Revue choisie aujourd'hui")).size(14.5).color(t.ink)).truncate());
                    ui.add(egui::Label::new(RichText::new(tf("Inactif depuis {0} jours · {1}", &[("0", &p.idle_days), ("1", &p.kind)])).size(12.5).color(t.faint)).truncate());
                });
            });
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            icons::show(ui, p.icon, 26.0, t.muted);
            widgets::title(ui, &p.name, 20.0);
            widgets::status_chip(ui, p.status);
            widgets::attention_chip(ui, p.attention);
            if one_line {
                widgets::vsep(ui, 26.0);
                // Les deux boutons occupent environ 330 points à droite.
                blurb(ui, ui.available_width() - 340.0);
                buttons(ui, act);
            }
        });
        if !one_line {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(38.0);
                blurb(ui, ui.available_width() - 340.0);
                buttons(ui, act);
            });
        }
    });
}

// ---- « Ajouter un projet » ---------------------------------------------------------------------

/// La fenêtre « Ajouter un projet » : un dossier (sélecteur Windows), son analyse en arrière-plan
/// (Ollama ou règles locales), puis la fiche proposée, modifiable avant création.
pub fn add_project(ctx: &egui::Context, data: &Data, a: &mut AddProjectState, act: &mut Vec<Action>) -> bool {
    let t = Tokens::get(ctx);
    let mut open = true;
    let frame = egui::Frame::new().fill(t.surface).stroke(Stroke::new(1.0, t.line_strong)).corner_radius(16).shadow(theme::shadow(&t)).inner_margin(egui::Margin::same(26));
    let modal = egui::Modal::new(Id::unique("add-project")).backdrop_color(theme::scrim(&t)).frame(frame).show(ctx, |ui| {
        ui.set_width(640.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                widgets::title(ui, tr("Ajouter un projet"), 24.0);
                let how = if data.config.ai_mode == crate::model::AiMode::Ollama && !data.config.ollama_model.is_empty() { tf("Ollama ({0})", &[("0", &data.config.ollama_model)]) } else { tr("les règles locales").to_owned() };
                widgets::muted(ui, &tf("Choisis un dossier : il est analysé par {how}, puis tu valides la fiche.", &[("how", &how)]));
            });
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if widgets::icon_button(ui, Icon::X, tr("Fermer"), 34.0).clicked() {
                    open = false;
                }
            });
        });
        ui.add_space(14.0);
        let w = ui.available_width();
        let path_resp = widgets::labeled(ui, tr("Dossier du projet"), |ui| widgets::path_input(ui, &mut a.path, w));
        if path_resp.changed() {
            // Un autre dossier : l'analyse précédente ne vaut plus.
            a.proposal = None;
            a.error = None;
        }
        ui.add_space(8.0);
        if a.analyzing {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(16.0).color(t.accent));
                ui.label(RichText::new(tr("Analyse du dossier…")).color(t.muted));
            });
        } else if a.proposal.is_none() {
            ui.horizontal(|ui| {
                let ready = std::path::Path::new(a.path.trim()).is_dir();
                if ui.add_enabled_ui(ready, |ui| widgets::button(ui, Some(Icon::ScanSearch), tr("Analyser le dossier"), Kind::Primary)).inner.clicked() {
                    a.analyzing = true;
                    a.error = None;
                    act.push(Action::AnalyzeFolder(a.path.trim().to_owned()));
                }
                if !ready && !a.path.trim().is_empty() {
                    ui.label(RichText::new(tr("Ce dossier n'existe pas.")).size(13.0).color(t.faint));
                }
            });
        }
        if let Some(e) = &a.error {
            ui.horizontal(|ui| {
                icons::show(ui, Icon::TriangleAlert, 16.0, t.danger);
                ui.label(RichText::new(e).size(13.0).color(t.danger));
            });
        }
        if let Some(p) = &mut a.proposal {
            widgets::card(&t).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.horizontal(|ui| {
                    widgets::chip(ui, a.by.label(), Tone::Neutral);
                    for s in &p.stack {
                        widgets::tag(ui, s);
                    }
                });
                let iw = ui.available_width();
                widgets::labeled(ui, tr("Nom"), |ui| widgets::text_input(ui, &mut p.name, tr("Nom du projet"), iw));
                widgets::labeled(ui, tr("Description (une phrase)"), |ui| widgets::text_input(ui, &mut p.description, tr("Ce qu'est le projet, en une phrase"), iw));
                ui.horizontal(|ui| {
                    let half = (iw - 12.0) / 2.0;
                    ui.vertical(|ui| {
                        ui.set_width(half);
                        widgets::labeled(ui, tr("Domaine"), |ui| widgets::text_input(ui, &mut p.domain, tr("ex. Formation"), half - 4.0));
                    });
                    ui.vertical(|ui| {
                        ui.set_width(half);
                        widgets::labeled(ui, tr("Type"), |ui| widgets::text_input(ui, &mut p.kind, tr("ex. Application web"), half - 4.0));
                    });
                });
                widgets::labeled(ui, tr("Attention"), |ui| {
                    let options: Vec<(Attention, Option<Icon>, &str)> = Attention::ALL.iter().map(|x| (*x, Some(x.icon()), x.label())).collect();
                    widgets::segmented(ui, "add-attention", &mut a.attention, &options, Some(iw));
                });
            });
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            icons::show(ui, Icon::Lightbulb, 15.0, t.faint);
            ui.label(RichText::new(tr("Les projets rangés dans tes dossiers sources sont découverts tout seuls à chaque scan.")).size(12.5).color(t.faint));
        });
        ui.add_space(6.0);
        widgets::separator(ui);
        let ready = a.proposal.as_ref().is_some_and(|p| !p.name.trim().is_empty());
        let (cancel, ok) = footer_buttons(ui, tr("Ajouter le projet"), ready);
        if cancel {
            open = false;
        }
        if ok && let Some(p) = a.proposal.clone() {
            act.push(Action::AddProposal(p, a.by, a.attention));
            open = false;
        }
    });
    open && !modal.should_close()
}
