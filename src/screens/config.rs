//! « Configuration » : huit sections. Les réglages modifient directement `Data::config` (ce qui
//! n'a pas d'effet visible au-delà de l'interface reste simulé, c'est une maquette).

use egui::{Align, Color32, CornerRadius, FontFamily, FontId, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::{Action, CfgSection, UiState, UpdatePhase, VERSION};
use crate::screens::setup::Step;
use crate::icons::{self, Icon};
use crate::model::{Attention, Data, ReadDepth, ScanFreq, Status, VARIABLES};
use crate::ollama::ModelInfo;
use crate::theme::{self, Accent, ThemeChoice, Tokens, tint};
use crate::widgets::{self, Kind, Tone};
use crate::i18n::{tf, tr};

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
    widgets::title(ui, tr("Configuration"), 28.0);
    ui.add_space(12.0);
    ui.spacing_mut().item_spacing.y = 4.0;
    for section in CfgSection::ALL {
        let selected = st.cfg == section;
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 54.0), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, selected, section.label()));
        let hover = ui.ctx().animate_bool(resp.id.with("hover"), resp.hovered());
        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::same(10), if selected { t.accent_soft } else { tint(t.ink, 0.05 * hover) });
        let color = if selected { t.ink_strong } else { t.muted };
        icons::paint(p, pos2(rect.left() + 24.0, rect.center().y), section.icon(), 19.0, color);
        p.text(pos2(rect.left() + 46.0, rect.center().y - 9.0), egui::Align2::LEFT_CENTER, section.label(), FontId::new(14.5, theme::semibold()), if selected { t.ink_strong } else { t.ink });
        p.text(pos2(rect.left() + 46.0, rect.center().y + 10.0), egui::Align2::LEFT_CENTER, section.hint(), FontId::proportional(12.0), if selected { t.muted } else { t.faint });
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
            ui.label(RichText::new(tf("Parcours des dossiers… {0} relevés", &[("0", &st.scan_count)])).size(13.5).color(t.ink));
        });
    }
    if let Some(a) = st.analysis.as_ref().filter(|a| !a.finished) {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            let who = if a.with_ai { tr("L'IA classe") } else { tr("Les règles classent") };
            ui.label(RichText::new(tf("{who} les dossiers… {0}/{1} · {2}", &[("who", &who), ("0", &a.done), ("1", &a.total), ("2", &a.current)])).size(13.5).color(t.ink));
        });
        widgets::progress(ui, a.done as f32 / a.total.max(1) as f32, t.accent, ui.available_width(), 6.0);
        ui.label(RichText::new(tf("{0} projets créés, {1} dossiers écartés", &[("0", &a.added), ("1", &a.skipped)])).size(12.5).color(t.muted));
    }
}

// ---- Dossiers sources --------------------------------------------------------------------------

