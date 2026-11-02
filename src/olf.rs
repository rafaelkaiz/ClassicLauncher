//! OMSI's own text tables (`Languages\<LANG>_basic*.olf`, lines of `KEY<TAB>text`), read-only. Used for the staff-file window so its
//! labels are exactly the ones OMSI shows. English is loaded first and the chosen language is laid over it.

use crate::i18n::Lang;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

/// The languages OMSI has (`Languages\<CODE>_basic.olf`), found once at start with `init`.
static OMSI_LANGS: OnceLock<Vec<Lang>> = OnceLock::new();

pub fn init(omsi: &Path) {
    let _ = OMSI_LANGS.set(crate::i18n::omsi_languages(omsi));
}

/// OMSI's language code for a launcher language: its own when OMSI has texts in it, English otherwise (OMSI has no Czech or
/// Portuguese, for instance).
pub fn omsi_code(l: Lang) -> &'static str {
    match OMSI_LANGS.get() {
        Some(known) if !known.is_empty() => {
            if known.contains(&l) {
                l.code()
            } else {
                "ENG"
            }
        }
        _ => match l.code() {
            "ESP" | "DEU" | "POL" | "FRA" => l.code(),
            _ => "ENG",
        },
    }
}

fn decode(b: &[u8]) -> String {
    if let Some(rest) = b.strip_prefix(&[0xFF, 0xFE]) {
        let u: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u)
    } else if let Ok(s) = std::str::from_utf8(b) {
        s.to_string() // some of the key texts are UTF-8 without a mark
    } else {
        b.iter().map(|&c| cp1252(c)).collect() // Windows-1252 (ISO-8859-1 plus the curly quotes, dashes and so on at 0x80-0x9F)
    }
}

fn cp1252(c: u8) -> char {
    match c {
        0x80 => '€',
        0x82 => '‚',
        0x83 => 'ƒ',
        0x84 => '„',
        0x85 => '…',
        0x86 => '†',
        0x87 => '‡',
        0x88 => 'ˆ',
        0x89 => '‰',
        0x8A => 'Š',
        0x8B => '‹',
        0x8C => 'Œ',
        0x8E => 'Ž',
        0x91 => '‘',
        0x92 => '’',
        0x93 => '“',
        0x94 => '”',
        0x95 => '•',
        0x96 => '–',
        0x97 => '—',
        0x98 => '˜',
        0x99 => '™',
        0x9A => 'š',
        0x9B => '›',
        0x9C => 'œ',
        0x9E => 'ž',
        0x9F => 'Ÿ',
        _ => c as char,
    }
}

fn load_into(map: &mut HashMap<String, String>, dir: &Path, code: &str) {
    for suffix in ["basic", "basic_103", "basic_200", "key_veh_gen", "key_veh_gen_103", "key_game", "key_200"] {
        let Ok(bytes) = std::fs::read(dir.join(format!("{code}_{suffix}.olf"))) else { continue };
        for line in decode(&bytes).lines() {
            if let Some((k, v)) = line.split_once('\t') {
                map.insert(k.trim().to_string(), v.trim_end_matches('\r').to_string());
            }
        }
    }
}

pub fn load(omsi: &Path, lang: Lang) -> HashMap<String, String> {
    let dir = omsi.join("Languages");
    let mut map = HashMap::new();
    load_into(&mut map, &dir, "ENG");
    let code = omsi_code(lang);
    if code != "ENG" {
        load_into(&mut map, &dir, code);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_staff_file_labels_in_each_encoding() {
        let omsi = Path::new(r"C:\Program Files (x86)\Steam\steamapps\common\OMSI 2");
        if !omsi.join("Languages").is_dir() {
            return;
        }
        assert_eq!(load(omsi, Lang::Es)["DG_Driver"], "Expediente del personal");
        assert!(load(omsi, Lang::En)["DG_Driver"].to_lowercase().contains("driver") || load(omsi, Lang::En).contains_key("DG_Driver"));
        let pl = load(omsi, Lang::from_code("POL"));
        assert!(!pl["DG_Driver"].is_empty() && pl["DG_Driver"] != load(omsi, Lang::En)["DG_Driver"]);
        assert!(load(omsi, Lang::from_code("FRA"))["DG_Driver_Busstops_Late"].starts_with("Arriv"));
    }
}
