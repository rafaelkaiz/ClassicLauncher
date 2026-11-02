//! openOMSI's settings file, `~/.openomsi/settings.cfg`: `key=value` lines. Lines this launcher does not edit (comments, keys it has no
//! row for, other versions' keys) are kept exactly as they were, in their place.

use std::path::PathBuf;

pub struct GameCfg {
    pub path: PathBuf,
    lines: Vec<String>,
    /// The file as it was is copied to `.bak` the first time it is written in this session, not at every change.
    backed_up: std::cell::Cell<bool>,
}

pub fn path() -> Option<PathBuf> {
    crate::omsi::data_dir().map(|d| d.join("settings.cfg"))
}

impl GameCfg {
    /// The file as it is now; a missing file is an empty one (the game fills in its defaults).
    pub fn load() -> Option<Self> {
        let path = path()?;
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        Some(Self::parse(path, &text))
    }

    pub fn parse(path: PathBuf, text: &str) -> Self {
        Self { path, lines: text.lines().map(str::to_string).collect(), backed_up: std::cell::Cell::new(false) }
    }

    fn find(&self, key: &str) -> Option<usize> {
        // the last one counts, as the game reads the file top to bottom
        self.lines.iter().rposition(|l| l.split_once('=').is_some_and(|(k, _)| k.trim() == key))
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.find(key).and_then(|i| self.lines[i].split_once('=')).map(|(_, v)| v.trim().to_string())
    }

    pub fn set(&mut self, key: &str, value: &str) {
        let line = format!("{key}={value}");
        match self.find(key) {
            Some(i) => self.lines[i] = line,
            None => self.lines.push(line),
        }
    }

    pub fn flag(&self, key: &str, def: bool) -> bool {
        match self.get(key).as_deref() {
            Some("1") | Some("true") | Some("on") => true,
            Some("0") | Some("false") | Some("off") => false,
            _ => def,
        }
    }

    pub fn text(&self) -> String {
        let mut t = self.lines.join("\n");
        t.push('\n');
        t
    }

    /// Writes the file, after copying the old one to `settings.cfg.bak`.
    pub fn save(&self) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        if self.path.is_file() && !self.backed_up.replace(true) {
            let _ = std::fs::copy(&self.path, self.path.with_extension("cfg.bak"));
        }
        std::fs::write(&self.path, self.text()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_keep_the_other_lines_in_place() {
        let mut c = GameCfg::parse("x".into(), "# comment\nversion=4\nmsaa=4\nunknown_key=7\nvsync=1\n");
        c.set("msaa", "8");
        c.set("clouds", "0");
        assert_eq!(c.text(), "# comment\nversion=4\nmsaa=8\nunknown_key=7\nvsync=1\nclouds=0\n");
        assert!(!c.flag("clouds", true) && c.flag("missing", true));
    }
}
