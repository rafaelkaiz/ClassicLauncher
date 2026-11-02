#![windows_subsystem = "windows"]
//! ClassicLauncher: a fast launcher for openOMSI, laid out like the OMSI 2 Launcher (and OMSI 2's own start screen). It picks the
//! map, vehicle, start place, driver, date and time and starts `openomsi.exe` with them; its options window edits openOMSI's own
//! settings.

mod ailists;
mod ailists_ui;
mod app_icon;
mod chrono;
mod bus_prompt;
mod showroom;
mod buses;
mod free;
mod controllers;
mod controllers_ui;
mod dinput;
mod ffb_calibration;
mod keybindings;
mod keys_ui;
mod troubleshoot_ui;
mod driver_files;
mod drivers;
mod game;
mod gamecfg;
mod i18n;
mod icons;
mod log;
mod logo;
mod olf;
mod omsi_updates;
mod omsi;
mod options_extra;
mod options_ui;
mod radio;
mod paper;
mod pick;
mod prefs;
mod presets;
mod schema;
mod situations;
mod texts;
mod texts_more;
mod texts_hs;
mod texts_es;
mod texts_es2;
mod theme;
mod tutorials;
mod update;

use eframe::egui::{self, Color32, RichText};
use game::{Plan, Start};
use i18n::{tr, trf};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use theme::pal;

#[derive(Clone, Copy, PartialEq)]
enum TimeMode {
    /// The game's own date and time.
    Game,
    Custom,
}

#[derive(Clone, Copy, PartialEq)]
enum StartMode {
    /// The map as the game left it (`laststn.osn`).
    Last,
    Free,
    Situation,
}

struct DriverEdit {
    existing: Option<drivers::Driver>,
    name: String,
    female: bool,
    day: u32,
    month: u32,
    year: u32,
    error: String,
}

impl DriverEdit {
    fn new(existing: Option<&drivers::Driver>) -> Self {
        let (name, female, birth) = existing.map(|d| (d.name.clone(), d.female, d.birth.clone())).unwrap_or_else(|| (String::new(), false, "19900101".into()));
        let num = |r: std::ops::Range<usize>, default: u32| birth.get(r).and_then(|s| s.parse().ok()).unwrap_or(default);
        Self { existing: existing.cloned(), name, female, year: num(0..4, 1990), month: num(4..6, 1), day: num(6..8, 1), error: String::new() }
    }

    fn birth(&self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }
}

/// A game that was started: followed until its window is up, and until it ends.
struct Launching {
    child: std::process::Child,
    started: std::time::Instant,
    map: Option<omsi::MapInfo>,
    /// Its window is up: the launcher has closed or stepped aside.
    playing: bool,
    minimized: bool,
}

struct TimeDialog {
    time: (u32, u32, u32),
    date: (u32, u32, u32),
    date_touched: bool,
}

struct App {
    prefs: prefs::Prefs,
    omsi: Option<PathBuf>,
    game: Option<PathBuf>,
    maps: Vec<omsi::MapInfo>,
    drivers: Vec<drivers::Driver>,
    situations: Vec<situations::Situation>,
    map: usize,
    driver: usize,
    places: Vec<omsi::StartPlace>,
    /// The chosen start place of the map (the first one at first).
    place: usize,
    place_map: Option<usize>,
    mode: StartMode,
    situation: usize,
    only_map: bool,
    time_mode: TimeMode,
    clock: (u32, u32, u32),
    date: (u32, u32, u32),
    date_touched: bool,
    time_dialog: Option<TimeDialog>,
    bus_prompt: Option<bus_prompt::BusPrompt>,
    options_open: bool,
    options: options_ui::State,
    settings_open: bool,
    settings_tab: usize,
    update: update::Shared,
    status: String,
    locate_dir: String,
    locate_error: bool,
    chrono_cache: Option<((String, chrono::Date, String), String)>,
    tut_open: bool,
    tut_sel: usize,
    tut_entries: Vec<tutorials::Entry>,
    staff_open: bool,
    staff_sel: usize,
    staff_page: u8,
    staff_rec: Option<(PathBuf, drivers::Record)>,
    staff_labels: Arc<HashMap<String, String>>,
    driver_edit: Option<DriverEdit>,
    confirm_delete: Option<usize>,
    launching: Option<Launching>,
    born: std::time::Instant,
    entered_full: bool,
    preview: Option<String>,
    /// The area of the launcher's interface (1080 x 780 in the middle of the screen when full screen): the windows open in its middle.
    host_rect: egui::Rect,
    /// Where the personnel file window opened (kept while it is open).
    staff_pos: Option<egui::Pos2>,
    /// Whether the window is full screen now (the setting is what it is at the next start).
    full_now: bool,
    install_seen: bool,
    upd: omsi_updates::Upd,
    /// The version of the game program (`Release 0.3.46` on the main screen), read in the background.
    game_version: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    version_for: Option<PathBuf>,
}

pub fn big_button(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).size(22.0).color(pal().light)).fill(pal().dark).corner_radius(0.0).min_size(egui::vec2(width, 40.0)))
}

/// Paints `icon` at the left end of a big button.
fn paint_icon(ui: &egui::Ui, r: &egui::Response, icon: icons::Icon, color: Color32) {
    let c = egui::pos2(r.rect.left() + 30.0, r.rect.center().y);
    icons::paint(ui.painter(), icon, c, 10.0, color);
}

fn big_icon_button(ui: &mut egui::Ui, text: &str, width: f32, icon: icons::Icon) -> egui::Response {
    let r = big_button(ui, text, width);
    paint_icon(ui, &r, icon, pal().light);
    r
}

/// A block of the start screen: content followed by a thick orange rule, like OMSI's own panels.
fn section(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui);
    });
    let r = ui.max_rect();
    let y = ui.cursor().top();
    ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(r.left(), y), egui::pos2(r.right(), y + 6.0)), 0.0, pal().line);
    ui.add_space(6.0);
}

fn logo_image(theme: theme::Theme, w: f32, h: f32) -> egui::Image<'static> {
    let uri = format!("bytes://omsi-logo-{theme:?}.svg");
    egui::Image::from_bytes(uri, logo::svg(theme, &pal()).into_bytes()).fit_to_exact_size(egui::vec2(w, h))
}

/// How long the opening screen stays: it hides the window being made and the pictures loading.
const SPLASH: std::time::Duration = std::time::Duration::from_secs(3);

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn set_fullscreen(ctx: &egui::Context, on: bool) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(on));
    // (the windows remember where they were on the screen of before: for a moment after the change they are put back to where they open)
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("areas-reset-at"), std::time::Instant::now()));
}

/// `%LOCALAPPDATA%Programs`, where openOMSI usually installs.
fn local_programs() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("Programs"))
}

/// Where a window of about this size opens to sit in the middle (it can be moved from there: an anchored window cannot).
fn centred_pos(centre: egui::Pos2, size: [f32; 2]) -> egui::Pos2 {
    let c = centre;
    egui::pos2((c.x - size[0] / 2.0).max(0.0), (c.y - size[1] / 2.0).max(0.0))
}

