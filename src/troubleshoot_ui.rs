//! The "Troubleshooting" tab of the options window: the model test. The game itself loads a bus, a map or any file or folder
//! (`openomsi --check`) and answers every warning and error of its loaders as one line of JSON, which is listed here.

use crate::ailists::{self, LibVehicle};
use crate::i18n::{tr, trf};
use crate::omsi::MapInfo;
use crate::theme::pal;
use eframe::egui::{self, RichText, Ui};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Bus,
    Map,
    Path,
}

type Answer = Arc<Mutex<Option<Result<Value, String>>>>;

pub struct State {
    omsi: PathBuf,
    content: Option<PathBuf>,
    kind: Kind,
    buses: Arc<Mutex<Option<Vec<LibVehicle>>>>,
    bus: String,
    bus_filter: String,
    maps: Vec<MapInfo>,
    map: usize,
    path: String,
    deep: bool,
    all: bool,
    busy: bool,
    answer: Answer,
    result: Option<Result<Value, String>>,
}

impl State {
    pub fn new(omsi: &Path, content: Option<&Path>) -> Self {
        let buses = Arc::new(Mutex::new(None));
        {
            let (b, o, c) = (buses.clone(), omsi.to_path_buf(), content.map(Path::to_path_buf));
            std::thread::spawn(move || {
                let v = ailists::library(&o, c.as_deref());
                *b.lock().unwrap() = Some(v);
            });
        }
        Self { omsi: omsi.to_path_buf(), content: content.map(Path::to_path_buf), kind: Kind::Bus, buses, bus: String::new(), bus_filter: String::new(), maps: crate::omsi::list_maps(omsi), map: 0, path: String::new(), deep: false, all: false, busy: false, answer: Arc::new(Mutex::new(None)), result: None }
    }

    /// A vehicle of the library as a path on disk: the content folder's, else the installation's.
    fn resolve(&self, rel: &str) -> PathBuf {
        let rel = rel.replace('\\', "/");
        self.content.iter().map(|c| c.join(&rel)).find(|p| p.exists()).unwrap_or_else(|| self.omsi.join(&rel))
    }

    fn target(&self) -> Option<PathBuf> {
        match self.kind {
            Kind::Bus => (!self.bus.is_empty()).then(|| self.resolve(&self.bus)),
            Kind::Map => self.maps.get(self.map).map(|m| self.omsi.join("maps").join(&m.folder).join("global.cfg")),
            Kind::Path => (!self.path.trim().is_empty()).then(|| PathBuf::from(self.path.trim())),
        }
    }
}

/// Runs the game's own check on `target` and answers its JSON.
fn run_check(game: &Path, root: &Path, target: &Path, deep: bool) -> Result<Value, String> {
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new(game);
    cmd.arg("--check").arg(target).arg("--root").arg(root).env("OMSI_NO_GUARD", "1").creation_flags(0x0800_0000);
    if deep {
        cmd.arg("--deep");
    }
    let out = cmd.output().map_err(|e| trf("The game cannot start: {}", &[&e.to_string()]))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().rev().find(|l| l.trim_start().starts_with('{')).ok_or_else(|| {
        let err = String::from_utf8_lossy(&out.stderr);
        trf("The game gave no answer ({})", &[err.lines().last().unwrap_or("").trim()])
    })?;
    serde_json::from_str(line).map_err(|e| e.to_string())
}

fn level_color(level: &str) -> egui::Color32 {
    match level {
        "error" => egui::Color32::from_rgb(200, 60, 60),
        "warn" => egui::Color32::from_rgb(220, 140, 20),
        _ => egui::Color32::GRAY,
    }
}

