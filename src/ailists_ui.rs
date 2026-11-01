//! The "AI lists" tab of the options window: which cars and lorries make up a map's random traffic (with the weight of each) and
//! which buses its depots hold. The player's changes are saved at once to the map's `ailists_#user.cfg` in the game's content
//! folder (see `ailists`).

use crate::ailists::{self, Entry, Group, LibVehicle, Line, Rule, TypGroup, Vehicle};
use crate::i18n::{tr, trf};
use crate::omsi::MapInfo;
use crate::pick;
use crate::theme::pal;
use eframe::egui::{self, RichText, Ui};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Traffic,
    Depot,
    Lines,
}

#[derive(Clone, Copy)]
enum PickKind {
    /// A vehicle of the traffic group: added, or put in the place of the one at this index.
    Vehicle(Option<usize>),
    /// A type of bus added to the depot.
    TypeAdd,
    /// The type at this index replaced by another (its buses stay).
    TypeReplace(usize),
    /// A type added to the rule of the chosen line.
    RuleType,
}

struct Picker {
    kind: PickKind,
    search: String,
}

pub struct State {
    omsi: PathBuf,
    content: Option<PathBuf>,
    maps: Vec<MapInfo>,
    map: usize,
    tab: Tab,
    base: Vec<Group>,
    edited: Vec<Group>,
    /// The chosen group of each tab, by name.
    sel: String,
    sel_depot: String,
    rules: Vec<Rule>,
    lines: Vec<Line>,
    sel_line: String,
    line_filter: String,
    paints: std::collections::HashMap<String, Vec<String>>,
    library: Arc<Mutex<Option<Vec<LibVehicle>>>>,
    picker: Option<Picker>,
    pending: bool,
    message: String,
}

fn is_rail(g: &Group) -> bool {
    !g.vehicles.is_empty() && g.vehicles.iter().all(|v| v.file.to_lowercase().ends_with(".zug"))
}

impl State {
    pub fn new(omsi: &Path, content: Option<&Path>, maps: Vec<MapInfo>, start_map: Option<&str>) -> Self {
        let library: Arc<Mutex<Option<Vec<LibVehicle>>>> = Arc::new(Mutex::new(None));
        {
            let (lib, o, c) = (library.clone(), omsi.to_path_buf(), content.map(Path::to_path_buf));
            std::thread::spawn(move || {
                let v = ailists::library(&o, c.as_deref());
                *lib.lock().unwrap() = Some(v);
            });
        }
        let map = start_map.and_then(|f| maps.iter().position(|m| m.folder == f)).unwrap_or(0);
        let mut s = Self { omsi: omsi.to_path_buf(), content: content.map(Path::to_path_buf), maps, map, tab: Tab::Traffic, base: vec![], edited: vec![], sel: String::new(), sel_depot: String::new(), rules: vec![], lines: vec![], sel_line: String::new(), line_filter: String::new(), paints: Default::default(), library, picker: None, pending: false, message: String::new() };
        s.load_map();
        // (to look at the depot tab without clicking: CL_AI=depot)
        match std::env::var("CL_AI").as_deref() {
            Ok("depot") => s.tab = Tab::Depot,
            Ok("lines") => s.tab = Tab::Lines,
            _ => {}
        }
        s
    }

    fn folder(&self) -> String {
        self.maps.get(self.map).map(|m| m.folder.clone()).unwrap_or_default()
    }

    fn load_map(&mut self) {
        let (base, edited) = ailists::load(&self.omsi, self.content.as_deref(), &self.folder());
        self.base = base;
        self.edited = edited;
        self.sel = self.edited.iter().find(|g| !g.depot && !is_rail(g)).map(|g| g.name.clone()).unwrap_or_default();
        self.sel_depot = self.edited.iter().find(|g| g.depot).map(|g| g.name.clone()).unwrap_or_default();
        self.rules = self.content.as_deref().map(|c| ailists::load_rules(c, &self.folder())).unwrap_or_default();
        self.lines = ailists::lines(&self.omsi, &self.folder());
        self.sel_line.clear();
        self.pending = false;
    }