fn sources(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    header(ui, tr("Dossiers sources"), tr("Les dossiers où se trouvent tes projets. Rust Project n'y lit que les noms de dossiers, les README et les manifestes, sur 1 à 3 niveaux."));
    // L'assistant d'abord : le chemin le plus simple, pas à pas.
    Frame::new().fill(tint(t.accent, 0.07)).stroke(Stroke::new(1.0, tint(t.accent, 0.35))).corner_radius(12).inner_margin(Margin::same(16)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            widgets::icon_tile(ui, Icon::WandSparkles, t.accent_ink, 40.0);
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() - 220.0).max(160.0));
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(tr("Assistant pas à pas")).font(FontId::new(15.0, theme::semibold())).color(t.ink_strong));
                widgets::muted(ui, tr("Choisir les dossiers, scanner, vérifier ce qui a été trouvé, créer les fiches."));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button(ui, Some(Icon::ArrowRight), tr("Lancer l'assistant"), Kind::Primary).clicked() {
                    act.push(Action::OpenSetup(Step::Where));
                }
            });
        });
    });
    widgets::settings_card(ui, tr("Dossiers surveillés"), Some(tr("Chacun a sa profondeur : le nombre de niveaux de sous-dossiers parcourus. Un changement relance le scan.")), |ui| {
        if data.config.roots.is_empty() {
            widgets::muted(ui, tr("Aucun dossier pour l'instant."));
        }
        for (i, root) in data.config.roots.iter_mut().enumerate() {
            Frame::new().fill(t.bg).stroke(Stroke::new(1.0, t.line)).corner_radius(10).inner_margin(Margin::same(12)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if widgets::switch(ui, &mut root.enabled, &tf("Activer {0}", &[("0", &root.path)])).changed() {
                        act.push(Action::StartScan);
                    }
                    let w = ui.available_width() - 210.0;
                    let resp = widgets::path_input(ui, &mut root.path, w);
                    if resp.changed() {
                        // Un autre chemin : les chiffres de l'ancien ne valent plus.
                        root.found = 0;
                        root.last_scan = 0;
                    }
                    // Validé (Entrée, clic ailleurs, « Parcourir… ») : on relève le nouveau dossier.
                    if (resp.lost_focus() || (resp.changed() && !resp.has_focus())) && root.last_scan == 0 && std::path::Path::new(root.path.trim()).is_dir() {
                        act.push(Action::StartScan);
                    }
                    let mut depth = root.depth.clamp(1, 3);
                    if widgets::segmented(ui, &format!("depth-{i}"), &mut depth, &[(1, None, "1"), (2, None, "2"), (3, None, "3")], Some(120.0)) {
                        root.depth = depth;
                        act.push(Action::StartScan);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, Icon::Trash2, tr("Retirer ce dossier source"), 32.0).clicked() {
                            act.push(Action::RemoveRoot(i));
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.add_space(48.0);
                    if !std::path::Path::new(root.path.trim()).is_dir() {
                        widgets::chip(ui, tr("dossier introuvable"), Tone::Danger);
                    } else if root.last_scan == 0 {
                        widgets::chip(ui, tr("pas encore scanné"), Tone::Neutral);
                    } else {
                        widgets::chip(ui, &tf("{0} dossiers relevés", &[("0", &root.found)]), if root.found > 0 { Tone::Good } else { Tone::Neutral });
                        ui.label(RichText::new(if root.depth > 1 { tf("Dernier scan : {0} · {1} niveaux", &[("0", &crate::model::ago(root.last_scan)), ("1", &root.depth)]) } else { tf("Dernier scan : {0} · 1 niveau", &[("0", &crate::model::ago(root.last_scan))]) }).size(12.5).color(t.faint));
                    }
                    if !root.enabled {
                        ui.label(RichText::new(tr("· en pause")).size(12.5).color(t.faint));
                    }
                });
            });
        }
        if widgets::dashed_button(ui, Icon::FolderPlus, tr("Ajouter un dossier source"), vec2(ui.available_width(), 44.0)).clicked() {
            act.push(Action::PickRoot);
        }
    });

    // ---- Scan : les chiffres ; le tri des nouveaux projets se fait dans l'assistant.
    let new_likely = st.scan.as_ref().map(|s| crate::screens::setup::review_rows(data, s, false).len()).unwrap_or(0);
    widgets::settings_card(ui, tr("Scan"), Some(tr("Le scan relève les dossiers ; tu choisis ensuite ceux qui deviennent des fiches.")), |ui| {
        if let Some(scan) = &st.scan {
            // Les briques (frontend, api…) appartiennent à leur projet : ni « suivies », ni « nouvelles ».
            let bricks = scan.dirs.iter().filter(|f| f.brick_of.is_some()).count();
            let tracked = scan.dirs.iter().filter(|f| f.brick_of.is_none() && data.find_by_folder(&f.path).is_some()).count();
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 3.0 * 10.0) / 4.0;
                ui.spacing_mut().item_spacing.x = 10.0;
                stat(ui, Icon::FolderSearch, &scan.dirs.len().to_string(), tr("dossiers relevés"), t.info, w);
                stat(ui, Icon::FolderGit2, &scan.likely().count().to_string(), tr("projets probables"), t.good_fg, w);
                stat(ui, Icon::CircleCheck, &tracked.to_string(), tr("déjà suivis"), t.muted, w);
                stat(ui, Icon::Sparkles, &new_likely.to_string(), tr("nouveaux à trier"), t.accent_ink, w);
            });
            if bricks > 0 {
                ui.label(RichText::new(tf("{bricks} briques (frontend, api, db…) rattachées à leur projet parent : elles ne deviennent pas des fiches.", &[("bricks", &bricks)])).size(12.5).color(t.faint));
            }
            if scan.truncated {
                ui.label(RichText::new(tr("Scan arrêté à 8 000 dossiers : réduis la profondeur ou ajoute des exclusions.")).size(12.5).color(t.danger));
            }
        }
        scan_progress(ui, st);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            ui.add_enabled_ui(st.scan_started.is_none(), |ui| {
                if widgets::button(ui, Some(Icon::RefreshCw), tr("Scanner maintenant"), Kind::Primary).clicked() {
                    act.push(Action::StartScan);
                }
            });
            let busy = st.analysis.as_ref().is_some_and(|a| !a.finished);
            ui.add_enabled_ui(new_likely > 0 && !busy, |ui| {
                let label = if new_likely == 1 { tr("Trier le nouveau projet").to_owned() } else { tf("Trier les {0} nouveaux projets", &[("0", &new_likely)]) };
                if widgets::button(ui, Some(Icon::ListChecks), &label, Kind::Secondary).on_hover_text(tr("Dans l'assistant : tu coches ce qui devient une fiche.")).clicked() {
                    act.push(Action::OpenSetup(Step::Review));
                }
            });
        });
    });

    // ---- GitHub
    widgets::settings_card(ui, tr("GitHub"), Some(tr("Tes dépôts (les tiens, ceux de tes organisations, tes collaborations), via la CLI gh.")), |ui| {
        widgets::toggle_row(ui, &mut data.config.github_enabled, tr("Proposer mes dépôts GitHub"), Some(tr("Un dépôt sans dossier local devient une fiche « GitHub ».")));
        if !data.config.github_enabled {
            return;
        }
        github_account(ui, data, st, act);
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!st.github_loading, |ui| {
                if widgets::button(ui, Some(Icon::GitBranch), tr("Lister mes dépôts"), Kind::Secondary).clicked() {
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
                    ui.label(RichText::new(tf("{0} dépôts · {tracked} déjà suivis", &[("0", &repos.len()), ("tracked", &tracked)])).size(13.5).color(t.ink));
                    ui.add_enabled_ui(todo > 0, |ui| {
                        if widgets::button_sized(ui, Some(Icon::ListPlus), &tf("Ajouter les {todo} non suivis", &[("todo", &todo)]), Kind::Primary, 32.0, None).clicked() {
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
                                    widgets::chip_sized(ui, tr("archivé"), Tone::Neutral, 11.0, 20.0);
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
                            } else if widgets::button_sized(ui, Some(Icon::Plus), tr("Ajouter"), Kind::Secondary, 30.0, None).clicked() {
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

/// Le compte GitHub dont on liste les dépôts, parmi ceux connectés à la CLI gh.
fn github_account(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    match &st.github_accounts {
        None => {
            act.push(Action::ListGithubAccounts);
            ui.label(RichText::new(tr("Comptes GitHub connectés…")).size(13.0).color(t.muted));
        }
        Some(Err(e)) => {
            ui.label(RichText::new(e).size(13.0).color(t.danger));
        }
        Some(Ok((list, active))) => {
            let active = active.clone().unwrap_or_default();
            let default = tf("Le compte actif de gh ({0})", &[("0", &active)]);
            let mut options: Vec<(usize, &str)> = vec![(0, default.as_str())];
            options.extend(list.iter().enumerate().map(|(i, l)| (i + 1, l.as_str())));
            let mut current = list.iter().position(|l| *l == data.config.github_account).map(|i| i + 1).unwrap_or(0);
            ui.horizontal(|ui| {
                icons::show(ui, Icon::User, 16.0, t.muted);
                ui.label(RichText::new(tr("Compte")).size(13.5).color(t.ink));
                if widgets::combo(ui, tr("Compte GitHub"), &mut current, &options, 280.0) {
                    data.config.github_account = if current == 0 { String::new() } else { list[current - 1].clone() };
                    // Un autre compte : la liste affichée n'est plus la sienne.
                    st.github = None;
                }
            });
        }
    }
}

// ---- Découverte --------------------------------------------------------------------------------

fn discovery(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let c = &mut data.config;
    header(ui, tr("Découverte"), tr("La partie algorithmique : rapide, locale, déterministe. Elle trouve les projets ; l'IA les décrit ensuite."));
    widgets::settings_card(ui, tr("Qu'est-ce qui fait un projet ?"), Some(tr("Un dossier qui contient l'un de ces marqueurs devient un projet.")), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for (marker, on) in c.markers.iter_mut() {
                toggle_chip(ui, on, marker);
            }
        });
        ui.label(RichText::new(tr("Le README est un indice faible : il signale un projet sans empêcher de regarder ses sous-dossiers (un dossier de rangement en a souvent un).")).size(12.5).color(t.faint));
    });
    let ollama_ready = c.ollama_on && !c.ollama_model.is_empty();
    widgets::settings_card(ui, tr("Création des fiches"), None, |ui| {
        let auto_help = if ollama_ready {
            tr("Après chaque scan, l'IA classe les nouveaux dossiers et crée les fiches.")
        } else if c.mcp_on {
            tr("En mode agent MCP, c'est l'agent qui crée les fiches : rien d'automatique ici.")
        } else {
            tr("Après chaque scan, les dossiers avec un marqueur (.git, manifeste…) deviennent des fiches.")
        };
        widgets::toggle_row(ui, &mut c.auto_add, tr("Créer les projets automatiquement"), Some(auto_help));
        ui.label(RichText::new(tr("Sinon, tu tries toi-même les nouveaux projets dans l'assistant (Dossiers sources).")).size(12.5).color(t.faint));
    });
    if !c.ignored.is_empty() {
        widgets::settings_card(ui, tr("Dossiers écartés"), Some(tr("Écartés à la main ou par l'agent IA (copies, sauvegardes…) : le scan ne les relève plus.")), |ui| {
            for (i, path) in c.ignored.iter().enumerate() {
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::EyeOff, 16.0, t.faint);
                    let font = FontId::new(12.0, FontFamily::Monospace);
                    let shown = widgets::elide_middle(ui.painter(), path, &font, ui.available_width() - 120.0);
                    ui.label(RichText::new(shown).font(font).color(t.muted)).on_hover_text(path);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::button_sized(ui, Some(Icon::Undo2), tr("Rétablir"), Kind::Ghost, 28.0, None).clicked() {
                            act.push(Action::RestoreFolder(i));
                        }
                    });
                });
            }
        });
    }
    widgets::settings_card(ui, tr("Exclusions"), Some(tr("Ces dossiers ne sont jamais parcourus, où qu'ils soient. Les dossiers cachés (.git, .cache…) non plus.")), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            let mut remove = None;
            for (i, ex) in c.excludes.iter().enumerate() {
                let galley = ui.painter().layout_no_wrap(ex.clone(), FontId::new(12.5, FontFamily::Monospace), t.ink);
                let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 40.0, 30.0), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tf("Retirer {ex}", &[("ex", &ex)])));
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
            let resp = widgets::mono_input(ui, &mut st.new_exclude, tr("nom de dossier, ex. coverage"), 280.0);
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (widgets::button(ui, Some(Icon::Plus), tr("Ajouter"), Kind::Secondary).clicked() || enter) && !st.new_exclude.trim().is_empty() {
                let ex = st.new_exclude.trim().to_lowercase();
                if !c.excludes.contains(&ex) {
                    c.excludes.push(ex);
                }
                st.new_exclude.clear();
            }
        });
    });
    widgets::settings_card(ui, tr("Règles"), None, |ui| {
        widgets::toggle_row(ui, &mut c.skip_nested, tr("Ne pas chercher de projets dans un projet"), Some(tr("Un dossier reconnu n'est pas fouillé plus loin (sous-modules, exemples…).")));
        widgets::separator(ui);
        widgets::toggle_row(ui, &mut c.group_monorepos, tr("Rattacher les briques à leur projet"), Some(tr("frontend, backend, api, db, site, packages/x… deviennent des dossiers du projet parent, pas des fiches à part.")));
        widgets::separator(ui);
        widgets::toggle_row(ui, &mut c.ignore_older, tr("Ignorer les dossiers très anciens"), Some(tr("Sans aucune modification depuis la durée ci-dessous.")));
        ui.add_enabled_ui(c.ignore_older, |ui| {
            ui.horizontal(|ui| {
                widgets::named(ui.add(egui::Slider::new(&mut c.ignore_older_days, 90..=1825).show_value(false).trailing_fill(true)), egui::Role::Slider, tr("Ancienneté maximale"));
                ui.label(RichText::new(tf("{0} jours (≈ {1} ans)", &[("0", &c.ignore_older_days), ("1", &(format!("{:.1}", c.ignore_older_days as f32 / 365.0)))])).size(13.0).color(t.muted));
            });
        });
    });
    widgets::settings_card(ui, tr("Quand scanner ?"), None, |ui| {
        let options = [(ScanFreq::Startup, Some(Icon::Power), tr("Au démarrage")), (ScanFreq::Hourly, Some(Icon::Clock), tr("Toutes les heures")), (ScanFreq::Manual, Some(Icon::Hand), tr("À la demande"))];
        widgets::segmented(ui, "scan-freq", &mut c.freq, &options, Some(ui.available_width().min(560.0)));
        scan_progress(ui, st);
        ui.add_enabled_ui(st.scan_started.is_none(), |ui| {
            if widgets::button(ui, Some(Icon::RefreshCw), tr("Scanner maintenant"), Kind::Secondary).clicked() {
                act.push(Action::StartScan);
            }
        });
    });
    widgets::settings_card(ui, tr("Ce que l'algorithme relève"), Some(tr("Lu sur le disque, sans rien envoyer nulle part.")), |ui| {
        let items = [
            (Icon::GitBranch, tr("Git"), tr("Dernier commit, branche, dépôt distant")),
            (Icon::FileJson, tr("Manifestes"), tr("Cargo.toml, package.json, pyproject.toml…")),
            (Icon::Code, tr("Langages"), tr("Répartition des extensions de fichiers")),
            (Icon::Globe, tr("Port de dev"), tr("Scripts et fichiers .env (pour {port})")),
            (Icon::Activity, tr("Activité"), tr("Date de dernière modification, taille")),
            (Icon::FileText, tr("Documentation"), tr("README, CLAUDE.md, notes")),
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
    header(ui, tr("IA & modèles"), tr("L'algorithme trouve les dossiers ; l'IA décide lesquels sont des projets et rédige leur fiche en une phrase."));
    // Les deux cohabitent : Ollama (embeddings, et le modèle de langage s'il est choisi) et
    // l'agent MCP (qui écrit les fiches, les tags et les mots-clés).
    widgets::settings_card(ui, tr("Les IA utilisées"), Some(tr("Ollama et l'agent MCP travaillent ensemble : le premier cherche par le sens et peut classer, le second range et décrit tes projets.")), |ui| {
        widgets::toggle_row(ui, &mut data.config.ollama_on, tr("Ollama (API)"), Some(tr("Rust Project envoie à un modèle local (Ollama) un résumé de chaque dossier : nom, marqueurs, début du README. Rien ne sort de ta machine.")));
        ui.add_space(4.0);
        widgets::toggle_row(ui, &mut data.config.mcp_on, tr("Agent MCP"), Some(tr("Un agent IA (Claude Code…) se connecte à Rust Project par le protocole MCP : il scanne, décide et crée les fiches lui-même.")));
        if !data.config.ollama_on && !data.config.mcp_on {
            ui.add_space(4.0);
            ui.label(RichText::new(tr("Pas d'IA : les dossiers avec un marqueur (.git, Cargo.toml, package.json…) deviennent des projets, décrits par leur manifeste ou leur README.")).size(13.0).color(t.muted));
        }
    });
    if data.config.ollama_on {
        ollama_card(ui, data, st, act);
        semantic_card(ui, data, st, act);
    }
    if data.config.mcp_on {
        mcp_card(ui, data, st, act);
    }
    if data.config.ollama_on {
        widgets::settings_card(ui, tr("Ce que l'IA lit"), Some(tr("Ce qui est envoyé au modèle pour chaque dossier.")), |ui| {
            let options = [(ReadDepth::Readme, Some(Icon::FileText), tr("README")), (ReadDepth::Manifests, Some(Icon::FileJson), tr("+ manifestes")), (ReadDepth::Tree, Some(Icon::FolderTree), tr("+ sous-dossiers"))];
            widgets::segmented(ui, "read-depth", &mut data.config.read_depth, &options, Some(ui.available_width().min(560.0)));
            let explain = match data.config.read_depth {
                ReadDepth::Readme => tr("Le nom, les marqueurs et le début du README."),
                ReadDepth::Manifests => tr("Recommandé : en plus, le nom, la description et les technos des manifestes."),
                ReadDepth::Tree => tr("En plus, la liste des sous-dossiers et le nombre de fichiers (jamais leur contenu)."),
            };
            ui.label(RichText::new(explain).size(13.0).color(t.muted));
            ui.horizontal(|ui| {
                icons::show(ui, Icon::Lock, 16.0, t.good_fg);
                ui.label(RichText::new(tr("Le code source et les fichiers .env ne sont jamais envoyés.")).size(13.0).color(t.muted));
            });
        });
    }
    ollaya_card(ui, data, st, act);
}

/// Ollaya : le serveur des modèles de décision, qui classent les projets dans l'ontologie.
fn ollaya_card(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::settings_card(ui, tr("Ollaya (classement)"), Some(tr("Des modèles de décision locaux : ils rangent chaque projet dans les concepts de l'ontologie, avec une probabilité. Rien ne sort de ta machine.")), |ui| {
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 250.0).max(160.0);
            widgets::labeled(ui, tr("Adresse du serveur"), |ui| widgets::mono_input(ui, &mut data.config.ollaya_url, "http://127.0.0.1:11435", w));
            ui.vertical(|ui| {
                ui.add_space(22.0);
                ui.add_enabled_ui(!st.ollaya_loading, |ui| {
                    if widgets::button(ui, Some(Icon::Plug), tr("Tester et lister"), Kind::Secondary).on_hover_text(tr("Les modèles installés dans Ollaya")).clicked() {
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
                widgets::muted(ui, tr("Clique « Tester et lister » pour voir les modèles installés dans Ollaya."));
            }
            Some(Err(e)) => {
                ui.horizontal(|ui| {
                    widgets::chip(ui, "injoignable", Tone::Danger);
                    ui.label(RichText::new(e).size(12.5).color(t.danger));
                });
                ui.label(RichText::new(tr("Lance l'application Ollaya (ou `ollaya serve`), puis réessaie.")).size(12.5).color(t.faint));
            }
            Some(Ok(models)) => {
                ui.horizontal(|ui| {
                    widgets::chip(ui, &if models.len() > 1 { tf("connecté · {0} modèles", &[("0", &models.len())]) } else { tr("connecté · 1 modèle").to_owned() }, Tone::Good);
                    if models.is_empty() {
                        ui.label(RichText::new(tr("Aucun modèle : télécharges-en un ci-dessous (decider:2b conseillé).")).size(12.5).color(t.muted));
                    }
                });
                let all: Vec<&ModelInfo> = models.iter().collect();
                let w = ui.available_width();
                widgets::labeled(ui, tr("Modèle de décision (classement dans l'ontologie)"), |ui| model_combo(ui, tr("Modèle de décision"), &mut data.config.ollaya_model, &all, w.min(420.0)));
            }
        }
        ui.add_space(4.0);
        widgets::separator(ui);
        widgets::field_label(ui, tr("Télécharger un modèle de décision"));
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 150.0).max(160.0);
            widgets::mono_input(ui, &mut st.ollaya_pull_input, tr("ex. decider:2b"), w);
            ui.add_enabled_ui(st.ollaya_pull.is_none() && !st.ollaya_pull_input.trim().is_empty(), |ui| {
                if widgets::button(ui, Some(Icon::Download), tr("Télécharger"), Kind::Primary).clicked() {
                    act.push(Action::PullOllaya(st.ollaya_pull_input.clone()));
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            ui.label(RichText::new(tr("Conseillés :")).size(12.5).color(t.faint));
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
        if widgets::link(ui, tr("Régler l'ontologie et le classement"), Some(Icon::ArrowRight), 13.5).clicked() {
            act.push(Action::GoConfig(CfgSection::Ontology));
        }
    });
}

/// Choisir un modèle parmi une liste (`""` = aucun). Renvoie `true` au changement.
fn model_combo(ui: &mut Ui, name: &str, current: &mut String, models: &[&ModelInfo], width: f32) -> bool {
    let labels: Vec<String> = models.iter().map(|m| format!("{}  ·  {}", m.name, m.size_label())).collect();
    let mut sel = models.iter().position(|m| m.name == *current).unwrap_or(usize::MAX);
    let mut options: Vec<(usize, &str)> = vec![(usize::MAX, tr("— aucun —"))];
    options.extend(labels.iter().enumerate().map(|(i, l)| (i, l.as_str())));
    let changed = widgets::combo(ui, name, &mut sel, &options, width);
    if changed {
        *current = models.get(sel).map(|m| m.name.clone()).unwrap_or_default();
    }
    changed
}

fn ollama_card(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::settings_card(ui, tr("Ollama"), Some(tr("Le serveur local et les modèles utilisés.")), |ui| {
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 250.0).max(160.0);
            widgets::labeled(ui, tr("Adresse du serveur"), |ui| widgets::mono_input(ui, &mut data.config.ollama_url, "http://localhost:11434", w));
            ui.vertical(|ui| {
                ui.add_space(22.0);
                ui.add_enabled_ui(!st.models_loading, |ui| {
                    if widgets::button(ui, Some(Icon::Plug), tr("Tester et lister"), Kind::Secondary).clicked() {
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
                widgets::muted(ui, tr("Clique « Tester et lister » pour voir les modèles installés (l'équivalent de ollama list)."));
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
                    widgets::chip(ui, &if models.len() > 1 { tf("connecté · {0} modèles", &[("0", &models.len())]) } else { tr("connecté · 1 modèle").to_owned() }, Tone::Good);
                    if models.is_empty() {
                        ui.label(RichText::new(tr("Aucun modèle installé : télécharges-en un ci-dessous.")).size(12.5).color(t.muted));
                    }
                });
                let w = (ui.available_width() - 16.0) / 2.0;
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        widgets::labeled(ui, tr("Modèle de langage (classement)"), |ui| model_combo(ui, tr("Modèle de langage"), &mut data.config.ollama_model, &chat, w - 8.0));
                    });
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        widgets::labeled(ui, tr("Modèle d'embeddings (recherche)"), |ui| model_combo(ui, tr("Modèle d'embeddings"), &mut data.config.ollama_embed_model, &embed, w - 8.0));
                    });
                });
            }
        }
        ui.add_space(4.0);
        widgets::separator(ui);
        widgets::field_label(ui, tr("Télécharger un modèle"));
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 150.0).max(160.0);
            widgets::mono_input(ui, &mut st.pull_input, tr("ex. qwen2.5:3b ou bge-m3"), w);
            ui.add_enabled_ui(st.pull.is_none() && !st.pull_input.trim().is_empty(), |ui| {
                if widgets::button(ui, Some(Icon::Download), tr("Télécharger"), Kind::Primary).clicked() {
                    act.push(Action::PullModel(st.pull_input.clone()));
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            ui.label(RichText::new(tr("Conseillés :")).size(12.5).color(t.faint));
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
                if widgets::button(ui, Some(Icon::Sparkles), tr("Classer les dossiers non suivis"), Kind::Primary).clicked() {
                    act.push(Action::AnalyzeCandidates);
                }
            });
            let hint = if !ready {
                tr("Choisis d'abord un modèle de langage.")
            } else if st.scan.is_none() {
                tr("Lance d'abord un scan (Dossiers sources).")
            } else {
                tr("Les projets probables, et les dossiers de premier niveau sans marqueur.")
            };
            ui.label(RichText::new(hint).size(12.5).color(t.faint));
        });
        if let Some(a) = st.analysis.as_ref().filter(|a| a.finished && !a.warnings.is_empty()) {
            ui.label(RichText::new(tf("{0} réponses illisibles : fiches faites par les règles (ex. {1}).", &[("0", &a.warnings.len()), ("1", &a.warnings[0])])).size(12.5).color(t.faint));
        }
    });
}

