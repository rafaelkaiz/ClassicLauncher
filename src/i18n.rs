//! Launcher translations. The English text of each string is its key (the same words openOMSI's own settings use); `tr` returns the
//! text in the current language and falls back to English when a string has no translation. Strings with `{}` are templates
//! filled in with `trf`.
//!
//! Spanish is built in. Any other language comes from a file
//! `%APPDATA%\ClassicLauncher\lang\<CODE>.txt`; `write_template` creates one to translate.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, RwLock};

/// A language, named by the three letters OMSI and openOMSI use for it (`language=ESP` in settings.cfg).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Lang(&'static str);

#[allow(non_upper_case_globals)]
impl Lang {
    pub const En: Lang = Lang("ENG");
    pub const Es: Lang = Lang("ESP");
    pub const De: Lang = Lang("DEU");
    pub const Pl: Lang = Lang("POL");
    pub const Cs: Lang = Lang("CZE");
    pub const Fr: Lang = Lang("FRA");
    pub const Pt: Lang = Lang("POR");
    pub const Hu: Lang = Lang("HUN");
    pub const Sr: Lang = Lang("SRB");
    pub const BUILTIN: [Lang; 9] = [Lang::En, Lang::Es, Lang::De, Lang::Pl, Lang::Cs, Lang::Fr, Lang::Pt, Lang::Hu, Lang::Sr];

    /// Column of the table of the other built-in languages; `None` for English (the key) and Spanish (its own table).
    fn column(self) -> Option<usize> {
        match self.0 {
            "DEU" => Some(0),
            "POL" => Some(1),
            "CZE" => Some(2),
            "FRA" => Some(3),
            "POR" => Some(4),
            _ => None,
        }
    }

    /// Column of the Hungarian and Serbian table (`texts_hs.rs`).
    fn column_hs(self) -> Option<usize> {
        match self.0 {
            "HUN" => Some(0),
            "SRB" => Some(1),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        self.0
    }

    /// The language with that code; two-letter codes are understood too.
    pub fn from_code(s: &str) -> Lang {
        let s = s.trim().to_uppercase();
        let full = match s.as_str() {
            "ES" => "ESP",
            "EN" => "ENG",
            "DE" => "DEU",
            "PL" => "POL",
            "CS" => "CZE",
            "FR" => "FRA",
            "PT" => "POR",
            "HU" => "HUN",
            "SR" => "SRB",
            other => other,
        };
        if full.is_empty() {
            return Lang::En;
        }
        Lang::BUILTIN.into_iter().find(|l| l.0 == full).unwrap_or_else(|| Lang(intern(full)))
    }

    /// Name of the language in that language.
    pub fn name(self) -> &'static str {
        match self.0 {
            "ESP" => "Español",
            "ENG" => "English",
            "DEU" => "Deutsch",
            "POL" => "Polski",
            "CZE" => "Čeština",
            "FRA" => "Français",
            "POR" => "Português",
            "HUN" => "Magyar",
            "SRB" => "Srpski",
            code => external_name(code).unwrap_or(code),
        }
    }
}

fn intern(code: &str) -> &'static str {
    static CODES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut codes = CODES.lock().unwrap();
    if let Some(c) = codes.iter().find(|c| **c == code) {
        return c;
    }
    let leaked: &'static str = Box::leak(code.to_string().into_boxed_str());
    codes.push(leaked);
    leaked
}

type Texts = HashMap<&'static str, &'static str>;

struct Current {
    lang: Lang,
    /// Texts read from the language's file in the `lang` folder; they win over the built-in ones.
    external: Texts,
}

fn current() -> &'static RwLock<Current> {
    static C: OnceLock<RwLock<Current>> = OnceLock::new();
    C.get_or_init(|| RwLock::new(Current { lang: Lang::En, external: Texts::new() }))
}

fn names() -> &'static Mutex<HashMap<String, &'static str>> {
    static N: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    N.get_or_init(|| Mutex::new(HashMap::new()))
}

