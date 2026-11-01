//! The showroom: a bus seen in 3D, turned with the mouse, drawn by openOMSI's own renderer - the same helper process
//! (`openomsi --launcher-service`) and the same showroom the official launcher uses. Requests go to it a line at a time on its
//! standard input, the pictures come back as JPEGs; this module keeps the process, asks on a thread of its own so the window never
//! waits, and hands the pictures over.

use eframe::egui::ColorImage;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};

/// What is asked of the helper.
enum Cmd {
    Look { bus: String, paint: String, map: String },
    Frame { w: u32, h: u32, force: bool, dx: f32, dy: f32, zoom: f32, soft: bool, view: Option<String> },
    Quit,
}

/// One answer to a picture request.
#[derive(Default)]
pub struct Frame {
    /// The helper is still reading the bus (a picture of the previous one may be on show).
    pub busy: bool,
    /// The picture is still easing after a turn: ask again.
    pub moving: bool,
    pub failed: bool,
    pub error: Option<String>,
    /// A picture exists (of this bus or the previous one).
    pub picture: bool,
    pub view: String,
    pub views: Vec<String>,
    /// The new picture, when it changed.
    pub image: Option<ColorImage>,
}

pub enum Event {
    Frame(Frame),
    /// The helper could not be started or ended.
    Down(String),
}

pub struct Service {
    tx: Sender<Cmd>,
    rx: Receiver<Event>,
}

fn log_file() -> Option<std::fs::File> {
    let dir = std::env::var_os("APPDATA").map(PathBuf::from)?.join("ClassicLauncher");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::File::create(dir.join("preview-service.log")).ok()
}

/// One line of request out, one line of answer and its bytes back.
fn ask(stdin: &mut std::process::ChildStdin, out: &mut BufReader<std::process::ChildStdout>, id: u64, mut req: Value) -> Result<(Value, Vec<u8>), String> {
    req["id"] = json!(id);
    writeln!(stdin, "{req}").and_then(|_| stdin.flush()).map_err(|_| "the preview helper ended".to_string())?;
    loop {
        let mut line = String::new();
        if out.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Err("the preview helper ended".into());
        }
        let Ok(head) = serde_json::from_str::<Value>(line.trim()) else { continue };
        let mut body = vec![0u8; head.get("len").and_then(Value::as_u64).unwrap_or(0) as usize];
        out.read_exact(&mut body).map_err(|e| e.to_string())?;
        if head.get("id").and_then(Value::as_u64) == Some(id) {
            return Ok((head, body));
        }
    }
}

fn picture(jpeg: &[u8]) -> Option<ColorImage> {
    let img = image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

impl Service {
    /// Starts the helper (it takes a few seconds to be ready; requests wait their turn).
    pub fn start(game: PathBuf) -> Service {
        let (tx, rx_cmd) = channel::<Cmd>();
        let (tx_ev, rx) = channel::<Event>();
        std::thread::spawn(move || {
            use std::os::windows::process::CommandExt;
            let mut cmd = Command::new(&game);
            cmd.arg("--launcher-service").env("RUST_LOG", "warn").stdin(Stdio::piped()).stdout(Stdio::piped()).creation_flags(0x0800_0000);
            match log_file() {
                Some(f) => cmd.stderr(Stdio::from(f)),
                None => cmd.stderr(Stdio::null()),
            };
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx_ev.send(Event::Down(e.to_string()));
                    return;
                }
            };
            let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
                let _ = tx_ev.send(Event::Down("no pipes".into()));
                return;
            };
            let mut out = BufReader::new(stdout);
            let mut id = 0u64;
            for c in rx_cmd {
                id += 1;
                match c {
                    Cmd::Quit => {
                        let _ = ask(&mut stdin, &mut out, id, json!({ "cmd": "quit" }));
                        break;
                    }
                    Cmd::Look { bus, paint, map } => {
                        // (midday, the game's own weather)
                        if let Err(e) = ask(&mut stdin, &mut out, id, json!({ "cmd": "look", "bus": bus, "paint": paint, "map": map, "weather": "", "time": 720, "date": "" })) {
                            let _ = tx_ev.send(Event::Down(e));
                            break;
                        }
                    }
                    Cmd::Frame { w, h, force, dx, dy, zoom, soft, view } => match ask(&mut stdin, &mut out, id, json!({ "cmd": "frame", "w": w, "h": h, "force": force, "dx": dx, "dy": dy, "zoom": zoom, "quality": if soft { 70 } else { 90 }, "view": view })) {
                        Ok((head, body)) => {
                            let s = |k: &str| head.get(k).and_then(Value::as_bool).unwrap_or(false);
                            let f = Frame {
                                busy: s("busy"),
                                moving: s("moving"),
                                failed: s("failed") || head.get("ok").and_then(Value::as_bool) == Some(false),
                                error: head.get("error").and_then(Value::as_str).map(str::to_string),
                                picture: s("picture"),
                                view: head.get("view").and_then(Value::as_str).unwrap_or("outside").to_string(),
                                views: head.get("views").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default(),
                                image: (!body.is_empty()).then(|| picture(&body)).flatten(),
                            };
                            let _ = tx_ev.send(Event::Frame(f));
                        }
                        Err(e) => {
                            let _ = tx_ev.send(Event::Down(e));
                            break;
                        }
                    },
                }
            }
            drop(stdin);
            let _ = child.kill();
            let _ = child.wait();
        });
        Service { tx, rx }
    }

    /// Another bus (and repaint) to show.
    pub fn look(&self, bus: &str, paint: &str, map: &str) {
        let _ = self.tx.send(Cmd::Look { bus: bus.into(), paint: paint.into(), map: map.into() });
    }

    /// The next picture, after turning it by a drag (`dx`, `dy` pixels) and bringing it nearer by `zoom` (above 1: further away).
    #[allow(clippy::too_many_arguments)]
    pub fn frame(&self, w: u32, h: u32, force: bool, dx: f32, dy: f32, zoom: f32, soft: bool, view: Option<String>) {
        let _ = self.tx.send(Cmd::Frame { w, h, force, dx, dy, zoom, soft, view });
    }

    /// What has come back since the last call.
    pub fn poll(&self) -> Vec<Event> {
        self.rx.try_iter().collect()
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Quit);
    }
}