    fn save(&mut self) {
        let Some(c) = self.content.clone() else { return };
        let saved = ailists::save(&c, &self.folder(), &self.base, &self.edited).and_then(|()| ailists::save_rules(&c, &self.folder(), &self.rules));
        self.message = match saved {
            Ok(()) => tr("Saved.").to_string(),
            Err(e) => e,
        };
        self.pending = false;
    }

    /// Is the group different from the map's own?
    fn changed(&self, g: &Group) -> bool {
        self.base.iter().find(|b| b.depot == g.depot && b.name.trim().eq_ignore_ascii_case(g.name.trim())).map_or(true, |b| b != g)
    }

    fn reset_group(&mut self, i: usize) -> bool {
        let g = self.edited[i].clone();
        match self.base.iter().find(|b| b.depot == g.depot && b.name.trim().eq_ignore_ascii_case(g.name.trim())) {
            Some(b) => {
                self.edited[i] = b.clone();
                true
            }
            None => false,
        }
    }
}

fn group_index(st: &State) -> Option<usize> {
    match st.tab {
        Tab::Traffic => st.edited.iter().position(|g| !g.depot && !is_rail(g) && g.name == st.sel),
        Tab::Depot => st.edited.iter().position(|g| g.depot && g.name == st.sel_depot),
        Tab::Lines => None,
    }
}

/// The next fleet number of a depot: one above the largest number there.
fn next_number(g: &Group) -> u32 {
    g.typgroups.iter().flat_map(|t| t.entries.iter()).filter_map(|e| e.number.trim().parse::<u32>().ok()).max().unwrap_or(0) + 1
}

fn new_entries(g: &Group, n: usize) -> Vec<Entry> {
    let mut probe = g.clone();
    probe.typgroups.push(TypGroup::default());
    let mut out = vec![];
    for _ in 0..n {
        let e = Entry { number: next_number(&probe).to_string(), ..Default::default() };
        probe.typgroups.last_mut().unwrap().entries.push(e.clone());
        out.push(e);
    }
    out
}

fn stem_of(file: &str) -> String {
    ailists::pretty(file.rsplit('\\').next().unwrap_or(file).rsplit_once('.').map_or(file, |x| x.0))
}

fn folder_of(file: &str) -> &str {
    file.rsplit_once('\\').map(|x| x.0).unwrap_or("")
}