fn file_uri(p: &std::path::Path) -> String {
    format!("file:///{}", p.display().to_string().replace('\\', "/"))
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut prefs = prefs::Prefs::load();
        if std::env::var_os("CL_FULL").is_some() {
            prefs.fullscreen = true; // (to look at the full-screen layout; not saved)
        }
        egui_extras::install_image_loaders(&cc.egui_ctx);
        i18n::set_lang(prefs.lang);
        theme::apply(&cc.egui_ctx, prefs.theme);
        if prefs.sync_theme {
            prefs::sync_official_theme(prefs.theme);
        }
        let omsi = omsi::find_omsi_dir(prefs.omsi_dir.clone());
        let game = game::find_game(prefs.game.as_deref());
        let (y, mo, d, h, mi, s) = omsi::local_now();
        let mut app = Self {
            prefs,
            omsi: None,
            game,
            maps: vec![],
            drivers: vec![],
            situations: vec![],
            map: 0,
            driver: 0,
            places: vec![],
            place: 0,
            place_map: None,
            mode: StartMode::Free,
            situation: 0,
            only_map: true,
            time_mode: TimeMode::Game,
            clock: (h, mi, s),
            date: (d, mo, y),
            date_touched: false,
            time_dialog: None,
            bus_prompt: None,
            options_open: false,
            options: options_ui::State::new(),
            settings_open: false,
            settings_tab: 0,
            update: Default::default(),
            status: tr("Ready.").to_string(),
            locate_dir: String::new(),
            locate_error: false,
            chrono_cache: None,
            tut_open: false,
            tut_sel: 0,
            tut_entries: vec![],
            staff_open: false,
            staff_sel: 0,
            staff_page: 0,
            staff_rec: None,
            staff_labels: Arc::new(HashMap::new()),
            driver_edit: None,
            confirm_delete: None,
            launching: None,
            born: std::time::Instant::now(),
            entered_full: false,
            preview: None,
            host_rect: egui::Rect::NOTHING,
            staff_pos: None,
            full_now: false,
            install_seen: false,
            upd: Default::default(),
            game_version: Default::default(),
            version_for: None,
        };
        if let Some(o) = omsi {
            app.load_content(o);
        }
        app.preview = std::env::var("CL_OPEN").ok();
        app
    }

    /// For looking at a window without clicking (CL_OPEN=options|settings|situation|fail, CL_TAB the tab): done once the screen is up.
    fn apply_preview(&mut self, ctx: &egui::Context) {
        let Some(what) = self.preview.take() else { return };
        let app = self;
        match what.as_str() {
            "options" => {
                if std::env::var_os("CL_CHECK").is_some() {
                    omsi_updates::start(&app.upd.rel, || {});
                }
                app.options_open = true;
                app.options.tab = std::env::var("CL_TAB").ok().and_then(|t| t.parse().ok()).unwrap_or(0);
            }
            "situation" => {
                app.mode = StartMode::Situation;
                let want = std::env::var("CL_MAP").unwrap_or_else(|_| "Berlin-Spandau".into());
                if let Some(m) = app.maps.iter().position(|m| m.folder == want) {
                    app.map = m;
                }
            }
            "staff" => {
                app.staff_open = true;
            }
            "busask" => {
                if let Some(o) = app.omsi.clone() {
                    app.bus_prompt = Some(bus_prompt::BusPrompt::new(&o, None, app.game.as_deref()));
                }
            }
            "bus" => {
                if let Some(o) = app.omsi.clone() {
                    let mut p = bus_prompt::BusPrompt::new(&o, None, app.game.as_deref());
                    p.open_picker();
                    app.bus_prompt = Some(p);
                }
            }
            "fail" => {
                app.start(ctx, vec!["--no-such-option".into()]);
            }
            "settings" => {
                app.settings_open = true;
                app.settings_tab = std::env::var("CL_TAB").ok().and_then(|t| t.parse().ok()).unwrap_or(0);
                if std::env::var_os("CL_CHECK").is_some() {
                    omsi_updates::start(&app.upd.rel, || {});
                }
            }
            _ => {}
        }
    }

    /// The middle of the interface, where a window of its own opens.
    fn centre(&self, ctx: &egui::Context) -> egui::Pos2 {
        if self.host_rect.is_positive() { self.host_rect.center() } else { ctx.content_rect().center() }
    }

    /// The game's content folder: where it keeps its own copies of the drivers, the maps' last states and the AI lists you edit.
    fn content_dir(&self) -> Option<PathBuf> {
        omsi::content_dir(self.game.as_deref()?)
    }

    fn load_content(&mut self, omsi: PathBuf) {
        olf::init(&omsi);
        self.maps = omsi::list_maps(&omsi);
        self.drivers = drivers::list_drivers(&omsi, self.content_dir().as_deref());
        self.situations = situations::list(&omsi);
        self.map = self.maps.iter().position(|m| m.folder == self.prefs.map).or_else(|| self.maps.iter().position(|m| m.folder == "Grundorf")).unwrap_or(0);
        self.driver = self.drivers.iter().position(|d| d.file == self.prefs.driver).unwrap_or(0);
        self.omsi = Some(omsi);
        self.place_map = None;
    }

    fn refresh_places(&mut self) {
        if self.place_map == Some(self.map) {
            return;
        }
        self.place_map = Some(self.map);
        self.place = 0;
        self.places = match (&self.omsi, self.maps.get(self.map)) {
            (Some(o), Some(m)) => omsi::start_places(o, m),
            _ => vec![],
        };
    }

    /// The time and date to pass: the chosen ones, else the game's own - or today's when the game's settings ask for the real time or date.
    fn time_strings(&self) -> (Option<String>, Option<String>) {
        match self.time_mode {
            TimeMode::Game => {
                let flag = |k: &str| self.options.cfg.as_ref().is_some_and(|c| c.flag(k, false));
                let (y, mo, d, h, mi, _) = omsi::local_now();
                (flag("use_real_time").then(|| format!("{h:02}:{mi:02}")), flag("use_real_date").then(|| format!("{y:04}-{mo:02}-{d:02}")))
            }
            TimeMode::Custom => (
                Some(format!("{:02}:{:02}", self.clock.0, self.clock.1)),
                self.date_touched.then(|| format!("{:04}-{:02}-{:02}", self.date.2, self.date.1, self.date.0)),
            ),
        }
    }

    fn plan(&self) -> Option<Plan> {
        let omsi = self.omsi.clone()?;
        let map = self.maps.get(self.map)?.clone();
        let (time, date) = self.time_strings();
        let start = match self.mode {
            StartMode::Last => Start::Situation(format!("maps/{}/laststn.osn", map.folder)),
            StartMode::Situation => {
                let s = self.selected_situation()?;
                Start::Situation(format!("Situations/{}", s.file))
            }
            StartMode::Free => {
                let chosen = self.places.get(self.place);
                Start::Free { place: chosen.map(|p| p.index), view: chosen.and_then(|p| p.view) }
            }
        };
        let no_plugins = self.options.cfg.as_ref().is_some_and(|c| c.flag("disable_plugins", false));
        Some(Plan {
            omsi,
            map,
            start,
            driver: self.drivers.get(self.driver).map(|d| d.file.clone()),
            time,
            date,
            traffic: self.prefs.traffic,
            passengers: self.prefs.passengers,
            schedule: self.prefs.schedule,
            no_plugins,
            bus: None,
        })
    }

    fn remember(&mut self) {
        if let Some(m) = self.maps.get(self.map) {
            self.prefs.map = m.folder.clone();
        }
        self.prefs.driver = self.drivers.get(self.driver).map(|d| d.file.clone()).unwrap_or_default();
        let _ = self.prefs.save();
    }

    fn start(&mut self, ctx: &egui::Context, args: Vec<String>) {
        let Some(exe) = self.game.clone() else { return };
        if self.launching.is_some() {
            return;
        }
        log::line(&format!("start: {}", game::command_text(&exe, &args)));
        match game::launch(&exe, &args) {
            Ok(child) => {
                self.remember();
                self.status = tr("Starting openOMSI...").to_string();
                self.launching = Some(Launching { child, started: std::time::Instant::now(), map: self.maps.get(self.map).cloned(), playing: false, minimized: false });
                ctx.request_repaint();
            }
            Err(e) => self.status = trf("Could not start openOMSI: {}", &[&e]),
        }
    }

    /// Follows the game once started: until its window is up the menu shows "Starting...", then the launcher closes or steps aside, and
    /// comes back when the game ends. A game that ends before its window shows (a wrong argument, say) is reported, not hidden.
    fn poll_launch(&mut self, ctx: &egui::Context) {
        let Some(l) = self.launching.as_mut() else { return };
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
        if let Ok(Some(status)) = l.child.try_wait() {
            let (was_playing, minimized) = (l.playing, l.minimized);
            self.launching = None;
            if minimized {
                omsi::restore_own_window();
            }
            self.status = if was_playing && status.success() {
                tr("Ready.").to_string()
            } else {
                let why = game::last_error().unwrap_or_default();
                if why.is_empty() { trf("openOMSI ended ({})", &[&status.to_string()]) } else { trf("openOMSI ended: {}", &[&why]) }
            };
            return;
        }
        // (after a minute and a half the game is taken to be up even if its window was not seen: the launcher must not stay in the way)
        if !l.playing && (omsi::has_window(l.child.id()) || l.started.elapsed() > std::time::Duration::from_secs(90)) {
            l.playing = true;
            if self.prefs.close_on_start {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                omsi::minimize_own_window();
                l.minimized = true;
            }
        }
    }

    /// The language for OMSI's own texts (tutorials, staff file, holidays): the launcher's, so the window reads in one language;
    /// English where OMSI has none (`olf::omsi_code`).
    fn game_lang(&self) -> i18n::Lang {
        self.prefs.lang
    }

    fn locate(&mut self, ui: &mut egui::Ui) {
        let need_omsi = self.omsi.is_none();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(pal().bg)).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.2).max(10.0));
                ui.add(logo_image(self.prefs.theme, 420.0, 420.0 / logo::ASPECT));
                ui.add_space(24.0);
                if need_omsi {
                    ui.label(RichText::new(tr("OMSI 2 not found")).size(26.0).strong());
                    ui.label(tr("Tell me where OMSI 2 is installed (the folder with Vehicles and maps)."));
                    ui.add_space(10.0);
                    ui.add(egui::TextEdit::singleline(&mut self.locate_dir).desired_width(460.0));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(((ui.available_width() - 190.0) / 2.0).max(0.0));
                        if ui.button(tr("Browse...")).clicked() {
                            if let Some(p) = omsi::browse_folder(tr("Choose the OMSI 2 folder"), self.prefs.omsi_dir.as_deref().or(self.omsi.as_deref())) {
                                self.locate_dir = p.display().to_string();
                            }
                        }
                        if ui.button(tr("Accept")).clicked() {
                            let dir = PathBuf::from(self.locate_dir.trim().trim_matches('"'));
                            if omsi::is_omsi_root(&dir) {
                                self.prefs.omsi_dir = Some(dir.clone());
                                let _ = self.prefs.save();
                                self.load_content(dir);
                                self.locate_error = false;
                            } else {
                                self.locate_error = true;
                            }
                        }
                    });
                    if self.locate_error {
                        ui.colored_label(Color32::from_rgb(190, 0, 0), tr("That folder has no maps and Vehicles."));
                    }
                } else {
                    ui.label(RichText::new(tr("openOMSI not found")).size(26.0).strong());
                    ui.label(tr("Tell me where openomsi.exe is."));
                    ui.add_space(10.0);
                    if ui.button(tr("Browse...")).clicked() {
                        if let Some(p) = omsi::browse_exe(tr("Choose openomsi.exe"), local_programs().as_deref()) {
                            self.game = game::find_game(Some(&p));
                            if self.game.is_some() {
                                self.prefs.game = self.game.clone();
                                let _ = self.prefs.save();
                            }
                        }
                    }
                }
            });
        });
    }

    fn picker_row(ui: &mut egui::Ui, label: &str, id: &str, items: &[String], sel: &mut usize) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(label);
            changed = pick::pick(ui, id, (ui.available_width() - 8.0).max(120.0), items, sel);
        });
        changed
    }

    fn driver_section(&mut self, ui: &mut egui::Ui) {
        let photo = self.drivers.get(self.driver).and_then(|d| {
            let p = d.path.with_extension("bmp");
            let p = if p.is_file() { p } else { self.omsi.as_ref()?.join("Drivers").join("nopic.bmp") };
            p.is_file().then(|| file_uri(&p))
        });
        ui.horizontal_top(|ui| {
            if let Some(uri) = photo {
                ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(120.0, 160.0)));
            }
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("Current driver:"));
                    let names: Vec<String> = self.drivers.iter().map(|d| d.name.clone()).collect();
                    pick::pick(ui, "driver", 150.0, &names, &mut self.driver);
                });
                if ui.add_enabled(!self.drivers.is_empty(), egui::Button::new(tr("Open personnel file...")).min_size(egui::vec2(300.0, 28.0))).clicked() {
                    self.staff_open = true;
                    self.staff_sel = self.driver;
                    self.staff_labels = Arc::new(self.omsi.as_deref().map(|d| olf::load(d, self.game_lang())).unwrap_or_default());
                }
                if ui.add_enabled(!driver_files::game_running(), egui::Button::new(tr("Create new driver")).min_size(egui::vec2(300.0, 28.0))).clicked() {
                    self.driver_edit = Some(DriverEdit::new(None));
                }
            });
        });
    }

    /// The situation the list has selected: with "only the selected map's situations" and none for this map, there is none (the index
    /// would still point at one of another map).
    fn selected_situation(&self) -> Option<&situations::Situation> {
        let s = self.situations.get(self.situation)?;
        let map = self.maps.get(self.map)?;
        (!self.only_map || s.map_folder.eq_ignore_ascii_case(&map.folder)).then_some(s)
    }

    /// The file of the map's last state, if the game (or OMSI 2) left one: the game's own copy first.
    fn last_state_path(&self) -> Option<PathBuf> {
        let (omsi, map) = (self.omsi.as_ref()?, self.maps.get(self.map)?);
        let rel = std::path::Path::new("maps").join(&map.folder).join("laststn.osn");
        let mut candidates = vec![];
        candidates.extend(self.content_dir().map(|c| c.join(&rel)));
        if let Some(d) = omsi::data_dir() {
            candidates.push(d.join("content").join(&rel));
        }
        candidates.push(omsi.join(&rel));
        candidates.into_iter().find(|p| p.is_file())
    }

    /// The date and time of the situation that will be loaded (the chosen saved one, or the map's last state).
    fn situation_datetime(&self) -> Option<(chrono::Date, (u32, u32, u32))> {
        match self.mode {
            StartMode::Situation => {
                let s = self.selected_situation()?;
                Some((s.date?, s.time?))
            }
            StartMode::Last => chrono::situation_datetime(&self.last_state_path()?),
            StartMode::Free => None,
        }
    }

    /// The date and time the start screen shows: the situation's, or what a free start will use (None: the game's own).
    fn shown_datetime(&self) -> (Option<chrono::Date>, Option<(u32, u32, u32)>) {
        if self.mode != StartMode::Free {
            return match self.situation_datetime() {
                Some((d, t)) => (Some(d), Some(t)),
                None => (None, None),
            };
        }
        let (time, date) = self.time_strings();
        let t = time.and_then(|t| Some((t.get(0..2)?.parse().ok()?, t.get(3..5)?.parse().ok()?, 0)));
        let d = date.and_then(|d| Some((d.get(0..4)?.parse::<i64>().ok()?, d.get(5..7)?.parse::<u32>().ok()?, d.get(8..10)?.parse::<u32>().ok()?)));
        (d, t)
    }

    fn time_section(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(tr("Date and time")).strong());
        let free = self.mode == StartMode::Free;
        // (a saved situation brings its own date and time: they are shown, and cannot be changed here)
        ui.add_enabled_ui(free, |ui| {
            if ui.radio(self.time_mode == TimeMode::Custom && free, tr("Change date and time...")).clicked() {
                self.time_dialog = Some(TimeDialog { time: self.clock, date: self.date, date_touched: self.date_touched });
            }
        });
        let label = if free { tr("Use the game's date and time") } else { tr("Use the situation's date and time") };
        if ui.radio(self.time_mode == TimeMode::Game || !free, label).clicked() && free {
            self.time_mode = TimeMode::Game;
        }
        let (date, time) = self.shown_datetime();
        let date = date.map(|(y, m, d)| format!("{d:02}/{m:02}/{y}")).unwrap_or_else(|| "-".into());
        let time = time.map(|(h, m, s)| format!("{h:02}:{m:02}:{s:02}")).unwrap_or_else(|| "-".into());
        ui.horizontal(|ui| {
            ui.label(format!("{} {}", tr("Date:"), date));
            ui.add_space(16.0);
            ui.label(format!("{} {}", tr("Time:"), time));
        });
    }

    /// The map's timetable and tariff periods that cover the selected date, from its `Chrono` folder.
    fn chrono_box(&mut self, ui: &mut egui::Ui) {
        ui.label(tr("Information about the selected time:"));
        let mut text = String::new();
        if let (Some(omsi), Some(map)) = (self.omsi.clone(), self.maps.get(self.map)) {
            let date = self.shown_datetime().0;
            if let Some(date) = date {
                let lang = olf::omsi_code(self.game_lang());
                let key = (map.folder.clone(), date, lang.to_string());
                if self.chrono_cache.as_ref().map(|c| &c.0) != Some(&key) {
                    let t = chrono::info(&omsi.join("maps").join(&map.folder), date, lang);
                    self.chrono_cache = Some((key, t));
                }
                text = self.chrono_cache.as_ref().map(|c| c.1.clone()).unwrap_or_default();
            }
        }
        egui::Frame::new().fill(pal().field).stroke(egui::Stroke::new(1.0, pal().dark)).inner_margin(6).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("chrono").min_scrolled_height(90.0).max_height(90.0).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(egui::Label::new(RichText::new(text).strong().size(12.0)).wrap());
            });
        });
    }

    fn map_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(tr("Current map:"));
            let names: Vec<String> = self.maps.iter().map(|m| m.name.clone()).collect();
            if pick::pick(ui, "map", ui.available_width() - 8.0, &names, &mut self.map) {
                self.place_map = None;
            }
        });
        self.refresh_places();
        ui.add_space(6.0);
        let has_last = self.has_last_state();
        ui.add_enabled_ui(has_last, |ui| {
            ui.radio_value(&mut self.mode, StartMode::Last, tr("Load last state of the map"));
        });
        if !has_last && self.mode == StartMode::Last {
            self.mode = StartMode::Free;
        }
        ui.radio_value(&mut self.mode, StartMode::Free, tr("Load map without vehicles"));
        if self.mode == StartMode::Free {
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(tr("Appear at:"));
                let places: Vec<String> = self.places.iter().map(|p| p.name.clone()).collect();
                if places.is_empty() {
                    ui.label(tr("The map has no start places"));
                } else {
                    pick::pick(ui, "place", ui.available_width() - 8.0, &places, &mut self.place);
                }
            });
        }
        ui.radio_value(&mut self.mode, StartMode::Situation, tr("Load saved situation"));
        if self.mode == StartMode::Situation {
            ui.add_space(6.0);
            ui.checkbox(&mut self.only_map, tr("Only the selected map's situations"));
            let folder = self.maps.get(self.map).map(|m| m.folder.to_lowercase()).unwrap_or_default();
            let shown: Vec<usize> = (0..self.situations.len()).filter(|&i| !self.only_map || self.situations[i].map_folder.to_lowercase() == folder).collect();
            if !shown.contains(&self.situation) {
                self.situation = shown.first().copied().unwrap_or(0);
            }
            egui::Frame::new().fill(pal().field).stroke(egui::Stroke::new(1.0, pal().dark)).inner_margin(4).show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("situations").max_height(240.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if shown.is_empty() {
                        ui.label(RichText::new(tr("No saved situations for this map.")).italics());
                    }
                    for &i in &shown {
                        let s = &self.situations[i];
                        if ui.selectable_label(self.situation == i, &s.name).clicked() {
                            self.situation = i;
                            if let Some(m) = self.maps.iter().position(|m| m.folder.eq_ignore_ascii_case(&s.map_folder)) {
                                if m != self.map {
                                    self.map = m;
                                    self.place_map = None;
                                }
                            }
                        }
                    }
                });
            });
        }
        if self.mode != StartMode::Situation {
            if let Some(p) = self.maps.get(self.map).and_then(|m| m.picture.as_ref()) {
            ui.add_space(8.0);
            ui.add(egui::Image::new(file_uri(p)).max_width(ui.available_width()).max_height(260.0).maintain_aspect_ratio(true));
            }
        }
    }

    /// Has the game left a last state on this map? (its own autosave in the content folder, or the file OMSI 2 left)
    fn has_last_state(&self) -> bool {
        self.last_state_path().is_some()
    }

    fn bottom_bar(&mut self, host: &mut egui::Ui, ctx: &egui::Context) {
        let can_go = self.plan().is_some();
        egui::Panel::bottom("bar").frame(egui::Frame::new().fill(pal().bg)).show(host, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(&self.status);
            });
            if self.prefs.show_command {
                if let (Some(exe), Some(plan)) = (&self.game, self.plan()) {
                    let mut text = game::command_text(exe, &plan.args());
                    ui.add(egui::TextEdit::multiline(&mut text).desired_width(f32::INFINITY).desired_rows(2));
                }
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 3.0 * ui.spacing().item_spacing.x) / 4.0;
                let go = egui::Button::new(RichText::new(tr("Start!")).size(22.0).color(Color32::WHITE)).fill(pal().dark).corner_radius(0.0).min_size(egui::vec2(w, 40.0));
                let go_resp = ui.add_enabled(can_go, go);
                paint_icon(ui, &go_resp, icons::Icon::Play, Color32::WHITE);
                if go_resp.clicked() {
                    match (&self.omsi, self.mode) {
                        // a free start asks whether to add a bus; a saved situation has its own
                        (Some(o), StartMode::Free) => {
                            let mut p = bus_prompt::BusPrompt::new(o, self.game.as_deref().and_then(omsi::content_dir).as_deref(), self.game.as_deref());
                            if let Some(m) = self.maps.get(self.map) {
                                p.map = format!("maps/{}/global.cfg", m.folder);
                            }
                            self.bus_prompt = Some(p);
                        }
                        _ => {
                            if let Some(plan) = self.plan() {
                                self.start(ctx, plan.args());
                            }
                        }
                    }
                }
                if big_icon_button(ui, tr("Tutorials"), w, icons::Icon::Book).clicked() {
                    self.tut_open = !self.tut_open;
                    if self.tut_open {
                        if let Some(o) = &self.omsi {
                            self.tut_entries = tutorials::load(o, olf::omsi_code(self.game_lang()));
                        }
                        self.tut_sel = 0;
                    }
                }
                if big_icon_button(ui, tr("Options"), w, icons::Icon::Sliders).clicked() {
                    self.options = options_ui::State::new();
                    self.options_open = !self.options_open;
                }
                if big_icon_button(ui, tr("Close"), w, icons::Icon::Close).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    fn main_screen(&mut self, host: &mut egui::Ui, ctx: &egui::Context) {
        self.bottom_bar(host, ctx);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(pal().bg)).show(host, |ui| {
            let full = ui.max_rect();
            ui.columns(2, |cols| {
                // left: the program, the driver, date and time
                let ui = &mut cols[0];
                section(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(logo_image(self.prefs.theme, 290.0, 290.0 / logo::ASPECT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.vertical(|ui| {
                                // (the game program's version, read in the background; the launcher's own is in its settings)
                                let release = self.game_version.lock().ok().and_then(|v| v.clone());
                                let text = match release {
                                    Some(v) => format!("ClassicLauncher\nRelease {v}"),
                                    None => "ClassicLauncher".to_string(),
                                };
                                ui.label(RichText::new(text).italics());
                                if ui.small_button(tr("Launcher settings")).clicked() {
                                    self.settings_open = !self.settings_open;
                                }
                            });
                        });
                    });
                });
                section(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.hyperlink_to(RichText::new(tr("Help? Info? News? openomsi.org")).strong().underline(), "https://openomsi.org/");
                    });
                });
                section(ui, |ui| self.driver_section(ui));
                section(ui, |ui| self.time_section(ui));
                section(ui, |ui| self.chrono_box(ui));

                // right: the map
                let ui = &mut cols[1];
                section(ui, |ui| self.map_section(ui));
                let (heading, body) = if self.mode == StartMode::Situation {
                    let text = match self.selected_situation() {
                        Some(s) => s.description.clone(),
                        None => tr("No situation selected!").to_string(),
                    };
                    (tr("Information about the selected situation:"), Some(text))
                } else {
                    (tr("Information about the selected map:"), self.maps.get(self.map).map(|m| m.description.clone()))
                };
                if let Some(body) = body {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.label(heading);
                    });
                    egui::Frame::new().fill(pal().field).stroke(egui::Stroke::new(1.0, pal().dark)).inner_margin(6).outer_margin(egui::Margin::symmetric(16, 4)).show(ui, |ui| {
                        egui::ScrollArea::vertical().id_salt("desc").max_height((ui.available_height() - 24.0).max(60.0)).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.add(egui::Label::new(RichText::new(body).strong()).wrap());
                        });
                    });
                }
            });
            // the vertical rule between the columns, as in OMSI's own screen
            let x = full.center().x;
            ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(x - 3.0, full.top()), egui::pos2(x + 3.0, full.bottom())), 0.0, pal().line);
        });
    }

    /// The question Start asks on a free start: add a bus or not.
    fn bus_prompt_window(&mut self, ctx: &egui::Context) {
        let Some(mut p) = self.bus_prompt.take() else { return };
        let outcome = bus_prompt::show(ctx, &mut p);
        match outcome {
            bus_prompt::Outcome::Open => self.bus_prompt = Some(p),
            bus_prompt::Outcome::Cancel => {}
            bus_prompt::Outcome::Go(bus) => {
                if let Some(mut plan) = self.plan() {
                    plan.bus = bus;
                    self.start(ctx, plan.args());
                }
            }
        }
    }

    fn time_dialog_window(&mut self, ctx: &egui::Context) {
        let Some(mut dlg) = self.time_dialog.take() else { return };
        let (mut accept, mut cancel) = (false, false);
        let mut open = true;
        let two = |v: f64, _: std::ops::RangeInclusive<usize>| format!("{:02}", v as u32);
        open = crate::free::window(ctx, "time", tr("Select time and date..."), egui::vec2(430.0, 330.0), |ui| {
            ui.set_width(430.0);
            ui.horizontal(|ui| {
                ui.label(tr("Time:"));
                ui.add(egui::DragValue::new(&mut dlg.time.0).range(0..=23).custom_formatter(two));
                ui.label(":");
                ui.add(egui::DragValue::new(&mut dlg.time.1).range(0..=59).custom_formatter(two));
                ui.label(":");
                ui.add(egui::DragValue::new(&mut dlg.time.2).range(0..=59).custom_formatter(two));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr("Current time")).clicked() {
                        let (_, _, _, h, m, s) = omsi::local_now();
                        dlg.time = (h, m, s);
                    }
                });
            });
            ui.horizontal(|ui| {
                for (name, hour) in [("Day", 12), ("Afternoon", 17), ("Night", 0), ("Morning", 7)] {
                    if ui.button(tr(name)).clicked() {
                        dlg.time = (hour, 0, 0);
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(tr("Date:"));
                let before = dlg.date;
                ui.add(egui::DragValue::new(&mut dlg.date.0).range(1..=31));
                ui.label("/");
                ui.add(egui::DragValue::new(&mut dlg.date.1).range(1..=12));
                ui.label("/");
                ui.add(egui::DragValue::new(&mut dlg.date.2).range(1900..=2100));
                if dlg.date != before {
                    dlg.date_touched = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr("Current year")).clicked() {
                        dlg.date.2 = omsi::local_now().0;
                        dlg.date_touched = true;
                    }
                    if ui.button(tr("Current date")).clicked() {
                        let (y, m, d, ..) = omsi::local_now();
                        dlg.date = (d, m, y);
                        dlg.date_touched = true;
                    }
                });
            });
            dlg.date.0 = dlg.date.0.min(days_in_month(dlg.date.2, dlg.date.1));
            let day = (dlg.date.2 as i64, dlg.date.1, dlg.date.0);
            let weekdays = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
            ui.horizontal(|ui| {
                ui.label(tr("Weekday:"));
                ui.label(RichText::new(tr(weekdays[chrono::weekday(day)])).strong());
            });
            let holidays = match (&self.omsi, self.maps.get(self.map)) {
                (Some(o), Some(m)) => chrono::holidays(&o.join("maps").join(&m.folder), olf::omsi_code(self.game_lang()), day),
                _ => vec![],
            };
            ui.horizontal(|ui| {
                ui.label(tr("Holidays:"));
                ui.label(RichText::new(holidays.join(", ")).strong());
            });
            if !dlg.date_touched {
                ui.label(RichText::new(tr("If you do not change the date, the game uses its own.")).italics().size(12.0));
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if big_button(ui, tr("Accept"), 120.0).clicked() {
                    accept = true;
                }
                if big_button(ui, tr("Cancel"), 120.0).clicked() {
                    cancel = true;
                }
            });
        });
        if accept {
            self.clock = dlg.time;
            self.date = dlg.date;
            self.date_touched = dlg.date_touched;
            self.time_mode = TimeMode::Custom;
        } else if !cancel && open {
            self.time_dialog = Some(dlg);
        }
    }

    /// A window like OMSI's own tutorial menu: four tutorials, a picture and a description.
    fn tutorials_window(&mut self, ctx: &egui::Context) {
        if !self.tut_open {
            return;
        }
        let mut open = self.tut_open;
        let mut start = None;
        open = crate::free::window(ctx, "tutorials", tr("Tutorials"), egui::vec2(760.0, 480.0), |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    for (i, name) in [tr("Tutorial I"), tr("Tutorial II"), tr("Tutorial III"), tr("Tutorial IV")].iter().enumerate() {
                        let sel = self.tut_sel == i + 1;
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(260.0, 64.0), egui::Sense::click());
                        let fg = if sel { Color32::WHITE } else { pal().light };
                        ui.painter().rect_filled(rect, 0.0, if sel { pal().line } else if resp.hovered() { pal().dark.gamma_multiply(0.85) } else { pal().dark });
                        let tile = egui::Rect::from_min_size(rect.left_top() + egui::vec2(8.0, 8.0), egui::vec2(48.0, 48.0));
                        ui.painter().rect_filled(tile, 0.0, Color32::from_black_alpha(70));
                        let icon = [icons::Icon::Clock, icons::Icon::Bus, icons::Icon::People, icons::Icon::Fleet][i];
                        icons::paint(ui.painter(), icon, tile.center(), 16.0, fg);
                        let text_area = egui::Rect::from_min_max(egui::pos2(tile.right() + 8.0, rect.top()), rect.right_bottom());
                        ui.painter().text(text_area.center(), egui::Align2::CENTER_CENTER, *name, egui::FontId::proportional(20.0), fg);
                        if resp.clicked() {
                            self.tut_sel = i + 1;
                        }
                        ui.add_space(4.0);
                    }
                });
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.set_width(380.0);
                    if let Some(e) = self.tut_entries.get(self.tut_sel) {
                        if let Some(p) = &e.picture {
                            ui.add(egui::Image::new(file_uri(p)).fit_to_exact_size(egui::vec2(380.0, 209.0)));
                        }
                        ui.label(RichText::new(&e.title).strong());
                        egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.add(egui::Label::new(&e.text).wrap());
                        });
                    }
                });
            });
            ui.add_space(6.0);
            let label = if self.tut_sel == 0 { tr("Choose a tutorial").to_string() } else { trf("Tutorial {}", &[&self.tut_sel.to_string()]) };
            if ui.add_enabled(self.tut_sel > 0, egui::Button::new(RichText::new(label).size(20.0).color(Color32::WHITE)).fill(pal().dark).corner_radius(0.0).min_size(egui::vec2(240.0, 40.0))).clicked() {
                start = Some(self.tut_sel as u8);
            }
        });
        self.tut_open = open;
        if let Some(n) = start {
            self.tut_open = false;
            if let Some(o) = self.omsi.clone() {
                self.start(ctx, game::tutorial_args(&o, n));
            }
        }
    }

    /// OMSI's staff file of the selected driver, on its sheet of paper (read-only).
    fn staff_window(&mut self, ctx: &egui::Context) {
        if !self.staff_open || self.drivers.is_empty() {
            self.staff_open = false;
            self.staff_pos = None;
            return;
        }
        self.staff_sel = self.staff_sel.min(self.drivers.len() - 1);
        let path = self.drivers[self.staff_sel].path.clone();
        if self.staff_rec.as_ref().map(|(p, _)| p) != Some(&path) {
            self.staff_rec = drivers::read_record(&path).map(|r| (path.clone(), r));
        }
        let labels = self.staff_labels.clone();
        let l = |k: &str| labels.get(k).cloned().unwrap_or_else(|| k.to_string());
        let mut close_staff = false;
        let names: Vec<String> = self.drivers.iter().map(|d| d.name.clone()).collect();
        let title = if labels.contains_key("DG_Driver") { l("DG_Driver") } else { tr("Personnel file").to_string() };
        // The file is a window of its own (free of the launcher's: it can be taken out of it, over the desktop, and is never clipped by
        // it). The sheet is sized for the monitor; the window opens in the middle of the launcher once.
        let monitor_h = ctx.input(|i| i.viewport().monitor_size).map_or(1080.0, |m| m.y);
        let k = ((monitor_h - 140.0 - 140.0) / paper::HEIGHT).clamp(0.55, 1.0);
        let size = egui::vec2(paper::WIDTH * k + 30.0, paper::HEIGHT * k + 140.0);
        if self.staff_pos.is_none() {
            let parent = ctx.input(|i| i.viewport().outer_rect);
            self.staff_pos = Some(parent.map_or(egui::pos2(120.0, 60.0), |r| egui::pos2((r.center().x - size.x / 2.0).max(0.0), (r.center().y - size.y / 2.0).max(0.0))));
        }
        let builder = egui::ViewportBuilder::default().with_title(title).with_inner_size(size).with_resizable(false).with_minimize_button(false).with_maximize_button(false).with_position(self.staff_pos.unwrap_or_default());
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("staff-file"), builder, |ui, _class| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                close_staff = true;
            }
            let fill = ui.visuals().window_fill;
            egui::CentralPanel::default().frame(egui::Frame::new().fill(fill).inner_margin(egui::Margin::same(8))).show_inside(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(if labels.contains_key("DG_Driver_Label1") { l("DG_Driver_Label1") } else { tr("Driver:").to_string() });
                pick::pick(ui, "staff-driver", 220.0, &names, &mut self.staff_sel);
                ui.end_row();
                let current = self.staff_sel == self.driver;
                let set = if current { l("DG_Driver_Button1_NotEnabled") } else { l("DG_Driver_Button1_Enabled") };
                if ui.add_enabled(!current, egui::Button::new(set)).clicked() {
                    self.driver = self.staff_sel;
                }
                let idle = !driver_files::game_running();
                if ui.add_enabled(idle, egui::Button::new(if labels.contains_key("DG_Driver_Button2") { l("DG_Driver_Button2") } else { tr("Create new driver").to_string() })).clicked() {
                    self.driver_edit = Some(DriverEdit::new(None));
                }
                if ui.add_enabled(idle, egui::Button::new(if labels.contains_key("DG_Driver_Button3") { l("DG_Driver_Button3") } else { tr("Edit driver").to_string() })).clicked() {
                    self.driver_edit = Some(DriverEdit::new(self.drivers.get(self.staff_sel)));
                }
                if ui.add_enabled(idle, egui::Button::new(if labels.contains_key("DG_Driver_Button4") { l("DG_Driver_Button4") } else { tr("Delete").to_string() })).clicked() {
                    self.confirm_delete = Some(self.staff_sel);
                }
            });
            ui.separator();
            let d = self.drivers[self.staff_sel].clone();
            let date = |s: &str| if s.len() == 8 { format!("{}/{}/{}", &s[6..8], &s[4..6], &s[0..4]) } else { s.to_string() };
            let photo = d.path.with_extension("bmp");
            let photo = if photo.is_file() { Some(photo) } else { self.omsi.as_ref().map(|o| o.join("Drivers").join("nopic.bmp")) };
            let (page, _) = ui.vertical_centered(|ui| ui.allocate_exact_size(egui::vec2(paper::WIDTH * k, paper::HEIGHT * k), egui::Sense::hover())).inner;
            let p = ui.painter_at(page);
            paper::paint(&p, page, k);
            let at = |x: f32, y: f32| page.left_top() + egui::vec2(x * k, y * k);
            let ink = Color32::from_rgb(18, 18, 18);
            let mono = egui::FontId::monospace(13.0 * k);
            // the paper's lettering is heavy: each text is drawn twice, a hair apart
            let bold = |pos: egui::Pos2, anchor: egui::Align2, text: &str, font: &egui::FontId, color: Color32| {
                p.text(pos, anchor, text, font.clone(), color);
                p.text(pos + egui::vec2(0.8 * k, 0.0), anchor, text, font.clone(), color);
            };
            let title_text = l("DG_Driver");
            let title_size = (30.0 * k).min(270.0 * k / (title_text.chars().count() as f32 * 0.5).max(1.0));
            bold(at(185.0, 30.0), egui::Align2::CENTER_CENTER, &title_text, &egui::FontId::proportional(title_size), ink);
            bold(at(350.0, 21.0), egui::Align2::LEFT_CENTER, &trf("Page {}", &[&(self.staff_page + 1).to_string()]), &egui::FontId::proportional(14.0 * k), ink);
            if let Some((_, r)) = &self.staff_rec {
                if self.staff_page == 0 {
                    if let Some(img) = photo.filter(|f| f.is_file()) {
                        let rect = egui::Rect::from_min_size(at(59.0, 73.0), egui::vec2(120.0 * k, 160.0 * k));
                        egui::Image::new(file_uri(&img)).fit_to_exact_size(rect.size()).paint_at(ui, rect);
                        p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 55, 45)), egui::StrokeKind::Outside);
                    }
                    bold(at(270.0, 256.0), egui::Align2::LEFT_CENTER, &if d.female { l("DG_Driver_Mrs") } else { l("DG_Driver_Mr") }, &mono, ink);
                    let driving = r.driving().map(|v| format!("{v:.1} % ({})", l("DG_Driver_Rating_Driving_0"))).unwrap_or_else(|| "-".into());
                    let comfort = r.comfort().map(|(v, got, max)| format!("{v:.1} % ({got}/{max})")).unwrap_or_else(|| "-".into());
                    // (height on the sheet, OMSI's label, value, shown in red)
                    let rows: [(f32, &str, String, bool); 16] = [
                        (276.0, "DG_Driver_Name", d.name.clone(), false),
                        (296.0, "DG_Driver_Birthday", date(&d.birth), false),
                        (316.0, "DG_Driver_DateOfHire", date(&r.hired), false),
                        (356.0, "DG_Driver_Range", format!("{:.1} km", r.hektometers as f64 / 10.0), false),
                        (376.0, "DG_Driver_Busstops", r.stops.to_string(), false),
                        (396.0, "DG_Driver_Busstops_Early", r.early.to_string(), true),
                        (416.0, "DG_Driver_Busstops_Late", r.late.to_string(), true),
                        (456.0, "DG_Driver_Tickets", r.tickets_sold.to_string(), false),
                        (476.0, "DG_Driver_Cash", format!("{:.2}", r.cash), false),
                        (516.0, "DG_Driver_Crashs", r.crashes[0].to_string(), true),
                        (536.0, "DG_Driver_HitAndRun", r.crashes[1].to_string(), true),
                        (556.0, "DG_Driver_HitAndRun_Heavy", r.crashes[2].to_string(), true),
                        (576.0, "DG_Driver_PedCrashs", r.crashes[3].to_string(), true),
                        (616.0, "DG_Driver_Rating_Driving", driving, false),
                        (636.0, "DG_Driver_Rating_Ticket", "-".to_string(), false), // OMSI's ticket-selling rating formula is not known
                        (656.0, "DG_Driver_Rating_Comfort", comfort, false),
                    ];
                    for (y, key, value, bad) in rows {
                        // long labels (some languages) are shrunk so they stop before the values
                        let label = l(key);
                        let fit = (170.0 * k / (label.chars().count() as f32 * 6.4 * k)).min(1.0);
                        p.text(at(74.0, y), egui::Align2::LEFT_CENTER, &label, egui::FontId::proportional(13.5 * k * fit), ink);
                        bold(at(269.0, y), egui::Align2::LEFT_CENTER, &value, &mono, if bad { Color32::from_rgb(205, 0, 0) } else { ink });
                        p.line_segment([at(249.0, y + 9.0), at(438.0, y + 9.0)], egui::Stroke::new(1.0, Color32::from_rgb(70, 64, 52)));
                    }
                } else {
                    bold(at(84.0, 71.0), egui::Align2::LEFT_CENTER, &l("DG_Driver_Buses"), &egui::FontId::proportional(14.5 * k), ink);
                    // name, dots up to a fixed column, then the distance driven with that bus
                    for (i, (bus, hm)) in r.buses.iter().take(22).enumerate() {
                        let mut name: String = bus.chars().take(32).collect();
                        if bus.chars().count() > 32 {
                            name.push_str("..");
                        }
                        let line = format!("{name:.<34} {:.1} km", *hm as f64 / 10.0);
                        bold(at(78.0, 127.0 + i as f32 * 25.0), egui::Align2::LEFT_CENTER, &line, &mono, ink);
                    }
                }
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for n in 0..2u8 {
                    if ui.selectable_label(self.staff_page == n, trf("Page {}", &[&(n + 1).to_string()])).clicked() {
                        self.staff_page = n;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr("Close")).clicked() {
                        close_staff = true;
                    }
                });
            });
            });
        });
        if close_staff {
            self.staff_pos = None;
            self.staff_open = false;
        }
    }

    fn reload_drivers(&mut self, select: Option<&str>) {
        let Some(omsi) = self.omsi.clone() else { return };
        self.drivers = drivers::list_drivers(&omsi, self.content_dir().as_deref());
        if let Some(name) = select {
            if let Some(i) = self.drivers.iter().position(|d| d.name == name) {
                self.driver = i;
            }
        }
        self.driver = self.driver.min(self.drivers.len().saturating_sub(1));
        self.staff_sel = self.staff_sel.min(self.drivers.len().saturating_sub(1));
        self.staff_rec = None;
    }

    /// The window to create a driver or change one's name, sex and birth date.
    fn driver_window(&mut self, ctx: &egui::Context) {
        let Some(mut edit) = self.driver_edit.take() else { return };
        let mut open = true;
        let mut done = false;
        let mut save = false;
        let title = if edit.existing.is_some() { tr("Edit driver") } else { tr("Create new driver") };
        open = crate::free::window(ctx, "driver", &title, egui::vec2(520.0, 360.0), |ui| {
            egui::Grid::new("driver_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label(tr("Name:"));
                ui.text_edit_singleline(&mut edit.name);
                ui.end_row();
                ui.label(tr("Sex:"));
                ui.horizontal(|ui| {
                    ui.radio_value(&mut edit.female, false, tr("Man"));
                    ui.radio_value(&mut edit.female, true, tr("Woman"));
                });
                ui.end_row();
                ui.label(tr("Birth date:"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut edit.day).range(1..=31));
                    ui.add(egui::DragValue::new(&mut edit.month).range(1..=12));
                    ui.add(egui::DragValue::new(&mut edit.year).range(1900..=2100));
                    ui.label(tr("(day / month / year)"));
                });
                ui.end_row();
            });
            if !edit.error.is_empty() {
                ui.colored_label(Color32::from_rgb(170, 0, 0), &edit.error);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if big_button(ui, tr("Save"), 120.0).clicked() {
                    save = true;
                }
                if big_button(ui, tr("Cancel"), 120.0).clicked() {
                    done = true;
                }
            });
        });
        if save {
            let Some(dir) = self.omsi.clone() else { return };
            let res = if driver_files::game_running() {
                Err(tr("Close the game to edit drivers.").to_string())
            } else {
                match &edit.existing {
                    Some(old) => driver_files::update(old, &edit.name, edit.female, &edit.birth()),
                    None => driver_files::create(&dir, &edit.name, edit.female, &edit.birth()),
                }
            };
            match res {
                Ok(()) => {
                    self.reload_drivers(Some(edit.name.trim()));
                    self.status = tr("Driver saved (backup in %APPDATA%\\ClassicLauncher\\backups).").to_string();
                    done = true;
                }
                Err(e) => edit.error = e,
            }
        }
        if open && !done {
            self.driver_edit = Some(edit);
        }
    }

    /// Asks before deleting a driver; the driver's file is copied to the backup folder first.
    fn confirm_window(&mut self, ctx: &egui::Context) {
        let Some(i) = self.confirm_delete else { return };
        let Some(d) = self.drivers.get(i).cloned() else {
            self.confirm_delete = None;
            return;
        };
        let (mut done, mut delete) = (false, false);
        if !crate::free::window(ctx, "confirm", tr("Delete"), egui::vec2(380.0, 110.0), |ui| {
            ui.label(trf("Delete the driver {}? A backup is kept first.", &[&d.name]));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if big_button(ui, tr("Delete"), 120.0).clicked() {
                    delete = true;
                }
                if big_button(ui, tr("Cancel"), 120.0).clicked() {
                    done = true;
                }
            });
        }) {
            done = true; // (closed with the window's own button)
        }
        if delete {
            if driver_files::game_running() {
                self.status = tr("Close the game to edit drivers.").to_string();
            } else {
                match driver_files::delete(&d) {
                    Ok(_) => {
                        self.reload_drivers(None);
                        self.status = tr("Driver deleted (backup in %APPDATA%\\ClassicLauncher\\backups).").to_string();
                    }
                    Err(e) => self.status = e,
                }
            }
            done = true;
        }
        if done {
            self.confirm_delete = None;
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = self.settings_open;
        let mut changed = false;
        let mut open_url: Option<String> = None;
        let mut message: Option<String> = None;
        // one rounded card per topic, each with its title
        let card = |ui: &mut egui::Ui, title: &str, show: bool, add: &mut dyn FnMut(&mut egui::Ui)| {
            if !show {
                return;
            }
            egui::Frame::new().fill(ui.visuals().faint_bg_color).stroke(ui.visuals().widgets.noninteractive.bg_stroke).corner_radius(6.0).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(title).strong().size(15.0).color(pal().dark));
                ui.add_space(4.0);
                add(ui);
            });
            ui.add_space(8.0);
        };
        open = crate::free::window(ctx, "settings", tr("Launcher settings"), egui::vec2(870.0, 660.0), |ui| {
            ui.set_width(820.0);
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 9.0);
            ui.label(RichText::new(tr("These settings are the launcher's; openOMSI's own are in the Options button.")).italics().size(12.0));
            ui.add_space(8.0);
            // one button per section, like OMSI's own tabs
            ui.horizontal(|ui| {
                for (i, name) in [tr("Appearance"), "openOMSI", tr("Backups"), tr("Updates")].into_iter().enumerate() {
                    let on = self.settings_tab == i;
                    let b = egui::Button::new(RichText::new(name).strong().color(if on { Color32::WHITE } else { pal().text })).fill(if on { pal().dark } else { pal().field }).min_size(egui::vec2(0.0, 30.0));
                    if ui.add(b).clicked() {
                        self.settings_tab = i;
                    }
                }
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical().max_height(500.0).min_scrolled_height(500.0).auto_shrink([false, false]).show(ui, |ui| {
                card(ui, tr("Appearance"), self.settings_tab == 0, &mut |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("Launcher language"));
                        egui::ComboBox::from_id_salt("launcher_lang").selected_text(self.prefs.lang.name()).show_ui(ui, |ui| {
                            for l in i18n::available(None) {
                                if ui.selectable_value(&mut self.prefs.lang, l, l.name()).changed() {
                                    i18n::set_lang(l);
                                    self.status = tr("Ready.").to_string();
                                    changed = true;
                                }
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        // another language: its texts go in a file of the "lang" folder (the template has every text to translate)
                        if ui.small_button(tr("Create translation template")).clicked() {
                            match i18n::write_template() {
                                Ok(p) => omsi::open_folder(p.parent().unwrap_or(std::path::Path::new("."))),
                                Err(e) => message = Some(e),
                            }
                        }
                        if ui.small_button(tr("Open the languages folder")).clicked() {
                            let dir = i18n::lang_dir();
                            let _ = std::fs::create_dir_all(&dir);
                            omsi::open_folder(&dir);
                        }
                    });
                    ui.label(RichText::new(tr("To add a language to the launcher, put a <CODE>.txt file in the languages folder.")).italics().size(12.0));
                    ui.horizontal(|ui| {
                        ui.label(tr("Theme"));
                        for t in theme::Theme::ALL {
                            if ui.radio_value(&mut self.prefs.theme, t, t.label()).changed() {
                                theme::apply(ui.ctx(), t);
                                changed = true;
                                if self.prefs.sync_theme {
                                    prefs::sync_official_theme(t);
                                }
                            }
                        }
                    });
                    if ui.checkbox(&mut self.prefs.sync_theme, tr("Use this look for openOMSI's own launcher and the game too")).changed() {
                        changed = true;
                        if self.prefs.sync_theme {
                            prefs::sync_official_theme(self.prefs.theme);
                        }
                    }
                    ui.label(RichText::new(tr("Light and the ClassicLauncher orange are the light look of openOMSI, Dark is its dark one; it is written to its settings.")).italics().size(12.0));
                    if ui.checkbox(&mut self.prefs.fullscreen, tr("Open the launcher full screen (F11)")).changed() {
                        changed = true;
                    }
                    if self.prefs.fullscreen != self.full_now {
                        ui.label(RichText::new(tr("This will be applied when the launcher is restarted (F11 changes it now).")).italics().size(12.0));
                    }
                });
                card(ui, tr("When the game starts"), self.settings_tab == 1, &mut |ui| {
                    egui::Grid::new("start-grid").num_columns(2).spacing([20.0, 10.0]).show(ui, |ui| {
                        ui.label(tr("Close the launcher when the game starts"));
                        changed |= ui.checkbox(&mut self.prefs.close_on_start, "").changed();
                        ui.end_row();
                        ui.label(tr("Traffic:"));
                        changed |= ui.add(egui::Slider::new(&mut self.prefs.traffic, 0..=100)).changed();
                        ui.end_row();
                        ui.label(tr("Passengers at the stops"));
                        changed |= ui.checkbox(&mut self.prefs.passengers, "").changed();
                        ui.end_row();
                        ui.label(tr("Timetable (scheduled buses)"));
                        changed |= ui.checkbox(&mut self.prefs.schedule, "").changed();
                        ui.end_row();
                        ui.label(tr("Show the command"));
                        changed |= ui.checkbox(&mut self.prefs.show_command, "").changed();
                        ui.end_row();
                    });
                });
                card(ui, tr("Folders"), self.settings_tab == 1, &mut |ui| {
                    let show = |p: &Option<PathBuf>| p.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| tr("Not found").to_string());
                    ui.label(RichText::new(tr("OMSI 2 folder")).strong());
                    ui.label(show(&self.omsi));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(tr("Other folder:"));
                        let mut dir = self.prefs.omsi_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
                        if ui.add(egui::TextEdit::singleline(&mut dir).desired_width(430.0)).changed() {
                            self.prefs.omsi_dir = (!dir.trim().is_empty()).then(|| PathBuf::from(dir.trim()));
                            changed = true;
                        }
                        if ui.button(tr("Browse...")).clicked() {
                            if let Some(p) = omsi::browse_folder(tr("Choose the OMSI 2 folder"), self.prefs.omsi_dir.as_deref().or(self.omsi.as_deref())) {
                                self.prefs.omsi_dir = Some(p);
                                changed = true;
                            }
                        }
                    });
                    ui.label(RichText::new(tr("(the new folder is used when the launcher restarts)")).italics().size(12.0));
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr("openOMSI program")).strong());
                    ui.label(show(&self.game));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(tr("Other program:"));
                        let mut exe = self.prefs.game.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
                        if ui.add(egui::TextEdit::singleline(&mut exe).desired_width(430.0)).changed() {
                            self.prefs.game = (!exe.trim().is_empty()).then(|| PathBuf::from(exe.trim()));
                            self.game = game::find_game(self.prefs.game.as_deref());
                            changed = true;
                        }
                        if ui.button(tr("Browse...")).clicked() {
                            if let Some(p) = omsi::browse_exe(tr("Choose openomsi.exe"), self.game.as_ref().and_then(|g| g.parent()).or(local_programs().as_deref())) {
                                self.prefs.game = Some(p);
                                self.game = game::find_game(self.prefs.game.as_deref());
                                changed = true;
                            }
                        }
                    });
                    ui.label(RichText::new(tr("(the path of openomsi.exe)")).italics().size(12.0));
                });
                card(ui, tr("Backups"), self.settings_tab == 2, &mut |ui| {
                    ui.label(RichText::new(tr("The 3 latest copies of each file are kept; a new one deletes the oldest.")).italics().size(12.0));
                    if ui.button(tr("Open the backups folder")).clicked() {
                        let dir = driver_files::backup_dir();
                        let _ = std::fs::create_dir_all(&dir);
                        omsi::open_folder(&dir);
                    }
                });
                card(ui, tr("Updates"), self.settings_tab == 3, &mut |ui| {
                    ui.label(trf("Installed version: {}", &[update::LABEL]));
                    let state = self.update.lock().unwrap().clone();
                    ui.horizontal(|ui| {
                        let checking = state == update::State::Checking;
                        if ui.add_enabled(!checking, egui::Button::new(tr("Look for updates"))).clicked() {
                            let c = ui.ctx().clone();
                            update::start(&self.update, move || c.request_repaint());
                        }
                        match &state {
                            update::State::Idle => {}
                            update::State::Checking => {
                                ui.spinner();
                                ui.label(tr("Checking..."));
                            }
                            update::State::UpToDate => {
                                ui.label(tr("You have the latest version."));
                            }
                            update::State::Newer { tag, url } => {
                                ui.label(RichText::new(trf("There is a new version: {}", &[tag])).strong());
                                if ui.button(tr("Open the download page")).clicked() {
                                    open_url = Some(url.clone());
                                }
                            }
                            update::State::Failed(_) => {
                                ui.label(tr("Could not check (no connection, or no versions published yet?)."));
                            }
                        }
                    });
                    ui.label(RichText::new(trf("Versions are published at github.com/{}. The launcher only tells you: it downloads and changes nothing by itself.", &[update::REPO])).italics().size(12.0));
                });
            });
        });
        self.settings_open = open;
        if let Some(u) = open_url {
            omsi::open_folder(std::path::Path::new(&u));
        }
        if let Some(m) = message {
            self.status = m;
        }
        if changed {
            let _ = self.prefs.save();
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.data(|d| d.get_temp::<std::time::Instant>(egui::Id::new("areas-reset-at"))).is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(700)) {
            ctx.memory_mut(|m| m.reset_areas());
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            self.full_now = !self.full_now;
            self.prefs.fullscreen = self.full_now;
            set_fullscreen(&ctx, self.full_now);
            let _ = self.prefs.save();
        }
        // The opening screen (the window opens normal, goes full screen after it), as in the original launcher.
        if !self.entered_full && self.born.elapsed() >= SPLASH {
            self.entered_full = true;
            if self.prefs.fullscreen {
                set_fullscreen(&ctx, true);
                self.full_now = true;
            }
        }
        if !self.entered_full {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
            let theme = self.prefs.theme;
            egui::CentralPanel::default().frame(egui::Frame::new().fill(pal().bg)).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space((ui.available_height() * 0.30).max(10.0));
                    ui.add(logo_image(theme, 420.0, 420.0 / logo::ASPECT));
                    ui.add_space(24.0);
                    ui.add(egui::Spinner::new().size(32.0));
                    ui.add_space(10.0);
                    ui.label(RichText::new(tr("Loading...")).size(18.0));
                });
            });
            return;
        }
        // Full screen: the interface sits in the middle of a black screen, like OMSI's own start screen.
        let mut centred;
        let host: &mut egui::Ui = if self.full_now {
            let screen = ui.max_rect();
            ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
            let size = egui::vec2(1080.0_f32.min(screen.width()), 780.0_f32.min(screen.height()));
            centred = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(screen.center(), size)));
            &mut centred
        } else {
            ui
        };
        if self.preview.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        if self.born.elapsed() >= SPLASH + std::time::Duration::from_millis(800) {
            self.apply_preview(&ctx);
        }
        self.host_rect = host.max_rect();
        if self.omsi.is_none() || self.game.is_none() {
            self.locate(host);
            return;
        }
        self.poll_launch(&ctx);
        // a finished installation: the version is read again from the new program
        if matches!(*self.upd.install.lock().unwrap(), omsi_updates::Install::Done(_)) {
            if !self.install_seen {
                self.install_seen = true;
                self.version_for = None;
            }
        } else {
            self.install_seen = false;
        }
        if self.version_for != self.game {
            self.version_for = self.game.clone();
            if let Ok(mut v) = self.game_version.lock() {
                *v = None;
            }
            if let Some(exe) = self.game.clone() {
                let slot = self.game_version.clone();
                let ctx2 = ctx.clone();
                std::thread::spawn(move || {
                    let v = game::version_of(&exe);
                    if let Ok(mut s) = slot.lock() {
                        *s = v;
                    }
                    ctx2.request_repaint();
                });
            }
        }
        self.main_screen(host, &ctx);
        self.time_dialog_window(&ctx);
        self.bus_prompt_window(&ctx);
        self.settings_window(&ctx);
        self.driver_window(&ctx);
        self.confirm_window(&ctx);
        let installed = self.game_version.lock().ok().and_then(|v| v.clone());
        self.upd.windows(&ctx, installed, self.game.as_deref());
        self.tutorials_window(&ctx);
        self.staff_window(&ctx);
        if self.options_open {
            let mut open = true;
            // a window of its own size that can be moved and stretched; it opens a bit smaller than the launcher's 1080 x 780 area
            let size = egui::vec2(1000.0_f32.min(ctx.content_rect().width() - 80.0), 664.0_f32.min(ctx.content_rect().height() - 100.0));
            let installed = self.game_version.lock().ok().and_then(|v| v.clone());
            let mut page: Option<String> = None;
            open = crate::free::window(&ctx, "options", tr("openOMSI settings"), egui::vec2(size[0], size[1]), |ui| {
                // (the content sets the window's size, so the title bar is as wide as the window; `fixed_size` left it a few pixels short)
                ui.set_width(size[0] - 14.0);
                ui.set_height(size[1] - 48.0);
                let (upd, prefs) = (&mut self.upd, &mut self.prefs);
                options_ui::show(ui, &mut self.options, self.omsi.as_deref(), self.game.as_deref(), &mut |ui| {
                    if upd.card(ui, &mut prefs.omsi_beta, installed.clone(), &mut page) {
                        let _ = prefs.save();
                    }
                });
            });
            if let Some(p) = page {
                omsi::open_folder(std::path::Path::new(&p));
            }
            self.options_open = open;
        }
    }
}

fn main() -> eframe::Result {
    std::panic::set_hook(Box::new(|info| log::line(&format!("PANIC: {info}"))));
    let viewport = egui::ViewportBuilder::default()
        .with_title("ClassicLauncher")
        .with_icon(egui::IconData { rgba: app_icon::rgba(64), width: 64, height: 64 })
        .with_inner_size([1080.0, 780.0])
        .with_min_inner_size([900.0, 640.0]);
    let options = eframe::NativeOptions { viewport, centered: true, ..Default::default() };
    eframe::run_native(
        "ClassicLauncher",
        options,
        Box::new(|cc| {
            Ok(Box::new(App::new(cc)))
        }),
    )
}
