//! The game controllers half of the "Controls" tab: wheels, pedals and gamepads (`Inputs/gamectrler.cfg`), laid out like the original's
//! page: the devices, the axes with their live bars, the wheel's forces and the buttons by button or by action. Every change is saved
//! at once.

use crate::controllers::{self as ctl, Device};
use crate::i18n::{self, tr, trf};
use crate::options_ui::tab_button;
use crate::gamecfg::GameCfg;
use crate::options_ui::draw_row;
use crate::schema::{self, Row};
use crate::theme::pal;
use eframe::egui::{self, RichText, Ui};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

/// The guided set-up: asks for each control in turn and takes the axis that moved most.
struct Wizard {
    step: usize,
    baseline: [f32; 8],
    best: [f32; 8],
}

const WIZARD_STEPS: [(i32, &str); 4] = [(0, "Turn the wheel (or move the stick) all the way left and right."), (1, "Press the throttle pedal all the way down."), (2, "Press the brake pedal all the way down."), (3, "Press the clutch pedal all the way down (skip it if there is none).")];

pub struct State {
    devices: Vec<Device>,
    selected: String,
    message: String,
    new_name: String,
    names: HashMap<String, String>,
    /// (label, action) of every key action, by group.
    actions: Vec<(String, String)>,
    connected: Vec<String>,
    by_action: bool,
    filter: String,
    live: Option<([Option<f32>; 8], u128)>,
    wizard: Option<Wizard>,
    checked: Instant,
    r_axes: Vec<Row>,
    r_wheel: Vec<Row>,
    r_ff: Vec<Row>,
    r_gear: Vec<Row>,
    cfg_dirty: bool,
    ffb_win: bool,
    learning: Option<String>,
    prev_buttons: u128,
    ffb_run: Option<FfbRun>,
    ffb_strength: f32,
    ffb_result: Option<Result<bool, String>>,
}

impl State {
    pub fn new(omsi: Option<&Path>) -> Self {
        let names: HashMap<String, String> = omsi.map(|o| crate::olf::load(o, i18n::lang())).unwrap_or_default().into_iter().filter_map(|(k, v)| k.strip_prefix("KY_").map(|a| (a.to_ascii_lowercase(), v))).collect();
        let mut actions: Vec<(String, String)> = ctl::action_names(omsi).into_iter().map(|a| (names.get(&a.to_ascii_lowercase()).cloned().unwrap_or_else(|| a.clone()), a)).collect();
        actions.sort_by_key(|(l, a)| (l.to_lowercase(), a.clone()));
        let devices = ctl::load(omsi);
        let connected = ctl::connected();
        let selected = devices.iter().find(|d| connected.iter().any(|c| d.name.to_lowercase().starts_with(&c.to_lowercase()) || c.to_lowercase().starts_with(&d.name.to_lowercase()))).or(devices.first()).map(|d| d.name.clone()).or_else(|| connected.first().cloned()).unwrap_or_default();
        Self { devices, selected, message: String::new(), new_name: String::new(), names, actions, connected, by_action: false, filter: String::new(), live: None, wizard: None, checked: Instant::now(), r_axes: schema::rows_of(&["ctrl_deadzone", "pedal_throttle", "pedal_brake"]), r_wheel: schema::rows_of(&["wheel_range", "wheel_lock", "cab_wheel_rotation"]), r_ff: schema::rows_of(&["ff_enabled", "ff_road_vib", "ff_engine_vib", "ff_fade"]), r_gear: schema::rows_of(&["momentary_gears"]), cfg_dirty: false, ffb_win: false, learning: None, prev_buttons: 0, ffb_run: None, ffb_strength: crate::ffb_calibration::PULSE_FORCE, ffb_result: None }
    }

    fn label(&self, action: &str) -> String {
        self.names.get(&action.to_ascii_lowercase()).filter(|n| !n.trim().is_empty()).cloned().unwrap_or_else(|| action.to_string())
    }

    fn save(&mut self) {
        self.message = match ctl::save(&self.devices) {
            Ok(_) => String::new(),
            Err(e) => trf("Could not save the settings: {}", &[&e]),
        };
    }
}

/// A label of a set width, so the controls beside it line up.
fn fixed_label(ui: &mut Ui, text: &str) {
    ui.allocate_ui_with_layout(egui::vec2(230.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.label(text);
    });
}

