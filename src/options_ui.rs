//! The options window: openOMSI's settings, drawn from the table in `schema.rs` and kept in `~/.openomsi/settings.cfg`. Like the
//! original's page, every change is saved at once (a slider when the button is let go) and the game reads the file when it starts.

use crate::gamecfg::GameCfg;
use crate::i18n::{self, tr, trf};
use crate::options_extra::{self as extra, Extra, help_text, split_row};
use crate::schema::{self, Kind, Row, Tab, When};
use crate::theme::pal;
use eframe::egui::{self, RichText, Ui};
use std::path::Path;

pub struct State {
    pub cfg: Option<GameCfg>,
    tabs: Vec<Tab>,
    pub tab: usize,
    /// A change not yet written (a slider being dragged).
    pending: bool,
    controllers: Option<crate::controllers_ui::State>,
    keys: Option<crate::keys_ui::State>,
    /// The Controls tab shows the game controllers instead of the keyboard.
    pads: bool,
    ailists: Option<crate::ailists_ui::State>,
    trouble: Option<crate::troubleshoot_ui::State>,
    extra: Extra,
    /// The settings the installed game knows (`known.rs`); `Some(None)` is "could not tell", `None` "not looked yet".
    known: Option<Option<std::collections::HashSet<String>>>,
    message: String,
}

impl State {
    pub fn new() -> Self {
        Self { cfg: GameCfg::load(), tabs: schema::tabs(), tab: 0, pending: false, controllers: None, keys: None, pads: std::env::var_os("CL_PADS").is_some(), ailists: None, trouble: None, extra: Extra::new(), known: None, message: String::new() }
    }
}

fn value(cfg: &GameCfg, row: &Row) -> Option<String> {
    cfg.get(row.key)
}

fn text_is(cfg: &GameCfg, key: &str, def: &str, wanted: &str) -> bool {
    cfg.get(key).unwrap_or_else(|| def.to_string()) == wanted
}

/// Is the row shown with the values the file has now?
fn shown(cfg: &GameCfg, when: When) -> bool {
    let mode = {
        let g = cfg.get("graphics").unwrap_or_else(|| "vanilla_plus".into());
        if g == "enhanced_plus" { "enhanced".to_string() } else { g }
    };
    let classic = mode == "vanilla";
    let enhanced = mode == "enhanced";
    let traced = enhanced && (cfg.flag("ray_tracing", false) || cfg.get("graphics").as_deref() == Some("enhanced_plus"));
    let reflective = cfg.get("reflections").and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.0) > 0.005;
    match when {
        When::Always => true,
        When::NotClassic => !classic,
        When::Enhanced => enhanced,
        When::NotTraced => !traced,
        When::NotClassicNotTraced => !classic && !traced,
        When::EnhancedNotTraced => enhanced && !traced,
        When::EnhancedClouds => enhanced && cfg.flag("clouds", true),
        When::ReflectionsEnhanced => enhanced && reflective,
        When::ReflectionsPlain => !enhanced && reflective,
        When::Fsr => text_is(cfg, "upscaler", "default", "fsr1"),
        When::Voice => cfg.flag("voice_enabled", false),
        When::VoiceOpen => cfg.flag("voice_enabled", false) && text_is(cfg, "voice_mode", "ptt", "open"),
        When::HeadTracking => cfg.flag("head_tracking", false),
        When::HeadTrackingTobii => cfg.flag("head_tracking", false) && !text_is(cfg, "head_tracking_source", "auto", "opentrack"),
        When::Triple => cfg.flag("triple_screen", false),
        When::PlainView => !cfg.flag("triple_screen", false) || cfg.flag("vr", false),
        When::TripleView => cfg.flag("triple_screen", false) && !cfg.flag("vr", false),
        When::PassengerConnections => cfg.flag("passenger_connections", false),
        When::Vr => cfg.flag("vr", false),
    }
}

