//! Everything read from the OMSI 2 installation that openOMSI plays on: maps, their start places, drivers, vehicles. Read-only.

use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct MapInfo {
    /// Folder name under `maps`.
    pub folder: String,
    pub name: String,
    pub description: String,
    pub picture: Option<PathBuf>,
}

impl MapInfo {
    /// The map as openOMSI's `--map` takes it.
    pub fn arg(&self) -> String {
        format!("maps/{}/global.cfg", self.folder)
    }
}

/// A place the vehicle can start at: the name shown and the index `--entry` takes.
#[derive(Clone, Debug, PartialEq)]
pub struct StartPlace {
    pub name: String,
    pub index: usize,
    /// Where the map recorded it (world metres), when it can be worked out.
    pub view: Option<View>,
}

/// A place on the map: world x, y, the height and the heading in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub heading: f64,
}

#[derive(Clone)]
pub struct Bus {
    /// Shown in the list: `folder / file`.
    pub label: String,
    /// `Vehicles/Folder/file.bus`, the way `--bus` takes it.
    pub arg: String,
}

/// A complete enough OMSI 2 folder for openOMSI: the content folders it reads.
pub fn is_omsi_root(p: &Path) -> bool {
    p.join("maps").is_dir() && p.join("Vehicles").is_dir()
}

/// The OMSI 2 folder: the saved one, `OMSI_ROOT`, the one openOMSI remembers, then the usual Steam and Aerosoft places.
pub fn find_omsi_dir(saved: Option<PathBuf>) -> Option<PathBuf> {
    let from_env = std::env::var_os("OMSI_ROOT").map(PathBuf::from);
    let remembered = home().and_then(|h| std::fs::read_to_string(h.join(".openomsi-root")).ok()).map(|t| PathBuf::from(t.trim()));
    let defaults = [
        r"C:\Program Files (x86)\Steam\steamapps\common\OMSI 2",
        r"C:\Program Files\Steam\steamapps\common\OMSI 2",
        r"C:\Program Files (x86)\Aerosoft\OMSI 2",
    ]
    .map(PathBuf::from);
    saved.into_iter().chain(from_env).chain(remembered).chain(defaults).find(|p| is_omsi_root(p))
}

pub fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from)
}

/// openOMSI's data folder, `~/.openomsi`.
pub fn data_dir() -> Option<PathBuf> {
    home().map(|h| h.join(".openomsi"))
}

