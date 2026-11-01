//! Game controllers - wheels, pedals, joysticks, gamepads - as openOMSI reads them from `Inputs/gamectrler.cfg`. The launcher edits the
//! copy in `~/.openomsi/Inputs` (the one the game prefers; OMSI 2's own file is only read, to start from it), after copying the old
//! one to a `.bak`. The format is the game's own (ported from `omsi-input/src/controllers.rs`).

use std::path::{Path, PathBuf};

/// What an axis does: the file's number.
pub const FUNCS: [(i32, &str); 8] = [
    (-1, "Not used"),
    (0, "Steering"),
    (1, "Throttle"),
    (2, "Brake"),
    (3, "Clutch"),
    (4, "Throttle and brake"),
    (5, "Look left / right"),
    (6, "Look up / down"),
];

/// The response curve of an axis: bits 2, 3 and 4 of the axis' flags.
pub const SHAPES: [(i32, &str); 5] = [(0, "Linear"), (8, "Progressive"), (4, "Degressive"), (24, "Bi-progressive"), (20, "Bi-degressive")];
pub const CURVE_BITS: i32 = 4 | 8 | 0x10;

pub const AXES: [&str; 8] = ["X axis", "Y axis", "Z axis", "X rotation", "Y rotation", "Z rotation", "Slider 1", "Slider 2"];

/// Steering force weights: centring, damping, parking, minimum force, smoothing.
pub const TUNE_RANGES: [(f32, f32); 5] = [(0.0, 2.0), (0.0, 3.0), (0.0, 2.0), (0.0, 0.2), (0.0, 0.05)];
pub const TUNE_DEFAULT: [f32; 5] = [1.0, 1.0, 1.0, 0.0, 0.0];

#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub name: String,
    /// The line after the name (kept as the file has it).
    pub second: String,
    /// Per axis: the function (-1: none) and whether it runs the other way.
    pub axes: [(i32, bool); 8],
    /// Per axis the file's flags beyond bit 0 (the response curve and what else the file keeps).
    pub axis_flags: [i32; 8],
    /// Per button: the key action (empty: none) and the number after it.
    pub buttons: Vec<(String, String)>,
    pub ff_scale: Option<(f32, f32)>,
    pub ff_tune: Option<[f32; 5]>,
    pub ff_invert: Option<bool>,
    /// Buttons (from 0) that latch: a lever, a lit button.
    pub latching: Vec<usize>,
    /// More actions a button sends (button from 0, action).
    pub more: Vec<(usize, String)>,
}

impl Device {
    pub fn new(name: &str) -> Device {
        Device { name: name.to_string(), second: "0".into(), axes: [(-1, false); 8], axis_flags: [0; 8], buttons: vec![], ff_scale: None, ff_tune: None, ff_invert: None, latching: vec![], more: vec![] }
    }

    pub fn tune(&self) -> [f32; 5] {
        self.ff_tune.unwrap_or(TUNE_DEFAULT)
    }

    /// The tuning back at its defaults is saved as none, so the file stays as it was.
    pub fn set_tune(&mut self, t: [f32; 5]) {
        self.ff_tune = (t.iter().zip(TUNE_DEFAULT).any(|(a, b)| (a - b).abs() > 1e-6)).then_some(t);
    }
}