fn semantic_card(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    widgets::settings_card(ui, tr("Recherche sémantique"), Some(tr("Chercher par le sens : « logement » trouve ImmoFormation. Les fiches sont transformées en vecteurs par Ollama.")), |ui| {
        let model = data.config.ollama_embed_model.clone();
        if model.is_empty() {
            ui.horizontal(|ui| {
                widgets::chip(ui, "inactive", Tone::Neutral);
                ui.label(RichText::new(tr("Choisis un modèle d'embeddings (qwen3-embedding:4b conseillé pour le français).")).size(13.0).color(t.muted));
            });
            if data.config.mcp_on {
                ui.label(RichText::new(tf("Elle passe par le serveur Ollama ({0}), même en mode agent MCP.", &[("0", &data.config.ollama_url)])).size(12.5).color(t.faint));
            }
            return;
        }
        let total = data.projects.len();
        ui.horizontal(|ui| {
            if st.semantic_busy {
                ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            }
            let tone = if st.semantic_indexed == total && total > 0 { Tone::Good } else { Tone::Neutral };
            widgets::chip(ui, &tf("{0}/{total} fiches indexées", &[("0", &st.semantic_indexed), ("total", &total)]), tone);
            widgets::tag(ui, &model);
        });
        if let Some(e) = &st.semantic_error {
            ui.label(RichText::new(e).size(12.5).color(t.danger));
        }
        ui.horizontal(|ui| {
            if widgets::button(ui, Some(Icon::RefreshCw), tr("Réindexer"), Kind::Secondary).clicked() {
                act.push(Action::Reindex);
            }
            ui.label(RichText::new(tr("L'index se met à jour tout seul quand une fiche change.")).size(12.5).color(t.faint));
        });
        // Les mots-clés : ce qui donne le sens à l'index. L'agent MCP les écrit, ou le modèle de
        // langage d'Ollama.
        ui.add_space(4.0);
        let missing = data.projects.iter().filter(|p| p.keywords.is_empty()).count();
        ui.label(RichText::new(tf("{0} fiches sans mots-clés. L'agent MCP les écrit (prompt « ajouter_mots_cles »), ou le modèle de langage d'Ollama.", &[("0", &missing)])).size(12.5).color(t.muted));
        let llm = data.config.ollama_on && !data.config.ollama_model.trim().is_empty();
        ui.horizontal(|ui| {
            match st.keywords_job {
                Some((done, total)) => {
                    ui.add(egui::Spinner::new().size(16.0).color(t.accent));
                    ui.label(RichText::new(tf("Mots-clés : {0}/{1}", &[("0", &done), ("1", &total)])).size(13.0).color(t.ink));
                }
                None => {
                    ui.add_enabled_ui(llm && missing > 0, |ui| {
                        let resp = widgets::button(ui, Some(Icon::Sparkles), &tf("Proposer les mots-clés manquants ({0})", &[("0", &missing)]), Kind::Secondary);
                        if resp.clicked() {
                            act.push(Action::SuggestKeywords(None));
                        }
                        if !llm {
                            resp.on_disabled_hover_text(tr("Choisis d'abord un modèle de langage dans la carte Ollama."));
                        }
                    });
                }
            }
        });
    });
}

