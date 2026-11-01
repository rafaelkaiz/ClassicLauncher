//! What Start asks on a free start: "Do you want to add a bus?" and, if so, the list of buses as OMSI 2 orders it (by family, then
//! variant) with a repaint, plate and number to choose. No is a start on foot, as before.

use crate::buses::{self, Bus};
use crate::game::BusChoice;
use crate::i18n::{tr, trf};
use crate::theme::pal;
use eframe::egui::{self, RichText};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub enum Outcome {
    Open,
    Cancel,
    /// Start, with this bus (None: on foot).
    Go(Option<BusChoice>),
}

pub struct BusPrompt {
    /// Asking yes or no (false), or choosing the bus.
    picking: bool,
    omsi: PathBuf,
    content: Option<PathBuf>,
    buses: Arc<Mutex<Option<Vec<Bus>>>>,
    family: Option<String>,
    filter: String,
    choice: BusChoice,
    paints: Vec<String>,
    /// The repaint the chosen bus comes in (the third line of its friendly name): what "no repaint" means.
    default_paint: String,
    /// The chosen bus's description, and whether it is open for reading.
    description: String,
    reading: bool,
    game: Option<PathBuf>,
    view: Showroom,
    /// The map the preview is lit for (`maps/<folder>/global.cfg`).
    pub map: String,
    /// The width the 3D view has on this screen.
    view_w: f32,
}

impl BusPrompt {
    pub fn new(omsi: &Path, content: Option<&Path>, game: Option<&Path>) -> Self {
        Self { picking: false, omsi: omsi.to_path_buf(), content: content.map(Path::to_path_buf), buses: Arc::new(Mutex::new(None)), family: None, filter: String::new(), choice: BusChoice::default(), paints: vec![], default_paint: String::new(), description: String::new(), reading: false, game: game.map(Path::to_path_buf), view: Showroom { service: game.map(|g| crate::showroom::Service::start(g.to_path_buf())), ..Default::default() }, map: "maps/Grundorf/global.cfg".into(), view_w: 440.0 }
    }

    /// Straight to the list of buses (for looking at it without clicking).
    pub fn open_picker(&mut self) {
        if let Ok(f) = std::env::var("CL_BUSFILE") {
            self.choice.file = f;
            self.family = std::env::var("CL_BUSFAMILY").ok();
            self.description = crate::omsi::read_cfg(&self.omsi.join(&self.choice.file)).map(|t| crate::buses::description(&t)).unwrap_or_default();
            self.reading = std::env::var_os("CL_READ").is_some();
        }
        self.picking = true;
        self.load();
    }

    fn load(&self) {
        let (b, o, c) = (self.buses.clone(), self.omsi.clone(), self.content.clone());
        std::thread::spawn(move || {
            let v = buses::list(&o, c.as_deref());
            *b.lock().unwrap() = Some(v);
        });
    }
}

