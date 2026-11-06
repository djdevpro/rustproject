//! Le thème de Rust Project : des design tokens (couleurs nommées par rôle), une couleur d'accent au
//! choix, des polices, et les `Visuals` d'egui réécrits à partir de tout ça.
//!
//! Aucun widget ne code une couleur en dur : tous lisent `Tokens::get(ctx)`.

use egui::epaint::Shadow;
use egui::epaint::text::VariationCoords;
use serde::{Deserialize, Serialize};
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, FontTweak, Id, Stroke, TextStyle, Visuals};

// ---- choix de l'utilisateur --------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    System,
    #[default]
    Dark,
    Light,
}

impl ThemeChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "Système",
            Self::Dark => "Sombre",
            Self::Light => "Clair",
        }
    }
}

/// La couleur d'accent : l'ambre des maquettes par défaut, quatre autres au choix.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Accent {
    #[default]
    Amber,
    Coral,
    Green,
    Blue,
    Violet,
}

/// Les cinq teintes dérivées d'un accent, pour une apparence.
struct AccentSet {
    accent: Color32,
    hover: Color32,
    ink: Color32,
    soft: Color32,
    on: Color32,
}

impl Accent {
    pub const ALL: [Self; 5] = [Self::Amber, Self::Coral, Self::Green, Self::Blue, Self::Violet];

    pub fn label(self) -> &'static str {
        match self {
            Self::Amber => "Ambre",
            Self::Coral => "Corail",
            Self::Green => "Vert",
            Self::Blue => "Bleu",
            Self::Violet => "Violet",
        }
    }

    /// La teinte principale, pour les pastilles du sélecteur.
    pub fn swatch(self) -> Color32 {
        self.set(true).accent
    }

    fn set(self, dark: bool) -> AccentSet {
        let s = |accent, hover, ink, soft, on| AccentSet { accent: hex(accent), hover: hex(hover), ink: hex(ink), soft: hex(soft), on: hex(on) };
        match (self, dark) {
            (Self::Amber, true) => s(0xf4a945, 0xffbb62, 0xf6b65c, 0x4a3215, 0x1d1407),
            (Self::Amber, false) => s(0xf0a03a, 0xf6b052, 0xa35f08, 0xfbe5c4, 0x1d1407),
            (Self::Coral, true) => s(0xff7f66, 0xff9883, 0xff9a85, 0x4a2219, 0x1f0c07),
            (Self::Coral, false) => s(0xf26b52, 0xf6826c, 0xb8432d, 0xfde0d9, 0x1f0c07),
            (Self::Green, true) => s(0x6fcf8e, 0x86dba2, 0x8fdca6, 0x1f3a28, 0x07140c),
            (Self::Green, false) => s(0x46ad6a, 0x58bd7a, 0x23744a, 0xd8f0df, 0x07140c),
            (Self::Blue, true) => s(0x6aa6ff, 0x84b6ff, 0x8db9ff, 0x1b2c47, 0x06101f),
            (Self::Blue, false) => s(0x4a88f0, 0x5f97f3, 0x2459b8, 0xdce8fd, 0x06101f),
            (Self::Violet, true) => s(0xb48cff, 0xc3a3ff, 0xc6a6ff, 0x33244f, 0x120a20),
            (Self::Violet, false) => s(0x9668f2, 0xa67ef4, 0x6236c4, 0xebe2fd, 0x120a20),
        }
    }
}

// ---- tokens ------------------------------------------------------------------------------------

/// Les couleurs d'une apparence, nommées par leur rôle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub dark: bool,
    /// Fond de la fenêtre.
    pub bg: Color32,
    /// Cartes et panneaux.
    pub surface: Color32,
    /// Survol, éléments en relief.
    pub raised: Color32,
    /// Champs de saisie.
    pub input: Color32,
    /// Filets fins.
    pub line: Color32,
    /// Contours des boutons secondaires et des champs.
    pub line_strong: Color32,
    pub ink: Color32,
    pub ink_strong: Color32,
    pub muted: Color32,
    pub faint: Color32,
    /// Accent : boutons principaux.
    pub accent: Color32,
    pub accent_hover: Color32,
    /// Accent pour du texte ou une icône sur le fond.
    pub accent_ink: Color32,
    /// Fond doux d'un élément sélectionné (onglet de navigation, option choisie).
    pub accent_soft: Color32,
    /// Texte posé sur l'accent.
    pub on_accent: Color32,
    /// Pastille « En cours ».
    pub good_bg: Color32,
    pub good_fg: Color32,
    /// Pastilles neutres (« Au fil de l'eau », « En réserve »).
    pub neutral_bg: Color32,
    pub neutral_fg: Color32,
    pub danger: Color32,
    pub info: Color32,
}

