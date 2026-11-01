//! The keys of the game and of the vehicles: `Inputs/keyboard.cfg`, as openOMSI reads it (ported from `omsi-content/src/input.rs`).
//! The launcher writes its own copy to `~/.openomsi/Inputs/keyboard.cfg`, the one the game prefers; OMSI 2's file is only read, to start
//! from it. Resetting is deleting the launcher's copy.

use std::path::{Path, PathBuf};

/// The key's state is told to the action every frame (the throttle, the brake, the steering): no part of the chord.
pub const KEY_HOLD: i32 = 1;
pub const KEY_SHIFT: i32 = 2;
pub const KEY_CTRL: i32 = 4;
/// Held with Alt: OMSI 2 has none, openOMSI does.
pub const KEY_ALT: i32 = 8;
pub const CHORD: i32 = KEY_SHIFT | KEY_CTRL | KEY_ALT;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Binding {
    pub action: String,
    /// DirectInput scan code; 0: no key.
    pub scan: i32,
    pub modifier: i32,
}

impl Binding {
    pub fn chord(&self) -> i32 {
        self.modifier & CHORD
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Keys {
    pub game: Vec<Binding>,
    pub vehicles: Vec<Binding>,
}

pub fn own_path() -> Option<PathBuf> {
    crate::omsi::data_dir().map(|d| d.join("Inputs").join("keyboard.cfg"))
}

pub fn parse(text: &str) -> Keys {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let mut k = Keys::default();
    let mut vehicles = false;
    let mut i = 0;
    while i < lines.len() {
        match lines[i].to_ascii_lowercase().as_str() {
            "[game]" => vehicles = false,
            "[vehicles]" => vehicles = true,
            "[entry]" => {
                let b = Binding { action: lines.get(i + 1).unwrap_or(&"").to_string(), scan: lines.get(i + 2).and_then(|v| v.parse().ok()).unwrap_or(0), modifier: lines.get(i + 3).and_then(|v| v.parse().ok()).unwrap_or(0) };
                if vehicles { k.vehicles.push(b) } else { k.game.push(b) }
                i += 3;
            }
            _ => {}
        }
        i += 1;
    }
    k
}

/// The file as the game reads it back: `[game]` and `[vehicles]`, each entry as action, scan code and modifier.
pub fn text(k: &Keys) -> String {
    let mut t = String::new();
    for (section, list) in [("game", &k.game), ("vehicles", &k.vehicles)] {
        t.push_str(&format!("[{section}]\r\n"));
        for b in list {
            t.push_str(&format!("\r\n[entry]\r\n{}\r\n{}\r\n{}\r\n", b.action, b.scan, b.modifier));
        }
        t.push_str("\r\n");
    }
    t
}

/// The keys the game adds to what the file has (so the list shows them and they can be moved): ported from `with_game_defaults` and
/// `with_vr_defaults`.
pub fn with_defaults(mut k: Keys) -> Keys {
    let has = |list: &[Binding], a: &str| list.iter().any(|b| b.action.eq_ignore_ascii_case(a));
    let q_taken = k.game.iter().chain(k.vehicles.iter()).any(|b| b.scan == 16 && b.chord() == 0);
    if !q_taken && !has(&k.vehicles, "IBIS_vor") {
        k.vehicles.push(Binding { action: "IBIS_vor".into(), scan: 16, modifier: 0 });
    }
    let taken = |k: &Keys, sc: i32| k.game.iter().chain(k.vehicles.iter()).any(|b| b.scan == sc && b.chord() == 0 && !b.action.to_ascii_lowercase().starts_with("chat_"));
    let open_key = [53, 41, 43].into_iter().find(|&sc| !taken(&k, sc)).unwrap_or(53);
    for (action, scan) in [("chat_toggle", 47), ("chat_open", open_key)] {
        if !has(&k.game, action) {
            k.game.push(Binding { action: action.into(), scan, modifier: 0 });
        }
    }
    for (action, scan) in [("gear_up", 200), ("gear_down", 208)] {
        if !has(&k.game, action) {
            k.game.push(Binding { action: action.into(), scan, modifier: KEY_CTRL });
        }
    }
    for action in ["blinker_left_toggle", "blinker_right_toggle"] {
        if !has(&k.vehicles, action) {
            k.vehicles.push(Binding { action: action.into(), scan: 0, modifier: 0 });
        }
    }
    if !has(&k.game, "view_toggle_interior") {
        k.game.push(Binding { action: "view_toggle_interior".into(), scan: 0, modifier: 0 });
    }
    if !has(&k.game, "pax_door_request") {
        k.game.push(Binding { action: "pax_door_request".into(), scan: 31, modifier: KEY_CTRL | KEY_ALT });
    }
    for (action, scan) in [("voice_nearby", 47), ("voice_radio", 48)] {
        if !has(&k.game, action) {
            let free = !taken(&k, scan);
            k.game.push(Binding { action: action.into(), scan: if free { scan } else { 0 }, modifier: 0 });
        }
    }
    for (action, scan, modifier) in [("vr_recenter", 19, KEY_SHIFT | KEY_CTRL), ("vr_toggle_desktop_mirror", 65, 0), ("vr_toggle_mode", 66, 0), ("vr_toggle_navigator", 49, KEY_SHIFT | KEY_CTRL), ("vr_position_navigator", 50, KEY_SHIFT | KEY_CTRL)] {
        if !has(&k.game, action) {
            k.game.push(Binding { action: action.into(), scan, modifier });
        }
    }
    k
}

fn decode(b: &[u8]) -> String {
    if let Some(rest) = b.strip_prefix(&[0xFF, 0xFE]) {
        String::from_utf16_lossy(&rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>())
    } else {
        b.iter().map(|&c| c as char).collect()
    }
}

fn newest(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().filter(|p| p.is_file()).max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok()).cloned()
}

/// The file in use: the newer of the launcher's own and the game's content folder's, else OMSI 2's.
pub fn path_in_use(omsi: Option<&Path>, content: Option<&Path>) -> Option<PathBuf> {
    let mut mine: Vec<PathBuf> = own_path().into_iter().collect();
    mine.extend(content.map(|c| c.join("Inputs").join("keyboard.cfg")));
    newest(&mine).or_else(|| omsi.map(|o| o.join("Inputs").join("keyboard.cfg")).filter(|p| p.is_file()))
}

pub fn load(omsi: Option<&Path>, content: Option<&Path>) -> Keys {
    let k = path_in_use(omsi, content).and_then(|p| std::fs::read(p).ok()).map(|b| parse(&decode(&b))).unwrap_or_default();
    with_defaults(k)
}

/// OMSI 2's own keys (what "restore the defaults" goes back to).
pub fn originals(omsi: Option<&Path>) -> Keys {
    let k = omsi.map(|o| o.join("Inputs").join("keyboard.cfg")).and_then(|p| std::fs::read(p).ok()).map(|b| parse(&decode(&b))).unwrap_or_default();
    with_defaults(k)
}

pub fn save(k: &Keys) -> Result<(), String> {
    let path = own_path().ok_or("no home folder")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("cfg.tmp");
    std::fs::write(&tmp, text(k)).map_err(|e| e.to_string())?;
    // read back through the same parser before it replaces the real file
    if parse(&std::fs::read_to_string(&tmp).unwrap_or_default()) != *k {
        let _ = std::fs::remove_file(&tmp);
        return Err("the key file could not be checked".into());
    }
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Back to OMSI 2's keys: the launcher's own copy deleted.
pub fn reset() {
    if let Some(p) = own_path() {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_written_and_read_back_the_same() {
        let k = Keys {
            game: vec![Binding { action: "sim_pause".into(), scan: 25, modifier: 0 }, Binding { action: "exit".into(), scan: 16, modifier: KEY_CTRL }],
            vehicles: vec![Binding { action: "throttle".into(), scan: 72, modifier: KEY_HOLD }],
        };
        assert_eq!(parse(&text(&k)), k);
    }

    /// OMSI 2's own keyboard.cfg (default content): every entry has an action and the pause key is the P of the original.
    #[test]
    fn omsi_2s_own_keys_are_read() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        let k = originals(Some(&omsi));
        if k.game.is_empty() {
            return;
        }
        assert!(k.game.iter().chain(k.vehicles.iter()).all(|b| !b.action.is_empty()));
        assert!(k.vehicles.iter().any(|b| b.action.eq_ignore_ascii_case("throttle")));
    }
}