pub fn show(ctx: &egui::Context, p: &mut BusPrompt) -> Outcome {
    let mut out = Outcome::Open;
    let mut open = true;
    let centre = ctx.content_rect().center();
    if !p.picking {
        open = crate::free::window(ctx, "bus-ask", tr("Start!"), egui::vec2(460.0, 170.0), |ui| {
            ui.set_width(420.0);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // a round badge with the bus on it
                let (rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 32.0, pal().dark);
                crate::icons::paint(ui.painter(), crate::icons::Icon::Bus, rect.center(), 19.0, egui::Color32::WHITE);
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr("Do you want to add a bus?")).size(21.0).strong());
                    ui.add(egui::Label::new(RichText::new(tr("No: you start on foot and can put a bus down from the game.")).weak().size(13.0)).wrap());
                });
            });
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let w = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                let yes = egui::Button::new(RichText::new(tr("Yes")).size(17.0).strong().color(egui::Color32::WHITE)).fill(pal().dark).corner_radius(4.0).min_size(egui::vec2(w, 42.0));
                if ui.add(yes).clicked() {
                    p.picking = true;
                    p.load();
                }
                let no = egui::Button::new(RichText::new(tr("No")).size(17.0).strong()).corner_radius(4.0).min_size(egui::vec2(w, 42.0));
                if ui.add(no).clicked() {
                    out = Outcome::Go(None);
                }
            });
            ui.add_space(4.0);
        });
    } else {
        let lib = p.buses.lock().unwrap().clone();
        open = crate::free::window(ctx, "bus-pick", tr("Choose a bus"), egui::vec2(1130.0, 640.0), |ui| {
            // (the window fits what there is: on a smaller window the columns and the view are narrower instead of running off the edge)
            let monitor_w = ctx.input(|i| i.viewport().monitor_size).map_or(1920.0, |m| m.x);
            let total = (monitor_w - 80.0).clamp(620.0, 1090.0);
            let preview_w = if total >= 1090.0 { 440.0 } else { (total - 230.0 - 260.0 - 30.0).max(180.0) };
            let variants_w = (total - 230.0 - preview_w - 30.0).max(180.0);
            p.view_w = preview_w;
            ui.set_width(total);
            let Some(list) = lib else {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr("Looking for the installed vehicles..."));
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
                return;
            };
            ui.add(egui::TextEdit::singleline(&mut p.filter).hint_text(tr("Search by name")).desired_width(f32::INFINITY));
            let q = p.filter.trim().to_lowercase();
            let shown: Vec<&Bus> = list.iter().filter(|b| q.is_empty() || b.family.to_lowercase().contains(&q) || b.variant.to_lowercase().contains(&q) || b.folder.to_lowercase().contains(&q)).collect();
            let mut families: Vec<&str> = vec![];
            for b in &shown {
                if families.last() != Some(&b.family.as_str()) {
                    families.push(&b.family);
                }
            }
            if p.family.as_deref().is_none_or(|f| !families.contains(&f)) {
                p.family = families.first().map(|s| s.to_string());
            }
            ui.add_space(4.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(egui::vec2(230.0, 322.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(230.0);
                    ui.label(RichText::new(tr("Manufacturer")).strong());
                    egui::ScrollArea::vertical().id_salt("bus-families").max_height(300.0).auto_shrink([false, false]).show(ui, |ui| {
                        for f in &families {
                            if ui.selectable_label(p.family.as_deref() == Some(*f), *f).clicked() {
                                p.family = Some(f.to_string());
                            }
                        }
                    });
                });
                ui.allocate_ui_with_layout(egui::vec2(variants_w, 322.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(variants_w);
                    ui.set_max_width(variants_w);
                    ui.label(RichText::new(tr("Model")).strong());
                    egui::ScrollArea::vertical().id_salt("bus-variants").max_height(300.0).auto_shrink([false, false]).show(ui, |ui| {
                        for b in shown.iter().filter(|b| Some(b.family.as_str()) == p.family.as_deref()) {
                            let label = if b.variant.is_empty() { b.file.rsplit('/').next().unwrap_or(&b.file).to_string() } else { b.variant.clone() };
                            let on = p.choice.file == b.file;

                            // (one line that wraps inside the column, so the window keeps its width)
                            let text = label.clone();
                            let r = ui.selectable_label(on, text);
                            let clicked = r.clicked();
                            r.on_hover_text(&b.file);
                            if clicked && !on {
                                p.choice = BusChoice { file: b.file.clone(), ..Default::default() };
                                p.paints = crate::ailists::paints(&p.omsi, p.content.as_deref(), &b.file);
                                p.default_paint = b.paint.trim().to_string();
                                p.description = crate::buses::description_in(&p.omsi, p.content.as_deref(), &b.file, crate::olf::omsi_code(crate::i18n::lang()), &b.description);
                                p.reading = false;
                            }
                        }
                    });
                });
                {
                    ui.vertical(|ui| {
                        ui.set_width(p.view_w);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(tr("Showroom")).strong());
                        });
                        showroom_ui(ui, p);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(tr("Drag to turn it, the wheel zooms.")).weak().size(11.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if !p.description.is_empty() && ui.small_button(tr("Read description")).clicked() {
                                    p.reading = !p.reading;
                                }
                            });
                        });
                    });
                }
            });
            ui.separator();
            let chosen = !p.choice.file.is_empty();
            ui.add_enabled_ui(chosen, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("Paint"));
                    // "no repaint" is the bus's own: named by the repaint it comes in
                    let own = if p.default_paint.is_empty() { tr("Default paint").to_string() } else { p.default_paint.clone() };
                    let shown = if p.choice.paint.is_empty() { own.clone() } else { p.choice.paint.clone() };
                    egui::ComboBox::from_id_salt("bus-paint").selected_text(shown).width(170.0).show_ui(ui, |ui| {
                        if ui.selectable_label(p.choice.paint.is_empty(), &own).clicked() {
                            p.choice.paint.clear();
                        }
                        for n in p.paints.clone() {
                            if n.eq_ignore_ascii_case(&p.default_paint) {
                                continue;
                            }
                            if ui.selectable_label(p.choice.paint.eq_ignore_ascii_case(&n), &n).clicked() {
                                p.choice.paint = n;
                            }
                        }
                    });
                    ui.label(tr("Plate"));
                    ui.add(egui::TextEdit::singleline(&mut p.choice.plate).desired_width(110.0));
                    ui.label(tr("Number"));
                    ui.add(egui::TextEdit::singleline(&mut p.choice.number).desired_width(70.0));
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let go = egui::Button::new(RichText::new(tr("Start!")).color(egui::Color32::WHITE)).fill(pal().dark);
                if ui.add_enabled(chosen, go).clicked() {
                    out = Outcome::Go(Some(p.choice.clone()));
                }
                if ui.button(tr("Without a bus")).clicked() {
                    out = Outcome::Go(None);
                }
                ui.label(RichText::new(trf("{} buses", &[&shown.len().to_string()])).weak().size(12.0));
            });
        });
    }
    // the description of the chosen bus, in a window of its own beside the list
    if p.reading && !p.description.is_empty() {
        let mut keep = true;
        let title = format!("{} - {}", tr("Description"), p.family.clone().unwrap_or_default());
        keep = crate::free::window(ctx, "bus-desc", &title, egui::vec2(520.0, 460.0), |ui| {
            egui::ScrollArea::vertical().max_height(380.0).auto_shrink([false, true]).show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(&p.description).size(13.0)).wrap().selectable(true));
            });
        });
        if !keep {
            p.reading = false;
        }
    }
    if !open {
        out = Outcome::Cancel;
    }
    out
}

