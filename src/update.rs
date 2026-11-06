//! La fenêtre « Mise à jour » : la dernière release publiée sur GitHub, ses points de version,
//! le téléchargement (empreinte vérifiée), puis le redémarrage qui l'installe. Voir `release.rs`.

use egui::{Align, FontId, Frame, Id, Layout, Margin, RichText, Stroke};

use crate::app::{Action, UiState, UpdatePhase, VERSION};
use crate::icons::{self, Icon};
use crate::model::Data;
use crate::theme::{self, Tokens};
use crate::widgets::{self, Kind, Tone};

fn megabytes(bytes: u64) -> String {
    format!("{:.1} Mo", bytes as f64 / 1_000_000.0).replace('.', ",")
}

pub fn modal(ctx: &egui::Context, data: &Data, st: &UiState, act: &mut Vec<Action>) -> bool {
    let t = Tokens::get(ctx);
    let u = &st.updater;
    let mut open = true;
    let frame = Frame::new().fill(t.surface).stroke(Stroke::new(1.0, t.line_strong)).corner_radius(16).shadow(theme::shadow(&t)).inner_margin(Margin::same(26));
    let modal = egui::Modal::new(Id::unique("update")).backdrop_color(theme::scrim(&t)).frame(frame).show(ctx, |ui| {
        ui.set_width(520.0);
        ui.horizontal_top(|ui| {
            widgets::icon_tile(ui, Icon::CircleArrowUp, t.accent_ink, 44.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                widgets::title(ui, "Mise à jour d'Atelier", 22.0);
                ui.label(RichText::new(format!("Version installée : {VERSION}")).size(13.5).color(t.muted));
            });
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if widgets::icon_button(ui, Icon::X, "Fermer", 34.0).clicked() {
                    open = false;
                }
            });
        });
        ui.add_space(16.0);
        let w = ui.available_width();
        let dev = crate::release::is_dev_build();
        match u.phase {
            UpdatePhase::Idle | UpdatePhase::UpToDate | UpdatePhase::Failed => {
                ui.horizontal(|ui| match u.phase {
                    UpdatePhase::Failed => {
                        icons::show(ui, Icon::TriangleAlert, 20.0, t.danger);
                        ui.label(RichText::new(u.error.as_deref().unwrap_or("Vérification impossible")).size(14.5).color(t.ink));
                    }
                    UpdatePhase::UpToDate => {
                        icons::show(ui, Icon::CircleCheck, 20.0, t.good_fg);
                        ui.label(RichText::new(format!("Atelier {VERSION} est à jour.")).size(15.0).color(t.ink));
                    }
                    _ => {
                        icons::show(ui, Icon::Info, 20.0, t.muted);
                        ui.label(RichText::new("Pas encore vérifié.").size(15.0).color(t.ink));
                    }
                });
                ui.add_space(12.0);
                let label = if u.phase == UpdatePhase::Failed { "Réessayer" } else { "Vérifier maintenant" };
                if widgets::button(ui, Some(Icon::RefreshCw), label, Kind::Secondary).clicked() {
                    act.push(Action::CheckUpdate);
                }
            }
            UpdatePhase::Checking => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(18.0).color(t.accent));
                    ui.label(RichText::new("Recherche d'une nouvelle version…").size(15.0).color(t.ink));
                });
            }
            UpdatePhase::Available | UpdatePhase::Downloading => {
                let Some(r) = &u.release else { return };
                widgets::card(&t).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Version {}", r.version)).font(FontId::new(18.0, theme::bold())).color(t.ink_strong));
                        widgets::chip(ui, if r.prerelease { "Bêta" } else { "Nouvelle" }, Tone::Accent);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let size = r.asset.as_ref().map(|a| megabytes(a.size)).unwrap_or_default();
                            let when = if r.published.is_empty() { String::new() } else { format!("publiée le {}", r.published) };
                            ui.label(RichText::new([size, when].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")).size(12.5).color(t.faint));
                        });
                    });
                    ui.add_space(4.0);
                    if r.notes.is_empty() {
                        widgets::muted(ui, "Pas de notes pour cette version.");
                    }
                    for note in &r.notes {
                        ui.label(RichText::new(format!("•  {note}")).size(14.0).color(t.ink));
                    }
                    if !r.page.is_empty() {
                        ui.add_space(2.0);
                        ui.hyperlink_to(RichText::new("Voir la version sur GitHub").size(12.5), &r.page);
                    }
                });
                ui.add_space(14.0);
                if u.phase == UpdatePhase::Downloading {
                    let (done, total) = u.progress;
                    ui.label(RichText::new(format!("Téléchargement… {} / {}", megabytes(done), megabytes(total))).size(14.0).color(t.ink));
                    widgets::progress(ui, done as f32 / total.max(1) as f32, t.accent, w, 8.0);
                } else {
                    ui.horizontal(|ui| {
                        let installable = !dev && r.asset.is_some();
                        ui.add_enabled_ui(installable, |ui| {
                            if widgets::button(ui, Some(Icon::Download), "Télécharger et installer", Kind::Primary).clicked() {
                                act.push(Action::DownloadUpdate);
                            }
                        });
                        if widgets::button(ui, None, "Plus tard", Kind::Ghost).clicked() {
                            open = false;
                        }
                    });
                    if dev {
                        ui.label(RichText::new("Version de développement, compilée sur place. Les mises à jour s'installent sur l'exécutable d'une release.").size(12.5).color(t.faint));
                    } else if r.asset.is_none() {
                        ui.label(RichText::new("Cette version n'a pas de fichier pour cette plateforme.").size(12.5).color(t.faint));
                    }
                }
            }
            UpdatePhase::Ready => {
                let version = u.release.as_ref().map(|r| r.version.as_str()).unwrap_or("");
                ui.horizontal(|ui| {
                    icons::show(ui, Icon::PackageCheck, 22.0, t.good_fg);
                    ui.label(RichText::new(format!("La version {version} est prête.")).font(FontId::new(15.5, theme::semibold())).color(t.ink_strong));
                });
                ui.label(RichText::new("Atelier va se fermer et se rouvrir pour l'appliquer. Tes projets et réglages sont conservés.").size(13.5).color(t.muted));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if widgets::button(ui, Some(Icon::RotateCw), "Redémarrer maintenant", Kind::Primary).clicked() {
                        act.push(Action::Restart);
                    }
                    if widgets::button(ui, None, "Au prochain lancement", Kind::Ghost).clicked() {
                        open = false;
                    }
                });
            }
        }
        ui.add_space(14.0);
        widgets::separator(ui);
        ui.add_space(2.0);
        let auto = if data.config.update_auto { "Vérification automatique au démarrage" } else { "Vérification automatique désactivée" };
        ui.label(RichText::new(format!("{auto} · canal {} · réglable dans Configuration › Données", if data.config.update_beta { "bêta" } else { "stable" })).size(12.5).color(t.faint));
        ui.label(RichText::new(format!("Versions publiées sur github.com/{}", crate::release::REPO)).size(12.0).color(t.faint));
    });
    open && !modal.should_close()
}