/// A slider value rounded to its step, without the tail of the float.
fn snap(v: f64, step: f64) -> String {
    let n = (v / step).round() * step;
    let s = format!("{n:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A switch like the original's: a pill with a knob, the chosen state in the accent colour.
fn switch(ui: &mut Ui, on: &mut bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(38.0, 20.0), egui::Sense::click());
    let mut changed = false;
    if resp.clicked() {
        *on = !*on;
        changed = true;
    }
    let t = ui.ctx().animate_bool(resp.id, *on);
    let off = pal().border;
    let on_color = pal().dark;
    let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    let fill = egui::Color32::from_rgb(lerp(off.r(), on_color.r()), lerp(off.g(), on_color.g()), lerp(off.b(), on_color.b()));
    ui.painter().rect_filled(rect, 10.0, fill);
    let x = egui::lerp((rect.left() + 10.0)..=(rect.right() - 10.0), t);
    ui.painter().circle_filled(egui::pos2(x, rect.center().y), 7.0, egui::Color32::WHITE);
    changed
}

fn label_with_help(ui: &mut Ui, row: &Row) {
    ui.add(egui::Label::new(tr(row.label)).wrap());
    if !row.help.is_empty() {
        help_text(ui, tr(row.help));
    }
}

/// One row of a card.
pub fn draw_row(ui: &mut Ui, cfg: &mut GameCfg, row: &Row, languages: &[(String, String)]) -> bool {
    let mut changed = false;
    match &row.kind {
        Kind::Section | Kind::Custom(_) | Kind::Action(_) => {}
        Kind::Note => {
            help_text(ui, tr(row.label));
            ui.add_space(4.0);
        }
        Kind::Toggle { def } => {
            let mut on = cfg.flag(row.key, *def);
            split_row(
                ui,
                |ui| label_with_help(ui, row),
                |ui| {
                    if switch(ui, &mut on) {
                        cfg.set(row.key, if on { "1" } else { "0" });
                        changed = true;
                    }
                },
            );
        }
        Kind::Select { def, options } => {
            let current = value(cfg, row).unwrap_or_else(|| def.to_string());
            let shown_label = options.iter().find(|o| o.0 == current).map(|o| tr(o.1).to_string()).unwrap_or_else(|| current.clone());
            split_row(
                ui,
                |ui| label_with_help(ui, row),
                |ui| {
                    egui::ComboBox::from_id_salt(egui::Id::new(("opt", row.key, row.label))).selected_text(shown_label).width(210.0).truncate().show_ui(ui, |ui| {
                        for (v, l) in options.iter() {
                            if ui.selectable_label(current == *v, tr(l)).clicked() && current != *v {
                                cfg.set(row.key, v);
                                changed = true;
                            }
                        }
                    });
                },
            );
        }
        Kind::Slider { min, max, step, def, fmt } => {
            let mut v = value(cfg, row).and_then(|s| s.parse::<f64>().ok()).unwrap_or(*def);
            let before = v;
            // the name on the left, the value on the right, the slider across the whole card under them
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(tr(row.label)).wrap());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fmt.show(v)).weak());
                });
            });
            let width = ui.available_width() - 8.0;
            ui.scope(|ui| {
                // the rail dark enough to see on a light page, filled with the accent up to the knob
                ui.spacing_mut().slider_width = width;
                ui.visuals_mut().widgets.inactive.bg_fill = pal().border;
                ui.visuals_mut().widgets.hovered.bg_fill = pal().border;
                ui.visuals_mut().selection.bg_fill = pal().dark;
                ui.add(egui::Slider::new(&mut v, *min..=*max).step_by(*step).show_value(false).trailing_fill(true));
            });
            if !row.help.is_empty() {
                help_text(ui, tr(row.help));
            }
            ui.add_space(6.0);
            if (v - before).abs() > f64::EPSILON {
                let stored = if row.zero_below > 0.0 && v < row.zero_below { 0.0 } else { v };
                cfg.set(row.key, &snap(stored, *step));
                changed = true;
            }
        }
        Kind::Language => {
            let current = cfg.get(row.key).unwrap_or_else(|| "ENG".into());
            let name = languages.iter().find(|l| l.0.eq_ignore_ascii_case(&current)).map(|l| l.1.clone()).unwrap_or_else(|| current.clone());
            split_row(
                ui,
                |ui| label_with_help(ui, row),
                |ui| {
                    egui::ComboBox::from_id_salt("opt-language").selected_text(name).width(210.0).truncate().show_ui(ui, |ui| {
                        for (code, n) in languages {
                            if ui.selectable_label(current.eq_ignore_ascii_case(code), n).clicked() {
                                cfg.set(row.key, code);
                                changed = true;
                            }
                        }
                    });
                },
            );
        }
    }
    changed
}

