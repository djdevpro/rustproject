//! Le logo d'un projet, à la place de son icône Lucide : un SVG ou une image, choisi dans
//! l'onglet « Modifier » de la fiche (fichier, code collé, ou fichier déposé sur la fenêtre).
//! - Un SVG garde ses propres couleurs. Seul `currentColor` (celui des icônes monochromes)
//!   prend la couleur de l'icône, pour rester lisible en thème clair comme sombre.
//! - Une image (PNG, JPEG, WebP, ICO, GIF, BMP) est réduite à 128 px et gardée en PNG.

use std::path::Path;

use base64::Engine;
use egui::{Color32, CornerRadius, Pos2, Rect, Response, RichText, Sense, Ui, vec2};

use crate::app::ProjectDraft;
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::model::{Logo, Project};
use crate::theme::Tokens;
use crate::widgets::{self, Kind};

/// Le côté le plus long d'une image importée (le logo s'affiche à 60 px au plus).
const IMAGE_MAX: u32 = 128;
/// Le plus gros fichier image accepté.
const FILE_MAX: usize = 25_000_000;
/// Les extensions d'image acceptées (en plus de .svg).
const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "webp", "ico", "gif", "bmp"];

/// Un fichier choisi ou déposé.
pub fn from_file(path: &Path) -> Result<Logo, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    from_bytes(&bytes, &ext)
}