fn card<R>(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new().fill(ui.visuals().faint_bg_color).stroke(ui.visuals().widgets.noninteractive.bg_stroke).corner_radius(6.0).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).strong().size(15.0));
        ui.add_space(4.0);
        body(ui)
    }).inner
}

fn tag(ui: &mut Ui, text: &str, good: bool) {
    let c = if good { egui::Color32::from_rgb(40, 140, 70) } else { egui::Color32::GRAY };
    ui.label(RichText::new(text).color(c).size(11.5));
}

/// The action's group, like the original's: driving, the gearbox, lights, doors and tickets, views, the game.
fn group_of(action: &str) -> usize {
    let a = action.to_ascii_lowercase();
    if a.starts_with("kw_s_") || a.starts_with("automatic_") || a.starts_with("gear_") || a.contains("m_enginestart") && false {
        1
    } else if a.starts_with("blinker") || a.contains("licht") || a.contains("scheinwerfer") || a.contains("nebel") {
        2
    } else if a.starts_with("bus_door") || a.starts_with("door") || a.starts_with("ticket") || a.starts_with("change_") || a.starts_with("pax_") || a.starts_with("cashdesk") {
        3
    } else if a.starts_with("view_") {
        4
    } else if ["throttle", "brake", "clutch", "steering", "parking_brake", "horn", "kw_m_", "kw_wiper", "ego_"].iter().any(|p| a.starts_with(p)) {
        0
    } else {
        5
    }
}

const GROUPS: [&str; 6] = ["Driving", "Gearbox", "Lights & indicators", "Doors & tickets", "Views", "The game"];

fn axis_bar(ui: &mut Ui, v: Option<f32>) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(130.0, 12.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3.0, pal().field);
    ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, pal().border), egui::StrokeKind::Inside);
    if let Some(v) = v {
        let mut fill = rect;
        fill.set_right(rect.left() + rect.width() * v);
        ui.painter().rect_filled(fill, 3.0, pal().dark);
    }
    // the middle of the travel
    let x = rect.center().x;
    ui.painter().line_segment([egui::pos2(x, rect.top() + 1.0), egui::pos2(x, rect.bottom() - 1.0)], egui::Stroke::new(1.0, pal().text.gamma_multiply(0.6)));
}