/// What an action button does. Returns true when it changed the settings; `goto` asks for another tab.
fn do_action(id: &str, cfg: &mut GameCfg, x: &mut Extra, game: Option<&Path>, goto: &mut Option<&'static str>) -> bool {
    let set_all = |cfg: &mut GameCfg, pairs: &[(&str, &str)]| {
        for (k, v) in pairs {
            cfg.set(k, v);
        }
    };
    match id {
        "configure_wheel" => {
            *goto = Some("controllers");
            false
        }
        "reset_wheel" => {
            set_all(cfg, &[("wheel_range", "900"), ("wheel_lock", "0"), ("cab_wheel_rotation", "0"), ("ff_invert", "0"), ("ff_enabled", "1"), ("ff_road_vib", "1"), ("ff_engine_vib", "1"), ("ff_fade", "0.28")]);
            true
        }
        "reset_seat" => {
            set_all(cfg, &[("seat_x", "0"), ("seat_y", "0"), ("seat_z", "0"), ("seat_pitch_deg", "0")]);
            true
        }
        "reset_tracking" => {
            for axis in ["yaw", "pitch", "roll", "x", "y", "z"] {
                cfg.set(&format!("head_tracking_{axis}_sens"), "15");
                cfg.set(&format!("head_tracking_invert_{axis}"), "0");
            }
            true
        }
        "support_pack" => {
            x.message = support_package(game);
            false
        }
        _ => false,
    }
}

/// Has the game write its support package (the system, the graphics device and settings: no paths, names or log text) and shows it.
fn support_package(game: Option<&Path>) -> String {
    use std::os::windows::process::CommandExt;
    let (Some(game), Some(dir)) = (game, crate::omsi::data_dir()) else { return tr("openOMSI not found").to_string() };
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let out = dir.join(format!("support-{stamp}.zip"));
    let ok = std::process::Command::new(game).arg("--export-diagnostics").arg(&out).creation_flags(0x0800_0000).status().map(|s| s.success()).unwrap_or(false);
    if ok && out.exists() {
        let _ = std::process::Command::new("explorer").arg(format!("/select,{}", out.display())).spawn();
        trf("Support package written to {}", &[&out.display().to_string()])
    } else {
        tr("The game could not write the support package.").to_string()
    }
}

/// A card of the tab: its title and its rows, under each section heading.
/// Is the row left out because the installed game does not know its setting? (only the rows that are a setting)
fn unknown(row: &Row, known: Option<&std::collections::HashSet<String>>) -> bool {
    let Some(known) = known else { return false };
    matches!(row.kind, Kind::Toggle { .. } | Kind::Select { .. } | Kind::Slider { .. }) && !row.key.is_empty() && !known.contains(row.key)
}

