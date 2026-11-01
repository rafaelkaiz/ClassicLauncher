//! The blocks of the options window that are more than a row: the quality preset, the optimiser, the graphics profiles, the radio
//! stations and the navigator's corner. They keep their own state in `Extra`.

use crate::gamecfg::GameCfg;
use crate::i18n::{tr, trf};
use crate::presets::{self, Preset};
use eframe::egui::{self, RichText, Ui};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct Extra {
    profiles: Vec<Preset>,
    profile_sel: usize,
    profile_name: String,
    /// The name of a profile that exists and is waiting for the answer to "overwrite it?".
    confirm_overwrite: Option<String>,
    radio: Vec<(String, String)>,
    /// Aimed-at frame rate for the optimiser: 0 is the screen's rate.
    target: u32,
    opt: OptStatus,
    opt_checked: Instant,
    pub message: String,
    pub confirm_reset: bool,
}

#[derive(Default, Clone)]
struct OptStatus {
    armed: Option<u32>,
    running: bool,
    has_backup: bool,
    last: Option<serde_json::Value>,
}

fn data(name: &str) -> Option<PathBuf> {
    crate::omsi::data_dir().map(|d| d.join(name))
}

fn read_json(name: &str) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(data(name)?).ok()?).ok()
}

fn opt_status() -> OptStatus {
    OptStatus {
        armed: read_json("optimise-next-drive.json").and_then(|v| v.get("target")?.as_u64()).map(|t| t as u32),
        running: data("optimise-running.json").is_some_and(|p| p.exists()),
        has_backup: data("settings-before-optimise.json").is_some_and(|p| p.exists()),
        last: read_json("optimise.json"),
    }
}

/// A JSON value as the settings file writes it.
fn json_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// The quality levels' names in the current language.
pub fn quality_label(name: &str) -> String {
    if crate::i18n::lang() == crate::i18n::Lang::Es {
        match name {
            "Low" => "Bajo",
            "Medium" => "Medio",
            "High" => "Alto",
            "Extra" => "Extra",
            "Max" => "Máximo",
            "Ultra" => "Ultra",
            other => other,
        }
        .to_string()
    } else {
        name.to_string()
    }
}

impl Extra {
    pub fn new() -> Self {
        Self {
            profiles: presets::user_presets(),
            profile_sel: 0,
            profile_name: String::new(),
            confirm_overwrite: None,
            radio: crate::radio::load(),
            target: 0,
            opt: opt_status(),
            opt_checked: Instant::now(),
            message: String::new(),
            confirm_reset: false,
        }
    }

    fn refresh_optimiser(&mut self) {
        if self.opt_checked.elapsed() > Duration::from_secs(2) {
            self.opt = opt_status();
            self.opt_checked = Instant::now();
        }
    }
}

/// A row with the name (and a small help text under it) on the left and the control on the right.
pub fn split_row(ui: &mut Ui, left: impl FnOnce(&mut Ui), right: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        let w = ui.available_width() * 0.54;
        ui.allocate_ui_with_layout(egui::vec2(w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(w);
            left(ui);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
    });
    ui.add_space(5.0);
}

pub fn help_text(ui: &mut Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).size(11.0).weak()).wrap());
}

/// "Quality preset": the game's six levels, or Custom when the settings match none. Choosing a level sets its settings.
pub fn quality_row(ui: &mut Ui, cfg: &mut GameCfg) -> bool {
    let levels = presets::builtin();
    let current = levels.iter().position(|p| presets::matches(cfg, p));
    let mut changed = false;
    split_row(
        ui,
        |ui| {
            ui.label(tr("Quality preset"));
        },
        |ui| {
            let shown = current.map(|i| quality_label(&levels[i].name)).unwrap_or_else(|| tr("Custom").to_string());
            egui::ComboBox::from_id_salt("quality-preset").selected_text(shown).width(200.0).show_ui(ui, |ui| {
                for (i, p) in levels.iter().enumerate() {
                    if ui.selectable_label(current == Some(i), quality_label(&p.name)).clicked() && current != Some(i) {
                        presets::apply(cfg, p);
                        changed = true;
                    }
                }
                ui.selectable_label(current.is_none(), tr("Custom"));
            });
        },
    );
    changed
}

const TARGETS: [(u32, &str); 5] = [(0, "Automatic"), (60, "60 fps"), (90, "90 fps"), (120, "120 fps"), (144, "144 fps")];