pub fn parse(text: &str) -> Vec<Device> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out: Vec<Device> = vec![];
    let mut i = 0;
    let num = |s: Option<&&str>| s.map(|v| v.replace(',', ".").parse::<f32>().unwrap_or(1.0)).unwrap_or(1.0);
    while i < lines.len() {
        match lines[i] {
            "[ctrl]" => {
                let mut d = Device::new(lines.get(i + 1).unwrap_or(&""));
                d.second = lines.get(i + 2).unwrap_or(&"0").to_string();
                out.push(d);
                i += 3;
            }
            "[axis]" => {
                if let Some(d) = out.last_mut() {
                    for a in 0..8 {
                        let f: i32 = lines.get(i + 1 + a * 2).and_then(|v| v.parse().ok()).unwrap_or(-1);
                        let flags: i32 = lines.get(i + 2 + a * 2).and_then(|v| v.parse().ok()).unwrap_or(0);
                        d.axes[a] = (if FUNCS.iter().any(|x| x.0 == f) { f } else { -1 }, flags & 1 != 0);
                        d.axis_flags[a] = flags & !1;
                    }
                }
                i += 17;
            }
            "[buttons]" => {
                let n: usize = lines.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(0usize).min(512);
                if let Some(d) = out.last_mut() {
                    for b in 0..n {
                        d.buttons.push((lines.get(i + 2 + b * 2).unwrap_or(&"").to_string(), lines.get(i + 3 + b * 2).unwrap_or(&"0").to_string()));
                    }
                }
                i += 2 + n * 2;
            }
            "[ffscale]" | "[FFScale]" => {
                if let Some(d) = out.last_mut() {
                    d.ff_scale = Some((num(lines.get(i + 1)), num(lines.get(i + 2))));
                }
                i += 3;
            }
            "[openOMSI.Latching]" => {
                if let Some(d) = out.last_mut() {
                    d.latching = lines.get(i + 1).unwrap_or(&"").split_whitespace().filter_map(|v| v.parse::<usize>().ok()?.checked_sub(1)).collect();
                }
                i += 2;
            }
            "[openOMSI.MoreActions]" => {
                let n: usize = lines.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(0usize).min(4096);
                if let Some(d) = out.last_mut() {
                    for k in 0..n {
                        let button = lines.get(i + 2 + k * 2).and_then(|v| v.parse::<usize>().ok()?.checked_sub(1));
                        let action = lines.get(i + 3 + k * 2).unwrap_or(&"");
                        if let Some(b) = button.filter(|_| !action.is_empty()) {
                            d.more.push((b, action.to_string()));
                        }
                    }
                }
                i += 2 + n * 2;
            }
            "[openOMSI.FFTuning]" => {
                let given: Vec<f32> = lines[i + 1..].iter().take(5).take_while(|v| !v.is_empty() && !v.starts_with('[')).map(|v| v.replace(',', ".").parse::<f32>().unwrap_or(f32::NAN)).collect();
                if let Some(d) = out.last_mut() {
                    let t: [f32; 5] = std::array::from_fn(|k| given.get(k).copied().filter(|v| !v.is_nan()).map_or(TUNE_DEFAULT[k], |v| v.clamp(TUNE_RANGES[k].0, TUNE_RANGES[k].1) + 0.0));
                    d.set_tune(t);
                }
                i += 1 + given.len();
            }
            "[openOMSI.FFInvert]" => {
                if let Some(d) = out.last_mut() {
                    d.ff_invert = lines.get(i + 1).and_then(|v| match *v {
                        "0" => Some(false),
                        "1" => Some(true),
                        _ => None,
                    });
                }
                i += 2;
            }
            _ => i += 1,
        }
    }
    out
}

/// The file's text for `devices`, as OMSI writes it (CR LF).
pub fn text(devices: &[Device]) -> String {
    let mut t = String::new();
    for d in devices {
        t.push_str(&format!("\r\n[ctrl]\r\n{}\r\n{}\r\n\r\n[axis]\r\n", d.name, if d.second.is_empty() { "0" } else { &d.second }));
        for a in 0..8 {
            let (f, inv) = d.axes[a];
            t.push_str(&format!("{f}\r\n{}\r\n", d.axis_flags[a] | inv as i32));
        }
        t.push_str(&format!("\r\n[buttons]\r\n{}\r\n", d.buttons.len()));
        for (action, n) in &d.buttons {
            t.push_str(&format!("{action}\r\n{}\r\n", if n.is_empty() { "0" } else { n }));
        }
        let (a, b) = d.ff_scale.unwrap_or((1.0, 1.0));
        t.push_str(&format!("\r\n[FFScale]\r\n{a:.3}\r\n{b:.3}\r\n\r\n"));
        if let Some(tune) = d.ff_tune {
            t.push_str("[openOMSI.FFTuning]\r\n");
            for v in tune {
                t.push_str(&format!("{v:.3}\r\n"));
            }
            t.push_str("\r\n");
        }
        if let Some(invert) = d.ff_invert {
            t.push_str(&format!("[openOMSI.FFInvert]\r\n{}\r\n\r\n", invert as u8));
        }
        if !d.latching.is_empty() {
            let numbers: Vec<String> = d.latching.iter().map(|b| (b + 1).to_string()).collect();
            t.push_str(&format!("[openOMSI.Latching]\r\n{}\r\n\r\n", numbers.join(" ")));
        }
        if !d.more.is_empty() {
            t.push_str(&format!("[openOMSI.MoreActions]\r\n{}\r\n", d.more.len()));
            for (b, action) in &d.more {
                t.push_str(&format!("{}\r\n{action}\r\n", b + 1));
            }
            t.push_str("\r\n");
        }
    }
    t
}

