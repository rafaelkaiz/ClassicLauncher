//! Saved situations: the `.osn` files in OMSI's `Situations` folder (read-only). Each has a `[name]`, a `[description]` up to `[end]`
//! and the `[map]` it belongs to.

use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Situation {
    /// File name in `Situations`.
    pub file: String,
    pub name: String,
    pub description: String,
    /// Folder name of its map, e.g. "Berlin-Spandau" (from `maps\Berlin-Spandau\global.cfg`).
    pub map_folder: String,
    /// Which one it is among situations with the same name (OMSI lists them in the same order, by file name).
    pub occurrence: usize,
    /// Date the situation is set on, from its `[time]` block.
    pub date: Option<crate::chrono::Date>,
    /// Time of day the situation is set at (hour, minute, second).
    pub time: Option<(u32, u32, u32)>,
}

fn decode(b: &[u8]) -> String {
    if let Some(rest) = b.strip_prefix(&[0xFF, 0xFE]) {
        let u: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u)
    } else {
        b.iter().map(|&c| c as char).collect()
    }
}

fn parse(path: &Path) -> Option<(String, String, String)> {
    let text = decode(&std::fs::read(path).ok()?);
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
    let after = |tag: &str| lines.iter().position(|l| l.trim().eq_ignore_ascii_case(tag)).map(|i| i + 1);
    let name = lines.get(after("[name]")?)?.trim().to_string();
    let description = after("[description]")
        .map(|i| lines[i..].iter().take_while(|l| !l.trim().eq_ignore_ascii_case("[end]")).copied().collect::<Vec<_>>().join("\n"))
        .unwrap_or_default();
    let map = lines.get(after("[map]")?)?.trim().replace('/', "\\");
    let folder = map.split('\\').rev().nth(1).unwrap_or("").to_string();
    Some((name, description.trim().to_string(), folder))
}

pub fn list(omsi: &Path) -> Vec<Situation> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(omsi.join("Situations"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("osn")))
        .collect();
    files.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    let mut out: Vec<Situation> = vec![];
    for f in files {
        let Some((name, description, map_folder)) = parse(&f) else { continue };
        let occurrence = out.iter().filter(|s| s.name.eq_ignore_ascii_case(&name)).count();
        let (date, time) = match crate::chrono::situation_datetime(&f) {
            Some((d, t)) => (Some(d), Some(t)),
            None => (None, None),
        };
        out.push(Situation { file: f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), name, description, map_folder, occurrence, date, time });
    }
    out.sort_by_key(|s| (s.name.to_lowercase(), s.occurrence));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only content that OMSI 2 itself installs is used (the Berlin-Spandau situations): add-ons are not on every computer.
    #[test]
    fn reads_the_real_situations() {
        let omsi = Path::new(r"C:\Program Files (x86)\Steam\steamapps\common\OMSI 2");
        if !omsi.join("Situations").is_dir() {
            return;
        }
        let v = list(omsi);
        // no situation of Berlin-Spandau (a modified installation): nothing to compare with
        let Some(s) = v.iter().find(|s| s.name == "Mauerfall" && s.map_folder == "Berlin-Spandau") else { return };
        assert!(s.description.contains("durchlässig"), "umlauts must be read");
        assert!(!s.description.contains("[end]"));
        assert!(s.date.is_some() && s.time.is_some());
        if let Some(f) = v.iter().find(|s| s.name == "Nur für Fortgeschrittene") {
            assert_eq!(f.map_folder, "Berlin-Spandau");
        }
    }
}