pub fn show(ui: &mut Ui, st: &mut State) {
    ui.add(egui::Label::new(RichText::new(tr("Choose which AI cars and buses make up the traffic of a map. You choose among the vehicles you have installed; nothing new is added and the map's own lists stay as they are.")).size(12.0)).wrap());
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let names: Vec<String> = st.maps.iter().map(|m| m.name.clone()).collect();
        let mut map = st.map;
        if pick::pick(ui, "ai-map", 260.0, &names, &mut map) {
            st.map = map;
            st.load_map();
        }
        ui.add_space(8.0);
        if crate::options_ui::tab_button(ui, tr("Traffic"), st.tab == Tab::Traffic) {
            st.tab = Tab::Traffic;
        }
        if crate::options_ui::tab_button(ui, tr("Depot buses"), st.tab == Tab::Depot) {
            st.tab = Tab::Depot;
        }
        if crate::options_ui::tab_button(ui, tr("Lines"), st.tab == Tab::Lines) {
            st.tab = Tab::Lines;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(tr("Reset the map")).clicked() {
                if let Some(c) = &st.content {
                    ailists::reset_map(c, &st.folder());
                }
                st.load_map();
                st.message = tr("The map's own lists are back.").to_string();
            }
        });
    });
    egui::Frame::new().fill(ui.visuals().faint_bg_color).corner_radius(4.0).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(tr("The changes hold for every date of this map.")).size(12.0));
    });
    ui.add_space(6.0);
    if st.content.is_none() {
        ui.label(tr("openOMSI not found"));
        return;
    }
    if st.tab == Tab::Lines {
        lines_tab(ui, st);
        if st.pending && !ui.input(|i| i.pointer.any_down()) {
            st.save();
        }
        if !st.message.is_empty() {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.label(RichText::new(&st.message).size(12.0).weak());
            });
        }
        picker_window(ui.ctx(), st);
        return;
    }
    let depot = st.tab == Tab::Depot;
    // (name, count, changed) of the groups of this tab
    let groups: Vec<(String, usize, bool)> = st
        .edited
        .iter()
        .filter(|g| g.depot == depot && (depot || !is_rail(g)))
        .map(|g| (g.name.clone(), if depot { g.typgroups.iter().map(|t| t.entries.len()).sum() } else { g.vehicles.len() }, st.changed(g)))
        .collect();
    if groups.is_empty() {
        ui.label(RichText::new(if depot { tr("This map has no depot buses.") } else { tr("This map has no traffic groups.") }).italics());
    }
    if group_index(st).is_none() {
        let first = groups.first().map(|g| g.0.clone()).unwrap_or_default();
        if depot {
            st.sel_depot = first
        } else {
            st.sel = first
        }
    }
    let mut changed = false;
    ui.horizontal_top(|ui| {
        // the groups
        ui.allocate_ui_with_layout(egui::vec2(190.0, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(190.0);
            for (name, n, modified) in &groups {
                let on = if depot { *name == st.sel_depot } else { *name == st.sel };
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(190.0, 42.0), egui::Sense::click());
                let fill = if on { pal().dark } else if resp.hovered() { pal().hover } else { pal().field };
                ui.painter().rect_filled(rect, 4.0, fill);
                let fg = if on { egui::Color32::WHITE } else { pal().text };
                ui.painter().text(rect.left_top() + egui::vec2(10.0, 8.0), egui::Align2::LEFT_TOP, name, egui::FontId::proportional(14.0), fg);
                let sub = if depot { st.edited.iter().find(|g| g.depot && g.name == *name).and_then(|g| g.hof.clone()).unwrap_or_default() } else { tr("Random traffic").to_string() };
                ui.painter().text(rect.left_top() + egui::vec2(10.0, 25.0), egui::Align2::LEFT_TOP, sub, egui::FontId::proportional(11.0), fg.gamma_multiply(0.8));
                ui.painter().text(rect.right_center() - egui::vec2(10.0, 0.0), egui::Align2::RIGHT_CENTER, n.to_string(), egui::FontId::proportional(12.0), fg.gamma_multiply(0.8));
                if *modified {
                    ui.painter().circle_filled(rect.right_top() + egui::vec2(-8.0, 8.0), 3.0, egui::Color32::from_rgb(240, 160, 30));
                }
                if resp.clicked() {
                    if depot {
                        st.sel_depot = name.clone()
                    } else {
                        st.sel = name.clone()
                    }
                }
                ui.add_space(2.0);
            }
        });
        // the chosen group
        let Some(gi) = group_index(st) else { return };
        let card_width = ui.available_width();
        ui.vertical(|ui| {
            ui.set_width(card_width);
            egui::Frame::new().fill(ui.visuals().faint_bg_color).stroke(ui.visuals().widgets.noninteractive.bg_stroke).corner_radius(6.0).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&st.edited[gi].name).strong().size(16.0).color(pal().dark));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let add_label = if depot { tr("Add type") } else { tr("Add vehicle") };
                        let add = egui::Button::new(RichText::new(format!("+ {add_label}")).color(egui::Color32::WHITE)).fill(pal().dark);
                        if ui.add(add).clicked() {
                            st.picker = Some(Picker { kind: if depot { PickKind::TypeAdd } else { PickKind::Vehicle(None) }, search: String::new() });
                        }
                        let differs = st.changed(&st.edited[gi]);
                        if ui.add_enabled(differs, egui::Button::new(tr("Reset the group"))).clicked() && st.reset_group(gi) {
                            changed = true;
                        }
                    });
                });
                ui.label(RichText::new(if depot { tr("The buses of this depot: the timetable draws its buses from here, in proportion to how many of each type there are.") } else { tr("The vehicles that make up the random traffic of this type.") }).size(11.0).weak());
                ui.add_space(6.0);
                let avail = (ui.available_height() - 4.0).max(120.0);
                if depot {
                    changed |= depot_card(ui, st, gi, avail);
                } else {
                    changed |= traffic_card(ui, st, gi, avail);
                }
            });
        });
    });
    st.pending |= changed;
    if st.pending && !ui.input(|i| i.pointer.any_down()) {
        st.save();
    }
    if !st.message.is_empty() {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.label(RichText::new(&st.message).size(12.0).weak());
        });
    }
    picker_window(ui.ctx(), st);
}