/// "Optimise for my PC": the game measures during the next drive and adjusts the graphics; the launcher only arms it (a file the game
/// looks for), shows what it did and puts the old settings back.
pub fn optimise_block(ui: &mut Ui, cfg: &mut GameCfg, x: &mut Extra) -> bool {
    x.refresh_optimiser();
    let mut changed = false;
    ui.add_space(4.0);
    ui.separator();
    ui.label(RichText::new(tr("Optimise for my PC")).strong());
    help_text(ui, tr("During your next drive the game measures for about a minute and adjusts the graphics to reach the frame rate you aim at. Your settings are saved first."));
    ui.add_space(4.0);
    let armed = x.opt.armed.is_some();
    split_row(
        ui,
        |ui| {
            ui.label(tr("Frame rate aimed at"));
            help_text(ui, tr("Automatic: the screen's refresh rate, at most 100 fps"));
        },
        |ui| {
            let shown = TARGETS.iter().find(|t| t.0 == x.target).map(|t| tr(t.1).to_string()).unwrap_or_else(|| format!("{} fps", x.target));
            ui.add_enabled_ui(!armed, |ui| {
                egui::ComboBox::from_id_salt("optimise-target").selected_text(shown).width(200.0).show_ui(ui, |ui| {
                    for (v, name) in TARGETS {
                        ui.selectable_value(&mut x.target, v, tr(name));
                    }
                });
            });
        },
    );
    ui.horizontal(|ui| {
        if let Some(t) = x.opt.armed {
            ui.label(RichText::new(tr("Waiting for your next drive")).italics());
            if ui.button(tr("Cancel")).clicked() {
                if let Some(p) = data("optimise-next-drive.json") {
                    let _ = std::fs::remove_file(p);
                }
                x.opt = opt_status();
            }
            let _ = t;
        } else {
            let can = !x.opt.running && !crate::driver_files::game_running();
            let go = egui::Button::new(RichText::new(tr("Optimise on my next drive")).color(egui::Color32::WHITE)).fill(crate::theme::pal().dark);
            if ui.add_enabled(can, go).clicked() {
                let target = if x.target == 0 { crate::omsi::refresh_hz().map_or(60, |r| r.clamp(60, 100)) } else { x.target };
                if let Some(p) = data("optimise-next-drive.json") {
                    let _ = std::fs::write(p, format!("{{\"target\":{target}}}"));
                }
                x.opt = opt_status();
            }
        }
    });
    if let Some(last) = x.opt.last.clone() {
        let fps = |k: &str| last.get(k).and_then(|v| v.as_f64()).map(|v| format!("{v:.0}")).unwrap_or_else(|| "-".into());
        ui.add_space(4.0);
        ui.label(RichText::new(trf("Last optimisation: {} → {} fps", &[&fps("before_fps"), &fps("after_fps")])).size(12.0));
        let changes = last.get("changes").and_then(|c| c.as_array()).cloned().unwrap_or_default();
        if changes.is_empty() {
            help_text(ui, tr("Nothing needed changing"));
        }
        for c in changes {
            let key = c.get("key").and_then(|k| k.as_str()).unwrap_or("?");
            let (from, to) = (c.get("from").map(json_text).unwrap_or_default(), c.get("to").map(json_text).unwrap_or_default());
            help_text(ui, &format!("{key}: {from} → {to}"));
        }
    }
    if x.opt.has_backup {
        ui.add_space(4.0);
        let can = !x.opt.running && !crate::driver_files::game_running();
        if ui.add_enabled(can, egui::Button::new(tr("Restore my previous settings"))).clicked() {
            if let Some(saved) = read_json("settings-before-optimise.json").and_then(|v| v.as_object().cloned()) {
                for (k, v) in saved {
                    cfg.set(&k, &json_text(&v));
                }
                for name in ["settings-before-optimise.json", "optimise.json", "optimise-next-drive.json", "optimise-running.json"] {
                    if let Some(p) = data(name) {
                        let _ = std::fs::remove_file(p);
                    }
                }
                x.opt = opt_status();
                changed = true;
            }
        }
    }
    ui.add_space(4.0);
    changed
}