pub fn show(ui: &mut Ui, st: &mut State, game: Option<&Path>) {
    if let Some(r) = st.answer.lock().unwrap().take() {
        st.result = Some(r);
        st.busy = false;
    }
    ui.add(egui::Label::new(RichText::new(tr("Anything loaded the way the game loads it - a bus, a map, a mod, a scenery object, a spline, a human or a mesh - and every warning of its files listed, with what it names that is not there. Edit the content, press Check again: the list is fresh each time.")).size(12.0)).wrap());
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (k, label) in [(Kind::Bus, "Bus"), (Kind::Map, "Map"), (Kind::Path, "File or folder")] {
            if crate::options_ui::tab_button(ui, tr(label), st.kind == k) {
                st.kind = k;
                st.result = None;
            }
        }
    });
    ui.add_space(4.0);
    match st.kind {
        Kind::Bus => {
            ui.add(egui::TextEdit::singleline(&mut st.bus_filter).hint_text(tr("Search by name")).desired_width(f32::INFINITY));
            let lib = st.buses.lock().unwrap().clone();
            match lib {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr("Looking for the installed vehicles..."));
                    });
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
                }
                Some(list) => {
                    let q = st.bus_filter.to_lowercase();
                    egui::ScrollArea::vertical().id_salt("ts-buses").max_height(150.0).auto_shrink([false, true]).show(ui, |ui| {
                        for v in list.iter().filter(|v| q.is_empty() || v.name.to_lowercase().contains(&q) || v.file.to_lowercase().contains(&q)).take(300) {
                            if ui.selectable_label(st.bus == v.file, format!("{}   ({})", v.name, v.folder)).clicked() {
                                st.bus = v.file.clone();
                            }
                        }
                    });
                }
            }
        }
        Kind::Map => {
            let names: Vec<String> = st.maps.iter().map(|m| m.name.clone()).collect();
            let mut map = st.map;
            if crate::pick::pick(ui, "ts-map", 320.0, &names, &mut map) {
                st.map = map;
            }
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.deep, tr("Load every object and spline of a map whole: models, meshes and textures (slower)"));
            });
        }
        Kind::Path => {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut st.path).hint_text(tr("A .bus, .ovh, .sco, .sli, .hum, .o3d, a map's global.cfg, or a folder or .zip")).desired_width(ui.available_width() - 110.0));
                if ui.button(tr("Folder...")).clicked() {
                    if let Some(p) = crate::omsi::browse_folder(tr("Choose the folder to check"), None) {
                        st.path = p.display().to_string();
                    }
                }
            });
        }
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let can = !st.busy && st.target().is_some() && game.is_some();
        let go = egui::Button::new(RichText::new(tr(if st.busy { "Checking..." } else { "Check" })).color(egui::Color32::WHITE)).fill(pal().dark);
        if ui.add_enabled(can, go).clicked() {
            if let (Some(g), Some(t)) = (game, st.target()) {
                st.busy = true;
                st.result = None;
                let (game, root, ans, deep) = (g.to_path_buf(), st.omsi.clone(), st.answer.clone(), st.deep && st.kind != Kind::Bus);
                std::thread::spawn(move || {
                    let r = run_check(&game, &root, &t, deep);
                    *ans.lock().unwrap() = Some(r);
                });
            }
        }
        if st.busy {
            ui.spinner();
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
        if game.is_none() {
            ui.label(tr("openOMSI not found"));
        }
    });
    ui.separator();
    match &st.result {
        None => {
            ui.label(RichText::new(tr("Choose what to check and press Check.")).weak());
        }
        Some(Err(e)) => {
            ui.colored_label(level_color("error"), e);
            ui.label(RichText::new(tr("This needs a version of openOMSI that has the --check option.")).weak().size(12.0));
        }
        Some(Ok(v)) => {
            let ok = v["ok"].as_bool().unwrap_or(false);
            let errors = v["errors"].as_u64().unwrap_or(0);
            let warnings = v["warnings"].as_u64().unwrap_or(0);
            let name = v["summary"]["name"].as_str().or(v["file"].as_str()).unwrap_or("");
            if ok {
                ui.colored_label(egui::Color32::from_rgb(40, 140, 70), trf("Loaded without errors: {}", &[name]) + &if warnings > 0 { format!(" · {}", trf("{} warnings", &[&warnings.to_string()])) } else { String::new() });
            } else {
                ui.colored_label(level_color("error"), trf("It did not load cleanly: {} errors, {} warnings", &[&errors.to_string(), &warnings.to_string()]));
            }
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.all, tr("Everything the loaders said"));
                if let Some(s) = v["seconds"].as_f64() {
                    ui.label(RichText::new(format!("{s:.1} s")).weak().size(12.0));
                }
            });
            let findings: Vec<&Value> = v["findings"].as_array().map(|a| a.iter().filter(|f| st.all || f["level"].as_str() != Some("info")).collect()).unwrap_or_default();
            if findings.is_empty() {
                ui.label(tr("Nothing to report."));
            }
            egui::ScrollArea::vertical().id_salt("ts-findings").auto_shrink([false, false]).max_height((ui.available_height() - 6.0).max(100.0)).show(ui, |ui| {
                for f in findings.into_iter().take(2000) {
                    let level = f["level"].as_str().unwrap_or("info");
                    ui.horizontal_top(|ui| {
                        ui.label(RichText::new(tr(level)).color(level_color(level)).strong().size(12.0));
                        ui.vertical(|ui| {
                            ui.add(egui::Label::new(f["text"].as_str().unwrap_or("")).wrap());
                            let part = f["part"].as_str().map(|p| format!("{p} · ")).unwrap_or_default();
                            let hint = f["hint"].as_str().map(|h| format!(" · {h}")).unwrap_or_default();
                            ui.label(RichText::new(format!("{part}{}{hint}", f["what"].as_str().unwrap_or(""))).weak().size(11.0));
                        });
                    });
                    ui.separator();
                }
            });
        }
    }
}