/// OMSI writes its .cfg files as UTF-16 (with BOM) or plain ANSI depending on the author.
pub fn read_cfg(path: &Path) -> Option<String> {
    let b = std::fs::read(path).ok()?;
    if b.starts_with(&[0xFF, 0xFE]) {
        let u: Vec<u16> = b[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Some(String::from_utf16_lossy(&u))
    } else {
        match std::str::from_utf8(&b) {
            Ok(s) => Some(s.to_string()), // (some authors save UTF-8 without a mark)
            Err(_) => Some(b.iter().map(|&c| c as char).collect()),
        }
    }
}

fn cfg_value(cfg: &str, key: &str) -> Option<String> {
    let mut lines = cfg.lines();
    while let Some(l) = lines.next() {
        if l.trim().eq_ignore_ascii_case(key) {
            return lines.next().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        }
    }
    None
}

/// Lines under `key` up to the next `[tag]` line (the map description).
fn cfg_block(cfg: &str, key: &str) -> String {
    let is_tag = |l: &str| l.starts_with('[') && l.ends_with(']') && !l.contains(' ');
    let mut lines = cfg.lines().map(str::trim_end);
    lines.by_ref().find(|l| l.trim().eq_ignore_ascii_case(key));
    let body: Vec<&str> = lines.take_while(|l| !is_tag(l.trim())).collect();
    body.join("\n").trim().to_string()
}

pub fn list_maps(omsi: &Path) -> Vec<MapInfo> {
    let mut maps: Vec<MapInfo> = std::fs::read_dir(omsi.join("maps"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let dir = e.path();
            let cfg = read_cfg(&dir.join("global.cfg"))?;
            let name = cfg_value(&cfg, "[name]")?;
            let description = cfg_block(&cfg, "[description]");
            let picture = Some(dir.join("picture.jpg")).filter(|p| p.is_file());
            let folder = dir.file_name()?.to_string_lossy().into_owned();
            Some(MapInfo { folder, name, description, picture })
        })
        .collect();
    maps.sort_by_key(|m| m.name.to_lowercase());
    maps
}

/// The places a vehicle can start at: the names in the map's `[entrypoints]` (global.cfg), once each and in alphabetical order, as
/// openOMSI's start list shows them. Each is a record of 12 lines (index, object, 1 number, x, z, y, 4 for the heading, group,
/// name) after the count; the index is the record's place, and the first record of a name is the one used. Empty when the map
/// lists none.
pub fn start_places(omsi: &Path, map: &MapInfo) -> Vec<StartPlace> {
    let Some(cfg) = read_cfg(&omsi.join("maps").join(&map.folder).join("global.cfg")) else { return vec![] };
    entry_records(&cfg)
}

fn entry_records(cfg: &str) -> Vec<StartPlace> {
    let lines: Vec<&str> = cfg.lines().map(str::trim).collect();
    let Some(at) = lines.iter().position(|l| l.eq_ignore_ascii_case("[entrypoints]")) else { return vec![] };
    let count: usize = lines.get(at + 1).and_then(|l| l.parse().ok()).unwrap_or(0);
    // the tiles in the order of the `[map]` blocks: an entry point's group is a place in this list
    let tiles: Vec<(i32, i32)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.eq_ignore_ascii_case("[map]"))
        .filter_map(|(i, _)| Some((lines.get(i + 1)?.parse().ok()?, lines.get(i + 2)?.parse().ok()?)))
        .collect();
    let world = lines.iter().any(|l| l.eq_ignore_ascii_case("[worldcoordinates]"));
    let size = tile_size(world, &tiles);
    let mut out: Vec<StartPlace> = vec![];
    for i in 0..count {
        let base = at + 2 + i * 12;
        let Some(name) = lines.get(base + 11) else { break };
        let name = name.trim();
        if !name.is_empty() && !out.iter().any(|p| p.name.eq_ignore_ascii_case(name)) {
            let n = |k: usize| lines.get(base + k).and_then(|v| v.replace(',', ".").parse::<f64>().ok());
            // where the map recorded it: the tile's corner plus the place in the tile; the heading from the quaternion
            let view = (|| {
                let (tx, ty) = *usize::try_from(lines.get(base + 10)?.parse::<i64>().ok()?).ok().and_then(|g| tiles.get(g))?;
                let (x, z, y, q1, q3) = (n(3)?, n(4)?, n(5)?, n(7)?, n(9)?);
                let heading = (2.0 * q1.atan2(q3)).to_degrees().rem_euclid(360.0);
                Some(View { x: f64::from(tx) * size + x, y: f64::from(ty) * size + y, z, heading })
            })();
            out.push(StartPlace { name: name.to_string(), index: i, view });
        }
    }
    out.sort_by_key(|p| p.name.to_lowercase());
    out
}

/// The edge of a tile in metres, as the game works it out: 300 for a plain map; for a `[worldcoordinates]` map the width of its
/// median row (Web Mercator tiles of zoom 16).
fn tile_size(world: bool, tiles: &[(i32, i32)]) -> f64 {
    if !world {
        return 300.0;
    }
    let width = |ty: i32| {
        let lat = 2.0 * (std::f64::consts::TAU * f64::from(ty) / 65536.0).exp().atan() - std::f64::consts::FRAC_PI_2;
        40_075_016.69 * lat.cos() / 65536.0
    };
    let mut rows: Vec<i32> = tiles.iter().map(|t| t.1).collect();
    if rows.is_empty() {
        return 371.9;
    }
    rows.sort_unstable();
    let r = rows[rows.len() / 2];
    (width(r) + width(r + 1)) / 2.0
}

/// The vehicles: every `.bus` file in a folder of `Vehicles`.
pub fn list_buses(omsi: &Path) -> Vec<Bus> {
    let mut v: Vec<Bus> = vec![];
    for dir in std::fs::read_dir(omsi.join("Vehicles")).into_iter().flatten().flatten() {
        let folder = dir.file_name().to_string_lossy().into_owned();
        for f in std::fs::read_dir(dir.path()).into_iter().flatten().flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bus")) {
                let file = p.file_name().unwrap().to_string_lossy().into_owned();
                let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
                v.push(Bus { label: format!("{folder} / {stem}"), arg: format!("Vehicles/{folder}/{file}") });
            }
        }
    }
    v.sort_by_key(|b| b.label.to_lowercase());
    v
}

