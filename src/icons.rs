//! Les icônes Lucide, dessinées comme des glyphes de police : nettes à toutes les tailles et
//! recolorées gratuitement.

use std::sync::OnceLock;

use egui::{Align2, Color32, FontId, Painter, Pos2, Response, Sense, Ui, vec2};
pub use lucide_icons::Icon;

use crate::theme;

/// Le caractère d'une icône, à glisser dans un texte (la police Lucide est en secours partout).
pub fn ch(icon: Icon) -> char {
    char::from(icon)
}

/// Deux icônes sont-elles la même ? (`Icon` n'implémente pas `PartialEq`.)
pub fn same(a: Icon, b: Icon) -> bool {
    ch(a) == ch(b)
}

pub fn paint(p: &Painter, center: Pos2, icon: Icon, size: f32, color: Color32) {
    p.text(center, Align2::CENTER_CENTER, ch(icon), FontId::new(size, theme::icons()), color);
}

/// Une icône seule, dans un carré de `size` points.
pub fn show(ui: &mut Ui, icon: Icon, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    paint(ui.painter(), rect.center(), icon, size, color);
    resp
}

// ---- catalogue et recherche (sélecteur d'icône) ------------------------------------------------

/// Le nom Lucide d'une icône (« house », « gamepad-2 »…).
pub fn name(icon: Icon) -> String {
    icon.to_string()
}

/// Toutes les icônes Lucide (une par glyphe), triées par nom.
pub fn all() -> &'static [(Icon, String)] {
    static ALL: OnceLock<Vec<(Icon, String)>> = OnceLock::new();
    ALL.get_or_init(|| {
        // Les glyphes Lucide occupent la zone à usage privé de la police.
        let mut v: Vec<(Icon, String)> = ('\u{e000}'..='\u{f8ff}').filter_map(|c| Icon::try_from(c).ok()).map(|i| (i, name(i))).collect();
        v.sort_by(|a, b| a.1.cmp(&b.1));
        v
    })
}

/// Proposées d'emblée pour un projet, avant toute recherche.
const SUGGESTED: &[&str] = &[
    "layers", "house", "sparkles", "audio-waveform", "code", "laptop", "box", "gamepad-2", "graduation-cap", "globe", "shopping-basket", "mic", "notebook-pen",
    "rocket", "wrench", "compass", "book-open", "bot", "brain", "terminal", "database", "server", "cloud", "smartphone", "monitor", "music", "camera", "image",
    "video", "palette", "pen-tool", "puzzle", "heart", "leaf", "car", "plane", "building-2", "wallet", "chart-line", "shield", "lock", "message-circle", "mail",
    "calendar", "map", "flask-conical", "cpu", "network", "file-text", "package", "store", "users", "trophy", "dumbbell", "utensils", "coffee",
];