const fn hex(rgb: u32) -> Color32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

pub fn rgb(rgb: u32) -> Color32 {
    hex(rgb)
}

const DARK: Tokens = Tokens {
    dark: true,
    bg: hex(0x131211),
    surface: hex(0x1b1a18),
    raised: hex(0x262421),
    input: hex(0x23211f),
    line: hex(0x33302c),
    line_strong: hex(0x4b4741),
    ink: hex(0xece7df),
    ink_strong: hex(0xfbf8f3),
    muted: hex(0xb3ada3),
    faint: hex(0x7f786f),
    accent: hex(0xf4a945),
    accent_hover: hex(0xffbb62),
    accent_ink: hex(0xf6b65c),
    accent_soft: hex(0x4a3215),
    on_accent: hex(0x1d1407),
    good_bg: hex(0x2c4526),
    good_fg: hex(0xb4d89a),
    neutral_bg: hex(0x383532),
    neutral_fg: hex(0xd9d3ca),
    danger: hex(0xf2685e),
    info: hex(0x7aa7ff),
};

const LIGHT: Tokens = Tokens {
    dark: false,
    bg: hex(0xf4f1eb),
    surface: hex(0xffffff),
    raised: hex(0xf1ede6),
    input: hex(0xf8f6f2),
    line: hex(0xe5dfd5),
    line_strong: hex(0xcdc5b8),
    ink: hex(0x25211d),
    ink_strong: hex(0x0f0d0b),
    muted: hex(0x6b645a),
    faint: hex(0x9a9286),
    accent: hex(0xf0a03a),
    accent_hover: hex(0xf6b052),
    accent_ink: hex(0xa35f08),
    accent_soft: hex(0xfbe5c4),
    on_accent: hex(0x1d1407),
    good_bg: hex(0xdff0d5),
    good_fg: hex(0x2f6b1f),
    neutral_bg: hex(0xebe6de),
    neutral_fg: hex(0x4b453d),
    danger: hex(0xc4382d),
    info: hex(0x2f6fd6),
};

fn accent_id() -> Id {
    Id::unique("atelier-accent")
}

impl Tokens {
    /// Les tokens affichés : l'apparence (sombre/claire) et l'accent choisi par l'utilisateur.
    pub fn get(ctx: &egui::Context) -> Self {
        let dark = ctx.theme() == egui::Theme::Dark;
        let accent = ctx.data(|d| d.get_temp::<Accent>(accent_id())).unwrap_or_default();
        Self::of(dark, accent)
    }

    fn of(dark: bool, accent: Accent) -> Self {
        let base = if dark { DARK } else { LIGHT };
        let a = accent.set(dark);
        Self { accent: a.accent, accent_hover: a.hover, accent_ink: a.ink, accent_soft: a.soft, on_accent: a.on, ..base }
    }

    pub fn hairline(&self) -> Stroke {
        Stroke::new(1.0, self.line)
    }
}