fn draw_column(ui: &mut Ui, cfg: &mut GameCfg, col: &schema::Column, languages: &[(String, String)], x: &mut Extra, game: Option<&Path>, known: Option<&std::collections::HashSet<String>>, goto: &mut Option<&'static str>) -> bool {
    let mut changed = false;
    egui::Frame::new().fill(ui.visuals().faint_bg_color).stroke(ui.visuals().widgets.noninteractive.bg_stroke).corner_radius(6.0).inner_margin(egui::Margin::same(14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(tr(col.title)).strong().size(17.0).color(pal().dark));
        ui.add_space(8.0);
        for (i, row) in col.rows.iter().enumerate() {
            // a heading whose rows are all left out goes too
            let empty_section = matches!(row.kind, Kind::Section) && col.rows[i + 1..].iter().take_while(|r| !matches!(r.kind, Kind::Section)).all(|r| unknown(r, known));
            if !shown(cfg, row.when) || unknown(row, known) || empty_section {
                continue;
            }
            match &row.kind {
                Kind::Section => {
                    ui.add_space(10.0);
                    ui.label(RichText::new(tr(row.label)).weak().strong());
                    ui.separator();
                }
                Kind::Custom("quality") => changed |= extra::quality_row(ui, cfg),
                Kind::Custom("optimise") => changed |= extra::optimise_block(ui, cfg, x),
                Kind::Custom("profiles") => changed |= extra::profiles_block(ui, cfg, x),
                Kind::Custom("radio") => extra::radio_block(ui, x),
                Kind::Custom("corner") => changed |= extra::corner_block(ui, cfg),
                Kind::Custom(_) => {}
                Kind::Action("reset_all") => {
                    if x.confirm_reset {
                        ui.label(RichText::new(tr("Every setting goes back to the game's default. A copy of the file is kept.")).size(12.0));
                        ui.horizontal(|ui| {
                            if ui.button(tr("Yes, reset")).clicked() {
                                cfg.clear();
                                x.confirm_reset = false;
                                changed = true;
                            }
                            if ui.button(tr("Cancel")).clicked() {
                                x.confirm_reset = false;
                            }
                        });
                    } else if ui.add(egui::Button::new(RichText::new(tr(row.label)).color(egui::Color32::from_rgb(200, 60, 50)))).clicked() {
                        x.confirm_reset = true;
                    }
                    ui.add_space(6.0);
                }
                Kind::Action(id) => {
                    if ui.button(tr(row.label)).clicked() {
                        changed |= do_action(id, cfg, x, game, goto);
                    }
                    ui.add_space(6.0);
                }
                _ => changed |= draw_row(ui, cfg, row, languages),
            }
        }
    });
    changed
}

/// A tab button in the launcher's style: the chosen one filled.
pub fn tab_button(ui: &mut Ui, text: &str, on: bool) -> bool {
    let b = egui::Button::new(RichText::new(text).strong().color(if on { egui::Color32::WHITE } else { pal().text })).fill(if on { pal().dark } else { pal().field }).min_size(egui::vec2(0.0, 30.0));
    ui.add(b).clicked()
}