/// The file the game prefers and the launcher writes: `~/.openomsi/Inputs/gamectrler.cfg`.
pub fn own_path() -> Option<PathBuf> {
    crate::omsi::data_dir().map(|d| d.join("Inputs").join("gamectrler.cfg"))
}

fn decode(b: &[u8]) -> String {
    if let Some(rest) = b.strip_prefix(&[0xFF, 0xFE]) {
        String::from_utf16_lossy(&rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>())
    } else {
        // the files are Windows-1252; the device names are plain ASCII in practice
        b.iter().map(|&c| c as char).collect()
    }
}

/// The devices to edit: the launcher's own file, else OMSI 2's.
pub fn load(omsi: Option<&Path>) -> Vec<Device> {
    let own = own_path().filter(|p| p.is_file());
    let theirs = omsi.map(|o| o.join("Inputs").join("gamectrler.cfg")).filter(|p| p.is_file());
    own.or(theirs).and_then(|p| std::fs::read(p).ok()).map(|b| parse(&decode(&b))).unwrap_or_default()
}

pub fn save(devices: &[Device]) -> Result<PathBuf, String> {
    let path = own_path().ok_or("no home folder")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    if path.is_file() {
        let _ = std::fs::copy(&path, path.with_extension("cfg.bak"));
    }
    std::fs::write(&path, text(devices).as_bytes()).map_err(|e| e.to_string())?;
    Ok(path)
}