/// Des mots français, et les icônes qu'ils évoquent (les noms Lucide sont en anglais).
const FRENCH: &[(&str, &[&str])] = &[
    ("maison accueil", &["house", "house-plus", "door-open"]),
    ("immobilier logement appartement", &["building-2", "house", "key-round", "door-open"]),
    ("batiment immeuble entreprise bureau", &["building", "building-2", "briefcase"]),
    ("jeu jouer manette", &["gamepad-2", "joystick", "dices", "puzzle"]),
    ("musique chanson", &["music", "audio-waveform", "headphones", "guitar", "piano"]),
    ("son audio podcast", &["audio-waveform", "volume-2", "mic", "headphones", "podcast"]),
    ("micro voix", &["mic", "audio-waveform"]),
    ("code programmation developpement", &["code", "code-xml", "terminal", "braces", "git-branch"]),
    ("terminal console ligne commande", &["terminal", "square-terminal"]),
    ("web site internet navigateur", &["globe", "app-window", "monitor", "link"]),
    ("ordinateur pc ecran", &["laptop", "monitor", "computer"]),
    ("telephone mobile application appli", &["smartphone", "tablet-smartphone", "app-window"]),
    ("formation ecole cours apprendre education", &["graduation-cap", "school", "book-open", "presentation"]),
    ("livre lecture bibliotheque", &["book", "book-open", "library"]),
    ("ia intelligence artificielle agent llm", &["sparkles", "bot", "brain", "brain-circuit", "cpu"]),
    ("robot automatisation", &["bot", "workflow", "cog"]),
    ("cerveau idee reflexion", &["brain", "lightbulb"]),
    ("outil reparation bricolage", &["wrench", "hammer", "settings"]),
    ("fusee lancement startup", &["rocket"]),
    ("boutique magasin commerce vente achat panier", &["store", "shopping-basket", "shopping-cart", "shopping-bag"]),
    ("argent finance banque paiement budget", &["wallet", "banknote", "piggy-bank", "credit-card", "landmark", "euro"]),
    ("graphique statistique tableau bord analyse", &["chart-line", "chart-bar", "chart-pie", "gauge"]),
    ("donnees base", &["database", "table", "chart-bar"]),
    ("serveur hebergement", &["server", "hard-drive", "cloud"]),
    ("nuage", &["cloud", "cloud-upload"]),
    ("securite protection", &["shield", "shield-check", "lock", "key-round"]),
    ("cadenas mot passe cle", &["lock", "key-round"]),
    ("message discussion conversation", &["message-circle", "messages-square", "message-square"]),
    ("courriel email lettre", &["mail", "inbox", "send"]),
    ("calendrier agenda planning", &["calendar", "calendar-days", "clock"]),
    ("horloge temps minuteur", &["clock", "timer", "hourglass"]),
    ("photo appareil", &["camera", "image", "aperture"]),
    ("image dessin design graphisme", &["image", "palette", "pen-tool", "brush"]),
    ("video film cinema montage", &["video", "film", "clapperboard"]),
    ("sante medecine soin", &["heart-pulse", "stethoscope", "pill", "activity"]),
    ("sport musculation forme", &["dumbbell", "bike", "trophy", "activity"]),
    ("coeur amour favori", &["heart", "star"]),
    ("nature plante jardin ecologie", &["leaf", "sprout", "trees", "flower-2"]),
    ("voiture transport vehicule", &["car", "bus", "bike", "truck"]),
    ("voyage vacances avion", &["plane", "luggage", "map"]),
    ("carte plan lieu localisation", &["map", "map-pin", "compass", "navigation"]),
    ("cuisine recette repas", &["chef-hat", "utensils", "cooking-pot"]),
    ("cafe", &["coffee"]),
    ("equipe personnes utilisateur client", &["users", "user", "contact"]),
    ("document fichier texte", &["file-text", "file", "files"]),
    ("note carnet ecriture", &["notebook-pen", "sticky-note", "notebook", "pen-line", "pencil"]),
    ("paquet colis librairie module", &["package", "box", "boxes"]),
    ("couches pile", &["layers"]),
    ("etoile", &["star"]),
    ("science laboratoire chimie experience", &["flask-conical", "atom", "microscope"]),
    ("reseau connexion", &["network", "wifi", "share-2"]),
    ("parametres reglages configuration", &["settings", "settings-2", "sliders-horizontal"]),
    ("recherche loupe", &["search"]),
    ("liste tache todo", &["list-checks", "list-todo", "square-check"]),
    ("ampoule", &["lightbulb"]),
    ("trophee victoire recompense", &["trophy", "medal", "award"]),
    ("drapeau", &["flag"]),
    ("cible objectif", &["target", "crosshair"]),
    ("eclair rapide energie", &["zap"]),
    ("feu flamme", &["flame"]),
    ("monde terre planete", &["globe", "earth"]),
    ("langue traduction", &["languages"]),
    ("animal chien chat", &["dog", "cat", "paw-print"]),
    ("meteo soleil", &["sun", "cloud-sun"]),
    ("lune nuit", &["moon"]),
    ("electricite batterie prise", &["battery", "plug", "zap"]),
    ("sauvegarde disque stockage", &["hard-drive", "save", "archive"]),
    ("imprimante impression", &["printer"]),
    ("boussole", &["compass"]),
];

/// Les icônes qui répondent à une recherche, en français (« maison ») ou par leur nom Lucide
/// (« house »). Sans recherche : les suggestions.
pub fn search(query: &str) -> Vec<Icon> {
    let resolve = |names: &[&str]| names.iter().filter_map(|n| Icon::try_from(*n).ok()).collect::<Vec<_>>();
    let q = crate::search::fold(query.trim());
    if q.is_empty() {
        return resolve(SUGGESTED);
    }
    let mut out: Vec<Icon> = Vec::new();
    let mut push = |i: Icon| {
        if !out.iter().any(|x| same(*x, i)) {
            out.push(i);
        }
    };
    let starts = |words: &str, q: &str| words.split([' ', '-']).any(|w| w.starts_with(q));
    for (words, names) in FRENCH {
        if starts(words, &q) {
            resolve(names).into_iter().for_each(&mut push);
        }
    }
    let q = q.replace(' ', "-");
    // Noms Lucide : un mot qui commence par la recherche, puis la recherche n'importe où.
    all().iter().filter(|(_, n)| n.starts_with(q.as_str()) || starts(n, &q)).for_each(|(i, _)| push(*i));
    all().iter().filter(|(_, n)| n.contains(q.as_str())).for_each(|(i, _)| push(*i));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_complet_et_sans_faute() {
        assert!(all().len() > 1500, "{} icônes seulement", all().len());
        assert!(all().iter().any(|(i, _)| same(*i, Icon::House)));
        // Chaque nom des tables doit exister dans Lucide (une faute de frappe passerait inaperçue).
        let unknown: Vec<&str> = SUGGESTED.iter().chain(FRENCH.iter().flat_map(|(_, n)| n.iter())).copied().filter(|n| Icon::try_from(*n).is_err()).collect();
        assert!(unknown.is_empty(), "noms inconnus : {unknown:?}");
    }

    #[test]
    fn recherche_en_francais_et_en_anglais() {
        assert_eq!(search("").len(), SUGGESTED.len());
        assert!(same(search("maison")[0], Icon::House));
        assert!(same(search("Fusée")[0], Icon::Rocket), "accents et majuscules tolérés");
        assert!(search("house").iter().any(|i| same(*i, Icon::House)));
        assert!(search("gamepad").iter().any(|i| same(*i, Icon::Gamepad2)));
        assert!(search("zzzz").is_empty());
    }
}