/// `c` avec une opacité `alpha` (0–1).
pub fn tint(c: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Mélange de `a` vers `b` (`t` de 0 à 1), pour les transitions.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

pub fn shadow(t: &Tokens) -> Shadow {
    Shadow { offset: [0, 12], blur: 32, spread: 0, color: Color32::from_black_alpha(if t.dark { 160 } else { 45 }) }
}

pub fn scrim(t: &Tokens) -> Color32 {
    Color32::from_black_alpha(if t.dark { 150 } else { 90 })
}

// ---- mesures -----------------------------------------------------------------------------------

pub const TOPBAR_H: f32 = 60.0;
pub const STATUS_H: f32 = 40.0;
pub const CONTROL_H: f32 = 36.0;
pub const GAP: f32 = 12.0;
pub const PANEL_W: f32 = 420.0;
pub const R_CARD: u8 = 12;
pub const R_CTRL: u8 = 8;
pub const R_CHIP: u8 = 6;

// ---- Visuals -----------------------------------------------------------------------------------

fn visuals(t: &Tokens) -> Visuals {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.bg;
    v.window_fill = t.surface;
    v.extreme_bg_color = t.input;
    v.text_edit_bg_color = Some(t.input);
    v.faint_bg_color = t.raised;
    v.code_bg_color = t.raised;
    v.window_stroke = Stroke::new(1.0, t.line_strong);
    v.window_corner_radius = CornerRadius::same(R_CARD);
    v.menu_corner_radius = CornerRadius::same(R_CTRL);
    v.window_shadow = shadow(t);
    v.popup_shadow = shadow(t);
    v.override_text_color = Some(t.ink);
    v.hyperlink_color = t.accent_ink;
    v.selection.bg_fill = tint(t.accent, 0.32);
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.text_cursor.stroke = Stroke::new(2.0, t.accent);
    v.warn_fg_color = t.accent_ink;
    v.error_fg_color = t.danger;
    for (w, fill, stroke, fg) in [
        (&mut v.widgets.noninteractive, t.surface, t.line, t.muted),
        (&mut v.widgets.inactive, t.input, t.line_strong, t.ink),
        (&mut v.widgets.hovered, t.raised, t.faint, t.ink_strong),
        (&mut v.widgets.active, t.raised, t.accent, t.ink_strong),
        (&mut v.widgets.open, t.raised, t.line_strong, t.ink_strong),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, stroke);
        w.corner_radius = CornerRadius::same(R_CTRL);
        w.fg_stroke = Stroke::new(1.0, fg);
        w.expansion = 0.0;
    }
    v
}

/// Installe l'apparence, l'accent, les espacements et la typographie.
pub fn apply(ctx: &egui::Context, theme: ThemeChoice, accent: Accent, compact: bool) {
    ctx.data_mut(|d| d.insert_temp(accent_id(), accent));
    ctx.set_visuals_of(egui::Theme::Dark, visuals(&Tokens::of(true, accent)));
    ctx.set_visuals_of(egui::Theme::Light, visuals(&Tokens::of(false, accent)));
    ctx.set_theme(match theme {
        ThemeChoice::System => egui::ThemePreference::System,
        ThemeChoice::Dark => egui::ThemePreference::Dark,
        ThemeChoice::Light => egui::ThemePreference::Light,
    });
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = if compact { egui::vec2(6.0, 5.0) } else { egui::vec2(8.0, 8.0) };
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.interact_size.y = CONTROL_H;
        style.spacing.slider_width = 240.0;
        style.spacing.icon_spacing = 8.0;
        // Libellés non sélectionnables : glisser sur un texte fait défiler au lieu de le surligner.
        // (Les champs de saisie restent sélectionnables.)
        style.interaction.selectable_labels = false;
        // Barres de défilement fines, toujours un peu visibles, et un fondu plus net sur le bord
        // d'une zone qui continue : on voit qu'il y a de la suite (rangée focus, lanceurs…).
        style.spacing.scroll = egui::style::ScrollStyle {
            dormant_handle_opacity: 0.35,
            fade: egui::style::ScrollFadeStyle { strength: 0.9, size: 40.0 },
            ..egui::style::ScrollStyle::floating()
        };
        style.text_styles = [
            (TextStyle::Heading, FontId::new(26.0, bold())),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.0, semibold())),
            (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
        ]
        .into();
    });
}

// ---- polices -----------------------------------------------------------------------------------

pub fn medium() -> FontFamily {
    FontFamily::Name("medium".into())
}
pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}
pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}
/// Les icônes Lucide seules (voir `icons.rs`).
pub fn icons() -> FontFamily {
    FontFamily::Name("icons".into())
}

type Face<'a> = (&'static str, &'static str, &'a [([u8; 4], f32)], FontFamily);

