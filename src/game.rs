//! Starting openOMSI: where it is, and the command line for what the launcher chose. The game takes everything as arguments
//! (`openomsi --help`), so nothing here touches OMSI's windows.

use crate::omsi::MapInfo;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `openomsi.exe`: the saved path, next to the launcher, `OPENOMSI_BIN`, then the usual install folders.
pub fn find_game(saved: Option<&Path>) -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = vec![];
    c.extend(saved.map(Path::to_path_buf));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            c.push(d.join("openomsi.exe"));
        }
    }
    c.extend(std::env::var_os("OPENOMSI_BIN").map(PathBuf::from));
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        for name in ["openOMSI", "OpenOMSI", "openomsi"] {
            c.push(local.join("Programs").join(name).join("openomsi.exe"));
        }
    }
    for pf in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(p) = std::env::var_os(pf).map(PathBuf::from) {
            for name in ["openOMSI", "OpenOMSI"] {
                c.push(p.join(name).join("openomsi.exe"));
            }
        }
    }
    c.into_iter().find(|p| p.is_file())
}

#[derive(Clone)]
pub enum Start {
    /// A saved situation (`Situations\*.osn`): it holds the map, vehicle, date and time.
    Situation(String),
    /// The map with nobody in a vehicle: the player starts on foot at a start place and puts a vehicle down from the game.
    Free { place: Option<usize>, view: Option<crate::omsi::View> },
}

#[derive(Clone, Debug, Default)]
pub struct BusChoice {
    pub file: String,
    pub paint: String,
    pub plate: String,
    pub number: String,
}

#[derive(Clone)]
pub struct Plan {
    pub omsi: PathBuf,
    pub map: MapInfo,
    pub start: Start,
    pub driver: Option<String>,
    /// `HH:MM` and `YYYY-MM-DD`; `None` leaves the game's own (or the situation's).
    pub time: Option<String>,
    pub date: Option<String>,
    pub traffic: u32,
    pub passengers: bool,
    pub schedule: bool,
    pub no_plugins: bool,
    /// A bus to start in; without one the player starts on foot.
    pub bus: Option<BusChoice>,
}

impl Plan {
    pub fn args(&self) -> Vec<String> {
        let mut a: Vec<String> = vec!["--root".into(), self.omsi.display().to_string(), "--no-menu".into()];
        if self.no_plugins {
            a.push("--no-plugins".into());
        }
        match &self.start {
            Start::Situation(file) => a.extend(["--situation".into(), file.clone()]),
            Start::Free { place, view } => {
                a.extend(["--map".into(), self.map.arg()]);
                if self.bus.is_none() {
                    a.push("--on-foot".into());
                }
                if let Some(b) = &self.bus {
                    a.extend(["--bus".into(), b.file.clone()]);
                    for (flag, v) in [("--paint", &b.paint), ("--plate", &b.plate), ("--number", &b.number)] {
                        if !v.trim().is_empty() {
                            a.extend([flag.to_string(), v.trim().to_string()]);
                        }
                    }
                }
                if let Some(p) = place {
                    a.extend(["--entry".into(), p.to_string()]);
                }
                // without a vehicle the game stands the player where its camera is, whatever `--entry` says: so the camera goes to the place
                if let (Some(v), None) = (view, &self.bus) {
                    // (joined with =: a negative x would be taken for an option of its own)
                    a.push(format!("--cam={:.2},{:.2},{:.2},{:.1},0", v.x, v.y, v.z + 1.7, v.heading));
                }
                if let Some(t) = &self.time {
                    a.extend(["--time".into(), t.clone()]);
                }
                if let Some(d) = &self.date {
                    a.extend(["--date".into(), d.clone()]);
                }
                if self.schedule {
                    a.push("--schedule".into());
                }
            }
        }
        a.extend(["--traffic".into(), self.traffic.to_string()]);
        if self.passengers {
            a.push("--passengers".into());
        }
        if let Some(d) = &self.driver {
            a.extend(["--driver".into(), format!("Drivers/{d}.odr")]);
        }
        a
    }
}

pub fn command_text(exe: &Path, args: &[String]) -> String {
    let q = |s: &str| if s.contains(' ') { format!("\"{s}\"") } else { s.to_string() };
    std::iter::once(q(&exe.display().to_string())).chain(args.iter().map(|a| q(a))).collect::<Vec<_>>().join(" ")
}

/// Where the game's error output goes, so a game that ends at once can say why.
pub fn stderr_file() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("ClassicLauncher").join("game-stderr.txt")
}

