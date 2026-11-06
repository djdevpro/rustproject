//! Les langues de l'interface.
//!
//! Le texte source est le français, écrit tel quel dans le code : `tr("Mon bureau")` renvoie sa
//! traduction dans la langue courante, `tf("{n} projets", &[("n", &n)])` remplace en plus les
//! paramètres nommés.
//!
//! Un fichier par langue, `locales/<code>.json` : le texte français (exactement comme dans le code)
//! associé à sa traduction. Ajouter un texte, c'est ajouter la même clé dans chaque fichier ; un
//! test dit lequel en manque. Une traduction absente retombe sur l'anglais, puis sur le français.
//!
//! La langue est celle du système, sauf choix contraire dans Configuration › Apparence. Elle est
//! posée au démarrage réel de l'application ; sinon (tests, serveur MCP) c'est le français.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

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

    /// Le contenu de `locales/<code>.json` (rien pour le français, la langue du code).
    fn file(self) -> &'static str {
        match self {
            Self::Fr => "{}",
            Self::En => include_str!("../locales/en.json"),
            Self::Es => include_str!("../locales/es.json"),
            Self::De => include_str!("../locales/de.json"),
            Self::Pt => include_str!("../locales/pt.json"),
            Self::It => include_str!("../locales/it.json"),
            Self::Ru => include_str!("../locales/ru.json"),
            Self::Zh => include_str!("../locales/zh.json"),
            Self::Ja => include_str!("../locales/ja.json"),
            Self::Ko => include_str!("../locales/ko.json"),
        }
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

type Table = HashMap<String, String>;

/// Les traductions d'une langue, lues à la première utilisation.
fn table(lang: Lang) -> &'static Table {
    static TABLES: [OnceLock<Table>; 10] = [const { OnceLock::new() }; 10];
    TABLES[lang as usize].get_or_init(|| serde_json::from_str(lang.file()).unwrap_or_else(|e| panic!("locales/{}.json illisible : {e}", lang.code())))
}

/// La traduction d'un texte dans `lang` (anglais, puis français, si elle manque).
pub fn translate(fr: &str, lang: Lang) -> Option<&'static str> {
    if lang == Lang::Fr {
        return None;
    }
    let get = |l: Lang| table(l).get(fr).map(String::as_str).filter(|t| !t.is_empty());
    get(lang).or_else(|| get(Lang::En))
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
        assert_eq!(translate("un texte absent des fichiers", Lang::De), None);
        assert_eq!(tr("Mon bureau"), "Mon bureau");
        assert_eq!(tf("{n} projets", &[("n", &3)]), "3 projets");
    }

    fn params(s: &str) -> Vec<String> {
        let mut v: Vec<String> = s.split('{').skip(1).filter_map(|p| p.split_once('}').map(|(n, _)| n.to_owned())).collect();
        v.sort();
        v
    }

    /// Chaque fichier se lit, n'a pas de traduction vide, et garde les paramètres du français.
    #[test]
    fn fichiers_de_langue_coherents() {
        for lang in Lang::ALL.into_iter().skip(1) {
            for (fr, text) in table(lang) {
                assert!(!text.trim().is_empty(), "locales/{}.json : traduction vide pour « {fr} »", lang.code());
                assert_eq!(params(text), params(fr), "locales/{}.json : paramètres différents pour « {fr} »", lang.code());
            }
        }
    }

    /// Chaque texte passé à `tr(…)` ou `tf(…)` dans le code est traduit dans chaque langue.
    #[test]
    fn tous_les_textes_sont_traduits() {
        let mut keys = Vec::new();
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
                    keys.push((p.display().to_string(), rest[..end].replace("\\\"", "\"").replace("\\n", "\n")));
                }
            }
        }
        let mut missing = Vec::new();
        for lang in Lang::ALL.into_iter().skip(1) {
            for (file, key) in &keys {
                if !table(lang).contains_key(key) {
                    missing.push(format!("locales/{}.json : « {key} » ({file})", lang.code()));
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "{} traductions manquantes :\n{}", missing.len(), missing.join("\n"));
    }
}