/// The names of the actions that OMSI 2's `Inputs/keyboard.cfg` binds, for the button pickers.
pub fn action_names(omsi: Option<&Path>) -> Vec<String> {
    let mut names: Vec<String> = ["sim_pause", "open_menu", "screenshot", "photo_mode", "quicksave", "toggel_ctrler", "voice_radio", "voice_nearby", "doors_all", "doors_release"].iter().map(|s| s.to_string()).collect();
    if let Some(text) = omsi.map(|o| o.join("Inputs").join("keyboard.cfg")).and_then(|p| std::fs::read(p).ok()).map(|b| decode(&b)) {
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        for (i, l) in lines.iter().enumerate() {
            if *l == "[entry]" {
                if let Some(n) = lines.get(i + 1).filter(|n| !n.is_empty() && !n.starts_with('[')) {
                    names.push(n.to_string());
                }
            }
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    names
}

/// The joysticks, wheels and gamepads Windows has as HID devices, by their full product name (what the game shows). WinMM only knows
/// the ones registered the old way, which a modern wheel is often not.
fn hid_devices() -> Vec<String> {
    use windows_sys::Win32::Devices::HumanInterfaceDevice::HidD_GetProductString;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    use windows_sys::Win32::UI::Input::{GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICEINFO, RIDI_DEVICENAME, RID_DEVICE_INFO, RIM_TYPEHID};
    let mut out = vec![];
    unsafe {
        let size = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;
        let mut n = 0u32;
        if GetRawInputDeviceList(std::ptr::null_mut(), &mut n, size) == u32::MAX || n == 0 {
            return out;
        }
        let mut list: Vec<RAWINPUTDEVICELIST> = vec![std::mem::zeroed(); n as usize];
        let got = GetRawInputDeviceList(list.as_mut_ptr(), &mut n, size);
        if got == u32::MAX {
            return out;
        }
        for d in list.iter().take(got as usize).filter(|d| d.dwType == RIM_TYPEHID) {
            let mut info: RID_DEVICE_INFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<RID_DEVICE_INFO>() as u32;
            let mut isz = info.cbSize;
            if GetRawInputDeviceInfoW(d.hDevice, RIDI_DEVICEINFO, &mut info as *mut _ as *mut _, &mut isz) == u32::MAX {
                continue;
            }
            let (page, usage) = (info.Anonymous.hid.usUsagePage, info.Anonymous.hid.usUsage);
            if page != 1 || !matches!(usage, 4 | 5) {
                continue; // joystick or gamepad
            }
            let mut chars = 0u32;
            GetRawInputDeviceInfoW(d.hDevice, RIDI_DEVICENAME, std::ptr::null_mut(), &mut chars);
            let mut name = vec![0u16; chars as usize + 1];
            if GetRawInputDeviceInfoW(d.hDevice, RIDI_DEVICENAME, name.as_mut_ptr() as *mut _, &mut chars) == u32::MAX {
                continue;
            }
            let h = CreateFileW(name.as_ptr(), 0, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut());
            if h == INVALID_HANDLE_VALUE {
                continue;
            }
            let mut buf = [0u16; 128];
            if HidD_GetProductString(h, buf.as_mut_ptr() as *mut _, (buf.len() * 2) as u32) {
                let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                let n = String::from_utf16_lossy(&buf[..len]);
                if !n.trim().is_empty() && !out.contains(&n) {
                    out.push(n);
                }
            }
            CloseHandle(h);
        }
    }
    out
}

/// The game controllers Windows knows: HID joysticks and wheels by full name, plus what its old joystick driver lists.
pub fn connected() -> Vec<String> {
    let mut out = crate::dinput::connected();
    for n in hid_devices() {
        if !out.iter().any(|o| o.eq_ignore_ascii_case(&n)) {
            out.push(n);
        }
    }
    for n in connected_winmm() {
        if !out.iter().any(|o| o.eq_ignore_ascii_case(&n) || o.to_lowercase().starts_with(&n.to_lowercase())) {
            out.push(n);
        }
    }
    out
}

fn connected_winmm() -> Vec<String> {
    use windows_sys::Win32::Media::Multimedia::{JOYCAPSW, JOYERR_NOERROR, joyGetDevCapsW, joyGetNumDevs};
    let mut out = vec![];
    unsafe {
        for id in 0..joyGetNumDevs() {
            let mut caps: JOYCAPSW = std::mem::zeroed();
            if joyGetDevCapsW(id as usize, &mut caps, std::mem::size_of::<JOYCAPSW>() as u32) == JOYERR_NOERROR {
                let name = caps.szPname; // a copy: the struct is packed
                let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
                let n = String::from_utf16_lossy(&name[..len]);
                if !n.trim().is_empty() && !out.contains(&n) {
                    out.push(n);
                }
            }
        }
    }
    out
}

/// What a connected device reports now: its six axes from 0 to 1 (X, Y, Z, X rotation, Y rotation, Z rotation, as far as Windows'
/// joystick interface tells them apart) and which buttons are down. `name` is the device's name as the game has it (Windows cuts it
/// at 31 letters).
#[allow(dead_code)]
fn live_winmm(name: &str) -> Option<([f32; 6], u32)> {
    use windows_sys::Win32::Media::Multimedia::{JOYCAPSW, JOYERR_NOERROR, JOYINFOEX, joyGetDevCapsW, joyGetNumDevs, joyGetPosEx};
    let short: String = name.chars().take(31).collect();
    unsafe {
        for id in 0..joyGetNumDevs() {
            let mut caps: JOYCAPSW = std::mem::zeroed();
            if joyGetDevCapsW(id as usize, &mut caps, std::mem::size_of::<JOYCAPSW>() as u32) != JOYERR_NOERROR {
                continue;
            }
            let pname = caps.szPname;
            let len = pname.iter().position(|&c| c == 0).unwrap_or(pname.len());
            let found = String::from_utf16_lossy(&pname[..len]);
            // (a wheel the old driver lists only as the generic "PC-joystick": the one such device is taken for it)
            if found != short && !(found.contains("PC-joystick") && !name.is_empty()) {
                continue;
            }
            let (xmin, xmax, ymin, ymax, zmin, zmax, rmin, rmax, umin, umax, vmin, vmax) = (caps.wXmin, caps.wXmax, caps.wYmin, caps.wYmax, caps.wZmin, caps.wZmax, caps.wRmin, caps.wRmax, caps.wUmin, caps.wUmax, caps.wVmin, caps.wVmax);
            let mut info: JOYINFOEX = std::mem::zeroed();
            info.dwSize = std::mem::size_of::<JOYINFOEX>() as u32;
            info.dwFlags = 0xFF; // JOY_RETURNALL
            if joyGetPosEx(id, &mut info) != JOYERR_NOERROR {
                return None;
            }
            let norm = |v: u32, lo: u32, hi: u32| if hi > lo { ((v.saturating_sub(lo)) as f32 / (hi - lo) as f32).clamp(0.0, 1.0) } else { 0.5 };
            return Some(([norm(info.dwXpos, xmin, xmax), norm(info.dwYpos, ymin, ymax), norm(info.dwZpos, zmin, zmax), norm(info.dwUpos, umin, umax), norm(info.dwVpos, vmin, vmax), norm(info.dwRpos, rmin, rmax)], info.dwButtons));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\r\n[ctrl]\r\nTest Wheel\r\n0\r\n\r\n[axis]\r\n0\r\n24\r\n1\r\n1\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n-1\r\n0\r\n\r\n[buttons]\r\n2\r\nsim_pause\r\n0\r\n\r\n0\r\n\r\n[FFScale]\r\n1.000\r\n0.500\r\n\r\n[openOMSI.Latching]\r\n2\r\n\r\n";

    #[test]
    fn a_device_is_read_and_written_back_the_same() {
        let d = parse(SAMPLE);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].name, "Test Wheel");
        assert_eq!(d[0].axes[0], (0, false));
        assert_eq!(d[0].axes[1], (1, true));
        assert_eq!(d[0].axis_flags[0], 24);
        assert_eq!(d[0].buttons.len(), 2);
        assert_eq!(d[0].ff_scale, Some((1.0, 0.5)));
        assert_eq!(d[0].latching, vec![1]);
        let again = parse(&text(&d));
        assert_eq!(again, d);
    }

    #[test]
    fn default_tuning_is_not_written() {
        let mut d = Device::new("X");
        d.set_tune(TUNE_DEFAULT);
        assert!(!text(&[d.clone()]).contains("FFTuning"));
        d.set_tune([1.5, 1.0, 1.0, 0.0, 0.0]);
        assert!(text(&[d.clone()]).contains("FFTuning"));
        assert_eq!(parse(&text(&[d]))[0].tune()[0], 1.5);
    }
}

#[cfg(test)]
mod omsi_file_tests {
    use super::*;

    /// OMSI 2's own file (default content): whatever devices it holds are read and written back the same.
    #[test]
    fn omsi_2s_own_controller_file_round_trips() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        let Ok(bytes) = std::fs::read(omsi.join("Inputs").join("gamectrler.cfg")) else { return };
        let devices = parse(&decode(&bytes));
        assert_eq!(parse(&text(&devices)), devices);
    }
}

/// What a connected device reports now: its eight axes from 0 to 1 (None: it has no such axis) and its buttons down (a bit each).
pub fn live(name: &str) -> Option<([Option<f32>; 8], u128)> {
    crate::dinput::live(name)
}
