//! The keyboard half of the "Controls" tab: the keys of the game and of the bus (`Inputs/keyboard.cfg`), changed by pressing the new
//! one, like the original's Controls page.

use crate::i18n::{self, tr, trf};
use crate::keybindings::{self as kb, Binding, CHORD, KEY_ALT, KEY_CTRL, KEY_HOLD, KEY_SHIFT, Keys};
use crate::options_ui::tab_button;
use crate::theme::pal;
use eframe::egui::{self, Key, RichText, Ui};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Copy, PartialEq)]
enum Sec {
    Vehicles,
    Game,
}

/// The entry waiting for its key: one of the list (`index`) or a new one put in at `insert`.
struct Capture {
    sec: Sec,
    index: Option<usize>,
    insert: usize,
    action: String,
    modifier: i32,
    /// A key another entry has, said before it is taken.
    pending: Option<(i32, i32)>,
}

pub struct State {
    keys: Keys,
    sec: Sec,
    filters: [String; 2],
    names: HashMap<String, String>,
    scan_names: HashMap<i32, String>,
    capture: Option<Capture>,
    picker: Option<String>,
    confirm_reset: bool,
    message: String,
    omsi: Option<std::path::PathBuf>,
}

impl State {
    pub fn new(omsi: Option<&Path>, content: Option<&Path>) -> Self {
        let lang = i18n::lang();
        let names = omsi.map(|o| crate::olf::load(o, lang)).unwrap_or_default().into_iter().filter_map(|(k, v)| k.strip_prefix("KY_").map(|a| (a.to_ascii_lowercase(), v))).collect();
        let scan_names = omsi.map(|o| crate::olf::key_names(o, lang)).unwrap_or_default();
        Self { keys: kb::load(omsi, content), sec: Sec::Vehicles, filters: Default::default(), names, scan_names, capture: None, picker: None, confirm_reset: false, message: String::new(), omsi: omsi.map(Path::to_path_buf) }
    }

    fn label(&self, action: &str) -> String {
        if let Some(n) = self.names.get(&action.to_ascii_lowercase()).filter(|n| !n.trim().is_empty()) {
            return n.clone();
        }
        if let Some(rest) = action.strip_prefix("vr_") {
            return format!("VR: {}", rest.replace('_', " "));
        }
        action.to_string()
    }

    fn key_parts(&self, scan: i32, modifier: i32) -> Option<Vec<String>> {
        if scan == 0 {
            return None;
        }
        let mut p = vec![];
        for (bit, name) in [(KEY_SHIFT, "Shift"), (KEY_CTRL, "Ctrl"), (KEY_ALT, "Alt")] {
            if modifier & bit != 0 {
                p.push(name.to_string());
            }
        }
        p.push(match scan {
            90 => "Shift".to_string(),
            91 => "Ctrl".to_string(),
            92 => "Alt".to_string(),
            1004 => tr("Mouse 4").to_string(),
            1005 => tr("Mouse 5").to_string(),
            _ => self.scan_names.get(&scan).cloned().unwrap_or_else(|| format!("scan {scan}")),
        });
        Some(p)
    }

    fn list(&self, sec: Sec) -> &Vec<Binding> {
        if sec == Sec::Vehicles { &self.keys.vehicles } else { &self.keys.game }
    }

    fn list_mut(&mut self, sec: Sec) -> &mut Vec<Binding> {
        if sec == Sec::Vehicles { &mut self.keys.vehicles } else { &mut self.keys.game }
    }

    fn save(&mut self) -> bool {
        match kb::save(&self.keys) {
            Ok(()) => {
                self.message.clear();
                true
            }
            Err(e) => {
                self.message = trf("Could not save the settings: {}", &[&e]);
                false
            }
        }
    }
}

/// DirectInput's scan code of a key the window reports.
fn scan_of(k: Key) -> Option<i32> {
    use Key::*;
    Some(match k {
        Escape => 1,
        Num1 => 2,
        Num2 => 3,
        Num3 => 4,
        Num4 => 5,
        Num5 => 6,
        Num6 => 7,
        Num7 => 8,
        Num8 => 9,
        Num9 => 10,
        Num0 => 11,
        Minus => 12,
        Equals | Plus => 13,
        Backspace => 14,
        Tab => 15,
        Q => 16,
        W => 17,
        E => 18,
        R => 19,
        T => 20,
        Y => 21,
        U => 22,
        I => 23,
        O => 24,
        P => 25,
        OpenBracket => 26,
        CloseBracket => 27,
        Enter => 28,
        A => 30,
        S => 31,
        D => 32,
        F => 33,
        G => 34,
        H => 35,
        J => 36,
        K => 37,
        L => 38,
        Semicolon | Colon => 39,
        Quote => 40,
        Backtick => 41,
        Backslash | Pipe => 43,
        Z => 44,
        X => 45,
        C => 46,
        V => 47,
        B => 48,
        N => 49,
        M => 50,
        Comma => 51,
        Period => 52,
        Slash | Questionmark => 53,
        Space => 57,
        F1 => 59,
        F2 => 60,
        F3 => 61,
        F4 => 62,
        F5 => 63,
        F6 => 64,
        F7 => 65,
        F8 => 66,
        F9 => 67,
        F10 => 68,
        F11 => 87,
        F12 => 88,
        F13 => 100,
        F14 => 101,
        F15 => 102,
        Home => 199,
        ArrowUp => 200,
        PageUp => 201,
        ArrowLeft => 203,
        ArrowRight => 205,
        End => 207,
        ArrowDown => 208,
        PageDown => 209,
        Insert => 210,
        Delete => 211,
        _ => return None,
    })
}

