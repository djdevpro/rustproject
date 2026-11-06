//! « Configuration » : huit sections. Les réglages modifient directement `Data::config` (ce qui
//! n'a pas d'effet visible au-delà de l'interface reste simulé, c'est une maquette).

use egui::{Align, Color32, CornerRadius, FontFamily, FontId, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::{Action, CfgSection, UiState, UpdatePhase, VERSION};
use crate::icons::{self, Icon};
use crate::model::{AiMode, Attention, Data, ReadDepth, ScanFreq, Status, VARIABLES};
use crate::ollama::ModelInfo;
use crate::theme::{self, Accent, ThemeChoice, Tokens, tint};
use crate::widgets::{self, Kind, Tone};

pub fn show(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    egui::Panel::left("config-nav").exact_size(264.0).resizable(false).show_separator_line(false).frame(Frame::new().inner_margin(Margin { left: 0, right: 22, top: 0, bottom: 0 })).show(ui, |ui| nav(ui, st));
    widgets::draggable(ScrollArea::vertical()).id_salt("config-scroll").auto_shrink(false).show(ui, |ui| {
        widgets::scroll_content(ui, |ui| {
            // Au plus 900 points, mais jamais plus que la place disponible (fenêtre étroite).
            ui.set_max_width(ui.available_width().min(900.0));
            ui.spacing_mut().item_spacing.y = 14.0;
            match st.cfg {
                CfgSection::Sources => sources(ui, data, st, act),
                CfgSection::Discovery => discovery(ui, data, st, act),
                CfgSection::Ai => ai(ui, data, st, act),
                CfgSection::Ontology => crate::screens::ontology::show(ui, data, st, act),
                CfgSection::Launchers => launchers(ui, data, act),
                CfgSection::Attention => attention(ui, data),
                CfgSection::Appearance => appearance(ui, data, act),
                CfgSection::Data => data_section(ui, data, st, act),
            }
            ui.add_space(20.0);
        });
    });
}

fn nav(ui: &mut Ui, st: &mut UiState) {
    let t = Tokens::get(ui.ctx());
    widgets::title(ui, "Configuration", 28.0);
    ui.add_space(12.0);
    ui.spacing_mut().item_spacing.y = 4.0;
    for section in CfgSection::ALL {
        let selected = st.cfg == section;
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 54.0), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, selected, section.label()));
        let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::same(10), if selected { t.accent_soft } else { tint(t.ink, 0.05 * hover) });
        let color = if selected { t.accent_ink } else { t.muted };
        icons::paint(p, pos2(rect.left() + 24.0, rect.center().y), section.icon(), 19.0, color);
        p.text(pos2(rect.left() + 46.0, rect.center().y - 9.0), egui::Align2::LEFT_CENTER, section.label(), FontId::new(14.5, theme::semibold()), if selected { t.accent_ink } else { t.ink });
        p.text(pos2(rect.left() + 46.0, rect.center().y + 10.0), egui::Align2::LEFT_CENTER, section.hint(), FontId::proportional(12.0), t.faint);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            st.cfg = section;
        }
    }
}

fn header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        widgets::title(ui, title, 26.0);
        widgets::muted(ui, subtitle);
    });
}

/// Une petite tuile de statistique.
fn stat(ui: &mut Ui, icon: Icon, value: &str, label: &str, color: Color32, width: f32) {
    let t = Tokens::get(ui.ctx());
    Frame::new().fill(t.bg).stroke(Stroke::new(1.0, t.line)).corner_radius(10).inner_margin(Margin::same(14)).show(ui, |ui| {
        ui.set_width(width - 30.0);
        ui.horizontal(|ui| {
            widgets::icon_tile(ui, icon, color, 36.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(value).font(FontId::new(20.0, theme::bold())).color(t.ink_strong));
                ui.label(RichText::new(label).size(12.5).color(t.muted));
            });
        });
    });
}

/// Pastille cliquable on/off (marqueurs de projet).
fn toggle_chip(ui: &mut Ui, on: &mut bool, label: &str) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::new(12.5, FontFamily::Monospace), Color32::PLACEHOLDER);
    let (rect, mut resp) = ui.allocate_exact_size(vec2(galley.size().x + 40.0, 32.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let state = *on;
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, state, label));
    let (fill, stroke, fg) = if *on { (t.accent_soft, Stroke::new(1.0, t.accent), t.accent_ink) } else { (Color32::TRANSPARENT, Stroke::new(1.0, t.line_strong), t.faint) };
    ui.painter().rect(rect, CornerRadius::same(8), fill, stroke, StrokeKind::Inside);
    icons::paint(ui.painter(), pos2(rect.left() + 15.0, rect.center().y), if *on { Icon::Check } else { Icon::Plus }, 14.0, fg);
    ui.painter().galley(pos2(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0), galley, fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Avancement du scan et de l'analyse en cours.
fn scan_progress(ui: &mut Ui, st: &UiState) {
    let t = Tokens::get(ui.ctx());
    if st.scan_started.is_some() {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            ui.label(RichText::new(format!("Parcours des dossiers… {} relevés", st.scan_count)).size(13.5).color(t.ink));
        });
    }
    if let Some(a) = st.analysis.as_ref().filter(|a| !a.finished) {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            let who = if a.with_ai { "L'IA classe" } else { "Les règles classent" };
            ui.label(RichText::new(format!("{who} les dossiers… {}/{} · {}", a.done, a.total, a.current)).size(13.5).color(t.ink));
        });
        widgets::progress(ui, a.done as f32 / a.total.max(1) as f32, t.accent, ui.available_width(), 6.0);
        ui.label(RichText::new(format!("{} projets créés, {} dossiers écartés", a.added, a.skipped)).size(12.5).color(t.muted));
    }
}

// ---- Dossiers sources --------------------------------------------------------------------------