/// The vehicles of a traffic group with their weights.
fn traffic_card(ui: &mut Ui, st: &mut State, gi: usize, height: f32) -> bool {
    let mut changed = false;
    let total: f32 = st.edited[gi].vehicles.iter().map(|v| v.weight.max(0.0)).sum();
    let mut remove = None;
    let mut replace = None;
    egui::ScrollArea::vertical().id_salt("ai-vehicles").auto_shrink([false, false]).max_height(height).show(ui, |ui| {
        for (i, v) in st.edited[gi].vehicles.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(stem_of(&v.file)).strong());
                    ui.label(RichText::new(folder_of(&v.file)).size(11.0).weak());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text(tr("Delete")).clicked() {
                        remove = Some(i);
                    }
                    if ui.link(tr("Replace")).clicked() {
                        replace = Some(i);
                    }
                    let share = if total > 0.0 { v.weight.max(0.0) / total * 100.0 } else { 0.0 };
                    ui.label(RichText::new(format!("{share:.0}%")).weak());
                    let before = v.weight;
                    ui.add(egui::DragValue::new(&mut v.weight).range(0.0..=10000.0).speed(0.5).max_decimals(1));
                    if v.weight != before {
                        changed = true;
                    }
                });
            });
            ui.separator();
        }
    });
    if let Some(i) = remove {
        st.edited[gi].vehicles.remove(i);
        changed = true;
    }
    if let Some(i) = replace {
        st.picker = Some(Picker { kind: PickKind::Vehicle(Some(i)), search: String::new() });
    }
    changed
}

/// The types of a depot with their buses: fleet number, plate and repaint of each.
fn depot_card(ui: &mut Ui, st: &mut State, gi: usize, height: f32) -> bool {
    let mut changed = false;
    let mut remove_type = None;
    let mut replace_type = None;
    let mut add_bus = None;
    egui::ScrollArea::vertical().id_salt("ai-depot").auto_shrink([false, false]).max_height(height).show(ui, |ui| {
        let types = st.edited[gi].typgroups.len();
        for ti in 0..types {
            let file = st.edited[gi].typgroups[ti].file.clone();
            let paint_names = st.paints.entry(file.clone()).or_insert_with(|| ailists::paints(&st.omsi, st.content.as_deref(), &file)).clone();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(stem_of(&file)).strong());
                    ui.label(RichText::new(folder_of(&file)).size(11.0).weak());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text(tr("Remove the type")).clicked() {
                        remove_type = Some(ti);
                    }
                    if ui.button(format!("+ {}", tr("Add bus"))).clicked() {
                        add_bus = Some(ti);
                    }
                    if ui.button(tr("Replace type")).clicked() {
                        replace_type = Some(ti);
                    }
                });
            });
            let mut remove_entry = None;
            let entries = &mut st.edited[gi].typgroups[ti].entries;
            for (ei, e) in entries.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    changed |= ui.add(egui::TextEdit::singleline(&mut e.number).hint_text(tr("Number")).desired_width(70.0)).on_hover_text(tr("Fleet number")).changed();
                    changed |= ui.add(egui::TextEdit::singleline(&mut e.registration).hint_text(tr("Plate")).desired_width(120.0)).on_hover_text(tr("Plate")).changed();
                    if paint_names.is_empty() {
                        changed |= ui.add(egui::TextEdit::singleline(&mut e.paint).hint_text(tr("Its own paint")).desired_width(150.0)).on_hover_text(tr("Repaint")).changed();
                    } else {
                        let shown = if e.paint.is_empty() { tr("Default paint").to_string() } else { e.paint.clone() };
                        egui::ComboBox::from_id_salt(("ai-paint", ti, ei)).selected_text(shown).width(150.0).show_ui(ui, |ui| {
                            if ui.selectable_label(e.paint.is_empty(), tr("Default paint")).clicked() {
                                e.paint.clear();
                                changed = true;
                            }
                            for n in &paint_names {
                                if ui.selectable_label(e.paint.eq_ignore_ascii_case(n), n).clicked() {
                                    e.paint = n.clone();
                                    changed = true;
                                }
                            }
                        });
                    }
                    if e.from.is_some() || e.to.is_some() {
                        let d = |v: Option<i32>| v.map(|d| format!("{:04}-{:02}-{:02}", d / 10000, d / 100 % 100, d % 100)).unwrap_or_else(|| tr("always").to_string());
                        ui.label(RichText::new(trf("{} to {}", &[&d(e.from), &d(e.to)])).size(11.0).weak());
                    }
                    if ui.button("✕").on_hover_text(tr("Remove")).clicked() {
                        remove_entry = Some(ei);
                    }
                });
            }
            if entries.is_empty() {
                ui.label(RichText::new(tr("No bus of this type: it never runs.")).size(11.0).italics());
            }
            if let Some(ei) = remove_entry {
                entries.remove(ei);
                changed = true;
            }
            ui.separator();
        }
    });
    if let Some(ti) = remove_type {
        st.edited[gi].typgroups.remove(ti);
        changed = true;
    }
    if let Some(ti) = add_bus {
        let new = new_entries(&st.edited[gi], 1);
        st.edited[gi].typgroups[ti].entries.extend(new);
        changed = true;
    }
    if let Some(ti) = replace_type {
        st.picker = Some(Picker { kind: PickKind::TypeReplace(ti), search: String::new() });
    }
    changed
}