/// The 3D view of the chosen bus: the helper process of openOMSI that draws it, what was asked of it, the last picture and its state.
struct Showroom {
    service: Option<crate::showroom::Service>,
    /// The bus and paint the helper was last told to show.
    asked: (String, String),
    /// A picture has been asked and not come back yet.
    inflight: bool,
    /// The turn, zoom and view gathered since the last request.
    dx: f32,
    dy: f32,
    zoom: f32,
    view: Option<String>,
    force: bool,
    /// The last answer said the picture is still being made or still moving: ask again.
    more: bool,
    dragging: bool,
    tex: Option<egui::TextureHandle>,
    picture: bool,
    busy: bool,
    failed: bool,
    error: String,
    now: String,
    views: Vec<String>,
}

impl Default for Showroom {
    fn default() -> Self {
        Self { service: None, asked: (String::new(), String::new()), inflight: false, dx: 0.0, dy: 0.0, zoom: 1.0, view: None, force: false, more: false, dragging: false, tex: None, picture: false, busy: false, failed: false, error: String::new(), now: "outside".into(), views: vec![] }
    }
}


/// Tells the helper which bus to show, takes the pictures it sends back, asks for the next one when something moved, and draws it.
fn showroom_ui(ui: &mut egui::Ui, p: &mut BusPrompt) {
    if p.view.service.is_none() && p.view.error.is_empty() {
        match &p.game {
            Some(g) => p.view.service = Some(crate::showroom::Service::start(g.clone())),
            None => p.view.error = tr("openOMSI not found").to_string(),
        }
    }
    let want = (p.choice.file.clone(), p.choice.paint.clone());
    if let (Some(s), true) = (&p.view.service, want != p.view.asked && !want.0.is_empty()) {
        s.look(&want.0, &want.1, &p.map);
        p.view.asked = want;
        p.view.force = true;
        p.view.more = true;
    }
    // what came back
    let mut events = vec![];
    if let Some(s) = &p.view.service {
        events = s.poll();
    }
    for e in events {
        match e {
            crate::showroom::Event::Frame(f) => {
                p.view.inflight = false;
                p.view.busy = f.busy;
                p.view.failed = f.failed;
                p.view.picture = f.picture;
                p.view.error = f.error.unwrap_or_default();
                p.view.now = f.view;
                p.view.views = f.views;
                p.view.more = !f.failed && (f.busy || f.moving || !f.picture);
                if let Some(img) = f.image {
                    match &mut p.view.tex {
                        Some(h) => h.set(img, egui::TextureOptions::LINEAR),
                        None => p.view.tex = Some(ui.ctx().load_texture("showroom", img, egui::TextureOptions::LINEAR)),
                    }
                }
                // the sharp picture after the lighter ones of a turn
                if !p.view.more && !p.view.dragging && p.view.tex.is_some() {
                    // (nothing more is asked unless the bus is moved again)
                }
            }
            crate::showroom::Event::Down(e) => {
                p.view.service = None;
                p.view.inflight = false;
                p.view.error = e;
            }
        }
    }
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(p.view_w, p.view_w * 300.0 / 440.0), egui::Sense::click_and_drag());
    p.view.dragging = resp.dragged();
    if resp.dragged() {
        let d = resp.drag_delta();
        p.view.dx += d.x;
        p.view.dy += d.y;
    }
    if resp.drag_stopped() {
        p.view.force = true; // the sharp picture
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            // (the scroll arrives spread over several frames: the zoom follows how far it went, about 8 % to a notch of the wheel)
            p.view.zoom *= (-scroll * 0.0015).exp().clamp(0.9, 1.1);
        }
    }
    // the next picture: asked while something moves or changes, one at a time
    let moved = p.view.dx != 0.0 || p.view.dy != 0.0 || p.view.zoom != 1.0 || p.view.view.is_some();
    if let (Some(s), false, true) = (&p.view.service, p.view.inflight, !p.view.asked.0.is_empty() && (p.view.more || p.view.force || moved)) {
        let soft = p.view.dragging || p.view.more;
        s.frame((p.view_w * 2.0) as u32, (p.view_w * 300.0 / 440.0 * 2.0) as u32, p.view.force, p.view.dx, p.view.dy, p.view.zoom, soft, p.view.view.take());
        p.view.dx = 0.0;
        p.view.dy = 0.0;
        p.view.zoom = 1.0;
        p.view.force = false;
        p.view.inflight = true;
    }
    if p.view.inflight || p.view.more || p.view.service.is_some() && p.view.asked.0.is_empty() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
    }
    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(40, 42, 48));
    match &p.view.tex {
        Some(t) if p.view.picture => {
            ui.painter().image(t.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            if p.view.busy {
                ui.painter().text(rect.right_top() + egui::vec2(-8.0, 8.0), egui::Align2::RIGHT_TOP, "…", egui::FontId::proportional(18.0), egui::Color32::WHITE);
            }
        }
        _ => {
            let text = if !p.view.error.is_empty() {
                tr(&p.view.error).to_string()
            } else if p.view.failed {
                tr("No preview of this bus").to_string()
            } else if p.view.asked.0.is_empty() {
                tr("Choose a version to see it.").to_string()
            } else {
                tr("Preparing the preview...").to_string()
            };
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(14.0), egui::Color32::from_rgb(200, 200, 205));
        }
    }
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, pal().border), egui::StrokeKind::Inside);
    // outside, passenger and driver views, when the bus has them
    if p.view.views.len() > 1 {
        ui.horizontal(|ui| {
            for (id, label) in [("outside", "Outside view"), ("passenger", "Passenger view"), ("driver", "Driver's view")] {
                if p.view.views.iter().any(|v| v == id) && ui.selectable_label(p.view.now == id, tr(label)).clicked() && p.view.now != id {
                    p.view.view = Some(id.to_string());
                    p.view.force = true;
                }
            }
        });
    }
}