fn sources(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    header(ui, "Dossiers sources", "Les dossiers où se trouvent tes projets. Atelier n'y lit que les noms de dossiers, les README et les manifestes, sur 1 à 3 niveaux.");
    widgets::settings_card(ui, "Dossiers surveillés", Some("Autant de dossiers que tu veux ; chacun a sa profondeur (niveaux de sous-dossiers explorés)."), |ui| {
        let n = data.config.roots.len();
        if n == 0 {
            widgets::muted(ui, "Aucun dossier pour l'instant.");
        }
        for (i, root) in data.config.roots.iter_mut().enumerate() {
            Frame::new().fill(t.bg).stroke(Stroke::new(1.0, t.line)).corner_radius(10).inner_margin(Margin::same(12)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    widgets::switch(ui, &mut root.enabled, &format!("Activer {}", root.path));
                    let w = ui.available_width() - 210.0;
                    widgets::path_input(ui, &mut root.path, w);
                    let mut depth = root.depth.clamp(1, 3);
                    if widgets::segmented(ui, &format!("depth-{i}"), &mut depth, &[(1, None, "1"), (2, None, "2"), (3, None, "3")], Some(120.0)) {
                        root.depth = depth;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, Icon::Trash2, "Retirer ce dossier source", 32.0).clicked() {
                            act.push(Action::RemoveRoot(i));
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.add_space(48.0);
                    let exists = std::path::Path::new(root.path.trim()).is_dir();
                    if !exists {
                        widgets::chip(ui, "dossier introuvable", Tone::Danger);
                    } else {
                        widgets::chip(ui, &format!("{} dossiers relevés", root.found), if root.found > 0 { Tone::Good } else { Tone::Neutral });
                    }
                    ui.label(RichText::new(format!("Dernier scan : {} · {} niveau{}", crate::model::ago(root.last_scan), root.depth, if root.depth > 1 { "x" } else { "" })).size(12.5).color(t.faint));
                    if !root.enabled {
                        ui.label(RichText::new("· en pause").size(12.5).color(t.faint));
                    }
                });
            });
        }
        if widgets::dashed_button(ui, Icon::FolderPlus, "Ajouter un dossier source", vec2(ui.available_width(), 44.0)).clicked() {
            act.push(Action::PickRoot);
        }
    });

    // ---- Scan et classement
    // Les briques (frontend, api…) appartiennent à leur projet : ni « non suivies », ni « nouveaux projets ».
    let untracked: Vec<&crate::scan::Found> =
        st.scan.as_ref().map(|s| s.dirs.iter().filter(|f| f.brick_of.is_none() && data.find_by_folder(&f.path).is_none()).collect()).unwrap_or_default();
    let bricks = st.scan.as_ref().map(|s| s.dirs.iter().filter(|f| f.brick_of.is_some()).count()).unwrap_or(0);
    let new_likely = untracked.iter().filter(|f| f.likely_project).count();
    let ollama_ready = data.config.ai_mode == AiMode::Ollama && !data.config.ollama_model.is_empty();
    widgets::settings_card(ui, "Scan", Some("Le scan relève les dossiers ; les projets reconnus deviennent des fiches."), |ui| {
        if let Some(scan) = &st.scan {
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 3.0 * 10.0) / 4.0;
                ui.spacing_mut().item_spacing.x = 10.0;
                stat(ui, Icon::FolderSearch, &scan.dirs.len().to_string(), "dossiers relevés", t.info, w);
                stat(ui, Icon::FolderGit2, &scan.likely().count().to_string(), "projets probables", t.good_fg, w);
                stat(ui, Icon::CircleCheck, &(scan.dirs.len() - untracked.len() - bricks).to_string(), "déjà suivis", t.muted, w);
                stat(ui, Icon::Sparkles, &new_likely.to_string(), "nouveaux probables", t.accent_ink, w);
            });
            if bricks > 0 {
                ui.label(RichText::new(format!("{bricks} briques (frontend, api, db…) rattachées à leur projet parent : elles ne deviennent pas des fiches.")).size(12.5).color(t.faint));
            }
            if scan.truncated {
                ui.label(RichText::new("Scan arrêté à 8 000 dossiers : réduis la profondeur ou ajoute des exclusions.").size(12.5).color(t.danger));
            }
        }
        scan_progress(ui, st);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            ui.add_enabled_ui(st.scan_started.is_none(), |ui| {
                if widgets::button(ui, Some(Icon::RefreshCw), "Scanner maintenant", Kind::Primary).clicked() {
                    act.push(Action::StartScan);
                }
            });
            let busy = st.analysis.as_ref().is_some_and(|a| !a.finished);
            ui.add_enabled_ui(new_likely > 0 && !busy, |ui| {
                if widgets::button(ui, Some(Icon::ListPlus), &format!("Créer les {new_likely} projets reconnus"), Kind::Secondary).on_hover_text("Par les règles locales : marqueurs, manifestes, README.").clicked() {
                    act.push(Action::AddAllLikely);
                }
            });
            ui.add_enabled_ui(ollama_ready && !untracked.is_empty() && !busy, |ui| {
                let label = if ollama_ready { format!("Classer avec {}", data.config.ollama_model) } else { "Classer avec l'IA".to_owned() };
                let resp = widgets::button(ui, Some(Icon::Sparkles), &label, Kind::Secondary);
                if resp.clicked() {
                    act.push(Action::AnalyzeCandidates);
                }
                if !ollama_ready {
                    resp.on_disabled_hover_text("Choisis un modèle Ollama dans « IA & modèles ».");
                }
            });
        });
        let auto_help = match data.config.ai_mode {
            AiMode::Ollama if ollama_ready => "Après chaque scan, l'IA classe les nouveaux dossiers et crée les fiches.",
            AiMode::Mcp => "En mode agent MCP, c'est l'agent qui crée les fiches : rien d'automatique ici.",
            _ => "Après chaque scan, les dossiers avec un marqueur (.git, manifeste…) deviennent des fiches.",
        };
        widgets::toggle_row(ui, &mut data.config.auto_add, "Créer les projets automatiquement", Some(auto_help));
    });

    if st.scan.is_some() {
        widgets::settings_card(ui, "Dossiers non suivis", Some("Les projets probables d'abord. Ajoute ceux que les règles ou l'IA ont laissés de côté."), |ui| {
            if untracked.is_empty() {
                widgets::muted(ui, "Tous les dossiers relevés sont suivis.");
            }
            let mut rows: Vec<&&crate::scan::Found> = untracked.iter().collect();
            rows.sort_by_key(|f| (!f.likely_project, f.depth, f.name.to_lowercase()));
            for f in rows.iter().take(60) {
                ui.horizontal(|ui| {
                    icons::show(ui, if f.likely_project { Icon::FolderGit2 } else { Icon::Folder }, 18.0, if f.likely_project { t.good_fg } else { t.faint });
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - 150.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&f.name).font(FontId::new(14.0, theme::semibold())).color(t.ink));
                            widgets::chip_sized(ui, &format!("niveau {}", f.depth), Tone::Neutral, 11.0, 20.0);
                            for m in f.markers.iter().take(3) {
                                widgets::tag(ui, m);
                            }
                            if crate::scan::copy_like(&f.name) {
                                widgets::chip_sized(ui, "copie ?", Tone::Accent, 11.0, 20.0);
                            }
                        });
                        let font = FontId::new(11.5, FontFamily::Monospace);
                        let shown = widgets::elide_middle(ui.painter(), &f.path, &font, ui.available_width());
                        ui.label(RichText::new(shown).font(font).color(t.faint)).on_hover_text(&f.path);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, Icon::EyeOff, &format!("Écarter {} des scans", f.name), 30.0).clicked() {
                            act.push(Action::IgnoreFolder(f.path.clone()));
                        }
                        if widgets::button_sized(ui, Some(Icon::Plus), "Ajouter", Kind::Secondary, 30.0, None).clicked() {
                            act.push(Action::AddCandidate(f.path.clone()));
                        }
                    });
                });
            }
            if rows.len() > 60 {
                ui.label(RichText::new(format!("… et {} autres dossiers.", rows.len() - 60)).size(12.5).color(t.faint));
            }
        });
    }

    // ---- GitHub
    widgets::settings_card(ui, "GitHub", Some("Tes dépôts (les tiens, ceux de tes organisations, tes collaborations), via la CLI gh."), |ui| {
        widgets::toggle_row(ui, &mut data.config.github_enabled, "Proposer mes dépôts GitHub", Some("Un dépôt sans dossier local devient une fiche « GitHub »."));
        if !data.config.github_enabled {
            return;
        }
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!st.github_loading, |ui| {
                if widgets::button(ui, Some(Icon::GitBranch), "Lister mes dépôts", Kind::Secondary).clicked() {
                    act.push(Action::ListGithub);
                }
            });
            if st.github_loading {
                ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            }
        });
        match &st.github {
            Some(Err(e)) => {
                ui.label(RichText::new(e).size(13.0).color(t.danger));
            }
            Some(Ok(repos)) => {
                let tracked = repos.iter().filter(|r| crate::github::local_match(data, r).is_some()).count();
                let todo = repos.iter().filter(|r| !r.fork && crate::github::local_match(data, r).is_none()).count();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} dépôts · {tracked} déjà suivis", repos.len())).size(13.5).color(t.ink));
                    ui.add_enabled_ui(todo > 0, |ui| {
                        if widgets::button_sized(ui, Some(Icon::ListPlus), &format!("Ajouter les {todo} non suivis"), Kind::Primary, 32.0, None).clicked() {
                            act.push(Action::AddAllRepos);
                        }
                    });
                });
                for r in repos.iter().take(40) {
                    ui.horizontal(|ui| {
                        icons::show(ui, if r.private { Icon::Lock } else { Icon::GitBranch }, 16.0, t.muted);
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width() - 110.0);
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&r.full_name).font(FontId::new(13.5, theme::semibold())).color(t.ink));
                                if let Some(l) = &r.language {
                                    widgets::tag(ui, l);
                                }
                                if r.archived {
                                    widgets::chip_sized(ui, "archivé", Tone::Neutral, 11.0, 20.0);
                                }
                                if r.fork {
                                    widgets::chip_sized(ui, "fork", Tone::Neutral, 11.0, 20.0);
                                }
                            });
                            if let Some(d) = &r.description {
                                ui.label(RichText::new(d).size(12.5).color(t.muted));
                            }
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if crate::github::local_match(data, r).is_some() {
                                widgets::chip(ui, "suivi", Tone::Good);
                            } else if widgets::button_sized(ui, Some(Icon::Plus), "Ajouter", Kind::Secondary, 30.0, None).clicked() {
                                act.push(Action::AddRepo(r.full_name.clone()));
                            }
                        });
                    });
                }
            }
            None => {}
        }
    });
}