pub fn show(ui: &mut Ui, st: &mut State, omsi: Option<&Path>, mut cfg: Option<&mut GameCfg>) {
    let mut cfg_changed = false;
    let mut open_test = false;
    // the devices' live state, a few times a second
    if st.checked.elapsed() > Duration::from_millis(1500) {
        st.connected = ctl::connected();
        st.checked = Instant::now();
    }
    let sel = st.selected.to_lowercase();
    let is_connected = !sel.is_empty() && st.connected.iter().any(|c| sel.starts_with(&c.to_lowercase()) || c.to_lowercase().starts_with(&sel));
    st.live = if is_connected && st.ffb_run.is_none() { ctl::live(&st.selected) } else { None };
    ui.ctx().request_repaint_after(Duration::from_millis(60));
    let mut changed = false;
    let live = st.live;
    egui::ScrollArea::vertical().auto_shrink([false, false]).max_height((ui.available_height() - 6.0).max(100.0)).show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
        // --- devices ---
        card(ui, tr("Devices"), |ui| {
            ui.label(RichText::new(tr("Your wheel, pedals, gamepad or button box: set them up, give their buttons what they do and check that they answer.")).weak().size(12.0));
            let mut entries: Vec<(String, bool, bool)> = st.devices.iter().map(|d| (d.name.clone(), true, st.connected.iter().any(|c| d.name.to_lowercase().starts_with(&c.to_lowercase()) || c.to_lowercase().starts_with(&d.name.to_lowercase()))) ).collect();
            for c in &st.connected {
                if !entries.iter().any(|(n, _, _)| n.to_lowercase().starts_with(&c.to_lowercase()) || c.to_lowercase().starts_with(&n.to_lowercase())) {
                    entries.push((c.clone(), false, true));
                }
            }
            if entries.is_empty() {
                ui.label(tr("No controllers are configured yet: add the device by its name."));
            }
            for (name, configured, connected) in &entries {
                ui.horizontal(|ui| {
                    if ui.selectable_label(st.selected == *name, RichText::new(name).strong()).clicked() {
                        st.selected = name.clone();
                        st.wizard = None;
                    }
                    tag(ui, tr(if *connected { "Connected" } else { "Not connected" }), *connected);
                    if *configured {
                        tag(ui, tr("Set up"), true);
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut st.new_name).desired_width(260.0).hint_text(tr("Name as the game shows it")));
                let n = st.new_name.trim().to_string();
                let exists = st.devices.iter().any(|d| d.name.eq_ignore_ascii_case(&n));
                if ui.add_enabled(!n.is_empty() && !exists, egui::Button::new(tr("Add"))).clicked() {
                    st.devices.push(Device::new(&n));
                    st.selected = n;
                    st.new_name.clear();
                    changed = true;
                }
            });
            ui.label(RichText::new(tr("The name must be the one the game shows for the device; a name Windows cuts at 31 letters has to be typed in full.")).weak().size(11.5));
        });

        if st.selected.is_empty() {
            return;
        }
        let pos = st.devices.iter().position(|d| d.name == st.selected);
        // --- the chosen device's header ---
        let mut remove = false;
        let mut adopt = false;
        let mut start_wizard = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(&st.selected).strong().size(17.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pos.is_some() {
                    if ui.button(tr("Remove")).clicked() {
                        remove = true;
                    }
                    if ui.add_enabled(live.is_some(), egui::Button::new(tr("Set up step by step"))).clicked() {
                        start_wizard = true;
                    }
                } else if ui.button(tr("Use this device")).clicked() {
                    adopt = true;
                }
            });
        });
        if adopt {
            st.devices.push(Device::new(&st.selected));
            changed = true;
        }
        if remove {
            if let Some(p) = pos {
                st.devices.remove(p);
                st.selected = st.devices.first().map(|d| d.name.clone()).unwrap_or_default();
                st.wizard = None;
                changed = true;
            }
            return;
        }
        let Some(p) = st.devices.iter().position(|d| d.name == st.selected) else {
            ui.label(RichText::new(tr("This device is connected but the game does not use it yet.")).weak());
            return;
        };
        if start_wizard {
            if let Some((axes, _)) = live {
                st.wizard = Some(Wizard { step: 0, baseline: axes.map(|a| a.unwrap_or(0.5)), best: [0.0; 8] });
            }
        }
        // --- the guided set-up ---
        if let (Some(w), Some((axes, _))) = (st.wizard.as_mut(), live) {
            let mut finish = false;
            let mut assign: Option<usize> = None;
            let mut skip = false;
            card(ui, tr("Set up step by step"), |ui| {
                ui.label(tr(WIZARD_STEPS[w.step].1));
                for a in 0..8 {
                    w.best[a] = w.best[a].max((axes[a].unwrap_or(w.baseline[a]) - w.baseline[a]).abs());
                }
                let top = (0..8).max_by(|&x, &y| w.best[x].total_cmp(&w.best[y])).unwrap_or(0);
                if w.best[top] > 0.25 {
                    ui.label(trf("Found: {}", &[tr(ctl::AXES[top])]));
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(w.best[top] > 0.25, egui::Button::new(tr("Use it"))).clicked() {
                        assign = Some(top);
                    }
                    if ui.button(tr("Skip")).clicked() {
                        skip = true;
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        finish = true;
                    }
                });
            });
            if let Some(axis) = assign {
                let func = WIZARD_STEPS[w.step].0;
                for a in st.devices[p].axes.iter_mut() {
                    if a.0 == func {
                        a.0 = -1;
                    }
                }
                st.devices[p].axes[axis].0 = func;
                changed = true;
                skip = true;
            }
            if skip {
                if let Some(w) = st.wizard.as_mut() {
                    w.step += 1;
                    w.baseline = live.map(|l| l.0.map(|a| a.unwrap_or(0.5))).unwrap_or(w.baseline);
                    w.best = [0.0; 8];
                    finish |= w.step >= WIZARD_STEPS.len();
                }
            }
            if finish {
                st.wizard = None;
            }
        }
        let actions = st.actions.clone();
        let names = st.names.clone();
        let d = &mut st.devices[p];

        // --- axes ---
        card(ui, tr("Axes"), |ui| {
            ui.label(RichText::new(tr("Move a control: its line lights up.")).weak().size(12.0));
            // what each function is on: "Steering: X axis"
            ui.horizontal_wrapped(|ui| {
                for (code, name) in ctl::FUNCS.iter().skip(1).take(4) {
                    let on = d.axes.iter().position(|a| a.0 == *code).map(|a| tr(ctl::AXES[a]));
                    ui.label(RichText::new(format!("{}: {}", tr(name), on.unwrap_or(tr("Not used")))).size(12.0).color(if on.is_some() { pal().dark } else { pal().text.gamma_multiply(0.6) }));
                    ui.add_space(8.0);
                }
            });
            egui::Grid::new("ctl-axes").num_columns(5).spacing([14.0, 6.0]).show(ui, |ui| {
                for h in ["Axis", "Now", "Function", "Reversed", "Response"] {
                    ui.label(RichText::new(tr(h)).strong());
                }
                ui.end_row();
                for a in 0..8 {
                    ui.label(tr(ctl::AXES[a]));
                    axis_bar(ui, live.and_then(|l| l.0[a]));
                    let f = d.axes[a].0;
                    let label = ctl::FUNCS.iter().find(|x| x.0 == f).map(|x| tr(x.1)).unwrap_or("?");
                    egui::ComboBox::from_id_salt(("ctl-func", a)).selected_text(label).width(190.0).show_ui(ui, |ui| {
                        for (code, name) in ctl::FUNCS {
                            if ui.selectable_label(f == code, tr(name)).clicked() && f != code {
                                d.axes[a].0 = code;
                                changed = true;
                            }
                        }
                    });
                    changed |= ui.checkbox(&mut d.axes[a].1, "").changed();
                    let shape = d.axis_flags[a] & ctl::CURVE_BITS;
                    let shape_label = ctl::SHAPES.iter().find(|x| x.0 == shape).map(|x| tr(x.1)).unwrap_or("?");
                    egui::ComboBox::from_id_salt(("ctl-shape", a)).selected_text(shape_label).width(150.0).show_ui(ui, |ui| {
                        for (bits, name) in ctl::SHAPES {
                            if ui.selectable_label(shape == bits, tr(name)).clicked() && shape != bits {
                                d.axis_flags[a] = (d.axis_flags[a] & !ctl::CURVE_BITS) | bits;
                                changed = true;
                            }
                        }
                    });
                    ui.end_row();
                }
            });
            if let Some(c) = cfg.as_deref_mut() {
                ui.add_space(6.0);
                for row in &st.r_axes {
                    cfg_changed |= draw_row(ui, c, row, &[]);
                }
            }
        });

        // --- the wheel ---
        if let Some(c) = cfg.as_deref_mut() {
            card(ui, tr("Wheel"), |ui| {
                ui.label(RichText::new(tr("For all wheels")).weak().size(12.0));
                for row in &st.r_wheel {
                    cfg_changed |= draw_row(ui, c, row, &[]);
                }
            });
            // an H-pattern shifter has no neutral of its own
            let h_gears = d.buttons.iter().any(|(a, _)| a.to_ascii_lowercase().starts_with("kw_s_"));
            if h_gears && !c.flag("momentary_gears", false) {
                card(ui, tr("H-pattern gears"), |ui| {
                    ui.label(tr("H-pattern gears are assigned. If your shifter has no neutral button, turn on return to neutral:"));
                    for row in &st.r_gear {
                        cfg_changed |= draw_row(ui, c, row, &[]);
                    }
                });
            }
        }

        // --- force feedback ---
        card(ui, tr("Force feedback"), |ui| {
            if let Some(c) = cfg.as_deref_mut() {
                for row in &st.r_ff {
                    cfg_changed |= draw_row(ui, c, row, &[]);
                }
                ui.add_space(4.0);
            }
            let (mut steer, mut vib) = d.ff_scale.unwrap_or((1.0, 1.0));
            let pct = |v: f64, _: std::ops::RangeInclusive<usize>| format!("{:.0}%", v * 100.0);
            let before = (steer, vib);
            ui.horizontal(|ui| {
                fixed_label(ui, tr("Steering forces"));
                ui.add(egui::Slider::new(&mut steer, 0.0..=2.0).step_by(0.05).custom_formatter(pct));
            });
            ui.horizontal(|ui| {
                fixed_label(ui, tr("Vibration strength"));
                ui.add(egui::Slider::new(&mut vib, 0.0..=2.0).step_by(0.05).custom_formatter(pct));
            });
            if (steer, vib) != before {
                d.ff_scale = Some((steer, vib));
                changed = true;
            }
            ui.horizontal(|ui| {
                fixed_label(ui, tr("Direction of the forces"));
                let label = match d.ff_invert {
                    None => tr("Default"),
                    Some(true) => tr("Inverted"),
                    Some(false) => tr("Not inverted"),
                };
                egui::ComboBox::from_id_salt("ctl-ffinv").selected_text(label).show_ui(ui, |ui| {
                    for (v, name) in [(None, "Default"), (Some(true), "Inverted"), (Some(false), "Not inverted")] {
                        if ui.selectable_label(d.ff_invert == v, tr(name)).clicked() {
                            d.ff_invert = v;
                            changed = true;
                        }
                    }
                });
                let steer_axis = d.axes.iter().position(|a| a.0 == 0);
                if ui.add_enabled(live.is_some() && steer_axis.is_some(), egui::Button::new(tr("Test the direction"))).clicked() {
                    open_test = true;
                }
            });
            let mut tune = d.tune();
            let tune_names = ["Self-centring", "Damping", "Parking resistance", "Minimum force", "Smoothing"];
            let before = tune;
            for k in 0..5 {
                ui.horizontal(|ui| {
                    fixed_label(ui, tr(tune_names[k]));
                    let (lo, hi) = ctl::TUNE_RANGES[k];
                    let step = if k >= 3 { 0.005 } else { 0.05 };
                    ui.add(egui::Slider::new(&mut tune[k], lo..=hi).step_by(step));
                });
            }
            if tune != before {
                d.set_tune(tune);
                changed = true;
            }
            if ui.button(tr("Reset wheel settings")).clicked() {
                d.ff_scale = None;
                d.ff_tune = None;
                d.ff_invert = None;
                changed = true;
            }
        });

        // --- buttons ---
        card(ui, &trf("Buttons ({})", &[&d.buttons.len().to_string()]), |ui| {
            ui.horizontal(|ui| {
                if tab_button(ui, tr("By button"), !st.by_action) {
                    st.by_action = false;
                }
                if tab_button(ui, tr("By action"), st.by_action) {
                    st.by_action = true;
                }
                if !st.by_action {
                    if ui.button(tr("Add button")).clicked() {
                        d.buttons.push((String::new(), "0".into()));
                        changed = true;
                    }
                    if ui.add_enabled(!d.buttons.is_empty(), egui::Button::new(tr("Remove last button"))).clicked() {
                        d.buttons.pop();
                        let n = d.buttons.len();
                        d.latching.retain(|&b| b < n);
                        d.more.retain(|(b, _)| *b < n);
                        changed = true;
                    }
                }
            });
            let pressed = |b: usize| live.is_some_and(|l| b < 128 && l.1 & (1u128 << b) != 0);
            let action_label = |a: &str| if a.is_empty() { tr("None").to_string() } else { names.get(&a.to_ascii_lowercase()).cloned().unwrap_or_else(|| a.to_string()) };
            if !st.by_action {
                egui::Grid::new("ctl-buttons").num_columns(5).spacing([14.0, 4.0]).show(ui, |ui| {
                    for b in 0..d.buttons.len() {
                        let on = pressed(b);
                        ui.label(RichText::new(trf("Button {}", &[&(b + 1).to_string()])).strong().color(if on { pal().dark } else { ui.visuals().text_color() }));
                        let mut value = d.buttons[b].0.clone();
                        let shown = action_label(&value);
                        egui::ComboBox::from_id_salt(("ctl-btn", b)).selected_text(shown).width(260.0).show_ui(ui, |ui| {
                            if ui.selectable_label(value.is_empty(), tr("None")).clicked() {
                                value.clear();
                            }
                            for (label, a) in &actions {
                                if ui.selectable_label(value.eq_ignore_ascii_case(a), label).clicked() {
                                    value = a.clone();
                                }
                            }
                        });
                        if value != d.buttons[b].0 {
                            d.buttons[b].0 = value;
                            changed = true;
                        }
                        let mut cont = d.buttons[b].1.trim() == "1";
                        if ui.checkbox(&mut cont, tr("Repeats while held")).changed() {
                            d.buttons[b].1 = if cont { "1".into() } else { "0".into() };
                            changed = true;
                        }
                        let mut latch = d.latching.contains(&b);
                        if ui.checkbox(&mut latch, tr("Latching switch")).changed() {
                            if latch {
                                d.latching.push(b);
                                d.latching.sort_unstable();
                            } else {
                                d.latching.retain(|&x| x != b);
                            }
                            changed = true;
                        }
                        ui.end_row();
                    }
                });
            } else {
                ui.add(egui::TextEdit::singleline(&mut st.filter).desired_width(300.0).hint_text(tr("Filter...")));
                let q = st.filter.trim().to_lowercase();
                let count = d.buttons.len();
                // a button pressed while an action waits for one is that action's
                if let (Some(act), Some(l)) = (st.learning.clone(), live) {
                    let fresh = l.1 & !st.prev_buttons;
                    if fresh != 0 {
                        let b = fresh.trailing_zeros() as usize;
                        while d.buttons.len() <= b {
                            d.buttons.push((String::new(), "0".into()));
                        }
                        for x in d.buttons.iter_mut().filter(|x| x.0.eq_ignore_ascii_case(&act)) {
                            x.0.clear();
                        }
                        d.buttons[b].0 = act;
                        st.learning = None;
                        changed = true;
                    }
                }
                for (g, group) in GROUPS.iter().enumerate() {
                    let rows: Vec<&(String, String)> = actions.iter().filter(|(l, a)| group_of(a) == g && (q.is_empty() || l.to_lowercase().contains(&q) || a.to_lowercase().contains(&q))).collect();
                    if rows.is_empty() {
                        continue;
                    }
                    ui.label(RichText::new(tr(group)).strong());
                    for (label, action) in rows {
                        let current = d.buttons.iter().position(|(a, _)| a.eq_ignore_ascii_case(action));
                        ui.horizontal(|ui| {
                            fixed_label(ui, label);
                            let shown = current.map(|b| trf("Button {}", &[&(b + 1).to_string()])).unwrap_or_else(|| tr("None").to_string());
                            egui::ComboBox::from_id_salt(("ctl-act", action)).selected_text(shown).width(140.0).show_ui(ui, |ui| {
                                if ui.selectable_label(current.is_none(), tr("None")).clicked() {
                                    if let Some(c) = current {
                                        d.buttons[c].0.clear();
                                        changed = true;
                                    }
                                }
                                for b in 0..count {
                                    if ui.selectable_label(current == Some(b), trf("Button {}", &[&(b + 1).to_string()])).clicked() && current != Some(b) {
                                        if let Some(c) = current {
                                            d.buttons[c].0.clear();
                                        }
                                        d.buttons[b].0 = action.clone();
                                        changed = true;
                                    }
                                }
                            });
                            let waiting = st.learning.as_deref() == Some(action.as_str());
                            if ui.add_enabled(live.is_some(), egui::Button::new(tr(if waiting { "Press a button of the device... (click to cancel)" } else { "Detect" }))).clicked() {
                                st.learning = if waiting { None } else { Some(action.clone()) };
                            }
                        });
                    }
                }
            }
        });
    });
    if changed && !ui.input(|i| i.pointer.any_down()) {
        st.save();
    } else if changed {
        // a slider being dragged: written when the button is let go
        ui.memory_mut(|m| m.data.insert_temp(egui::Id::new("ctl-dirty"), true));
    } else if !ui.input(|i| i.pointer.any_down()) && ui.memory_mut(|m| m.data.remove_temp::<bool>(egui::Id::new("ctl-dirty"))).unwrap_or(false) {
        st.save();
    }
    if open_test {
        st.ffb_win = true;
        st.ffb_result = None;
    }
    if ffb_window(ui.ctx(), st) {
        changed = true;
    }
    if changed && !ui.input(|i| i.pointer.any_down()) {
        st.save();
    }
    // the settings file: written when the slider is let go
    st.cfg_dirty |= cfg_changed;
    if st.cfg_dirty && !ui.input(|i| i.pointer.any_down()) {
        if let Some(c) = cfg.as_deref() {
            if let Err(e) = c.save() {
                st.message = trf("Could not save the settings: {}", &[&e]);
            }
        }
        st.cfg_dirty = false;
    }
    if !st.message.is_empty() {
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), &st.message);
    }
    st.prev_buttons = st.live.map(|l| l.1).unwrap_or(0);
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        st.learning = None;
    }
    let _ = omsi;
}