/// The window's contents.
pub fn show(ui: &mut Ui, st: &mut State, omsi: Option<&Path>, game: Option<&Path>, updates: &mut dyn FnMut(&mut Ui)) {
    ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
    // a scroll bar that takes its own room, so it does not cover the cards' right edge
    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
    let languages: Vec<(String, String)> = {
        let mut v: Vec<(String, String)> = i18n::available(omsi).into_iter().map(|l| (l.code().to_string(), l.name().to_string())).collect();
        if v.is_empty() {
            v.push(("ENG".into(), "English".into()));
        }
        v
    };
    ui.label(RichText::new(tr("Every change is saved at once; the game reads them when it starts.")).italics().size(12.0));
    ui.horizontal(|ui| {
        for (i, t) in st.tabs.iter().enumerate() {
            if tab_button(ui, tr(t.label), st.tab == i) {
                st.tab = i;
            }
        }
        // after the settings' own tabs: the keys and the game controllers (their own files)
        if tab_button(ui, tr("Controls"), st.tab == st.tabs.len()) {
            st.tab = st.tabs.len();
        }
        if tab_button(ui, tr("AI lists"), st.tab == st.tabs.len() + 1) {
            st.tab = st.tabs.len() + 1;
        }
        if tab_button(ui, tr("Troubleshooting"), st.tab == st.tabs.len() + 2) {
            st.tab = st.tabs.len() + 2;
        }
        if tab_button(ui, tr("Updates"), st.tab == st.tabs.len() + 3) {
            st.tab = st.tabs.len() + 3;
        }
    });
    ui.separator();
    if st.tab == st.tabs.len() {
        ui.horizontal(|ui| {
            if tab_button(ui, tr("Keyboard"), !st.pads) {
                st.pads = false;
            }
            if tab_button(ui, tr("Game controllers"), st.pads) {
                st.pads = true;
            }
        });
        ui.add_space(2.0);
        if st.pads {
            let state = st.controllers.get_or_insert_with(|| crate::controllers_ui::State::new(omsi));
            crate::controllers_ui::show(ui, state, omsi, st.cfg.as_mut());
        } else {
            let content = game.and_then(crate::omsi::content_dir);
            let state = st.keys.get_or_insert_with(|| crate::keys_ui::State::new(omsi, content.as_deref()));
            if crate::keys_ui::show(ui, state) {
                // a key changed is a key wanted: the game uses the keys of this page
                if let Some(cfg) = st.cfg.as_mut() {
                    if cfg.get("drive_keys").as_deref().unwrap_or("simple") != "omsi" {
                        cfg.set("drive_keys", "omsi");
                        let _ = cfg.save();
                    }
                }
            }
        }
        return;
    }
    if st.tab == st.tabs.len() + 3 {
        egui::ScrollArea::vertical().id_salt("options-updates").auto_shrink([false, false]).show(ui, |ui| updates(ui));
        return;
    }
    if st.tab > st.tabs.len() {
        if let Some(o) = omsi {
            let content = game.and_then(crate::omsi::content_dir);
            if st.tab == st.tabs.len() + 2 {
                let state = st.trouble.get_or_insert_with(|| crate::troubleshoot_ui::State::new(o, content.as_deref()));
                crate::troubleshoot_ui::show(ui, state, game);
                return;
            }
            let state = st.ailists.get_or_insert_with(|| crate::ailists_ui::State::new(o, content.as_deref(), crate::omsi::list_maps(o), Some("Grundorf")));
            crate::ailists_ui::show(ui, state);
        }
        return;
    }
    let Some(cfg) = st.cfg.as_mut() else {
        ui.label(tr("Could not save the settings: {}"));
        return;
    };
    if st.known.is_none() {
        st.known = Some(game.and_then(|g| crate::known::known(g, &schema::keys())));
    }
    let known = st.known.clone().flatten();
    let tab = &st.tabs[st.tab.min(st.tabs.len() - 1)];
    let mut changed = false;
    let mut goto: Option<&'static str> = None;
    egui::ScrollArea::vertical().id_salt(("options-tab", st.tab)).auto_shrink([false, false]).max_height((ui.available_height() - 36.0).max(100.0)).show(ui, |ui| {
        ui.columns(tab.cols.len(), |cols| {
            for (ui, col) in cols.iter_mut().zip(&tab.cols) {
                changed |= draw_column(ui, cfg, col, &languages, &mut st.extra, game, known.as_ref(), &mut goto);
            }
        });
    });
    if goto == Some("controllers") {
        st.tab = st.tabs.len();
        st.pads = false;
    }
    st.pending |= changed;
    // saved at once; a slider is written when the button is let go
    if st.pending && !ui.input(|i| i.pointer.any_down()) {
        st.message = match cfg.save() {
            Ok(()) => tr("Saved.").to_string(),
            Err(e) => trf("Could not save the settings: {}", &[&e]),
        };
        st.pending = false;
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new(cfg.path.display().to_string()).size(12.0).weak());
        ui.label(&st.message);
        if !st.extra.message.is_empty() {
            ui.label(&st.extra.message);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping_drops_the_float_tail() {
        assert_eq!(snap(0.30000000000000004, 0.05), "0.3");
        assert_eq!(snap(900.0, 30.0), "900");
        assert_eq!(snap(1.3, 0.05), "1.3");
    }

    #[test]
    fn rows_follow_the_graphics_mode() {
        let mut c = GameCfg::parse("x".into(), "graphics=vanilla\n");
        assert!(!shown(&c, When::NotClassic) && !shown(&c, When::Enhanced));
        c.set("graphics", "enhanced");
        assert!(shown(&c, When::Enhanced) && shown(&c, When::NotTraced));
        c.set("ray_tracing", "1");
        assert!(!shown(&c, When::NotTraced));
    }
}