fn external_name(code: &str) -> Option<&'static str> {
    if let Some(n) = names().lock().unwrap().get(code) {
        return Some(n);
    }
    let (_, name) = read_external(code);
    let name: &'static str = Box::leak(name?.into_boxed_str());
    names().lock().unwrap().insert(code.to_string(), name);
    Some(name)
}

/// Where the translation files are kept: `%APPDATA%\ClassicLauncher\lang`.
pub fn lang_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("ClassicLauncher").join("lang")
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n")
}

/// Reads `<CODE>.txt`: `@name=` gives the language's own name, then pairs of lines, `> original (English)` and `= translation`.
fn read_external(code: &str) -> (Texts, Option<String>) {
    let mut texts = Texts::new();
    let mut name = None;
    let Ok(bytes) = std::fs::read(lang_dir().join(format!("{code}.txt"))) else { return (texts, name) };
    let text = String::from_utf8_lossy(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes)).into_owned();
    let mut pending: Option<&'static str> = None;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(n) = line.strip_prefix("@name=") {
            name = Some(n.trim().to_string());
        } else if let Some(k) = line.strip_prefix("> ") {
            let k = unescape(k);
            pending = crate::texts::all().iter().find(|(key, _)| *key == k).map(|(key, _)| *key);
        } else if let Some(v) = line.strip_prefix("= ") {
            if let (Some(k), false) = (pending.take(), v.trim().is_empty()) {
                texts.insert(k, Box::leak(unescape(v).into_boxed_str()) as &'static str);
            }
        }
    }
    (texts, name)
}

pub fn set_lang(l: Lang) {
    let (external, _) = read_external(l.code());
    *current().write().unwrap() = Current { lang: l, external };
}

pub fn lang() -> Lang {
    current().read().unwrap().lang
}

fn spanish() -> &'static HashMap<&'static str, &'static str> {
    static T: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    T.get_or_init(|| crate::texts::all().iter().copied().filter(|(_, es)| !es.is_empty()).collect())
}

/// The text for `key` in the current language.
pub fn tr(key: &str) -> &str {
    let cur = current().read().unwrap();
    if let Some(s) = cur.external.get(key) {
        return *s;
    }
    if cur.lang == Lang::Es {
        if let Some(s) = spanish().get(key) {
            return *s;
        }
    }
    if let Some(c) = cur.lang.column() {
        if let Some(row) = more().get(key) {
            return row[c];
        }
    }
    if let Some(c) = cur.lang.column_hs() {
        if let Some(row) = hs().get(key) {
            return row[c];
        }
    }
    key
}

/// The codes of the languages found in OMSI's `Languages` folder (a `<CODE>_basic.olf` file each), in alphabetical order. Ruhrdeutsch (RUH) is left out.
pub fn omsi_languages(omsi: &Path) -> Vec<Lang> {
    let mut codes: Vec<String> = std::fs::read_dir(omsi.join("Languages"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_uppercase();
            let code = name.strip_suffix("_BASIC.OLF")?;
            (code.len() == 3 && code != "RUH" && code.chars().all(|c| c.is_ascii_alphabetic())).then(|| code.to_string())
        })
        .collect();
    codes.sort();
    codes.dedup();
    codes.iter().map(|c| Lang::from_code(c)).collect()
}

/// Every language the launcher can be shown in: its built-in ones and those with a file in the `lang` folder; with a folder given, also the ones OMSI has (used by the options window for the game's own language).
pub fn available(omsi: Option<&Path>) -> Vec<Lang> {
    let mut all: Vec<Lang> = Lang::BUILTIN.to_vec();
    if let Some(o) = omsi {
        all.extend(omsi_languages(o));
    }
    if let Ok(rd) = std::fs::read_dir(lang_dir()) {
        for e in rd.flatten() {
            let file = e.file_name().to_string_lossy().to_uppercase();
            if let Some(code) = file.strip_suffix(".TXT") {
                if code.len() == 3 && code.chars().all(|c| c.is_ascii_alphabetic()) {
                    all.push(Lang::from_code(code));
                }
            }
        }
    }
    all.sort_by_key(|l| l.name().to_lowercase());
    all.dedup();
    all
}