// ---- Découverte --------------------------------------------------------------------------------

fn discovery(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let c = &mut data.config;
    header(ui, "Découverte", "La partie algorithmique : rapide, locale, déterministe. Elle trouve les projets ; l'IA les décrit ensuite.");
    widgets::settings_card(ui, "Qu'est-ce qui fait un projet ?", Some("Un dossier qui contient l'un de ces marqueurs devient un projet."), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for (marker, on) in c.markers.iter_mut() {
                toggle_chip(ui, on, marker);
            }
        });
        ui.label(RichText::new("Le README est un indice faible : il signale un projet sans empêcher de regarder ses sous-dossiers (un dossier de rangement en a souvent un).").size(12.5).color(t.faint));
    });
    if !c.ignored.is_empty() {
        widgets::settings_card(ui, "Dossiers écartés", Some("Écartés à la main ou par l'agent IA (copies, sauvegardes…) : le scan ne les relève plus."), |ui| {
            for (i, path) in c.ignored.iter().enumerate() {
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::EyeOff, 16.0, t.faint);
                    let font = FontId::new(12.0, FontFamily::Monospace);
                    let shown = widgets::elide_middle(ui.painter(), path, &font, ui.available_width() - 120.0);
                    ui.label(RichText::new(shown).font(font).color(t.muted)).on_hover_text(path);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::button_sized(ui, Some(Icon::Undo2), "Rétablir", Kind::Ghost, 28.0, None).clicked() {
                            act.push(Action::RestoreFolder(i));
                        }
                    });
                });
            }
        });
    }
    widgets::settings_card(ui, "Exclusions", Some("Ces dossiers ne sont jamais parcourus, où qu'ils soient. Les dossiers cachés (.git, .cache…) non plus."), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            let mut remove = None;
            for (i, ex) in c.excludes.iter().enumerate() {
                let galley = ui.painter().layout_no_wrap(ex.clone(), FontId::new(12.5, FontFamily::Monospace), t.ink);
                let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 40.0, 30.0), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Retirer {ex}")));
                ui.painter().rect(rect, CornerRadius::same(8), t.raised, Stroke::new(1.0, if resp.hovered() { t.danger } else { t.line_strong }), StrokeKind::Inside);
                ui.painter().galley(pos2(rect.left() + 12.0, rect.center().y - galley.size().y / 2.0), galley, t.ink);
                icons::paint(ui.painter(), pos2(rect.right() - 15.0, rect.center().y), Icon::X, 13.0, if resp.hovered() { t.danger } else { t.faint });
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    remove = Some(i);
                }
            }
            if let Some(i) = remove {
                c.excludes.remove(i);
            }
        });
        ui.horizontal(|ui| {
            let resp = widgets::mono_input(ui, &mut st.new_exclude, "nom de dossier, ex. coverage", 280.0);
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (widgets::button(ui, Some(Icon::Plus), "Ajouter", Kind::Secondary).clicked() || enter) && !st.new_exclude.trim().is_empty() {
                let ex = st.new_exclude.trim().to_lowercase();
                if !c.excludes.contains(&ex) {
                    c.excludes.push(ex);
                }
                st.new_exclude.clear();
            }
        });
    });
    widgets::settings_card(ui, "Règles", None, |ui| {
        widgets::toggle_row(ui, &mut c.skip_nested, "Ne pas chercher de projets dans un projet", Some("Un dossier reconnu n'est pas fouillé plus loin (sous-modules, exemples…)."));
        widgets::separator(ui);
        widgets::toggle_row(ui, &mut c.group_monorepos, "Rattacher les briques à leur projet", Some("frontend, backend, api, db, site, packages/x… deviennent des dossiers du projet parent, pas des fiches à part."));
        widgets::separator(ui);
        widgets::toggle_row(ui, &mut c.ignore_older, "Ignorer les dossiers très anciens", Some("Sans aucune modification depuis la durée ci-dessous."));
        ui.add_enabled_ui(c.ignore_older, |ui| {
            ui.horizontal(|ui| {
                widgets::named(ui.add(egui::Slider::new(&mut c.ignore_older_days, 90..=1825).show_value(false).trailing_fill(true)), egui::Role::Slider, "Ancienneté maximale");
                ui.label(RichText::new(format!("{} jours (≈ {:.1} ans)", c.ignore_older_days, c.ignore_older_days as f32 / 365.0)).size(13.0).color(t.muted));
            });
        });
    });
    widgets::settings_card(ui, "Quand scanner ?", None, |ui| {
        let options = [(ScanFreq::Startup, Some(Icon::Power), "Au démarrage"), (ScanFreq::Hourly, Some(Icon::Clock), "Toutes les heures"), (ScanFreq::Manual, Some(Icon::Hand), "À la demande")];
        widgets::segmented(ui, "scan-freq", &mut c.freq, &options, Some(ui.available_width().min(560.0)));
        scan_progress(ui, st);
        ui.add_enabled_ui(st.scan_started.is_none(), |ui| {
            if widgets::button(ui, Some(Icon::RefreshCw), "Scanner maintenant", Kind::Secondary).clicked() {
                act.push(Action::StartScan);
            }
        });
    });
    widgets::settings_card(ui, "Ce que l'algorithme relève", Some("Lu sur le disque, sans rien envoyer nulle part."), |ui| {
        let items = [
            (Icon::GitBranch, "Git", "Dernier commit, branche, dépôt distant"),
            (Icon::FileJson, "Manifestes", "Cargo.toml, package.json, pyproject.toml…"),
            (Icon::Code, "Langages", "Répartition des extensions de fichiers"),
            (Icon::Globe, "Port de dev", "Scripts et fichiers .env (pour {port})"),
            (Icon::Activity, "Activité", "Date de dernière modification, taille"),
            (Icon::FileText, "Documentation", "README, CLAUDE.md, notes"),
        ];
        for row in items.chunks(2) {
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 10.0) / 2.0;
                for (icon, title, desc) in row {
                    ui.horizontal(|ui| {
                        ui.set_width(w);
                        icons::show(ui, *icon, 18.0, t.muted);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(RichText::new(*title).font(FontId::new(14.0, theme::semibold())).color(t.ink));
                            ui.label(RichText::new(*desc).size(12.5).color(t.muted));
                        });
                    });
                }
            });
        }
    });
}