fn mcp_card(ui: &mut Ui, data: &mut Data, st: &mut UiState, act: &mut Vec<Action>) {
    let t = Tokens::get(ui.ctx());
    let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "rust-project.exe".into());
    widgets::settings_card(ui, tr("Agent MCP"), Some(tr("Rust Project est aussi un serveur MCP : `rust-project --mcp`. L'agent lit et écrit le même fichier que cette fenêtre, qui se met à jour toute seule.")), |ui| {
        let code = |ui: &mut Ui, label: &str, text: String, act: &mut Vec<Action>| {
            widgets::field_label(ui, label);
            ui.horizontal_top(|ui| {
                let width = (ui.available_width() - 48.0).max(120.0);
                Frame::new().fill(t.input).corner_radius(8).inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.set_width(width - 20.0);
                    code_text(ui, &text, t.ink);
                });
                if widgets::icon_button(ui, Icon::Copy, &tf("Copier : {label}", &[("label", &label)]), 34.0).clicked() {
                    act.push(Action::CopyText(text.clone()));
                }
            });
        };
        code(ui, "Dans Claude Code", format!(r#"claude mcp add rust-project -- "{exe}" --mcp"#), act);
        let json = format!(r#"{{ "mcpServers": {{ "rust-project": {{ "command": "{}", "args": ["--mcp"] }} }} }}"#, exe.replace('\\', "\\\\"));
        code(ui, tr("Autres clients (configuration JSON)"), json, act);
        widgets::field_label(ui, tr("Outils proposés à l'agent"));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for tool in crate::mcp::tool_names() {
                widgets::tag(ui, &tool);
            }
        });
        ui.horizontal(|ui| {
            icons::show(ui, Icon::Lightbulb, 15.0, t.faint);
            ui.label(RichText::new(tr("Puis demande à l'agent : « Range mes projets avec Rust Project ».")).size(12.5).color(t.muted));
        });
        if let Some(path) = &st.store_path {
            ui.label(RichText::new(tf("Fichier partagé : {path}", &[("path", &path)])).size(12.0).color(t.faint));
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
    header(ui, tr("Lanceurs"), tr("Les boutons « Ouvrir avec » du panneau projet. Ceux par défaut apparaissent partout ; chaque projet peut en ajouter ou en masquer."));
    widgets::settings_card(ui, tr("Lanceurs par défaut"), Some(tr("Dans cet ordre, sur tous les projets. Le premier est mis en avant, sauf choix contraire du projet.")), |ui| {
        let n = data.default_launchers.len();
        for (pos, &id) in data.default_launchers.iter().enumerate() {
            let l = &data.launchers[id];
            Frame::new().fill(t.bg).stroke(Stroke::new(1.0, if pos == 0 { tint(t.accent, 0.6) } else { t.line })).corner_radius(10).inner_margin(Margin::symmetric(12, 10)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::GripVertical, 16.0, t.faint);
                    widgets::launcher_icon_tile(ui, l, 36.0);
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
                        if widgets::icon_button(ui, Icon::X, &tf("Retirer {0} des lanceurs par défaut", &[("0", &l.name)]), 32.0).clicked() {
                            act.push(Action::RemoveDefault(id));
                        }
                        if widgets::icon_button(ui, Icon::Pencil, &tf("Modifier {0}", &[("0", &l.name)]), 32.0).clicked() {
                            act.push(Action::OpenEditor(Some(id)));
                        }
                        ui.add_enabled_ui(pos + 1 < n, |ui| {
                            if widgets::icon_button(ui, Icon::ArrowDown, tr("Descendre"), 32.0).clicked() {
                                act.push(Action::MoveDefault(id, 1));
                            }
                        });
                        ui.add_enabled_ui(pos > 0, |ui| {
                            if widgets::icon_button(ui, Icon::ArrowUp, tr("Monter"), 32.0).clicked() {
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
            if widgets::button(ui, Some(Icon::LayoutGrid), tr("Ajouter depuis la galerie"), Kind::Primary).clicked() {
                act.push(Action::OpenGallery(None));
            }
            if widgets::button(ui, Some(Icon::Plus), tr("Créer un lanceur"), Kind::Secondary).clicked() {
                act.push(Action::OpenEditor(None));
            }
        });
    });
    // Les lanceurs qui ne servent qu'à certains projets.
    let specific: Vec<usize> = (0..data.launchers.len()).filter(|id| !data.default_launchers.contains(id)).collect();
    widgets::settings_card(ui, tr("Lanceurs propres à des projets"), Some(tr("Ajoutés depuis le « + » d'un projet. Les modifier ici met à jour tous les projets qui s'en servent.")), |ui| {
        if specific.is_empty() {
            widgets::muted(ui, tr("Aucun pour l'instant."));
        }
        for id in specific {
            let l = &data.launchers[id];
            let users: Vec<&str> = data.projects.iter().filter(|p| p.launchers.contains(&id)).map(|p| p.name.as_str()).collect();
            ui.horizontal(|ui| {
                widgets::launcher_icon_tile(ui, l, 32.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new(&l.name).font(FontId::new(14.5, theme::semibold())).color(t.ink));
                    let used = if users.is_empty() { tr("Utilisé par aucun projet").to_owned() } else { tf("Utilisé par {0}", &[("0", &users.join(", "))]) };
                    ui.label(RichText::new(used).size(12.5).color(t.muted));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::button_sized(ui, Some(Icon::Layers), tr("Mettre par défaut"), Kind::Ghost, 32.0, None).clicked() {
                        act.push(Action::AddLauncher { launcher: l.clone(), project: None });
                    }
                    if widgets::icon_button(ui, Icon::Pencil, &tf("Modifier {0}", &[("0", &l.name)]), 32.0).clicked() {
                        act.push(Action::OpenEditor(Some(id)));
                    }
                });
            });
        }
    });
    widgets::settings_card(ui, tr("Variables"), Some(tr("Utilisables dans n'importe quelle commande. Elles sont remplacées au clic, pour le projet ouvert.")), |ui| {
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
            ui.label(RichText::new(tf("Exemple avec le projet {0}.", &[("0", &project.name)])).size(12.5).color(t.faint));
        }
    });
}