fn chord_of(m: egui::Modifiers) -> i32 {
    (if m.shift { KEY_SHIFT } else { 0 }) | (if m.ctrl || m.command { KEY_CTRL } else { 0 }) | (if m.alt { KEY_ALT } else { 0 })
}

/// A key's cap, like the original's `kbd` chips.
fn cap(ui: &mut Ui, text: &str) {
    egui::Frame::new().fill(pal().field).stroke(egui::Stroke::new(1.0, pal().border)).corner_radius(4.0).inner_margin(egui::Margin::symmetric(7, 2)).show(ui, |ui| {
        ui.label(RichText::new(text).size(12.5).strong());
    });
}

fn caps(ui: &mut Ui, st: &State, scan: i32, modifier: i32) {
    ui.spacing_mut().item_spacing.x = 4.0;
    match st.key_parts(scan, modifier) {
        Some(parts) => parts.iter().for_each(|p| cap(ui, p)),
        None => {
            ui.label(RichText::new(tr("(unbound)")).weak().size(12.0));
        }
    }
}

/// Draws the keys. Returns true when a key was changed (the settings then use these keys).
pub fn show(ui: &mut Ui, st: &mut State) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        if tab_button(ui, tr("Driving & the bus"), st.sec == Sec::Vehicles) {
            st.sec = Sec::Vehicles;
        }
        if tab_button(ui, tr("The game"), st.sec == Sec::Game) {
            st.sec = Sec::Game;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(tr("Reset to defaults")).clicked() {
                st.confirm_reset = true;
            }
        });
    });
    ui.label(RichText::new(tr(if st.sec == Sec::Vehicles { "The bus's own keys" } else { "Menus, views, pausing" })).weak().size(12.0));
    let idx = (st.sec == Sec::Game) as usize;
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut st.filters[idx]).desired_width(ui.available_width() - 170.0).hint_text(tr("Filter...")));
        if st.sec == Sec::Vehicles && ui.button(tr("Add binding")).clicked() {
            st.picker = Some(String::new());
        }
    });
    if !st.message.is_empty() {
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), &st.message);
    }
    ui.add_space(4.0);

    let sec = st.sec;
    let q = st.filters[idx].trim().to_lowercase();
    let list = st.list(sec).clone();
    let mut counts: HashMap<(i32, i32), usize> = HashMap::new();
    for b in list.iter().filter(|b| b.scan != 0) {
        *counts.entry((b.scan, b.chord())).or_default() += 1;
    }
    let mut rows: Vec<(usize, &Binding)> = list.iter().enumerate().filter(|(_, b)| {
        q.is_empty() || st.label(&b.action).to_lowercase().contains(&q) || b.action.to_lowercase().contains(&q) || st.key_parts(b.scan, b.modifier).is_some_and(|p| p.join("+").to_lowercase().contains(&q))
    }).collect();
    if sec == Sec::Game {
        rows.sort_by_key(|(_, b)| !b.action.starts_with("vr_")); // VR keys first, as the original lists them
    }
    let mut start: Option<Capture> = None;
    let mut clear: Option<usize> = None;
    let mut remove: Option<usize> = None;
    egui::ScrollArea::vertical().id_salt(("keys", idx)).auto_shrink([false, false]).max_height((ui.available_height() - 6.0).max(100.0)).show(ui, |ui| {
        if rows.is_empty() {
            ui.label(tr("No matching actions."));
            let name = st.filters[idx].trim().to_string();
            if sec == Sec::Vehicles && name.len() > 1 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && ui.button(trf("Add \"{}\" and give it a key", &[&name])).clicked() {
                start = Some(Capture { sec, index: None, insert: list.len(), action: name, modifier: 0, pending: None });
            }
        }
        for (i, b) in &rows {
            let clash = b.scan != 0 && counts.get(&(b.scan, b.chord())).copied().unwrap_or(0) > 1;
            egui::Frame::new().fill(ui.visuals().faint_bg_color).corner_radius(4.0).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - 330.0);
                        let label = st.label(&b.action);
                        ui.label(RichText::new(&label).strong());
                        if label != b.action {
                            ui.label(RichText::new(&b.action).weak().size(11.0));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.set_width(150.0);
                        caps(ui, st, b.scan, b.modifier);
                        if clash {
                            ui.label(RichText::new(tr("Same key")).color(egui::Color32::from_rgb(200, 60, 60)).size(11.0)).on_hover_text(tr("Another entry of this list has the same key."));
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("x").on_hover_text(tr("Remove the entry")).clicked() {
                            remove = Some(*i);
                        }
                        if ui.add_enabled(b.scan != 0, egui::Button::new("-").small()).on_hover_text(tr("Clear the key (the entry stays)")).clicked() {
                            clear = Some(*i);
                        }
                        if ui.small_button("+").on_hover_text(tr("Add another key for this action")).clicked() {
                            start = Some(Capture { sec, index: None, insert: i + 1, action: b.action.clone(), modifier: b.modifier & KEY_HOLD, pending: None });
                        }
                        if ui.small_button(tr("Edit")).clicked() {
                            start = Some(Capture { sec, index: Some(*i), insert: *i, action: b.action.clone(), modifier: b.modifier & KEY_HOLD, pending: None });
                        }
                    });
                });
            });
        }
    });
    if let Some(c) = start {
        st.capture = Some(c);
    }
    if let Some(i) = clear {
        let b = &mut st.list_mut(sec)[i];
        b.scan = 0;
        b.modifier = 0;
        changed |= st.save();
    }
    if let Some(i) = remove {
        st.list_mut(sec).remove(i);
        changed |= st.save();
    }
    changed |= capture_window(ui.ctx(), st);
    changed |= picker_window(ui.ctx(), st);
    changed |= reset_window(ui.ctx(), st);
    changed
}