/// Segoe UI Variable (texte, quatre graisses tirées d'un seul fichier), Cascadia Mono (chemins,
/// commandes) et Lucide (icônes). Lucide est aussi ajoutée en secours derrière chaque police : une
/// icône peut ainsi se glisser dans n'importe quel texte (`format!("{} Ajouter", icon)`).
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let proportional = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let monospace = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();

    const BODY: &str = "SegUIVar.ttf";
    const MONO: &str = "CascadiaMono.ttf";
    let faces: [Face<'_>; 5] = [
        ("Body", BODY, &[(*b"wght", 400.0), (*b"opsz", 10.5)], FontFamily::Proportional),
        ("Body-Medium", BODY, &[(*b"wght", 500.0), (*b"opsz", 10.5)], medium()),
        ("Body-SemiBold", BODY, &[(*b"wght", 600.0), (*b"opsz", 10.5)], semibold()),
        ("Body-Bold", BODY, &[(*b"wght", 700.0), (*b"opsz", 28.0)], bold()),
        ("Mono", MONO, &[(*b"wght", 400.0)], FontFamily::Monospace),
    ];
    // Chinois, japonais, coréen : la police du système pour ces écritures, en secours derrière
    // les autres (seulement pour la langue choisie : ces polices pèsent lourd).
    let cjk = match crate::i18n::current() {
        crate::i18n::Lang::Zh => Some(("msyh.ttc", "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc")),
        crate::i18n::Lang::Ja => Some(("YuGothM.ttc", "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc")),
        crate::i18n::Lang::Ko => Some(("malgun.ttf", "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc")),
        _ => None,
    }
    .and_then(|(windows, linux)| system_font(windows).or_else(|| system_font(linux)));
    if let Some(bytes) = cjk {
        fonts.font_data.insert("CJK".to_owned(), FontData::from_static(bytes).into());
    }
    let symbols = system_font("seguisym.ttf").or_else(|| system_font("/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf"));
    if let Some(bytes) = symbols {
        fonts.font_data.insert("Symbols".to_owned(), FontData::from_static(bytes).into());
    }
    fonts.font_data.insert(
        "Lucide".to_owned(),
        FontData::from_static(lucide_icons::LUCIDE_FONT_BYTES).tweak(FontTweak { y_offset_factor: 0.07, ..Default::default() }).into(),
    );
    for (name, file, coords, family) in faces {
        let fallback = if file == MONO { &monospace } else { &proportional };
        let mut stack = Vec::new();
        if let Some(bytes) = system_font(file) {
            let tweak = FontTweak { coords: VariationCoords::new(coords.iter().copied()), subpixel_binning: Some(false), ..Default::default() };
            fonts.font_data.insert(name.to_owned(), FontData::from_static(bytes).tweak(tweak).into());
            stack.push(name.to_owned());
        } else if let Some(bytes) = linux_font(name) {
            // Sans les polices de Windows (Linux) : DejaVu, qui a aussi les flèches et symboles.
            fonts.font_data.insert(name.to_owned(), FontData::from_static(bytes).into());
            stack.push(name.to_owned());
        }
        stack.extend(fallback.iter().cloned());
        if cjk.is_some() {
            stack.push("CJK".to_owned());
        }
        stack.push("Lucide".to_owned());
        // Les pictogrammes (⟳, 🔃…) des écrans d'inspection d'egui (outils de développement).
        if symbols.is_some() {
            stack.push("Symbols".to_owned());
        }
        fonts.families.insert(family, stack);
    }
    fonts.families.insert(icons(), vec!["Lucide".to_owned()]);
    ctx.set_fonts(fonts);
}

/// La police DejaVu qui remplace une face de Segoe UI ou de Cascadia, quand elles manquent.
fn linux_font(face: &str) -> Option<&'static [u8]> {
    const DIR: &str = "/usr/share/fonts/truetype/dejavu/";
    let file = match face {
        "Body-SemiBold" | "Body-Bold" => "DejaVuSans-Bold.ttf",
        "Mono" => "DejaVuSansMono.ttf",
        _ => "DejaVuSans.ttf",
    };
    system_font(&format!("{DIR}{file}"))
}

/// Lit une police de `C:\Windows\Fonts` (ou un chemin absolu) une seule fois (octets partagés en
/// `&'static [u8]`).
fn system_font(file: &str) -> Option<&'static [u8]> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    type FontCache = HashMap<String, Option<&'static [u8]>>;
    static CACHE: OnceLock<Mutex<FontCache>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().ok()?;
    *cache.entry(file.to_owned()).or_insert_with(|| {
        let path = if std::path::Path::new(file).is_absolute() {
            std::path::PathBuf::from(file)
        } else {
            let dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
            std::path::Path::new(&dir).join("Fonts").join(file)
        };
        let bytes = std::fs::read(path).ok()?;
        Some(&*Box::leak(bytes.into_boxed_slice()))
    })
}