/// The window to choose a vehicle of the installed content to add or put in the place of one.
fn picker_window(ctx: &egui::Context, st: &mut State) {
    let Some(mut p) = st.picker.take() else { return };
    let mut open = true;
    let mut chosen: Option<String> = None;
    let lib = st.library.lock().unwrap().clone();
    let title = match p.kind {
        PickKind::Vehicle(None) => tr("Add vehicle"),
        PickKind::Vehicle(Some(_)) => tr("Replace vehicle"),
        PickKind::TypeAdd => tr("Add type"),
        PickKind::TypeReplace(_) => tr("Replace type"),
        PickKind::RuleType => tr("Add a type to the line"),
    };
    open = crate::free::window(ctx, "ai-picker", title, egui::vec2(560.0, 560.0), |ui| {
        ui.set_width(520.0);
        ui.add(egui::TextEdit::singleline(&mut p.search).hint_text(tr("Search by name")).desired_width(f32::INFINITY));
        ui.add_space(4.0);
        match &lib {
            None => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr("Looking for the installed vehicles..."));
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(300));
            }
            Some(list) => {
                let q = p.search.to_lowercase();
                let shown: Vec<&LibVehicle> = list.iter().filter(|v| q.is_empty() || v.name.to_lowercase().contains(&q) || v.file.to_lowercase().contains(&q)).take(400).collect();
                ui.label(RichText::new(trf("{} vehicles", &[&shown.len().to_string()])).size(11.0).weak());
                egui::ScrollArea::vertical().id_salt("ai-library").max_height(420.0).show(ui, |ui| {
                    for v in shown {
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), egui::Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(rect, 3.0, pal().hover);
                        }
                        ui.painter().text(rect.left_top() + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP, &v.name, egui::FontId::proportional(14.0), pal().text);
                        ui.painter().text(rect.left_top() + egui::vec2(8.0, 21.0), egui::Align2::LEFT_TOP, &v.file, egui::FontId::proportional(11.0), pal().text.gamma_multiply(0.6));
                        if resp.clicked() {
                            chosen = Some(v.file.clone());
                        }
                    }
                });
            }
        }
    });
    if let (Some(file), PickKind::RuleType) = (chosen.clone(), p.kind) {
        add_rule_type(st, file);
        return;
    }
    if let Some(file) = chosen {
        if let Some(gi) = group_index(st) {
            match p.kind {
                PickKind::Vehicle(Some(i)) if i < st.edited[gi].vehicles.len() => st.edited[gi].vehicles[i].file = file,
                PickKind::Vehicle(_) => st.edited[gi].vehicles.push(Vehicle { file, weight: 10.0 }),
                PickKind::TypeReplace(ti) if ti < st.edited[gi].typgroups.len() => {
                    // the new type takes the buses; their repaints belonged to the old model, so they go
                    st.edited[gi].typgroups[ti].file = file;
                    for e in &mut st.edited[gi].typgroups[ti].entries {
                        e.paint.clear();
                    }
                }
                PickKind::TypeReplace(_) | PickKind::RuleType => {}
                PickKind::TypeAdd => {
                    if !st.edited[gi].typgroups.iter().any(|t| t.file.eq_ignore_ascii_case(&file)) {
                        let entries = new_entries(&st.edited[gi], 3);
                        st.edited[gi].typgroups.push(TypGroup { file, entries });
                    }
                }
            }
            st.pending = true;
        }
    } else if open {
        st.picker = Some(p);
    }
}