/// "Press a key": the key pressed (with Shift, Ctrl, Alt held) is the entry's; Escape leaves it, Backspace or Delete clears it.
fn capture_window(ctx: &egui::Context, st: &mut State) -> bool {
    let Some(mut c) = st.capture.take() else { return false };
    let label = st.label(&c.action);
    let mut keep = true;
    let mut apply: Option<(i32, i32)> = None;
    let mut open = true;
    let current = c.index.and_then(|i| st.list(c.sec).get(i)).cloned();
    // the other entries with the key pending
    let clash: Vec<String> = c.pending.map(|(scan, chord)| st.list(c.sec).iter().enumerate().filter(|(i, b)| Some(*i) != c.index && b.scan == scan && b.chord() == chord && !b.action.eq_ignore_ascii_case(&c.action)).map(|(_, b)| st.label(&b.action)).collect()).unwrap_or_default();
    if c.pending.is_none() {
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            if let egui::Event::Key { key, physical_key, pressed: true, modifiers, repeat: false } = e {
                match key {
                    Key::Escape => keep = false,
                    Key::Backspace | Key::Delete if current.is_some() => apply = Some((0, 0)),
                    Key::Backspace | Key::Delete => keep = false,
                    _ => {
                        if let Some(scan) = scan_of(physical_key.unwrap_or(key)).or_else(|| scan_of(key)) {
                            let chord = chord_of(modifiers) & CHORD;
                            let taken = st.list(c.sec).iter().enumerate().any(|(i, b)| Some(i) != c.index && b.scan == scan && b.chord() == chord && !b.action.eq_ignore_ascii_case(&c.action));
                            if taken {
                                c.pending = Some((scan, chord));
                            } else {
                                apply = Some((scan, chord));
                            }
                        }
                    }
                }
                break;
            }
        }
    }
    open = crate::free::window(ctx, "key-capture", &label, egui::vec2(420.0, 260.0), |ui| {
        ui.set_width(380.0);
        ui.vertical_centered(|ui| {
            ui.add_space(6.0);
            if c.action != st.label(&c.action) {
                ui.label(RichText::new(&c.action).weak().size(11.0));
            }
            match c.pending {
                Some((scan, chord)) => {
                    ui.horizontal(|ui| caps(ui, st, scan, chord));
                    ui.add_space(6.0);
                    ui.label(trf("This key is already used by: {}", &[&clash.join(", ")]));
                    ui.label(RichText::new(tr("Both will react to it. Press another key, or use it anyway.")).weak().size(12.0));
                    ui.add_space(6.0);
                    if ui.button(tr("Use it anyway")).clicked() {
                        apply = Some((scan, chord));
                    }
                }
                None => {
                    cap(ui, &format!("  {}  ", tr("press a key...")));
                    if let Some(b) = &current {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(tr("Now:")).weak());
                            caps(ui, st, b.scan, b.modifier);
                        });
                    }
                }
            }
            ui.add_space(6.0);
            ui.label(RichText::new(tr("Hold Shift, Ctrl or Alt for a combination. Escape cancels; Backspace or Delete clears the key.")).weak().size(12.0));
            ui.add_space(4.0);
            if ui.button(tr("Cancel")).clicked() {
                keep = false;
            }
        });
    });
    if !open {
        keep = false;
    }
    if let Some((scan, chord)) = apply {
        let hold = c.modifier & KEY_HOLD;
        let list = st.list_mut(c.sec);
        match c.index {
            Some(i) if i < list.len() => {
                let hold = list[i].modifier & KEY_HOLD;
                list[i].scan = scan;
                list[i].modifier = if scan == 0 { 0 } else { chord | hold };
            }
            None if scan != 0 => {
                let at = c.insert.min(list.len());
                list.insert(at, Binding { action: c.action.clone(), scan, modifier: chord | hold });
            }
            _ => {}
        }
        return st.save();
    }
    if keep {
        st.capture = Some(c);
    }
    false
}