/// Des octets, avec l'extension du fichier (vide si on ne la connaît pas) : du SVG, sinon une
/// image.
pub fn from_bytes(bytes: &[u8], ext: &str) -> Result<Logo, String> {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
    if ext == "svg" || (ext.is_empty() && head.contains("<svg")) {
        return crate::launchers::check_svg(&String::from_utf8_lossy(bytes)).map(Logo::Svg);
    }
    if !ext.is_empty() && !IMAGE_EXT.contains(&ext) {
        return Err(tr("Formats acceptés : SVG, PNG, JPEG, WebP, ICO, GIF et BMP.").into());
    }
    if bytes.len() > FILE_MAX {
        return Err(tr("Image trop lourde (25 Mo au plus).").into());
    }
    let img = image::load_from_memory(bytes).map_err(|_| tr("Cette image n'a pas pu être lue.").to_owned())?;
    let img = if img.width() > IMAGE_MAX || img.height() > IMAGE_MAX { img.resize(IMAGE_MAX, IMAGE_MAX, image::imageops::FilterType::Lanczos3) } else { img };
    let mut png = std::io::Cursor::new(Vec::new());
    img.to_rgba8().write_to(&mut png, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(Logo::Png(base64::engine::general_purpose::STANDARD.encode(png.into_inner())))
}

/// L'adresse du logo pour le chargeur d'images d'egui, en lui confiant les octets la première
/// fois. `None` : pas de logo (ou illisible), on dessine l'icône.
fn uri(ctx: &egui::Context, logo: &Logo, color: Color32) -> Option<String> {
    let hex = format!("{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());
    let uri = match logo {
        Logo::None => return None,
        Logo::Svg(svg) if svg.contains("currentColor") => format!("bytes://logo-{:x}-{hex}.svg", crate::store::text_hash(svg)),
        Logo::Svg(svg) => format!("bytes://logo-{:x}.svg", crate::store::text_hash(svg)),
        Logo::Png(b64) => format!("bytes://logo-{:x}.png", crate::store::text_hash(b64)),
    };
    if ctx.try_load_bytes(&uri).is_err() {
        let bytes = match logo {
            Logo::Svg(svg) => svg.replace("currentColor", &format!("#{hex}")).into_bytes(),
            Logo::Png(b64) => base64::engine::general_purpose::STANDARD.decode(b64).ok()?,
            Logo::None => return None,
        };
        ctx.include_bytes(uri.clone(), bytes);
    }
    Some(uri)
}

/// Un logo (ou à défaut l'icône) dans un carré de `size` centré sur `center`.
pub fn paint_logo(ui: &Ui, logo: &Logo, icon: Icon, center: Pos2, size: f32, color: Color32) {
    let Some(uri) = uri(ui.ctx(), logo, color) else {
        icons::paint(ui.painter(), center, icon, size, color);
        return;
    };
    let rect = Rect::from_center_size(center, vec2(size, size) * 0.92);
    egui::Image::new(uri).fit_to_exact_size(rect.size()).show_loading_spinner(false).paint_at(ui, rect);
}

/// L'icône d'un projet : son logo s'il en a un, sinon son icône. `color` : la couleur de
/// l'icône (et du `currentColor` d'un SVG).
pub fn paint(ui: &Ui, p: &Project, center: Pos2, size: f32, color: Color32) {
    paint_logo(ui, &p.logo, p.icon, center, size, color);
}

/// L'icône d'un projet seule, dans un carré de `size` points (comme `icons::show`).
pub fn show(ui: &mut Ui, p: &Project, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    paint(ui, p, rect.center(), size, color);
    resp
}

/// Une tuile carrée teintée avec l'icône d'un projet (comme `widgets::icon_tile`).
pub fn tile(ui: &mut Ui, p: &Project, color: Color32, size: f32) -> Response {
    tile_of(ui, &p.logo, p.icon, color, size)
}

fn tile_of(ui: &mut Ui, logo: &Logo, icon: Icon, color: Color32, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same((size * 0.24) as u8), crate::theme::soft(&Tokens::get(ui.ctx()), color, 0.16));
    paint_logo(ui, logo, icon, rect.center(), size * 0.56, color);
    resp
}

/// Dans l'onglet « Modifier » : l'icône, ou un logo à sa place. `paste` : la zone « coller le
/// code SVG » ouverte ; `error` : le dernier fichier refusé.
pub fn editor(ui: &mut Ui, d: &mut ProjectDraft, paste: &mut Option<String>, error: &mut Option<String>, width: f32) {
    let t = Tokens::get(ui.ctx());
    widgets::labeled(ui, tr("Icône ou logo"), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if d.logo.is_none() {
                widgets::icon_picker(ui, tr("Icône du projet"), &mut d.icon);
            } else {
                tile_of(ui, &d.logo, d.icon, t.accent_ink, 34.0);
            }
            let choose = if d.logo.is_none() { tr("Choisir un logo…") } else { tr("Remplacer…") };
            if widgets::button_sized(ui, Some(Icon::FileUp), choose, Kind::Secondary, 34.0, None).clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_title(tr("Logo du projet"))
                    .add_filter(tr("SVG ou image"), &["svg", "png", "jpg", "jpeg", "webp", "ico", "gif", "bmp"])
                    .pick_file()
            {
                set(d, error, from_file(&path));
            }
            if paste.is_none() && widgets::button_sized(ui, Some(Icon::ClipboardPaste), tr("Coller un SVG"), Kind::Secondary, 34.0, None).clicked() {
                *paste = Some(String::new());
            }
            if !d.logo.is_none() && widgets::button_sized(ui, Some(Icon::X), tr("Retirer le logo"), Kind::Ghost, 34.0, None).clicked() {
                d.logo = Logo::None;
                *error = None;
            }
        });
        if let Some(code) = paste.as_mut() {
            widgets::named(
                ui.add(egui::TextEdit::multiline(code).code_editor().desired_rows(4).desired_width(width).hint_text("<svg viewBox=\"0 0 24 24\">…</svg>")),
                egui::Role::MultilineTextInput,
                tr("Code SVG du logo"),
            );
            let mut close = false;
            ui.horizontal(|ui| {
                if widgets::button_sized(ui, Some(Icon::Check), tr("Utiliser ce logo"), Kind::Primary, 30.0, None).clicked() {
                    let result = crate::launchers::check_svg(code).map(Logo::Svg);
                    close = result.is_ok();
                    set(d, error, result);
                }
                if widgets::button_sized(ui, None, tr("Annuler"), Kind::Ghost, 30.0, None).clicked() {
                    close = true;
                }
            });
            if close {
                *paste = None;
            }
        }
        if let Some(e) = error {
            ui.label(RichText::new(e.as_str()).size(12.5).color(t.danger));
        }
        ui.label(RichText::new(tr("Un SVG garde ses couleurs, une image est réduite à 128 px. Tu peux aussi déposer le fichier sur la fenêtre.")).size(12.0).color(t.faint));
    });
}

fn set(d: &mut ProjectDraft, error: &mut Option<String>, result: Result<Logo, String>) {
    match result {
        Ok(logo) => (d.logo, *error) = (logo, None),
        Err(e) => *error = Some(e),
    }
}

/// Un fichier déposé sur la fenêtre pendant que l'onglet « Modifier » est ouvert devient le
/// logo du projet ; pendant le survol, un cadre invite à le déposer.
pub fn take_dropped(ctx: &egui::Context, d: &mut ProjectDraft, error: &mut Option<String>) {
    crate::launchers::drop_hint(ctx, tr("Dépose le fichier, il devient le logo du projet"));
    for file in ctx.input(|i| i.raw.dropped_files.clone()) {
        set(d, error, from_file(file.path()));
    }
}