/// "Profiles": the graphics settings kept under a name, loaded and deleted from here.
pub fn profiles_block(ui: &mut Ui, cfg: &mut GameCfg, x: &mut Extra) -> bool {
    let mut changed = false;
    x.profile_sel = x.profile_sel.min(x.profiles.len().saturating_sub(1));
    split_row(
        ui,
        |ui| {
            ui.label(tr("Saved profile"));
        },
        |ui| {
            let shown = x.profiles.get(x.profile_sel).map(|p| p.name.clone()).unwrap_or_else(|| tr("No saved profiles").to_string());
            ui.add_enabled_ui(!x.profiles.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("profile-sel").selected_text(shown).width(200.0).show_ui(ui, |ui| {
                    for (i, p) in x.profiles.iter().enumerate() {
                        ui.selectable_value(&mut x.profile_sel, i, &p.name);
                    }
                });
            });
        },
    );
    let have = !x.profiles.is_empty();
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 8.0) / 2.0;
        if ui.add_enabled(have, egui::Button::new(tr("Load")).min_size(egui::vec2(w, 26.0))).clicked() {
            if let Some(p) = x.profiles.get(x.profile_sel) {
                presets::apply(cfg, p);
                changed = true;
            }
        }
        if ui.add_enabled(have, egui::Button::new(tr("Delete")).min_size(egui::vec2(w, 26.0))).clicked() {
            if let Some(p) = x.profiles.get(x.profile_sel) {
                x.message = match presets::delete(&p.name) {
                    Ok(()) => String::new(),
                    Err(e) => e,
                };
                x.profiles = presets::user_presets();
            }
        }
    });
    ui.add(egui::TextEdit::singleline(&mut x.profile_name).hint_text(tr("Profile name")).desired_width(f32::INFINITY));
    let name = x.profile_name.trim().to_string();
    let save = egui::Button::new(RichText::new(tr("Save the current graphics settings as a profile")).color(egui::Color32::WHITE)).fill(crate::theme::pal().dark).min_size(egui::vec2(ui.available_width(), 28.0));
    let mut save_now = false;
    let mut over = false;
    if ui.add_enabled(!name.is_empty(), save).clicked() {
        if presets::exists(&name) {
            x.confirm_overwrite = Some(name.clone());
        } else {
            save_now = true;
        }
    }
    // a profile of that name is there already: ask before it is replaced
    if let Some(asked) = x.confirm_overwrite.clone() {
        if asked != name {
            x.confirm_overwrite = None;
        } else {
            ui.label(RichText::new(trf("A profile named {} already exists. Overwrite it?", &[&asked])).strong());
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 8.0) / 2.0;
                if ui.add(egui::Button::new(tr("Overwrite")).min_size(egui::vec2(w, 26.0))).clicked() {
                    save_now = true;
                    over = true;
                }
                if ui.add(egui::Button::new(tr("Cancel")).min_size(egui::vec2(w, 26.0))).clicked() {
                    x.confirm_overwrite = None;
                }
            });
        }
    }
    if save_now {
        x.confirm_overwrite = None;
        let result = if over { presets::save_over(cfg, &name) } else { presets::save_new(cfg, &name) };
        x.message = match result {
            Ok(()) => {
                x.profile_name.clear();
                x.profiles = presets::user_presets();
                x.profile_sel = x.profiles.iter().position(|p| p.name == name).unwrap_or(0);
                tr("Profile saved.").to_string()
            }
            Err(e) => e,
        };
    }
    changed
}

/// The radio stations: a numbered list of name and address, saved to `radio.cfg` as it is edited.
pub fn radio_block(ui: &mut Ui, x: &mut Extra) {
    let mut edited = false;
    let mut remove = None;
    let width = ui.available_width();
    for (i, (name, address)) in x.radio.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{}", i + 1)).weak());
            edited |= ui.add(egui::TextEdit::singleline(name).desired_width(width * 0.28)).changed();
            edited |= ui.add(egui::TextEdit::singleline(address).desired_width(width * 0.50)).changed();
            if ui.button("✕").on_hover_text(tr("Delete")).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        x.radio.remove(i);
        edited = true;
    }
    if ui.button(format!("+ {}", tr("Add station"))).clicked() {
        x.radio.push((String::new(), String::new()));
    }
    if edited {
        if let Err(e) = crate::radio::save(&x.radio) {
            x.message = e;
        }
    }
}

/// Which corner of the screen the navigator sits in: four squares, the chosen one filled.
pub fn corner_block(ui: &mut Ui, cfg: &mut GameCfg) -> bool {
    let current = cfg.get("navigator_corner").unwrap_or_else(|| "bottom-left".into());
    let mut changed = false;
    split_row(
        ui,
        |ui| {
            ui.label(tr("Corner"));
        },
        |ui| {
            ui.vertical(|ui| {
                for row in [["top-left", "top-right"], ["bottom-left", "bottom-right"]] {
                    ui.horizontal(|ui| {
                        for value in row {
                            let on = current == value;
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(46.0, 22.0), egui::Sense::click());
                            let fill = if on { crate::theme::pal().dark } else if resp.hovered() { crate::theme::pal().hover } else { crate::theme::pal().field };
                            ui.painter().rect_filled(rect, 3.0, fill);
                            if resp.clicked() && !on {
                                cfg.set("navigator_corner", value);
                                changed = true;
                            }
                        }
                    });
                }
            });
        },
    );
    changed
}
