//! The bus radios' stations: `~/.openomsi/radio.cfg`, one `name = address` line each (an MP3, AAC or Ogg stream, or an .m3u/.pls
//! playlist). The file's comments and its `volume =` line are kept as they are when the stations are saved.

use std::path::PathBuf;

pub fn path() -> Option<PathBuf> {
    crate::omsi::data_dir().map(|d| d.join("radio.cfg"))
}

fn is_station(line: &str) -> Option<(String, String)> {
    let l = line.trim();
    if l.is_empty() || l.starts_with('#') {
        return None;
    }
    let (name, address) = l.split_once('=')?;
    let name = name.trim();
    (!name.eq_ignore_ascii_case("volume")).then(|| (name.to_string(), address.trim().to_string()))
}

pub fn stations_of(text: &str) -> Vec<(String, String)> {
    text.lines().filter_map(is_station).collect()
}

/// `text` with its stations replaced by `list`: the other lines stay, the stations follow them.
pub fn with_stations(text: &str, list: &[(String, String)]) -> String {
    let mut out: Vec<String> = text.lines().filter(|l| is_station(l).is_none()).map(str::to_string).collect();
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    for (n, a) in list {
        if !n.trim().is_empty() || !a.trim().is_empty() {
            out.push(format!("{} = {}", n.trim(), a.trim()));
        }
    }
    out.join("\n") + "\n"
}

pub fn load() -> Vec<(String, String)> {
    path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| stations_of(&t)).unwrap_or_default()
}

pub fn save(list: &[(String, String)]) -> Result<(), String> {
    let p = path().ok_or("no home folder")?;
    let text = std::fs::read_to_string(&p).unwrap_or_default();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, with_stations(&text, list)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stations_are_replaced_and_the_rest_of_the_file_kept() {
        let text = "# comment\nvolume = 0.7\nA = http://a\nB = http://b\n";
        assert_eq!(stations_of(text), [("A".to_string(), "http://a".to_string()), ("B".to_string(), "http://b".to_string())]);
        let new = with_stations(text, &[("C".into(), "http://c".into()), ("".into(), "".into())]);
        assert_eq!(new, "# comment\nvolume = 0.7\nC = http://c\n");
    }
}