// ---- États & attention -------------------------------------------------------------------------

fn attention(ui: &mut Ui, data: &mut Data) {
    let t = Tokens::get(ui.ctx());
    let counts: Vec<usize> = Attention::ALL.iter().map(|a| data.count(*a)).collect();
    let c = &mut data.config;
    header(ui, tr("États & attention"), tr("Deux axes séparés : l'état dit où en est le projet, l'attention dit combien tu lui en donnes."));
    widgets::settings_card(ui, tr("Niveaux d'attention"), None, |ui| {
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
                    ui.label(RichText::new(tf("{0} projets", &[("0", &counts[k])])).size(13.5).color(t.faint));
                });
            });
        }
    });
    widgets::settings_card(ui, tr("Projets du moment"), Some(tr("Par défaut, autant de projets en focus que tu veux : la rangée du bureau défile.")), |ui| {
        widgets::toggle_row(ui, &mut c.limit_focus, tr("Limiter le nombre de projets en focus"), Some(tr("Au-delà, Rust Project te demande quel projet libérer avant d'en ajouter un.")));
        ui.add_enabled_ui(c.limit_focus, |ui| {
            ui.horizontal(|ui| {
                widgets::named(ui.add(egui::Slider::new(&mut c.focus_limit, 1..=8).show_value(false).trailing_fill(true)), egui::Role::Slider, tr("Limite du focus"));
                ui.label(RichText::new(tf("{0} projets", &[("0", &c.focus_limit)])).font(FontId::new(15.0, theme::semibold())).color(t.ink));
            });
        });
        let over = counts[0].saturating_sub(c.focus_limit);
        let (text, color) = if c.limit_focus && over > 0 {
            (tf("{0} projet(s) en focus en trop : ils restent visibles jusqu'à ce que tu choisisses.", &[("0", &over)]), t.danger)
        } else {
            (tf("Actuellement {0} en focus.", &[("0", &counts[0])]), t.faint)
        };
        ui.label(RichText::new(text).size(13.0).color(color));
    });
    widgets::settings_card(ui, tr("Mise en réserve automatique"), None, |ui| {
        widgets::toggle_row(ui, &mut c.auto_reserve, tr("Passer en réserve les projets « au fil de l'eau » endormis"), Some(tr("Ils ne disparaissent pas : la revue les fera ressortir.")));
        ui.add_enabled_ui(c.auto_reserve, |ui| {
            ui.horizontal(|ui| {
                widgets::named(ui.add(egui::Slider::new(&mut c.reserve_after, 14..=180).show_value(false).trailing_fill(true)), egui::Role::Slider, tr("Délai avant réserve"));
                ui.label(RichText::new(tf("après {0} jours sans activité", &[("0", &c.reserve_after)])).size(13.5).color(t.ink));
            });
        });
    });
    widgets::settings_card(ui, tr("Rythme de la revue"), Some(tr("Combien de projets en réserve ressortent chaque jour dans « À revisiter ».")), |ui| {
        widgets::segmented(ui, "reviews-per-day", &mut c.reviews_per_day, &[(1, None, tr("1 par jour")), (2, None, tr("2 par jour")), (3, None, tr("3 par jour"))], Some(420.0));
    });
    widgets::settings_card(ui, tr("États"), Some(tr("Le cycle de vie d'un projet.")), |ui| {
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
    header(ui, tr("Apparence"), tr("Le thème suit les maquettes ; l'accent et la densité sont à ton goût."));
    widgets::settings_card(ui, tr("Langue"), Some(tr("La langue de l'interface. En automatique, Rust Project suit celle du système.")), |ui| {
        let auto = tf("Automatique ({0})", &[("0", &crate::i18n::detect().native())]);
        let mut options: Vec<(Option<crate::i18n::Lang>, &str)> = vec![(None, auto.as_str())];
        options.extend(crate::i18n::Lang::ALL.map(|l| (Some(l), l.native())));
        let mut choice = c.language;
        if widgets::combo(ui, tr("Langue de l'interface"), &mut choice, &options, 260.0) {
            act.push(Action::SetLanguage(choice));
        }
    });
    widgets::settings_card(ui, tr("Thème"), None, |ui| {
        let options: Vec<(ThemeChoice, Option<Icon>, &str)> = vec![
            (ThemeChoice::System, Some(Icon::Monitor), ThemeChoice::System.label()),
            (ThemeChoice::Dark, Some(Icon::Moon), ThemeChoice::Dark.label()),
            (ThemeChoice::Light, Some(Icon::Sun), ThemeChoice::Light.label()),
        ];
        if widgets::segmented(ui, "cfg-theme", &mut c.theme, &options, Some(420.0)) {
            act.push(Action::ApplyTheme);
        }
    });
    widgets::settings_card(ui, tr("Couleur d'accent"), Some(tr("Boutons principaux, sélection, focus.")), |ui| {
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
    widgets::settings_card(ui, tr("Densité"), None, |ui| {
        if widgets::segmented(ui, "cfg-density", &mut c.compact, &[(false, Some(Icon::Rows3), tr("Confortable")), (true, Some(Icon::List), tr("Compacte"))], Some(420.0)) {
            act.push(Action::ApplyTheme);
        }
    });
    widgets::settings_card(ui, tr("Taille de l'interface"), None, |ui| {
        ui.horizontal(|ui| {
            let resp = widgets::named(ui.add(egui::Slider::new(&mut c.zoom, 0.8..=1.4).step_by(0.05).show_value(false).trailing_fill(true)), egui::Role::Slider, tr("Taille de l'interface"));
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
    header(ui, tr("Données"), tr("Tout reste en local, dans un seul fichier lisible."));
    widgets::settings_card(ui, tr("Outils de développement"), Some(tr("Comme les outils d'un navigateur : inspecter l'interface, les performances, la mémoire, l'état de l'application.")), |ui| {
        ui.horizontal(|ui| {
            if widgets::button(ui, Some(Icon::Bug), tr("Ouvrir les outils"), Kind::Secondary).clicked() {
                act.push(Action::OpenDevTools);
            }
            ui.label(RichText::new(tr("Raccourci : F12, ou Ctrl + Maj + I")).size(12.5).color(t.faint));
        });
    });
    widgets::settings_card(ui, tr("Mises à jour"), Some(tr("Rust Project évolue par versions : on télécharge, puis on redémarre pour appliquer.")), |ui| {
        ui.horizontal(|ui| {
            icons::show(ui, Icon::PackageCheck, 18.0, t.muted);
            ui.label(RichText::new(tf("Version installée : {VERSION}", &[("VERSION", &VERSION)])).font(FontId::new(14.0, theme::semibold())).color(t.ink));
            let latest = st.updater.release.as_ref().map(|r| r.version.clone()).unwrap_or_default();
            let (label, tone) = match st.updater.phase {
                UpdatePhase::Available => (tf("{latest} disponible", &[("latest", &latest)]), Tone::Accent),
                UpdatePhase::Ready => (tr("prête à installer").to_owned(), Tone::Accent),
                UpdatePhase::Downloading => (tr("téléchargement…").to_owned(), Tone::Neutral),
                UpdatePhase::Checking => (tr("vérification…").to_owned(), Tone::Neutral),
                UpdatePhase::UpToDate => (tr("à jour").to_owned(), Tone::Good),
                UpdatePhase::Idle => (tr("pas encore vérifié").to_owned(), Tone::Neutral),
                UpdatePhase::Failed => (tr("vérification impossible").to_owned(), Tone::Danger),
            };
            widgets::chip(ui, &label, tone);
        });
        widgets::toggle_row(ui, &mut data.config.update_auto, tr("Vérifier au démarrage"), Some(tr("Une pastille apparaît sur l'icône de mise à jour quand une version attend.")));
        widgets::toggle_row(ui, &mut data.config.update_beta, tr("Recevoir les versions bêta"), Some(tr("Plus tôt, parfois moins stables.")));
        ui.horizontal(|ui| {
            if widgets::button(ui, Some(Icon::CircleArrowUp), tr("Ouvrir la mise à jour"), Kind::Primary).clicked() {
                act.push(Action::OpenUpdate);
            }
            let checking = st.updater.busy();
            ui.add_enabled_ui(!checking, |ui| {
                if widgets::button(ui, Some(Icon::RefreshCw), tr("Vérifier maintenant"), Kind::Secondary).clicked() {
                    act.push(Action::CheckUpdate);
                }
            });
        });
    });
    widgets::settings_card(ui, tr("Fichier de données"), Some(tr("Projets, étapes, notes et réglages : un seul fichier JSON lisible, partagé avec l'agent MCP.")), |ui| {
        match &st.store_path {
            Some(path) => {
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::HardDrive, 18.0, t.muted);
                    ui.label(RichText::new(path).font(FontId::new(12.5, FontFamily::Monospace)).color(t.ink));
                    widgets::chip(ui, &format!("{:.0} Ko", size as f64 / 1024.0), Tone::Neutral);
                });
                ui.label(RichText::new(tf("{0} projets · enregistré à chaque changement · embeddings dans embeddings.json à côté", &[("0", &data.projects.len())])).size(12.5).color(t.faint));
                ui.horizontal(|ui| {
                    if widgets::button(ui, Some(Icon::Copy), tr("Copier le chemin"), Kind::Secondary).clicked() {
                        act.push(Action::CopyText(path.clone()));
                    }
                });
            }
            None => {
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::FlaskConical, 18.0, t.muted);
                    ui.label(RichText::new(tr("Données de démonstration : rien n'est enregistré.")).size(13.5).color(t.ink));
                });
            }
        }
    });
    widgets::settings_card(ui, tr("Données de démonstration"), Some(tr("Des projets fictifs, pour découvrir l'interface. Ton fichier n'est pas touché.")), |ui| {
        let mut demo = st.store_path.is_none();
        if widgets::toggle_row(ui, &mut demo, tr("Afficher les données de démonstration"), Some(tr("Désactivé, Rust Project revient à ton fichier et à tes vrais projets."))).changed() {
            act.push(Action::SetDemo(demo));
        }
    });
    widgets::settings_card(ui, tr("Confidentialité"), None, |ui| {
        for (icon, text) in [
            (Icon::Lock, tr("L'index ne quitte jamais ta machine.")),
            (Icon::Sparkles, tr("L'IA ne reçoit que ce qui est autorisé dans « IA & modèles ».")),
            (Icon::EyeOff, tr("Les fichiers .env et les secrets ne sont jamais lus par l'IA.")),
        ] {
            ui.horizontal(|ui| {
                icons::show(ui, icon, 16.0, t.good_fg);
                ui.label(RichText::new(text).size(13.5).color(t.ink));
            });
        }
    });
}