/// The bus's actions that can be given a key.
fn picker_window(ctx: &egui::Context, st: &mut State) -> bool {
    let Some(mut q) = st.picker.take() else { return false };
    let mut open = true;
    let mut chosen: Option<String> = None;
    let have: std::collections::HashSet<String> = st.keys.game.iter().map(|b| b.action.to_ascii_lowercase()).collect();
    let mut options: Vec<(String, String)> = st.names.iter().filter(|(a, _)| !have.contains(*a) && !is_game_only(a)).map(|(a, l)| (l.clone(), a.clone())).collect();
    // the real spelling of the names the bus's list has
    for b in &st.keys.vehicles {
        if !options.iter().any(|(_, a)| a.eq_ignore_ascii_case(&b.action)) {
            options.push((st.label(&b.action), b.action.clone()));
        }
    }
    options.sort_by_key(|(l, a)| (l.to_lowercase(), a.clone()));
    let spell: HashMap<String, String> = st.keys.vehicles.iter().map(|b| (b.action.to_ascii_lowercase(), b.action.clone())).collect();
    let needle = q.trim().to_lowercase();
    open = crate::free::window(ctx, "key-add", tr("Add a key binding"), egui::vec2(540.0, 480.0), |ui| {
        ui.set_width(520.0);
        ui.add(egui::TextEdit::singleline(&mut q).desired_width(f32::INFINITY).hint_text(tr("Search actions...")));
        let shown: Vec<&(String, String)> = options.iter().filter(|(l, a)| needle.is_empty() || l.to_lowercase().contains(&needle) || a.contains(&needle)).collect();
        ui.label(RichText::new(trf("{} results", &[&shown.len().to_string()])).weak().size(12.0));
        egui::ScrollArea::vertical().max_height(330.0).auto_shrink([false, false]).show(ui, |ui| {
            for (label, action) in shown.into_iter().take(150) {
                let real = spell.get(&action.to_ascii_lowercase()).cloned().unwrap_or_else(|| action.clone());
                ui.horizontal(|ui| {
                    if ui.selectable_label(false, RichText::new(label).strong()).clicked() {
                        chosen = Some(real.clone());
                    }
                    ui.label(RichText::new(&real).weak().size(11.0));
                });
            }
        });
    });
    if let Some(a) = chosen {
        st.filters[0] = a.clone();
        st.capture = Some(Capture { sec: Sec::Vehicles, index: None, insert: st.keys.vehicles.len(), action: a, modifier: 0, pending: None });
        return false;
    }
    if open {
        st.picker = Some(q);
    }
    false
}

/// The game's own actions, which the bus's list has no use for.
fn is_game_only(a: &str) -> bool {
    ["vr_", "chat_", "view_", "gear_up", "gear_down", "sim_pause", "open_menu", "screenshot", "photo_mode", "quicksave", "toggel_", "exit", "voice_radio", "pax_door_request", "doors_all", "doors_release"].iter().any(|p| a.starts_with(p))
}

fn reset_window(ctx: &egui::Context, st: &mut State) -> bool {
    if !st.confirm_reset {
        return false;
    }
    let mut done = false;
    let mut open = true;
    open = crate::free::window(ctx, "key-reset", tr("Reset to defaults"), egui::vec2(420.0, 140.0), |ui| {
        ui.set_width(380.0);
        ui.label(tr("Every key goes back to OMSI 2's own keyboard.cfg. Keys you added for a bus's own triggers are removed."));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(tr("Reset")).clicked() {
                done = true;
            }
            if ui.button(tr("Cancel")).clicked() {
                st.confirm_reset = false;
            }
        });
    });
    if !open {
        st.confirm_reset = false;
    }
    if done {
        st.confirm_reset = false;
        kb::reset();
        st.keys = kb::originals(st.omsi.as_deref());
        return st.save();
    }
    false
}
