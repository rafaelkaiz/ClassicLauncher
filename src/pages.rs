//! The pages of openOMSI's own launcher. Its program (`openomsi-launcher-app.exe`) opens on one page alone with
//! `--page <id> --no-sidebar`; a version without those arguments would open on its first page instead, so the program is asked
//! only when it has them (their name is in it as text).

use std::path::{Path, PathBuf};

/// (the page's name in the game's launcher, the text shown here), in the order they are offered.
pub const PAGES: [(&str, &str); 9] = [
    ("settings", "Settings"),
    ("controls", "Controls"),
    ("setup", "Setup"),
    ("addons", "Add-ons"),
    ("timetable", "Timetable"),
    ("mods", "Mods"),
    ("games", "Running games"),
    ("multiplayer", "Multiplayer"),
    ("depot", "Depot"),
];

fn program(game: &Path) -> Option<PathBuf> {
    let p = game.parent()?.join("openomsi-launcher-app.exe");
    p.is_file().then_some(p)
}

/// Does the installed game's launcher open on a page given on the command line?
pub fn supported(game: &Path) -> bool {
    let Some(p) = program(game) else { return false };
    std::fs::read(p).is_ok_and(|b| b.windows(19).any(|w| w == b"__openomsiNoSidebar"))
}

/// Opens the launcher on that page, alone.
pub fn open(game: &Path, page: &str) -> Result<(), String> {
    let p = program(game).ok_or_else(|| crate::i18n::tr("openOMSI not found").to_string())?;
    std::process::Command::new(p).args(["--page", page, "--no-sidebar"]).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_pages_of_the_game() {
        let ids: Vec<&str> = PAGES.iter().map(|p| p.0).collect();
        assert!(ids.contains(&"settings") && ids.contains(&"depot") && ids.len() == 9);
    }

    #[test]
    fn a_missing_game_supports_nothing() {
        assert!(!supported(Path::new("C:/no/such/openomsi.exe")));
    }
}