// ---- IA & modèles -------------------------------------------------------------------------------

fn ai(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    header(ui, "IA & modèles", "L'algorithme trouve les dossiers ; l'IA décide lesquels sont des projets et rédige leur fiche en une phrase.");
    widgets::settings_card(ui, "Mode", None, |ui| {
        let options = [(AiMode::Ollama, Some(Icon::Cpu), "Ollama (API)"), (AiMode::Mcp, Some(Icon::Network), "Agent MCP"), (AiMode::Off, Some(Icon::CircleOff), "Sans IA")];
        widgets::segmented(ui, "ai-mode", &mut data.config.ai_mode, &options, Some(ui.available_width().min(560.0)));
        let explain = match data.config.ai_mode {
            AiMode::Ollama => "Atelier envoie à un modèle local (Ollama) un résumé de chaque dossier : nom, marqueurs, début du README. Rien ne sort de ta machine.",
            AiMode::Mcp => "Un agent IA (Claude Code…) se connecte à Atelier par le protocole MCP : il scanne, décide et crée les fiches lui-même.",
            AiMode::Off => "Pas d'IA : les dossiers avec un marqueur (.git, Cargo.toml, package.json…) deviennent des projets, décrits par leur manifeste ou leur README.",
        };
        ui.label(RichText::new(explain).size(13.0).color(t.muted));
    });
    match data.config.ai_mode {
        AiMode::Ollama => ollama_card(ui, data, st, act),
        AiMode::Mcp => mcp_card(ui, data, st, act),
        AiMode::Off => {}
    }
    if data.config.ai_mode != AiMode::Off {
        semantic_card(ui, data, st, act);
    }
    if data.config.ai_mode == AiMode::Ollama {
        widgets::settings_card(ui, "Ce que l'IA lit", Some("Ce qui est envoyé au modèle pour chaque dossier."), |ui| {
            let options = [(ReadDepth::Readme, Some(Icon::FileText), "README"), (ReadDepth::Manifests, Some(Icon::FileJson), "+ manifestes"), (ReadDepth::Tree, Some(Icon::FolderTree), "+ sous-dossiers")];
            widgets::segmented(ui, "read-depth", &mut data.config.read_depth, &options, Some(ui.available_width().min(560.0)));
            let explain = match data.config.read_depth {
                ReadDepth::Readme => "Le nom, les marqueurs et le début du README.",
                ReadDepth::Manifests => "Recommandé : en plus, le nom, la description et les technos des manifestes.",
                ReadDepth::Tree => "En plus, la liste des sous-dossiers et le nombre de fichiers (jamais leur contenu).",
            };
            ui.label(RichText::new(explain).size(13.0).color(t.muted));
            ui.horizontal(|ui| {
                icons::show(ui, Icon::Lock, 16.0, t.good_fg);
                ui.label(RichText::new("Le code source et les fichiers .env ne sont jamais envoyés.").size(13.0).color(t.muted));
            });
        });
    }
    ollaya_card(ui, data, st, act);
}

