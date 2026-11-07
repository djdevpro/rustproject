//! « À revisiter » : la revue des projets en réserve. Un projet à la fois, relu avec l'aide de
//! l'IA, puis une décision : remettre en focus, au fil de l'eau, garder, archiver ou reporter.

use egui::{Align, FontId, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui, vec2};

use crate::app::{Action, CfgSection, Decision, UiState, idle_text};
use crate::icons::{self, Icon};
use crate::model::{Attention, Data};
use crate::screens::desk::rediscover_queue;
use crate::theme::{self, Tokens, tint};
use crate::widgets::{self, Kind, Tone};
use crate::i18n::{tf, tr};

pub fn show(ui: &mut Ui, data: &Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let queue = rediscover_queue(data, st);
    let current = st.revisit.filter(|c| queue.contains(c)).or_else(|| queue.first().copied());
    let per_day = data.config.reviews_per_day;
    let done_today = st.decisions.len();

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            widgets::title(ui, tr("À revisiter"), 30.0);
            widgets::muted(ui, tr("Un projet en réserve à la fois : on le relit, on décide de son sort. Rien ne dort sans qu'on l'ait choisi."));
        });
        ui.add_space((ui.available_width() - 230.0).max(0.0));
        ui.scope(|ui| {
            ui.vertical(|ui| {
                ui.set_width(220.0);
                ui.label(RichText::new(tf("Revue du jour · {0}/{1}", &[("0", &done_today.min(per_day)), ("1", &per_day)])).font(FontId::new(14.0, theme::semibold())).color(t.ink));
                widgets::progress(ui, done_today as f32 / per_day.max(1) as f32, t.accent, 220.0, 6.0);
            });
        });
    });
    ui.add_space(16.0);

    let side_w = 330.0;
    ui.horizontal_top(|ui| {
        let main_w = ui.available_width() - side_w - 16.0;
        ui.vertical(|ui| {
            ui.set_width(main_w);
            match current {
                Some(p) => review_card(ui, data, p, act),
                None => {
                    widgets::card(&t).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            icons::show(ui, Icon::CircleCheck, 44.0, t.good_fg);
                            widgets::title(ui, tr("Rien à revisiter"), 22.0);
                            widgets::muted(ui, tr("Toute la réserve a été passée en revue. Elle se remplira à nouveau toute seule."));
                        });
                        ui.add_space(40.0);
                    });
                }
            }
        });
        ui.vertical(|ui| {
            ui.set_width(side_w);
            side(ui, data, st, &queue, current, act);
        });
    });
}