/// Writes `lang\TEMPLATE.txt`: every text of the launcher with its English version, ready to be translated.
pub fn write_template() -> Result<PathBuf, String> {
    let dir = lang_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut out = String::from(
        "# Translation of ClassicLauncher. Translate the lines that start with \"= \" (they hold the English text now).\n\
         # Save the file as <CODE>.txt with the same three letters OMSI uses for the language (CHN.txt for Languages\\CHN_basic.olf),\n\
         # in UTF-8. Lines that start with \"> \" are the original keys: do not change them. Keep every {} where it is.\n\
         # A line left as it is, or removed, simply shows the English text.\n\
         @name=The name of the language, written in that language\n\n",
    );
    for (key, _) in crate::texts::all() {
        out.push_str(&format!("> {}\n= {}\n\n", escape(key), escape(key)));
    }
    let path = dir.join("TEMPLATE.txt");
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok(path)
}

/// `tr` for a template: each `{}` is replaced, in order, by the next argument.
pub fn trf(key: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut rest = tr(key);
    for a in args {
        match rest.split_once("{}") {
            Some((head, tail)) => {
                out.push_str(head);
                out.push_str(a);
                rest = tail;
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_fill_in_order() {
        assert_eq!(trf("Map: {}", &["Grundorf"]), "Map: Grundorf");
    }
}

fn more() -> &'static HashMap<&'static str, [&'static str; 5]> {
    static T: OnceLock<HashMap<&'static str, [&'static str; 5]>> = OnceLock::new();
    T.get_or_init(|| crate::texts_more::MORE.iter().copied().collect())
}

fn hs() -> &'static HashMap<&'static str, [&'static str; 2]> {
    static T: OnceLock<HashMap<&'static str, [&'static str; 2]>> = OnceLock::new();
    T.get_or_init(|| crate::texts_hs::HS.iter().copied().collect())
}

#[cfg(test)]
mod hs_tests {
    use super::*;

    #[test]
    fn hungarian_and_serbian_translate() {
        let _g = crate::i18n::LANG_TESTS.lock().unwrap_or_else(|e| e.into_inner());
        set_lang(Lang::Hu);
        assert_eq!(tr("Choose a bus"), "Válassz buszt");
        set_lang(Lang::Sr);
        assert_eq!(tr("Choose a bus"), "Izaberi autobus");
        assert_eq!(tr("A text nobody translated"), "A text nobody translated");
        set_lang(Lang::En);
    }

    /// Every row translates a text the other table also has, and keeps its placeholders.
    #[test]
    fn every_row_matches_the_other_table() {
        let more: std::collections::HashSet<&str> = crate::texts_more::MORE.iter().map(|(k, _)| *k).collect();
        for (key, row) in crate::texts_hs::HS {
            assert!(more.contains(key), "{key}");
            for t in row {
                assert_eq!(key.matches("{}").count(), t.matches("{}").count(), "{key} / {t}");
            }
        }
        assert_eq!(crate::texts_hs::HS.len(), crate::texts_more::MORE.len());
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;

    #[test]
    fn the_other_built_in_languages_translate_and_fall_back_to_english() {
        let _g = crate::i18n::LANG_TESTS.lock().unwrap_or_else(|e| e.into_inner());
        set_lang(Lang::De);
        assert_eq!(tr("Choose a bus"), "Bus auswählen");
        assert_eq!(tr("Check now"), "Jetzt prüfen");
        assert_eq!(tr("A text nobody translated"), "A text nobody translated");
        set_lang(Lang::Fr);
        assert_eq!(trf("Update to {}...", &["0.3.47"]), "Mettre à jour vers 0.3.47...");
        set_lang(Lang::En);
    }

    #[test]
    fn every_row_keeps_its_placeholders() {
        for (key, row) in crate::texts_more::MORE {
            for t in row {
                assert_eq!(key.matches("{}").count(), t.matches("{}").count(), "{key} / {t}");
            }
        }
    }
}

/// The language is one global: the tests that change it take turns.
#[cfg(test)]
pub static LANG_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());