/// Ollaya : le serveur des modèles de décision, qui classent les projets dans l'ontologie.
fn ollaya_card(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::settings_card(ui, "Ollaya (classement)", Some("Des modèles de décision locaux : ils rangent chaque projet dans les concepts de l'ontologie, avec une probabilité. Rien ne sort de ta machine."), |ui| {
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 250.0).max(160.0);
            widgets::labeled(ui, "Adresse du serveur", |ui| widgets::mono_input(ui, &mut data.config.ollaya_url, "http://127.0.0.1:11435", w));
            ui.vertical(|ui| {
                ui.add_space(22.0);
                ui.add_enabled_ui(!st.ollaya_loading, |ui| {
                    if widgets::button(ui, Some(Icon::Plug), "Tester et lister", Kind::Secondary).on_hover_text("Les modèles installés dans Ollaya").clicked() {
                        act.push(Action::ListOllaya);
                    }
                });
            });
        });
        match &st.ollaya_models {
            None if st.ollaya_loading => {
                ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            }
            None => {
                widgets::muted(ui, "Clique « Tester et lister » pour voir les modèles installés dans Ollaya.");
            }
            Some(Err(e)) => {
                ui.horizontal(|ui| {
                    widgets::chip(ui, "injoignable", Tone::Danger);
                    ui.label(RichText::new(e).size(12.5).color(t.danger));
                });
                ui.label(RichText::new("Lance l'application Ollaya (ou `ollaya serve`), puis réessaie.").size(12.5).color(t.faint));
            }
            Some(Ok(models)) => {
                ui.horizontal(|ui| {
                    widgets::chip(ui, &format!("connecté · {} modèle{}", models.len(), if models.len() > 1 { "s" } else { "" }), Tone::Good);
                    if models.is_empty() {
                        ui.label(RichText::new("Aucun modèle : télécharges-en un ci-dessous (decider:2b conseillé).").size(12.5).color(t.muted));
                    }
                });
                let all: Vec<&ModelInfo> = models.iter().collect();
                let w = ui.available_width();
                widgets::labeled(ui, "Modèle de décision (classement dans l'ontologie)", |ui| model_combo(ui, "Modèle de décision", &mut data.config.ollaya_model, &all, w.min(420.0)));
            }
        }
        ui.add_space(4.0);
        widgets::separator(ui);
        widgets::field_label(ui, "Télécharger un modèle de décision");
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 150.0).max(160.0);
            widgets::mono_input(ui, &mut st.ollaya_pull_input, "ex. decider:2b", w);
            ui.add_enabled_ui(st.ollaya_pull.is_none() && !st.ollaya_pull_input.trim().is_empty(), |ui| {
                if widgets::button(ui, Some(Icon::Download), "Télécharger", Kind::Primary).clicked() {
                    act.push(Action::PullOllaya(st.ollaya_pull_input.clone()));
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            ui.label(RichText::new("Conseillés :").size(12.5).color(t.faint));
            for (name, help) in crate::ollaya::SUGGESTED {
                if widgets::tag(ui, name).interact(Sense::click()).on_hover_text(help).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    st.ollaya_pull_input = name.to_owned();
                }
            }
        });
        if let Some((model, status, fraction)) = &st.ollaya_pull {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(16.0).color(t.accent));
                let pct = if *fraction >= 0.0 { format!(" · {:.0} %", fraction * 100.0) } else { String::new() };
                ui.label(RichText::new(format!("{model} : {status}{pct}")).size(13.0).color(t.ink));
            });
            if *fraction >= 0.0 {
                widgets::progress(ui, *fraction, t.accent, ui.available_width(), 6.0);
            }
        }
        if widgets::link(ui, "Régler l'ontologie et le classement", Some(Icon::ArrowRight), 13.5).clicked() {
            act.push(Action::GoConfig(CfgSection::Ontology));
        }
    });
}

/// Choisir un modèle parmi une liste (`""` = aucun). Renvoie `true` au changement.
fn model_combo(ui: &mut Ui, name: &str, current: &mut String, models: &[&ModelInfo], width: f32) -> bool {
    let labels: Vec<String> = models.iter().map(|m| format!("{}  ·  {}", m.name, m.size_label())).collect();
    let mut sel = models.iter().position(|m| m.name == *current).unwrap_or(usize::MAX);
    let mut options: Vec<(usize, &str)> = vec![(usize::MAX, "— aucun —")];
    options.extend(labels.iter().enumerate().map(|(i, l)| (i, l.as_str())));
    let changed = widgets::combo(ui, name, &mut sel, &options, width);
    if changed {
        *current = models.get(sel).map(|m| m.name.clone()).unwrap_or_default();
    }
    changed
}

fn ollama_card(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::settings_card(ui, "Ollama", Some("Le serveur local et les modèles utilisés."), |ui| {
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 250.0).max(160.0);
            widgets::labeled(ui, "Adresse du serveur", |ui| widgets::mono_input(ui, &mut data.config.ollama_url, "http://localhost:11434", w));
            ui.vertical(|ui| {
                ui.add_space(22.0);
                ui.add_enabled_ui(!st.models_loading, |ui| {
                    if widgets::button(ui, Some(Icon::Plug), "Tester et lister", Kind::Secondary).clicked() {
                        act.push(Action::ListModels);
                    }
                });
            });
        });
        match &st.models {
            None if st.models_loading => {
                ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            }
            None => {
                widgets::muted(ui, "Clique « Tester et lister » pour voir les modèles installés (l'équivalent de ollama list).");
            }
            Some(Err(e)) => {
                ui.horizontal(|ui| {
                    widgets::chip(ui, "injoignable", Tone::Danger);
                    ui.label(RichText::new(e).size(12.5).color(t.danger));
                });
            }
            Some(Ok(models)) => {
                let chat: Vec<&ModelInfo> = models.iter().filter(|m| !m.is_embedding()).collect();
                let embed: Vec<&ModelInfo> = models.iter().filter(|m| m.is_embedding()).collect();
                ui.horizontal(|ui| {
                    widgets::chip(ui, &format!("connecté · {} modèle{}", models.len(), if models.len() > 1 { "s" } else { "" }), Tone::Good);
                    if models.is_empty() {
                        ui.label(RichText::new("Aucun modèle installé : télécharges-en un ci-dessous.").size(12.5).color(t.muted));
                    }
                });
                let w = (ui.available_width() - 16.0) / 2.0;
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        widgets::labeled(ui, "Modèle de langage (classement)", |ui| model_combo(ui, "Modèle de langage", &mut data.config.ollama_model, &chat, w - 8.0));
                    });
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        widgets::labeled(ui, "Modèle d'embeddings (recherche)", |ui| model_combo(ui, "Modèle d'embeddings", &mut data.config.ollama_embed_model, &embed, w - 8.0));
                    });
                });
            }
        }
        ui.add_space(4.0);
        widgets::separator(ui);
        widgets::field_label(ui, "Télécharger un modèle");
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 150.0).max(160.0);
            widgets::mono_input(ui, &mut st.pull_input, "ex. qwen2.5:3b ou bge-m3", w);
            ui.add_enabled_ui(st.pull.is_none() && !st.pull_input.trim().is_empty(), |ui| {
                if widgets::button(ui, Some(Icon::Download), "Télécharger", Kind::Primary).clicked() {
                    act.push(Action::PullModel(st.pull_input.clone()));
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            ui.label(RichText::new("Conseillés :").size(12.5).color(t.faint));
            for (name, help) in crate::ollama::SUGGESTED_CHAT.iter().chain(crate::ollama::SUGGESTED_EMBED.iter()) {
                if widgets::tag(ui, name).interact(Sense::click()).on_hover_text(*help).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    st.pull_input = (*name).to_owned();
                }
            }
        });
        if let Some((model, status, fraction)) = &st.pull {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(16.0).color(t.accent));
                let pct = if *fraction >= 0.0 { format!(" · {:.0} %", fraction * 100.0) } else { String::new() };
                ui.label(RichText::new(format!("{model} : {status}{pct}")).size(13.0).color(t.ink));
            });
            if *fraction >= 0.0 {
                widgets::progress(ui, *fraction, t.accent, ui.available_width(), 6.0);
            }
        }
        ui.add_space(4.0);
        widgets::separator(ui);
        scan_progress(ui, st);
        let ready = !data.config.ollama_model.is_empty();
        let busy = st.analysis.as_ref().is_some_and(|a| !a.finished);
        ui.horizontal(|ui| {
            ui.add_enabled_ui(ready && st.scan.is_some() && !busy, |ui| {
                if widgets::button(ui, Some(Icon::Sparkles), "Classer les dossiers non suivis", Kind::Primary).clicked() {
                    act.push(Action::AnalyzeCandidates);
                }
            });
            let hint = if !ready {
                "Choisis d'abord un modèle de langage."
            } else if st.scan.is_none() {
                "Lance d'abord un scan (Dossiers sources)."
            } else {
                "Les projets probables, et les dossiers de premier niveau sans marqueur."
            };
            ui.label(RichText::new(hint).size(12.5).color(t.faint));
        });
        if let Some(a) = st.analysis.as_ref().filter(|a| a.finished && !a.warnings.is_empty()) {
            ui.label(RichText::new(format!("{} réponses illisibles : fiches faites par les règles (ex. {}).", a.warnings.len(), a.warnings[0])).size(12.5).color(t.faint));
        }
    });
}

