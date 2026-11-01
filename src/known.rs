//! Which settings the installed game knows. The game reports them (`--cli settings`); failing that its launcher program carries the names of its settings as text (`key=` pieces
//! of the lines it writes), so a row of the options window is shown only when its key is there: an older openOMSI does not get
//! rows for settings it ignores. When nothing readable is found every row is shown.

use std::collections::HashSet;
use std::path::Path;

/// Programs of the game to look in, in order; the first one that holds enough of this launcher's keys is used.
const PROGRAMS: [&str; 3] = ["openomsi-launcher-app.exe", "openomsi-launcher.exe", "openomsi.exe"];

/// Fewer matches than this and the file is taken as not being what was expected (nothing is hidden).
const ENOUGH: usize = 60;

/// The words of `bytes` that are followed by `=`: `[a-z0-9_]` runs of at least 2 characters.
fn words(bytes: &[u8]) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' {
            start.get_or_insert(i);
        } else {
            if b == b'=' {
                if let Some(s) = start {
                    if i - s >= 2 && i - s <= 48 {
                        out.insert(String::from_utf8_lossy(&bytes[s..i]).into_owned());
                    }
                }
            }
            start = None;
        }
    }
    out
}

/// The settings the game itself reports (`openomsi-launcher-app.exe --cli settings`: every setting of that version with its value).
/// Only asked when the program has the `--cli` command (an older one would open its window instead); gives up after a few seconds.
fn from_cli(dir: &Path) -> Option<HashSet<String>> {
    use std::os::windows::process::CommandExt;
    use std::io::Read;
    use std::process::{Command, Stdio};
    let exe = dir.join("openomsi-launcher-app.exe");
    let bytes = std::fs::read(&exe).ok()?;
    // (the names of its commands are in the program as text; the program that has none is not asked)
    if !bytes.windows(14).any(|w| w == b"ai_lists_reset") {
        return None;
    }
    let mut child = Command::new(&exe).args(["--cli", "settings"]).stdout(Stdio::piped()).stderr(Stdio::null()).stdin(Stdio::null()).creation_flags(0x0800_0000).spawn().ok()?;
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) if start.elapsed() > std::time::Duration::from_secs(5) => {
                let _ = child.kill();
                return None;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
    let text = reader.join().ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(v.as_object()?.keys().cloned().collect())
}

/// The keys among `wanted` that the game at `game` (its `openomsi.exe`) knows: what the game reports itself, else the names found in
/// its programs as text; `None` when they cannot be told.
pub fn known(game: &Path, wanted: &[&str]) -> Option<HashSet<String>> {
    let dir = game.parent()?;
    if let Some(from_game) = from_cli(dir) {
        let hits: HashSet<String> = wanted.iter().filter(|k| from_game.contains(**k)).map(|k| k.to_string()).collect();
        if hits.len() >= ENOUGH {
            return Some(hits);
        }
    }
    for p in PROGRAMS {
        let Ok(bytes) = std::fs::read(dir.join(p)) else { continue };
        let found = words(&bytes);
        let hits: HashSet<String> = wanted.iter().filter(|k| found.contains(**k)).map(|k| k.to_string()).collect();
        if hits.len() >= ENOUGH {
            return Some(hits);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_words_before_an_equals_sign() {
        let w = words(b"\0triple_fov_deg=\0\0view_distance=1200\0ab=1\0not a key");
        assert!(w.contains("triple_fov_deg") && w.contains("view_distance"));
        assert!(!w.contains("a") && !w.contains("not"));
    }

    /// With a game installed: its own settings are found, and the page of the game has none this launcher misses.
    #[test]
    fn the_installed_game_knows_most_of_the_rows() {
        let Some(game) = crate::game::find_game(None) else { return };
        let keys = crate::schema::keys();
        let Some(k) = known(&game, &keys) else { return };
        eprintln!("known: {} of {}; unknown: {:?}", k.len(), keys.len(), keys.iter().filter(|x| !k.contains(**x)).collect::<Vec<_>>());
        assert!(k.contains("view_distance") && k.contains("msaa"));
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    /// With a game installed whose program has the `--cli` command: it answers with its settings.
    #[test]
    fn the_game_reports_its_settings() {
        let Some(game) = crate::game::find_game(None) else { return };
        let Some(dir) = game.parent() else { return };
        let Some(keys) = from_cli(dir) else { return eprintln!("--cli not available: the names in the program are used") };
        eprintln!("the game reports {} settings", keys.len());
        assert!(keys.contains("view_distance"));
    }
}