/// A direction test under way: the wheel held, the pulses and what the wheel did.
struct FfbRun {
    test: crate::dinput::Ffb,
    cal: crate::ffb_calibration::Calibration,
    started: Instant,
}

/// "Test the direction": two short opposite forces on the wheel, and which way the wheel went says whether the game's forces must be
/// turned round. The result goes to this device's direction setting.
fn ffb_window(ctx: &egui::Context, st: &mut State) -> bool {
    if !st.ffb_win {
        return false;
    }
    // the pulses, frame by frame
    if let Some(run) = st.ffb_run.as_mut() {
        let now = run.started.elapsed().as_secs_f32();
        let pos = run.test.position();
        if let Some(f) = run.cal.update(now, pos) {
            run.test.pulse(f);
        }
        if let Some(r) = run.cal.result {
            st.ffb_result = Some(r.map_err(|e| e.to_string()));
            st.ffb_run = None; // the wheel let go
        }
        ctx.request_repaint();
    }
    let mut open = true;
    let mut start = false;
    let mut apply: Option<bool> = None;
    let running = st.ffb_run.is_some();
    let name = st.selected.clone();
    let current = st.devices.iter().find(|d| d.name == name).and_then(|d| d.ff_invert);
    open = crate::free::window(ctx, "ffb-test", tr("Force feedback direction"), egui::vec2(450.0, 320.0), |ui| {
        ui.set_width(430.0);
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), RichText::new(tr("INJURY RISK: TAKE YOUR HANDS OFF THE WHEEL. Keep hands and fingers clear before starting and throughout the test.")).strong());
        ui.add_space(4.0);
        ui.label(tr("The test applies two short forces in opposite directions."));
        ui.add_space(6.0);
        if running {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr("Testing: keep your hands off the wheel..."));
            });
        }
        match &st.ffb_result {
            Some(Ok(inv)) => {
                ui.colored_label(egui::Color32::from_rgb(40, 140, 70), tr(if *inv { "Direction detected: inverted" } else { "Direction detected: normal" }));
            }
            Some(Err(e)) => {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), tr(e));
            }
            None => {}
        }
        ui.add_enabled_ui(!running, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr("Test strength"));
                ui.add(egui::Slider::new(&mut st.ffb_strength, crate::ffb_calibration::PULSE_FORCE..=crate::ffb_calibration::MAX_PULSE_FORCE).step_by(0.01).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)));
            });
        });
        ui.label(RichText::new(tr("If the wheel barely moves, increase Test strength and retry. Keep your hands clear.")).weak().size(12.0));
        ui.add_space(6.0);
        let mut inv = match (&st.ffb_result, current) {
            (Some(Ok(v)), _) => *v,
            (_, c) => c.unwrap_or(false),
        };
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!running, |ui| {
                if ui.checkbox(&mut inv, tr("Invert force feedback")).changed() {
                    apply = Some(inv);
                }
            });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(!running, egui::Button::new(tr("Start test"))).clicked() {
                start = true;
            }
            if ui.add_enabled(!running, egui::Button::new(tr("Finish"))).clicked() {
                apply = Some(inv);
                st.ffb_win = false;
            }
        });
    });
    if !open {
        st.ffb_win = false;
    }
    if start {
        st.ffb_result = None;
        let steer = st.devices.iter().find(|d| d.name == name).and_then(|d| d.axes.iter().position(|a| a.0 == 0));
        match steer.ok_or_else(|| "No axis steers".to_string()).and_then(|a| crate::dinput::ffb_open(&name, a)) {
            Ok(test) => st.ffb_run = Some(FfbRun { test, cal: crate::ffb_calibration::Calibration::new(st.ffb_strength), started: Instant::now() }),
            Err(e) => st.ffb_result = Some(Err(e)),
        }
    }
    if !st.ffb_win {
        st.ffb_run = None;
    }
    if let Some(v) = apply {
        if let Some(d) = st.devices.iter_mut().find(|d| d.name == name) {
            d.ff_invert = Some(v);
            return true;
        }
    }
    false
}