fn semantic_card(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::settings_card(ui, "Recherche sémantique", Some("Chercher par le sens : « logement » trouve ImmoFormation. Les fiches sont transformées en vecteurs par Ollama."), |ui| {
        let model = data.config.ollama_embed_model.clone();
        if model.is_empty() {
            ui.horizontal(|ui| {
                widgets::chip(ui, "inactive", Tone::Neutral);
                ui.label(RichText::new("Choisis un modèle d'embeddings (qwen3-embedding:4b conseillé pour le français).").size(13.0).color(t.muted));
            });
            if data.config.ai_mode == AiMode::Mcp {
                ui.label(RichText::new(format!("Elle passe par le serveur Ollama ({}), même en mode agent MCP.", data.config.ollama_url)).size(12.5).color(t.faint));
            }
            return;
        }
        let total = data.projects.len();
        ui.horizontal(|ui| {
            if st.semantic_busy {
                ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            }
            let tone = if st.semantic_indexed == total && total > 0 { Tone::Good } else { Tone::Neutral };
            widgets::chip(ui, &format!("{}/{total} fiches indexées", st.semantic_indexed), tone);
            widgets::tag(ui, &model);
        });
        if let Some(e) = &st.semantic_error {
            ui.label(RichText::new(e).size(12.5).color(t.danger));
        }
        ui.horizontal(|ui| {
            if widgets::button(ui, Some(Icon::RefreshCw), "Réindexer", Kind::Secondary).clicked() {
                act.push(Action::Reindex);
            }
            ui.label(RichText::new("L'index se met à jour tout seul quand une fiche change.").size(12.5).color(t.faint));
        });
    });
}

fn mcp_card(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "atelier.exe".into());
    widgets::settings_card(ui, "Agent MCP", Some("Atelier est aussi un serveur MCP : `atelier --mcp`. L'agent lit et écrit le même fichier que cette fenêtre, qui se met à jour toute seule."), |ui| {
        let code = |ui: &mut Ui, label: &str, text: String, act: &mut Vec<Action>| {
            widgets::field_label(ui, label);
            ui.horizontal_top(|ui| {
                let width = (ui.available_width() - 48.0).max(120.0);
                Frame::new().fill(t.input).corner_radius(8).inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.set_width(width - 20.0);
                    code_text(ui, &text, t.ink);
                });
                if widgets::icon_button(ui, Icon::Copy, &format!("Copier : {label}"), 34.0).clicked() {
                    act.push(Action::CopyText(text.clone()));
                }
            });
        };
        code(ui, "Dans Claude Code", format!(r#"claude mcp add atelier -- "{exe}" --mcp"#), act);
        let json = format!(r#"{{ "mcpServers": {{ "atelier": {{ "command": "{}", "args": ["--mcp"] }} }} }}"#, exe.replace('\\', "\\\\"));
        code(ui, "Autres clients (configuration JSON)", json, act);
        widgets::field_label(ui, "Outils proposés à l'agent");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for tool in crate::mcp::tool_names() {
                widgets::tag(ui, &tool);
            }
        });
        ui.horizontal(|ui| {
            icons::show(ui, Icon::Lightbulb, 15.0, t.faint);
            ui.label(RichText::new("Puis demande à l'agent : « Range mes projets avec Atelier ».").size(12.5).color(t.muted));
        });
        if let Some(path) = &st.store_path {
            ui.label(RichText::new(format!("Fichier partagé : {path}")).size(12.0).color(t.faint));
        }
    });
    let _ = data;
}

/// Du code (commande, JSON, chemin) qui revient à la ligne n'importe où : un chemin sans espace
/// ne doit jamais élargir la colonne (sinon tout ce qui est à droite sort de la fenêtre).
fn code_text(ui: &mut Ui, text: &str, color: Color32) {
    let mut job = egui::text::LayoutJob::single_section(text.to_owned(), egui::TextFormat::simple(FontId::new(12.0, FontFamily::Monospace), color));
    job.wrap.break_anywhere = true;
    ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Wrap));
}

// ---- Lanceurs ----------------------------------------------------------------------------------

