//! Les langues de l'interface.
//!
//! Le texte source est le français, écrit tel quel dans le code : `tr("Mon bureau")` renvoie sa
//! traduction dans la langue courante, `tf("{n} projets", &[("n", &n)])` remplace en plus les
//! paramètres nommés. Toutes les traductions sont dans `locales/ui.tsv` : une ligne par texte, une
//! colonne par langue (fr, en, es…). Une cellule vide retombe sur l'anglais, puis sur le français.
//!
//! La langue est celle du système, sauf choix contraire dans Configuration › Apparence. Elle est
//! posée au démarrage réel de l'application ; sinon (tests, serveur MCP) c'est le français.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::fmt::Display;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lang {
    Fr,
    En,
    Es,
    De,
    Pt,
    It,
    Ru,
    Zh,
    Ja,
    Ko,
}

impl Lang {
    /// Dans l'ordre des colonnes de `locales/ui.tsv`.
    pub const ALL: [Self; 10] = [Self::Fr, Self::En, Self::Es, Self::De, Self::Pt, Self::It, Self::Ru, Self::Zh, Self::Ja, Self::Ko];

    pub fn code(self) -> &'static str {
        ["fr", "en", "es", "de", "pt", "it", "ru", "zh", "ja", "ko"][self as usize]
    }

    /// Le nom de la langue, dans la langue elle-même.
    pub fn native(self) -> &'static str {
        ["Français", "English", "Español", "Deutsch", "Português", "Italiano", "Русский", "中文", "日本語", "한국어"][self as usize]
    }

    /// `fr-FR`, `en_US.UTF-8`, `zh-Hans-CN`… → la langue, si elle est prise en charge.
    pub fn from_locale(locale: &str) -> Option<Self> {
        let code = locale.split(['-', '_', '.']).next()?.to_lowercase();
        Self::ALL.into_iter().find(|l| l.code() == code)
    }
}

/// La langue du système (anglais si elle n'est pas prise en charge).
pub fn detect() -> Lang {
    sys_locale::get_locale().as_deref().and_then(Lang::from_locale).unwrap_or(Lang::En)
}

static CURRENT: AtomicU8 = AtomicU8::new(Lang::Fr as u8);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    Lang::ALL[usize::from(CURRENT.load(Ordering::Relaxed)).min(Lang::ALL.len() - 1)]
}

const TABLE: &str = include_str!("../locales/ui.tsv");

/// Texte français → traductions (une par langue, dans l'ordre de `Lang::ALL`). Dans le fichier,
/// un retour à la ligne s'écrit `\n` (deux caractères).
fn table() -> &'static HashMap<String, Vec<String>> {
    static MAP: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    MAP.get_or_init(|| {
        TABLE
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let cells: Vec<String> = l.split('\t').map(|c| c.replace("\\n", "\n")).collect();
                (cells[0].clone(), cells)
            })
            .collect()
    })
}

/// La traduction d'un texte dans `lang` (anglais, puis français, si elle manque).
pub fn translate(fr: &str, lang: Lang) -> Option<&'static str> {
    if lang == Lang::Fr {
        return None;
    }
    let row = table().get(fr)?;
    let cell = |l: Lang| row.get(l as usize).map(String::as_str).filter(|c| !c.is_empty());
    cell(lang).or_else(|| cell(Lang::En))
}

/// Le texte dans la langue courante.
pub fn tr(fr: &str) -> &str {
    translate(fr, current()).unwrap_or(fr)
}

/// Le texte dans la langue courante, avec ses paramètres nommés (`{n}`, `{nom}`…) remplacés.
pub fn tf(fr: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut out = tr(fr).to_owned();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), &value.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn langue_du_systeme() {
        assert_eq!(Lang::from_locale("fr-FR"), Some(Lang::Fr));
        assert_eq!(Lang::from_locale("pt_BR.UTF-8"), Some(Lang::Pt));
        assert_eq!(Lang::from_locale("zh-Hans-CN"), Some(Lang::Zh));
        assert_eq!(Lang::from_locale("hi-IN"), None, "pas prise en charge : anglais");
    }

    #[test]
    fn traduire_et_parametres() {
        // Sans changer la langue globale : les autres tests tournent en même temps, en français.
        assert_eq!(translate("Mon bureau", Lang::En), Some("My desk"));
        assert_eq!(translate("Mon bureau", Lang::Fr), None);
        assert_eq!(translate("un texte absent de la table", Lang::De), None);
        assert_eq!(tr("Mon bureau"), "Mon bureau");
        assert_eq!(tf("{n} projets", &[("n", &3)]), "3 projets");
    }

    /// Chaque ligne a une colonne par langue, et les mêmes paramètres partout.
    #[test]
    fn table_complete_et_coherente() {
        let header: Vec<&str> = TABLE.lines().next().unwrap().split('\t').collect();
        assert_eq!(header, Lang::ALL.map(Lang::code));
        let params = |s: &str| {
            let mut v: Vec<String> = s.split('{').skip(1).filter_map(|p| p.split_once('}').map(|(n, _)| n.to_owned())).collect();
            v.sort();
            v
        };
        for (n, line) in TABLE.lines().enumerate().skip(1).filter(|(_, l)| !l.trim().is_empty()) {
            let cells: Vec<&str> = line.split('\t').collect();
            assert_eq!(cells.len(), Lang::ALL.len(), "ligne {} : {} colonnes", n + 1, cells.len());
            for (l, cell) in cells.iter().enumerate().skip(1) {
                assert!(!cell.is_empty(), "ligne {} : traduction {} manquante", n + 1, Lang::ALL[l].code());
                assert_eq!(params(cell), params(cells[0]), "ligne {} ({}) : paramètres différents", n + 1, Lang::ALL[l].code());
            }
        }
    }

    /// Chaque texte passé à `tr(…)` ou `tf(…)` dans le code a sa ligne dans la table.
    #[test]
    fn tous_les_textes_sont_traduits() {
        let mut missing = Vec::new();
        let mut files = vec![std::path::PathBuf::from("src")];
        while let Some(p) = files.pop() {
            if p.is_dir() {
                files.extend(std::fs::read_dir(&p).unwrap().flatten().map(|e| e.path()));
                continue;
            }
            if p.extension().is_none_or(|e| e != "rs") || p.ends_with("i18n.rs") {
                continue;
            }
            let src = std::fs::read_to_string(&p).unwrap();
            for start in ["tr(\"", "tf(\""] {
                for (i, _) in src.match_indices(start) {
                    // `tr(` seul, pas `str(` ni `attr(`.
                    if src[..i].chars().last().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                        continue;
                    }
                    let rest = &src[i + start.len()..];
                    let Some(end) = rest.find('"') else { continue };
                    let key = rest[..end].replace("\\\"", "\"").replace("\\n", "\n");
                    if !table().contains_key(key.as_str()) {
                        missing.push(format!("{} : {key}", p.display()));
                    }
                }
            }
        }
        assert!(missing.is_empty(), "{} textes sans traduction :\n{}", missing.len(), missing.join("\n"));
    }
}
