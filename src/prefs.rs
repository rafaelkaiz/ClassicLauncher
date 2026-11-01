//! The launcher's own settings (not openOMSI's): `%APPDATA%\ClassicLauncher\settings.ini`.

use crate::i18n::{Lang, trf};
use crate::theme::Theme;
use std::path::PathBuf;

pub struct Prefs {
    pub theme: Theme,
    pub lang: Lang,
    pub omsi_dir: Option<PathBuf>,
    /// The `openomsi.exe` to start; found by itself when empty.
    pub game: Option<PathBuf>,
    pub close_on_start: bool,
    /// Open full screen, the interface centred like OMSI 2's start screen (F11 toggles).
    pub fullscreen: bool,
    /// What was chosen last time, by name.
    pub map: String,
    pub driver: String,
    pub traffic: u32,
    pub passengers: bool,
    pub schedule: bool,
    /// Show the command the game will be started with, above the buttons.
    pub show_command: bool,
    /// openOMSI updates: the beta channel.
    pub omsi_beta: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            lang: Lang::En,
            omsi_dir: None,
            game: None,
            close_on_start: false,
            fullscreen: false,
            map: String::new(),
            driver: String::new(),
            traffic: 30,
            passengers: true,
            schedule: true,
            show_command: false,
            omsi_beta: false,
        }
    }
}

fn file() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("ClassicLauncher").join("settings.ini")
}

/// The Windows display language when the launcher has it (Spanish), English otherwise.
fn detect_lang() -> Lang {
    let mut buf = [0u16; 85];
    let n = unsafe { windows_sys::Win32::Globalization::GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    let name = String::from_utf16_lossy(&buf[..(n.max(1) as usize - 1).min(buf.len())]).to_lowercase();
    lang_from_locale(&name)
}

fn lang_from_locale(name: &str) -> Lang {
    match name.split(['-', '_']).next().unwrap_or("") {
        "es" => Lang::Es,
        _ => Lang::En,
    }
}

impl Prefs {
    /// The first time (no file yet) the language is taken from Windows.
    pub fn load() -> Self {
        let mut s = Self::default();
        let Ok(text) = std::fs::read_to_string(file()) else {
            s.lang = detect_lang();
            return s;
        };
        for l in text.lines() {
            let Some((k, v)) = l.split_once('=') else { continue };
            let v = v.trim();
            match k.trim() {
                "theme" => s.theme = Theme::from_key(v),
                "language" => s.lang = Lang::from_code(v),
                "omsi_dir" if !v.is_empty() => s.omsi_dir = Some(PathBuf::from(v)),
                "game" if !v.is_empty() => s.game = Some(PathBuf::from(v)),
                "close_on_start" => s.close_on_start = v == "1",
                "fullscreen" => s.fullscreen = v != "0",
                "map" => s.map = v.to_string(),
                "driver" => s.driver = v.to_string(),
                "traffic" => s.traffic = v.parse().unwrap_or(30),
                "passengers" => s.passengers = v != "0",
                "schedule" => s.schedule = v != "0",
                "show_command" => s.show_command = v == "1",
                "omsi_beta" => s.omsi_beta = v == "1",
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) -> Result<(), String> {
        let path = file();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| trf("Could not save the settings: {}", &[&e.to_string()]))?;
        }
        let path_text = |p: &Option<PathBuf>| p.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
        let text = format!(
            "theme={}\nlanguage={}\nomsi_dir={}\ngame={}\nclose_on_start={}\nfullscreen={}\nmap={}\ndriver={}\ntraffic={}\npassengers={}\nschedule={}\nshow_command={}\nomsi_beta={}\n",
            self.theme.key(),
            self.lang.code(),
            path_text(&self.omsi_dir),
            path_text(&self.game),
            u8::from(self.close_on_start),
            u8::from(self.fullscreen),
            self.map,
            self.driver,
            self.traffic,
            u8::from(self.passengers),
            u8::from(self.schedule),
            u8::from(self.show_command),
            u8::from(self.omsi_beta),
        );
        std::fs::write(path, text).map_err(|e| trf("Could not save the settings: {}", &[&e.to_string()]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_windows_locales_to_launcher_languages() {
        assert_eq!(lang_from_locale("es-mx"), Lang::Es);
        assert_eq!(lang_from_locale("de-de"), Lang::En);
    }
}

#[cfg(test)]
mod save_tests {
    use super::*;

    /// "Show the command" is remembered: written to the file and read back as it was.
    #[test]
    fn show_command_is_remembered() {
        let mut p = Prefs::default();
        assert!(!p.show_command);
        p.show_command = true;
        let text = format!("show_command={}\n", u8::from(p.show_command));
        let mut back = Prefs::default();
        for l in text.lines() {
            if let Some(("show_command", v)) = l.split_once('=') {
                back.show_command = v == "1";
            }
        }
        assert!(back.show_command);
    }
}