fn launchers(ui: &mut Ui, data: &mut Data, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    header(ui, "Lanceurs", "Les boutons « Ouvrir avec » du panneau projet. Ceux par défaut apparaissent partout ; chaque projet peut en ajouter ou en masquer.");
    widgets::settings_card(ui, "Lanceurs par défaut", Some("Dans cet ordre, sur tous les projets. Le premier est mis en avant, sauf choix contraire du projet."), |ui| {
        let n = data.default_launchers.len();
        for (pos, &id) in data.default_launchers.iter().enumerate() {
            let l = &data.launchers[id];
            Frame::new().fill(t.bg).stroke(Stroke::new(1.0, if pos == 0 { tint(t.accent, 0.6) } else { t.line })).corner_radius(10).inner_margin(Margin::symmetric(12, 10)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::GripVertical, 16.0, t.faint);
                    widgets::icon_tile(ui, l.icon, l.color, 36.0);
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - 300.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&l.name).font(FontId::new(15.0, theme::semibold())).color(t.ink_strong));
                            if pos == 0 {
                                widgets::chip_sized(ui, "favori", Tone::Accent, 11.5, 20.0);
                            }
                        });
                        let mut job = egui::text::LayoutJob::simple_singleline(l.command.clone(), FontId::new(12.0, FontFamily::Monospace), t.muted);
                        job.wrap.max_width = ui.available_width();
                        job.wrap.max_rows = 1;
                        job.wrap.overflow_character = Some('…');
                        ui.add(egui::Label::new(job).selectable(false));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if widgets::icon_button(ui, Icon::X, &format!("Retirer {} des lanceurs par défaut", l.name), 32.0).clicked() {
                            act.push(Action::RemoveDefault(id));
                        }
                        if widgets::icon_button(ui, Icon::Pencil, &format!("Modifier {}", l.name), 32.0).clicked() {
                            act.push(Action::OpenEditor(Some(id)));
                        }
                        ui.add_enabled_ui(pos + 1 < n, |ui| {
                            if widgets::icon_button(ui, Icon::ArrowDown, "Descendre", 32.0).clicked() {
                                act.push(Action::MoveDefault(id, 1));
                            }
                        });
                        ui.add_enabled_ui(pos > 0, |ui| {
                            if widgets::icon_button(ui, Icon::ArrowUp, "Monter", 32.0).clicked() {
                                act.push(Action::MoveDefault(id, -1));
                            }
                        });
                        ui.add_space(6.0);
                        if !l.shortcut.is_empty() {
                            widgets::tag(ui, &l.shortcut);
                        }
                        widgets::chip(ui, l.kind.label(), Tone::Neutral);
                    });
                });
            });
        }
        ui.horizontal(|ui| {
            if widgets::button(ui, Some(Icon::LayoutGrid), "Ajouter depuis la galerie", Kind::Primary).clicked() {
                act.push(Action::OpenGallery(None));
            }
            if widgets::button(ui, Some(Icon::Plus), "Créer un lanceur", Kind::Secondary).clicked() {
                act.push(Action::OpenEditor(None));
            }
        });
    });
    // Les lanceurs qui ne servent qu'à certains projets.
    let specific: Vec<usize> = (0..data.launchers.len()).filter(|id| !data.default_launchers.contains(id)).collect();
    widgets::settings_card(ui, "Lanceurs propres à des projets", Some("Ajoutés depuis le « + » d'un projet. Les modifier ici met à jour tous les projets qui s'en servent."), |ui| {
        if specific.is_empty() {
            widgets::muted(ui, "Aucun pour l'instant.");
        }
        for id in specific {
            let l = &data.launchers[id];
            let users: Vec<&str> = data.projects.iter().filter(|p| p.launchers.contains(&id)).map(|p| p.name.as_str()).collect();
            ui.horizontal(|ui| {
                widgets::icon_tile(ui, l.icon, l.color, 32.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new(&l.name).font(FontId::new(14.5, theme::semibold())).color(t.ink));
                    let used = if users.is_empty() { "Utilisé par aucun projet".to_owned() } else { format!("Utilisé par {}", users.join(", ")) };
                    ui.label(RichText::new(used).size(12.5).color(t.muted));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::button_sized(ui, Some(Icon::Layers), "Mettre par défaut", Kind::Ghost, 32.0, None).clicked() {
                        act.push(Action::AddLauncher { launcher: l.clone(), project: None });
                    }
                    if widgets::icon_button(ui, Icon::Pencil, &format!("Modifier {}", l.name), 32.0).clicked() {
                        act.push(Action::OpenEditor(Some(id)));
                    }
                });
            });
        }
    });
    widgets::settings_card(ui, "Variables", Some("Utilisables dans n'importe quelle commande. Elles sont remplacées au clic, pour le projet ouvert."), |ui| {
        for (var, help) in VARIABLES {
            ui.horizontal(|ui| {
                ui.allocate_ui(vec2(110.0, 24.0), |ui| widgets::tag(ui, var));
                ui.label(RichText::new(help).size(13.5).color(t.ink));
            });
        }
        if let Some(&p) = data.focus().first() {
            let project = &data.projects[p];
            let code = &data.launchers[1];
            Frame::new().fill(t.input).corner_radius(8).inner_margin(Margin::same(12)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("{}    →    {}", code.command, code.resolve(project))).font(FontId::new(12.0, FontFamily::Monospace)).color(t.ink));
            });
            ui.label(RichText::new(format!("Exemple avec le projet {}.", project.name)).size(12.5).color(t.faint));
        }
    });
}

// ---- États & attention -------------------------------------------------------------------------

fn attention(ui: &mut Ui, data: &mut Data) {
    let t = Tokens::get(ui.ctx());
    let counts: Vec<usize> = Attention::ALL.iter().map(|a| data.count(*a)).collect();
    let c = &mut data.config;
    header(ui, "États & attention", "Deux axes séparés : l'état dit où en est le projet, l'attention dit combien tu lui en donnes.");
    widgets::settings_card(ui, "Niveaux d'attention", None, |ui| {
        for (k, a) in Attention::ALL.iter().enumerate() {
            ui.horizontal(|ui| {
                widgets::icon_tile(ui, a.icon(), if *a == Attention::Focus { t.accent_ink } else { t.muted }, 36.0);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - 120.0);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    widgets::attention_chip(ui, *a);
                    ui.label(RichText::new(a.description()).size(13.0).color(t.muted));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("{} projets", counts[k])).size(13.5).color(t.faint));
                });
            });
        }
    });
    widgets::settings_card(ui, "Projets du moment", Some("Par défaut, autant de projets en focus que tu veux : la rangée du bureau défile."), |ui| {
        widgets::toggle_row(ui, &mut c.limit_focus, "Limiter le nombre de projets en focus", Some("Au-delà, Atelier te demande quel projet libérer avant d'en ajouter un."));
        ui.add_enabled_ui(c.limit_focus, |ui| {
            ui.horizontal(|ui| {
                widgets::named(ui.add(egui::Slider::new(&mut c.focus_limit, 1..=8).show_value(false).trailing_fill(true)), egui::Role::Slider, "Limite du focus");
                ui.label(RichText::new(format!("{} projets", c.focus_limit)).font(FontId::new(15.0, theme::semibold())).color(t.ink));
            });
        });
        let over = counts[0].saturating_sub(c.focus_limit);
        let (text, color) = if c.limit_focus && over > 0 {
            (format!("{} projet(s) en focus en trop : ils restent visibles jusqu'à ce que tu choisisses.", over), t.danger)
        } else {
            (format!("Actuellement {} en focus.", counts[0]), t.faint)
        };
        ui.label(RichText::new(text).size(13.0).color(color));
    });
    widgets::settings_card(ui, "Mise en réserve automatique", None, |ui| {
        widgets::toggle_row(ui, &mut c.auto_reserve, "Passer en réserve les projets « au fil de l'eau » endormis", Some("Ils ne disparaissent pas : la revue les fera ressortir."));
        ui.add_enabled_ui(c.auto_reserve, |ui| {
            ui.horizontal(|ui| {
                widgets::named(ui.add(egui::Slider::new(&mut c.reserve_after, 14..=180).show_value(false).trailing_fill(true)), egui::Role::Slider, "Délai avant réserve");
                ui.label(RichText::new(format!("après {} jours sans activité", c.reserve_after)).size(13.5).color(t.ink));
            });
        });
    });
    widgets::settings_card(ui, "Rythme de la revue", Some("Combien de projets en réserve ressortent chaque jour dans « À revisiter »."), |ui| {
        widgets::segmented(ui, "reviews-per-day", &mut c.reviews_per_day, &[(1, None, "1 par jour"), (2, None, "2 par jour"), (3, None, "3 par jour")], Some(420.0));
    });
    widgets::settings_card(ui, "États", Some("Le cycle de vie d'un projet."), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for (k, s) in Status::ALL.iter().enumerate() {
                widgets::status_chip(ui, *s);
                if k + 1 < Status::ALL.len() {
                    icons::show(ui, Icon::ChevronRight, 14.0, t.faint);
                }
            }
        });
    });
}