/// The chosen line's rule, if the player gave it one.
fn rule_index(st: &State) -> Option<usize> {
    st.rules.iter().position(|r| r.line.eq_ignore_ascii_case(&st.sel_line))
}

/// A type is added to the line's rule; a depot of the line that lacks it gets it, with a few buses, so that the line can run it.
fn add_rule_type(st: &mut State, file: String) {
    let Some(ri) = rule_index(st) else { return };
    if !st.rules[ri].types.iter().any(|t| t.eq_ignore_ascii_case(&file)) {
        st.rules[ri].types.push(file.clone());
        let depots: Vec<String> = st.lines.iter().find(|l| l.name.eq_ignore_ascii_case(&st.sel_line)).map(|l| l.groups.clone()).unwrap_or_default();
        for name in depots {
            if let Some(gi) = st.edited.iter().position(|g| g.depot && g.name.trim().eq_ignore_ascii_case(name.trim())) {
                if !st.edited[gi].typgroups.iter().any(|t| t.file.eq_ignore_ascii_case(&file)) {
                    let entries = new_entries(&st.edited[gi], 3);
                    st.edited[gi].typgroups.push(TypGroup { file: file.clone(), entries });
                }
            }
        }
    }
    st.pending = true;
}

/// Which bus types run on each line: a rule per line, written as a car-use file the game reads before the map's own.
fn lines_tab(ui: &mut Ui, st: &mut State) {
    let depot_names: Vec<String> = st.edited.iter().filter(|g| g.depot).map(|g| g.name.trim().to_ascii_lowercase()).collect();
    let q = st.line_filter.trim().to_lowercase();
    let shown: Vec<Line> = st.lines.iter().filter(|l| l.groups.iter().any(|g| depot_names.contains(&g.trim().to_ascii_lowercase())) && (q.is_empty() || l.name.to_lowercase().contains(&q))).cloned().collect();
    if st.lines.is_empty() {
        ui.label(RichText::new(tr("This map has no lines")).italics());
        return;
    }
    if !shown.iter().any(|l| l.name == st.sel_line) {
        st.sel_line = shown.first().map(|l| l.name.clone()).unwrap_or_default();
    }
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(190.0, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(190.0);
            ui.add(egui::TextEdit::singleline(&mut st.line_filter).hint_text(tr("Search by name")).desired_width(180.0));
            egui::ScrollArea::vertical().id_salt("ai-lines").auto_shrink([false, false]).max_height((ui.available_height() - 30.0).max(120.0)).show(ui, |ui| {
                for l in &shown {
                    let on = l.name == st.sel_line;
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(170.0, 42.0), egui::Sense::click());
                    let fill = if on { pal().dark } else if resp.hovered() { pal().hover } else { pal().field };
                    ui.painter().rect_filled(rect, 4.0, fill);
                    let fg = if on { egui::Color32::WHITE } else { pal().text };
                    ui.painter().text(rect.left_top() + egui::vec2(10.0, 8.0), egui::Align2::LEFT_TOP, &l.name, egui::FontId::proportional(14.0), fg);
                    ui.painter().text(rect.left_top() + egui::vec2(10.0, 25.0), egui::Align2::LEFT_TOP, trf("{} tours", &[&l.tours.to_string()]), egui::FontId::proportional(11.0), fg.gamma_multiply(0.8));
                    if st.rules.iter().any(|r| r.line.eq_ignore_ascii_case(&l.name)) {
                        ui.painter().circle_filled(rect.right_top() + egui::vec2(-8.0, 8.0), 3.0, egui::Color32::from_rgb(240, 160, 30));
                    }
                    if resp.clicked() {
                        st.sel_line = l.name.clone();
                    }
                    ui.add_space(2.0);
                }
            });
        });
        let Some(line) = shown.iter().find(|l| l.name == st.sel_line).cloned() else {
            ui.label(RichText::new(tr("No lines use a depot here")).italics());
            return;
        };
        let card_width = ui.available_width();
        ui.vertical(|ui| {
            ui.set_width(card_width);
            egui::Frame::new().fill(ui.visuals().faint_bg_color).stroke(ui.visuals().widgets.noninteractive.bg_stroke).corner_radius(6.0).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                let ri = rule_index(st);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(trf("Line {}", &[&line.name])).strong().size(16.0).color(pal().dark));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(ri.is_some(), egui::Button::new(tr("Reset line"))).clicked() {
                            if let Some(i) = ri {
                                st.rules.remove(i);
                                st.pending = true;
                            }
                        }
                    });
                });
                ui.label(RichText::new(tr("Which buses run on this line. The line keeps its tours and times: only the bus changes.")).size(11.0).weak());
                ui.add_space(6.0);
                if line.map_rule && ri.is_none() {
                    ui.label(RichText::new(tr("The map has rules of its own for this line (a bus type or a fleet number for some of its tours). Choosing one here goes before them.")).size(12.0));
                }
                let mode = match ri.map(|i| st.rules[i].only) {
                    None => 0,
                    Some(true) => 1,
                    Some(false) => 2,
                };
                let mut new_mode = mode;
                ui.horizontal(|ui| {
                    for (m, label) in [(0, "As the map has it"), (1, "Only these types"), (2, "Mostly these types")] {
                        if crate::options_ui::tab_button(ui, tr(label), mode == m) {
                            new_mode = m;
                        }
                    }
                });
                if new_mode != mode {
                    match (new_mode, ri) {
                        (0, Some(i)) => {
                            st.rules.remove(i);
                        }
                        (m, Some(i)) => st.rules[i].only = m == 1,
                        (m, None) => st.rules.push(Rule { line: line.name.clone(), only: m == 1, factor: 1.0, types: vec![] }),
                    }
                    st.pending = true;
                }
                let Some(ri) = rule_index(st) else {
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr("The buses of this line are drawn from its depot as the map says.")).weak());
                    return;
                };
                if !st.rules[ri].only {
                    ui.horizontal(|ui| {
                        ui.label(tr("Tours that get one of these types"));
                        let mut pct = (st.rules[ri].factor * 100.0).round();
                        if ui.add(egui::Slider::new(&mut pct, 10.0..=100.0).step_by(5.0).suffix("%")).changed() {
                            st.rules[ri].factor = pct / 100.0;
                            st.pending = true;
                        }
                    });
                }
                ui.add_space(4.0);
                let mut remove = None;
                egui::ScrollArea::vertical().id_salt("ai-rule-types").auto_shrink([false, false]).max_height((ui.available_height() - 40.0).max(100.0)).show(ui, |ui| {
                    for (i, t) in st.rules[ri].types.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(stem_of(t)).strong());
                                ui.label(RichText::new(folder_of(t)).size(11.0).weak());
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("✕").on_hover_text(tr("Remove")).clicked() {
                                    remove = Some(i);
                                }
                            });
                        });
                        ui.separator();
                    }
                    if st.rules[ri].types.is_empty() {
                        ui.label(RichText::new(tr("Add the types this line runs.")).weak());
                    }
                });
                if let Some(i) = remove {
                    st.rules[ri].types.remove(i);
                    st.pending = true;
                }
                if ui.add(egui::Button::new(RichText::new(format!("+ {}", tr("Add type"))).color(egui::Color32::WHITE)).fill(pal().dark)).clicked() {
                    st.picker = Some(Picker { kind: PickKind::RuleType, search: String::new() });
                }
            });
        });
    });
}