/// Windows' modern open dialog (the one with the address bar, where `%APPDATA%` can be typed and hidden folders are reachable), run
/// through PowerShell so the launcher needs no COM code. `folder` picks a folder (the dialog's usual trick: it returns the folder
/// shown when "Select this folder" is pressed); otherwise a file matching `filter` ("Programs|*.exe"). `initial` is where it opens.
fn open_dialog(title: &str, folder: bool, filter: &str, initial: Option<&Path>) -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    const SCRIPT: &str = r#"
Add-Type -AssemblyName System.Windows.Forms
$d = New-Object System.Windows.Forms.OpenFileDialog
$d.Title = $env:CL_TITLE
$d.RestoreDirectory = $true
if ($env:CL_DIR -and (Test-Path -LiteralPath $env:CL_DIR)) { $d.InitialDirectory = $env:CL_DIR }
if ($env:CL_FOLDER -eq '1') {
    $d.ValidateNames = $false
    $d.CheckFileExists = $false
    $d.CheckPathExists = $true
    $d.FileName = 'Select this folder'
} else {
    $d.Filter = $env:CL_FILTER
}
$f = New-Object System.Windows.Forms.Form
$f.TopMost = $true
if ($d.ShowDialog($f) -eq 'OK') {
    if ($env:CL_FOLDER -eq '1') { [Console]::Out.Write((Split-Path -Parent $d.FileName)) } else { [Console]::Out.Write($d.FileName) }
}
"#;
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-STA", "-WindowStyle", "Hidden", "-ExecutionPolicy", "Bypass", "-Command", SCRIPT])
        .env("CL_TITLE", title)
        .env("CL_FOLDER", if folder { "1" } else { "0" })
        .env("CL_FILTER", filter)
        .env("CL_DIR", initial.map(|p| p.display().to_string()).unwrap_or_default())
        .creation_flags(0x0800_0000) // no console window
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!text.is_empty()).then(|| PathBuf::from(text))
}

/// Asks for a folder, opening at `initial`.
pub fn browse_folder(title: &str, initial: Option<&Path>) -> Option<PathBuf> {
    open_dialog(title, true, "", initial)
}

/// Asks for a program (`*.exe`), opening at `initial`.
pub fn browse_exe(title: &str, initial: Option<&Path>) -> Option<PathBuf> {
    open_dialog(title, false, "Programs (*.exe)|*.exe|All files (*.*)|*.*", initial)
}

/// The local date and time of this computer: (year, month, day, hour, minute, second).
pub fn local_now() -> (u32, u32, u32, u32, u32, u32) {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let mut t = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut t) };
    (t.wYear as u32, t.wMonth as u32, t.wDay as u32, t.wHour as u32, t.wMinute as u32, t.wSecond as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(index: usize, name: &str) -> String {
        format!("{index}\n7\n0\n1\n2\n3\n0\n0\n0\n1\n0\n{name}\n")
    }

    #[test]
    fn start_places_use_the_first_record_of_each_name_in_alphabetical_order() {
        let cfg = format!("[entrypoints]\n4\n{}{}{}{}[next]\n", record(0, "Zeta"), record(1, "alpha"), record(2, "Zeta"), record(3, "[25 DIM] Conleau"));
        let v = entry_records(&cfg);
        let names: Vec<_> = v.iter().map(|p| (p.name.as_str(), p.index)).collect();
        assert_eq!(names, [("[25 DIM] Conleau", 3), ("alpha", 1), ("Zeta", 0)]);
    }

    /// Default content only: the places of Grundorf, whatever they are, are named, distinct and have valid indexes.
    #[test]
    fn reads_the_start_places_of_grundorf() {
        let Some(omsi) = find_omsi_dir(None) else { return };
        let Some(map) = list_maps(&omsi).into_iter().find(|m| m.folder == "Grundorf") else { return };
        let places = start_places(&omsi, &map);
        let mut names: Vec<_> = places.iter().map(|p| p.name.to_lowercase()).collect();
        names.dedup();
        assert_eq!(names.len(), places.len());
        assert!(places.iter().all(|p| !p.name.is_empty()));
    }
}

#[cfg(test)]
mod place_tests {
    use super::*;

    /// A plain map and a `[worldcoordinates]` one (Berlin-Spandau, installed by default): the tile sizes the game uses.
    #[test]
    fn tile_sizes_follow_the_game() {
        assert_eq!(tile_size(false, &[(1, 1)]), 300.0);
        let Some(omsi) = find_omsi_dir(None) else { return };
        let Some(cfg) = read_cfg(&omsi.join("maps").join("Berlin-Spandau").join("global.cfg")) else { return };
        let lines: Vec<&str> = cfg.lines().map(str::trim).collect();
        let tiles: Vec<(i32, i32)> = lines.iter().enumerate().filter(|(_, l)| l.eq_ignore_ascii_case("[map]")).filter_map(|(i, _)| Some((lines.get(i + 1)?.parse().ok()?, lines.get(i + 2)?.parse().ok()?))).collect();
        let world = lines.iter().any(|l| l.eq_ignore_ascii_case("[worldcoordinates]"));
        if !world || tiles.is_empty() {
            return;
        }
        let s = tile_size(true, &tiles);
        assert!((371.0..373.0).contains(&s), "{s}");
    }