fn review_card(ui: &mut Ui, data: &Data, p: usize, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let project = &data.projects[p];
    let domain = &data.domains[project.domain];
    widgets::card(&t).inner_margin(Margin::same(26)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        widgets::draggable(ScrollArea::vertical()).id_salt("revisit-card").auto_shrink([false, true]).max_height(ui.available_height() - 4.0).show(ui, |ui| {
            // En-tête.
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                crate::logo::tile(ui, project, domain.color, 60.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    widgets::title(ui, &project.name, 30.0);
                    ui.label(RichText::new(format!("{} · {}", project.kind, domain.name)).size(14.5).color(t.muted));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        widgets::status_chip(ui, project.status);
                        widgets::attention_chip(ui, project.attention);
                        for s in &project.stack {
                            widgets::tag(ui, s);
                        }
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            icons::show(ui, Icon::Hourglass, 18.0, t.accent_ink);
                            ui.label(RichText::new(tf("Inactif depuis {0} jours", &[("0", &project.idle_days)])).font(FontId::new(15.0, theme::semibold())).color(t.accent_ink));
                        });
                        ui.label(RichText::new(tf("Dernière activité {0}", &[("0", &idle_text(project.idle_days))])).size(12.5).color(t.faint));
                    });
                });
            });
            ui.add_space(18.0);
            widgets::separator(ui);
            ui.add_space(18.0);
            // Deux colonnes : la fiche, et ce que l'IA en retient.
            ui.horizontal_top(|ui| {
                let half = (ui.available_width() - 24.0) / 2.0;
                ui.vertical(|ui| {
                    ui.set_width(half);
                    ui.spacing_mut().item_spacing.y = 14.0;
                    widgets::info_row(ui, Icon::Crosshair, tr("Objectif"), |ui| {
                        widgets::muted(ui, &project.objective);
                    });
                    widgets::info_row(ui, Icon::FileText, tr("Dernier point"), |ui| {
                        widgets::muted(ui, &project.last_point);
                    });
                    widgets::info_row(ui, Icon::Zap, tr("Prochaine action"), |ui| {
                        ui.label(RichText::new(&project.next_action).size(14.5).color(t.ink));
                    });
                });
                ui.add_space(24.0);
                Frame::new().fill(tint(domain.color, 0.07)).stroke(Stroke::new(1.0, tint(domain.color, 0.3))).corner_radius(12).inner_margin(Margin::same(16)).show(ui, |ui| ui.vertical(|ui| {
                    ui.set_width(half - 32.0);
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.horizontal(|ui| {
                        icons::show(ui, Icon::Sparkles, 17.0, domain.color);
                        ui.label(RichText::new(tr("Ce que l'IA en retient")).font(FontId::new(15.0, theme::bold())).color(t.ink_strong));
                    });
                    let (done, total) = project.steps_done();
                    let bullets = [
                        tf("Projet {0} en {1}, rangé dans « {2} ».", &[("0", &project.kind.to_lowercase()), ("1", &project.stack.join(" + ")), ("2", &domain.name)]),
                        tf("Dernier commit {0} : « {1} » était en cours.", &[("0", &idle_text(project.idle_days)), ("1", &project.next_action.trim_end_matches('.'))]),
                        tf("{done} étape(s) faite(s) sur {total}. {0}", &[("done", &done), ("total", &total), ("0", &(if done == 0 { tr("Rien n'est entamé : la reprise sera légère.") } else { tr("Le plus dur est derrière.") }))]),
                        if project.idle_days > 180 {
                            tr("Suggestion : l'archiver si l'idée ne te parle plus, sinon le passer au fil de l'eau.").to_owned()
                        } else {
                            tr("Suggestion : le passer au fil de l'eau, une petite session suffit pour relancer.").to_owned()
                        },
                    ];
                    for b in bullets {
                        ui.label(RichText::new(tf("•  {b}", &[("b", &b)])).size(13.5).color(t.ink));
                    }
                }));
            });
            ui.add_space(18.0);
            // Relire vite : les lanceurs du projet en petit.
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Ouvrir pour relire")).size(13.5).color(t.muted));
                for id in data.launchers_of(p) {
                    let l = &data.launchers[id];
                    let (rect, resp) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::click());
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &l.name));
                    ui.painter().rect_filled(rect, egui::CornerRadius::same(8), tint(l.color, if resp.hovered() { 0.28 } else { 0.16 }));
                    icons::paint(ui.painter(), rect.center(), l.icon, 18.0, l.color);
                    if resp.on_hover_text(&l.name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        act.push(Action::Launch(p, id));
                    }
                }
            });
            ui.add_space(20.0);
            widgets::separator(ui);
            ui.add_space(16.0);
            widgets::caps(ui, tr("Ta décision"));
            ui.add_space(8.0);
            let focus_full = data.config.limit_focus && data.count(Attention::Focus) >= data.config.focus_limit;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                let focus = ui.add_enabled_ui(!focus_full, |ui| widgets::button_sized(ui, Some(Icon::Crosshair), tr("Remettre en focus"), Kind::Primary, 42.0, None)).inner;
                let focus = if focus_full { focus.on_disabled_hover_text(tf("Focus complet ({0}/{1}) : libère d'abord une place.", &[("0", &data.count(Attention::Focus)), ("1", &data.config.focus_limit)])) } else { focus };
                if focus.clicked() {
                    act.push(Action::Decide(p, Decision::Focus));
                }
                let kind = if focus_full { Kind::Primary } else { Kind::Secondary };
                if widgets::button_sized(ui, Some(Icon::Waves), tr("Au fil de l'eau"), kind, 42.0, None).clicked() {
                    act.push(Action::Decide(p, Decision::Flow));
                }
                if widgets::button_sized(ui, Some(Icon::Clock), tr("Garder en réserve"), Kind::Secondary, 42.0, None).clicked() {
                    act.push(Action::Decide(p, Decision::Keep));
                }
                if widgets::button_sized(ui, Some(Icon::Archive), tr("Archiver"), Kind::Danger, 42.0, None).clicked() {
                    act.push(Action::Decide(p, Decision::Archive));
                }
                if widgets::button_sized(ui, Some(Icon::ArrowRight), tr("Plus tard"), Kind::Ghost, 42.0, None).clicked() {
                    act.push(Action::Decide(p, Decision::Later));
                }
            });
            if focus_full {
                ui.label(RichText::new(tf("Le focus est plein ({0}/{1}) : « Au fil de l'eau » est proposé à la place.", &[("0", &data.count(Attention::Focus)), ("1", &data.config.focus_limit)])).size(12.5).color(t.faint));
            }
        });
    });
}