// ---- Apparence ---------------------------------------------------------------------------------

fn appearance(ui: &mut Ui, data: &mut Data, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let c = &mut data.config;
    header(ui, "Apparence", "Le thème suit les maquettes ; l'accent et la densité sont à ton goût.");
    widgets::settings_card(ui, "Thème", None, |ui| {
        let options: Vec<(ThemeChoice, Option<Icon>, &str)> = vec![
            (ThemeChoice::System, Some(Icon::Monitor), ThemeChoice::System.label()),
            (ThemeChoice::Dark, Some(Icon::Moon), ThemeChoice::Dark.label()),
            (ThemeChoice::Light, Some(Icon::Sun), ThemeChoice::Light.label()),
        ];
        if widgets::segmented(ui, "cfg-theme", &mut c.theme, &options, Some(420.0)) {
            act.push(Action::ApplyTheme);
        }
    });
    widgets::settings_card(ui, "Couleur d'accent", Some("Boutons principaux, sélection, focus."), |ui| {
        ui.horizontal(|ui| {
            for a in Accent::ALL {
                if widgets::swatch(ui, a.swatch(), c.accent == a, a.label()).clicked() {
                    c.accent = a;
                    act.push(Action::ApplyTheme);
                }
            }
            ui.add_space(10.0);
            ui.label(RichText::new(c.accent.label()).size(14.0).color(t.muted));
        });
    });
    widgets::settings_card(ui, "Densité", None, |ui| {
        if widgets::segmented(ui, "cfg-density", &mut c.compact, &[(false, Some(Icon::Rows3), "Confortable"), (true, Some(Icon::List), "Compacte")], Some(420.0)) {
            act.push(Action::ApplyTheme);
        }
    });
    widgets::settings_card(ui, "Taille de l'interface", None, |ui| {
        ui.horizontal(|ui| {
            let resp = widgets::named(ui.add(egui::Slider::new(&mut c.zoom, 0.8..=1.4).step_by(0.05).show_value(false).trailing_fill(true)), egui::Role::Slider, "Taille de l'interface");
            ui.label(RichText::new(format!("{:.0} %", c.zoom * 100.0)).font(FontId::new(15.0, theme::semibold())).color(t.ink));
            if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                act.push(Action::ApplyZoom);
            }
        });
    });
}

// ---- Données -----------------------------------------------------------------------------------

fn data_section(ui: &mut Ui, data: &mut Data, st: &UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    header(ui, "Données", "Tout reste en local, dans un seul fichier lisible.");
    widgets::settings_card(ui, "Mises à jour", Some("Atelier évolue par versions : on télécharge, puis on redémarre pour appliquer."), |ui| {
        ui.horizontal(|ui| {
            icons::show(ui, Icon::PackageCheck, 18.0, t.muted);
            ui.label(RichText::new(format!("Version installée : {VERSION}")).font(FontId::new(14.0, theme::semibold())).color(t.ink));
            let latest = st.updater.release.as_ref().map(|r| r.version.clone()).unwrap_or_default();
            let (label, tone) = match st.updater.phase {
                UpdatePhase::Available => (format!("{latest} disponible"), Tone::Accent),
                UpdatePhase::Ready => ("prête à installer".to_owned(), Tone::Accent),
                UpdatePhase::Downloading => ("téléchargement…".to_owned(), Tone::Neutral),
                UpdatePhase::Checking => ("vérification…".to_owned(), Tone::Neutral),
                UpdatePhase::UpToDate => ("à jour".to_owned(), Tone::Good),
                UpdatePhase::Idle => ("pas encore vérifié".to_owned(), Tone::Neutral),
                UpdatePhase::Failed => ("vérification impossible".to_owned(), Tone::Danger),
            };
            widgets::chip(ui, &label, tone);
        });
        widgets::toggle_row(ui, &mut data.config.update_auto, "Vérifier au démarrage", Some("Une pastille apparaît sur l'icône de mise à jour quand une version attend."));
        widgets::toggle_row(ui, &mut data.config.update_beta, "Recevoir les versions bêta", Some("Plus tôt, parfois moins stables."));
        ui.horizontal(|ui| {
            if widgets::button(ui, Some(Icon::CircleArrowUp), "Ouvrir la mise à jour", Kind::Primary).clicked() {
                act.push(Action::OpenUpdate);
            }
            let checking = st.updater.busy();
            ui.add_enabled_ui(!checking, |ui| {
                if widgets::button(ui, Some(Icon::RefreshCw), "Vérifier maintenant", Kind::Secondary).clicked() {
                    act.push(Action::CheckUpdate);
                }
            });
        });
    });
    widgets::settings_card(ui, "Fichier de données", Some("Projets, étapes, notes et réglages : un seul fichier JSON lisible, partagé avec l'agent MCP."), |ui| {
        match &st.store_path {
            Some(path) => {
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::HardDrive, 18.0, t.muted);
                    ui.label(RichText::new(path).font(FontId::new(12.5, FontFamily::Monospace)).color(t.ink));
                    widgets::chip(ui, &format!("{:.0} Ko", size as f64 / 1024.0), Tone::Neutral);
                });
                ui.label(RichText::new(format!("{} projets · enregistré à chaque changement · embeddings dans embeddings.json à côté", data.projects.len())).size(12.5).color(t.faint));
                ui.horizontal(|ui| {
                    if widgets::button(ui, Some(Icon::Copy), "Copier le chemin", Kind::Secondary).clicked() {
                        act.push(Action::CopyText(path.clone()));
                    }
                });
            }
            None => {
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::FlaskConical, 18.0, t.muted);
                    ui.label(RichText::new("Données de démonstration : rien n'est enregistré.").size(13.5).color(t.ink));
                });
            }
        }
    });
    widgets::settings_card(ui, "Données de démonstration", Some("Des projets fictifs, pour découvrir l'interface. Ton fichier n'est pas touché."), |ui| {
        let mut demo = st.store_path.is_none();
        if widgets::toggle_row(ui, &mut demo, "Afficher les données de démonstration", Some("Désactivé, Atelier revient à ton fichier et à tes vrais projets.")).changed() {
            act.push(Action::SetDemo(demo));
        }
    });
    widgets::settings_card(ui, "Confidentialité", None, |ui| {
        for (icon, text) in [
            (Icon::Lock, "L'index ne quitte jamais ta machine."),
            (Icon::Sparkles, "L'IA ne reçoit que ce qui est autorisé dans « IA & modèles »."),
            (Icon::EyeOff, "Les fichiers .env et les secrets ne sont jamais lus par l'IA."),
        ] {
            ui.horizontal(|ui| {
                icons::show(ui, icon, 16.0, t.good_fg);
                ui.label(RichText::new(text).size(13.5).color(t.ink));
            });
        }
    });
}