pub fn launch(exe: &Path, args: &[String]) -> Result<std::process::Child, String> {
    let file = stderr_file();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut cmd = Command::new(exe);
    cmd.args(args).current_dir(exe.parent().unwrap_or(Path::new(".")));
    if let Ok(f) = std::fs::File::create(&file) {
        cmd.stderr(f);
    }
    cmd.spawn().map_err(|e| e.to_string())
}

/// The last `error` line the game wrote when it ended.
pub fn last_error() -> Option<String> {
    let text = std::fs::read_to_string(stderr_file()).ok()?;
    text.lines().rev().map(str::trim).find(|l| !l.is_empty() && (l.to_lowercase().contains("error") || l.to_lowercase().contains("panic"))).map(|l| l.chars().take(160).collect())
}

/// `--tutorial N` starts OMSI's tutorial 1 to 4.
pub fn tutorial_args(omsi: &Path, n: u8) -> Vec<String> {
    vec!["--root".into(), omsi.display().to_string(), "--no-menu".into(), "--tutorial".into(), n.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(start: Start) -> Plan {
        Plan {
            omsi: PathBuf::from("D:/OMSI 2"),
            map: MapInfo { folder: "Grundorf".into(), name: "Grundorf".into(), description: String::new(), picture: None },
            start,
            driver: Some("Test".into()),
            time: Some("09:30".into()),
            date: Some("1989-05-30".into()),
            traffic: 20,
            passengers: true,
            schedule: true,
            no_plugins: false,
            bus: None,
        }
    }

    #[test]
    fn a_free_start_names_the_map_place_and_time_and_starts_on_foot() {
        let a = plan(Start::Free { place: Some(3), view: Some(crate::omsi::View { x: 100.5, y: 200.25, z: 33.0, heading: 90.0 }) }).args();
        let joined = a.join(" ");
        assert!(joined.contains("--map maps/Grundorf/global.cfg --on-foot --entry 3 --cam=100.50,200.25,34.70,90.0,0 --time 09:30 --date 1989-05-30"), "{joined}");
        assert!(joined.ends_with("--traffic 20 --passengers --driver Drivers/Test.odr"));
        assert!(!a.contains(&"--bus".to_string()));
    }

    #[test]
    fn a_negative_place_is_one_argument() {
        let a = plan(Start::Free { place: Some(1), view: Some(crate::omsi::View { x: -2217.8, y: -18126.4, z: 0.0, heading: 184.5 }) }).args();
        assert!(a.contains(&"--cam=-2217.80,-18126.40,1.70,184.5,0".to_string()), "{a:?}");
    }

    #[test]
    fn a_situation_brings_its_own_map_and_time() {
        let a = plan(Start::Situation("Situations/a.osn".into())).args();
        assert!(a.contains(&"Situations/a.osn".to_string()));
        assert!(!a.contains(&"--map".to_string()) && !a.contains(&"--time".to_string()));
    }
}

#[cfg(test)]
mod bus_tests {
    use super::*;

    #[test]
    fn a_free_start_with_a_bus_names_it_and_does_not_start_on_foot() {
        let plan = Plan {
            omsi: PathBuf::from("D:/OMSI 2"),
            map: MapInfo { folder: "Grundorf".into(), name: "Grundorf".into(), description: String::new(), picture: None },
            start: Start::Free { place: Some(2), view: Some(crate::omsi::View { x: 1.0, y: 2.0, z: 3.0, heading: 0.0 }) },
            driver: None,
            time: None,
            date: None,
            traffic: 10,
            passengers: false,
            schedule: false,
            no_plugins: false,
            bus: Some(BusChoice { file: "Vehicles/MAN_SD202/SD202.bus".into(), paint: "Red".into(), plate: String::new(), number: "12".into() }),
        };
        let j = plan.args().join(" ");
        assert!(j.contains("--bus Vehicles/MAN_SD202/SD202.bus --paint Red --number 12 --entry 2"), "{j}");
        assert!(!j.contains("--on-foot") && !j.contains("--cam"), "{j}");
    }
}

/// The version the program says it is (`openomsi --version` answers `openomsi 0.3.46`).
pub fn version_of(exe: &Path) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let out = Command::new(exe).arg("--version").creation_flags(0x0800_0000).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().rev().find_map(|l| {
        let v = l.trim().strip_prefix("openomsi ")?.trim();
        (!v.is_empty() && v.chars().next().is_some_and(|c| c.is_ascii_digit())).then(|| v.to_string())
    })
}

#[cfg(test)]
mod version_tests {
    /// The version line of the program, as the installed one prints it.
    #[test]
    fn the_version_is_read_from_the_program() {
        let Some(exe) = super::find_game(None) else { return };
        let v = super::version_of(&exe);
        assert!(v.as_deref().is_some_and(|v| v.split('.').count() >= 2), "{v:?}");
    }
}