fn side(ui: &mut Ui, data: &Data, st: &mut UiState, queue: &[usize], current: Option<usize>, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::card(&t).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("File d'attente")).font(FontId::new(17.0, theme::bold())).color(t.ink_strong));
            widgets::count_pill(ui, &queue.len().to_string());
        });
        widgets::muted(ui, tr("Les plus endormis d'abord."));
        ui.add_space(4.0);
        let max_idle = queue.iter().map(|&i| data.projects[i].idle_days).max().unwrap_or(1).max(1) as f32;
        widgets::draggable(ScrollArea::vertical()).id_salt("revisit-queue").max_height(330.0).auto_shrink([false, true]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for &i in queue {
                let p = &data.projects[i];
                let selected = current == Some(i);
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, &p.name));
                let painter = ui.painter().clone();
                if selected {
                    painter.rect_filled(rect, egui::CornerRadius::same(8), t.accent_soft);
                } else if resp.hovered() {
                    painter.rect_filled(rect, egui::CornerRadius::same(8), tint(t.ink, 0.05));
                }
                crate::logo::paint(ui, p, rect.left_center() + vec2(18.0, 0.0), 17.0, if selected { t.accent_ink } else { t.muted });
                widgets::paint_text(&painter, rect.left_top() + vec2(38.0, 6.0), &p.name, FontId::new(14.0, theme::semibold()), if selected { t.accent_ink } else { t.ink }, rect.width() - 110.0);
                // Barre de sommeil : plus elle est longue, plus le projet dort.
                let bar = egui::Rect::from_min_size(rect.left_top() + vec2(38.0, 30.0), vec2((rect.width() - 110.0) * p.idle_days as f32 / max_idle, 4.0));
                painter.rect_filled(bar, egui::CornerRadius::same(2), tint(t.accent, if selected { 0.9 } else { 0.45 }));
                painter.text(rect.right_center() - vec2(12.0, 0.0), egui::Align2::RIGHT_CENTER, format!("{} j", p.idle_days), FontId::new(12.5, theme::semibold()), t.faint);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    st.revisit = Some(i);
                }
            }
        });
    });
    ui.add_space(12.0);
    widgets::card(&t).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(tr("Décisions récentes")).font(FontId::new(17.0, theme::bold())).color(t.ink_strong));
        if st.decisions.is_empty() {
            widgets::muted(ui, tr("Aucune décision pour l'instant."));
        }
        for (i, d) in st.decisions.iter().rev().take(6) {
            ui.horizontal(|ui| {
                crate::logo::show(ui, &data.projects[*i], 16.0, t.muted);
                ui.label(RichText::new(&data.projects[*i].name).size(14.0).color(t.ink));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let tone = match d {
                        Decision::Focus => Tone::Accent,
                        Decision::Flow => Tone::Good,
                        Decision::Archive => Tone::Danger,
                        _ => Tone::Neutral,
                    };
                    widgets::chip(ui, d.label(), tone);
                });
            });
        }
    });
    ui.add_space(12.0);
    widgets::card(&t).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            icons::show(ui, Icon::Calendar, 17.0, t.muted);
            ui.label(RichText::new(tf("{0} projet(s) par jour", &[("0", &data.config.reviews_per_day)])).font(FontId::new(14.5, theme::semibold())).color(t.ink));
        });
        widgets::muted(ui, tr("Le rythme de la revue et la mise en réserve automatique se règlent dans la configuration."));
        if widgets::link(ui, tr("États & attention"), Some(Icon::ArrowRight), 13.5).clicked() {
            act.push(Action::GoConfig(CfgSection::Attention));
        }
    });
}