    /// Default content: every start place of Berlin-Spandau has a position on the map near the others (all inside a few km).
    #[test]
    fn the_places_of_berlin_spandau_have_positions() {
        let Some(omsi) = find_omsi_dir(None) else { return };
        let Some(map) = list_maps(&omsi).into_iter().find(|m| m.folder == "Berlin-Spandau") else { return };
        let places = start_places(&omsi, &map);
        if places.is_empty() {
            return;
        }
        let views: Vec<View> = places.iter().filter_map(|p| p.view).collect();
        assert_eq!(views.len(), places.len());
        let (minx, maxx) = views.iter().fold((f64::MAX, f64::MIN), |a, v| (a.0.min(v.x), a.1.max(v.x)));
        assert!(maxx - minx < 20_000.0 && views.iter().all(|v| (0.0..360.0).contains(&v.heading)));
    }
}

/// Opens a folder (or a web address) with Windows.
pub fn open_folder(dir: &Path) {
    let _ = std::process::Command::new("explorer").arg(dir).spawn();
}

/// Does the process `pid` have a visible window of its own (the game's, once it opens)?
pub fn has_window(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GW_OWNER, GetWindow, GetWindowThreadProcessId, IsWindowVisible};
    struct Ask {
        pid: u32,
        found: bool,
    }
    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> i32 {
        let ask = unsafe { &mut *(l as *mut Ask) };
        let mut p = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut p) };
        if p == ask.pid && unsafe { IsWindowVisible(h) } != 0 && unsafe { GetWindow(h, GW_OWNER) }.is_null() {
            ask.found = true;
            return 0;
        }
        1
    }
    let mut ask = Ask { pid, found: false };
    unsafe { EnumWindows(Some(each), &mut ask as *mut _ as LPARAM) };
    ask.found
}

/// This program's own main window (the visible top-level one of this process with no owner).
fn own_window() -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GW_OWNER, GetWindow, GetWindowThreadProcessId, IsWindowVisible};
    unsafe extern "system" fn find(h: HWND, l: LPARAM) -> i32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        if pid == unsafe { GetCurrentProcessId() } && unsafe { IsWindowVisible(h) } != 0 && unsafe { GetWindow(h, GW_OWNER) }.is_null() {
            unsafe { *(l as *mut Option<HWND>) = Some(h) };
            return 0;
        }
        1
    }
    let mut found: Option<HWND> = None;
    unsafe { EnumWindows(Some(find), &mut found as *mut _ as LPARAM) };
    found
}

/// Minimises the launcher through Windows, the way the taskbar does (egui's own command leaves a full-screen window that does not
/// repaint when it comes back).
pub fn minimize_own_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SW_MINIMIZE, ShowWindow};
    if let Some(h) = own_window() {
        unsafe { ShowWindow(h, SW_MINIMIZE) };
    }
}

/// Brings the minimised launcher back (found even when minimised: such a window is not "visible" to the search above).
pub fn restore_own_window() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GW_OWNER, GetWindow, GetWindowThreadProcessId, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow};
    unsafe extern "system" fn find(h: HWND, l: LPARAM) -> i32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        if pid == unsafe { GetCurrentProcessId() } && unsafe { IsIconic(h) } != 0 && unsafe { GetWindow(h, GW_OWNER) }.is_null() {
            unsafe { *(l as *mut Option<HWND>) = Some(h) };
            return 0;
        }
        1
    }
    let mut found: Option<HWND> = None;
    unsafe { EnumWindows(Some(find), &mut found as *mut _ as LPARAM) };
    if let Some(h) = found {
        unsafe {
            ShowWindow(h, SW_RESTORE);
            SetForegroundWindow(h);
        }
    }
}

/// The folder the game keeps its own files in (its copies of the drivers, the maps' last states, the AI lists you edit): the folder
/// of the program, or an `openOMSI` folder in it when the program sits in OMSI 2's own folder.
pub fn content_dir(game: &Path) -> Option<PathBuf> {
    let dir = game.parent()?;
    let has = |n: &str| std::fs::read_dir(dir).into_iter().flatten().flatten().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(n));
    Some(if has("Omsi.exe") && has("maps") { dir.join("openOMSI") } else { dir.to_path_buf() })
}
