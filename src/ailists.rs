//! The AI lists of a map (`ailists.cfg`): which cars, lorries and buses its traffic is made of. The map's own file is never touched:
//! the groups the player changes are written, in the game's own format, to `ailists_#user.cfg` in the map's folder of the game's
//! content folder, where the game reads them on top of the map's. Resetting is deleting that file.

use std::path::{Path, PathBuf};

/// The file the player's own groups are kept in, in the map's folder of the content folder.
pub const USER_FILE: &str = "ailists_#user.cfg";

#[derive(Clone, Debug, PartialEq)]
pub struct Vehicle {
    /// As the lists name it: `Vehicles\Folder\file.bus`.
    pub file: String,
    pub weight: f32,
}

/// A bus of a depot: its fleet number, plate and repaint, and the first and last day it runs (YYYYMMDD).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Entry {
    pub number: String,
    pub registration: String,
    pub paint: String,
    pub from: Option<i32>,
    pub to: Option<i32>,
}

/// A type of bus of a depot with the buses of that type.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TypGroup {
    pub file: String,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Group {
    pub name: String,
    pub hof: Option<String>,
    /// A depot: its buses (by type, with numbers and plates) instead of a weighted list of vehicles.
    pub depot: bool,
    pub vehicles: Vec<Vehicle>,
    pub typgroups: Vec<TypGroup>,
}

fn is_vehicle_line(l: &str) -> bool {
    let l = l.to_ascii_lowercase();
    l.contains(".bus") || l.contains(".ovh") || l.contains(".sco") || l.contains(".zug")
}

/// A bus of a type group: `number<TAB>plate<TAB>paint<TAB>from<TAB>to`, the empty fields at the end left out.
fn entry_of(line: &str) -> Entry {
    let mut f = line.split('\t');
    let mut next = || f.next().unwrap_or("").to_string();
    let (number, registration, paint, from, to) = (next(), next(), next(), next(), next());
    let date = |s: &str| s.trim().parse::<i32>().ok();
    Entry { number: number.trim().to_string(), registration, paint, from: date(&from), to: date(&to) }
}

fn vehicle_of(line: &str) -> Vehicle {
    match line.split_once('\t') {
        Some((f, w)) => Vehicle { file: f.trim().to_string(), weight: w.trim().replace(',', ".").parse().unwrap_or(1.0) },
        None => Vehicle { file: line.trim().to_string(), weight: 1.0 },
    }
}

/// The groups of an `ailists.cfg`: the car groups (`[aigroup_2]`) with their vehicles, and the depots' names.
pub fn parse(text: &str) -> Vec<Group> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
    let mut out: Vec<Group> = vec![];
    let mut i = 0;
    while i < lines.len() {
        match lines[i].trim().to_ascii_lowercase().as_str() {
            "[aigroup_2]" => {
                let name = lines.get(i + 1).unwrap_or(&"").trim().to_string();
                let second = lines.get(i + 2).unwrap_or(&"");
                let mut g = Group { name, ..Default::default() };
                let mut j = i + 3;
                // (a map that leaves the depot line out starts the list right there)
                if is_vehicle_line(second) {
                    g.vehicles.push(vehicle_of(second));
                } else if !second.trim().is_empty() {
                    g.hof = Some(second.trim().to_string());
                }
                while j < lines.len() && !lines[j].trim().eq_ignore_ascii_case("[end]") {
                    if !lines[j].trim().is_empty() {
                        g.vehicles.push(vehicle_of(lines[j]));
                    }
                    j += 1;
                }
                out.push(g);
                i = j;
            }
            "[aigroup_depot]" => {
                let name = lines.get(i + 1).unwrap_or(&"").trim().to_string();
                let hof = lines.get(i + 2).map(|h| h.trim().to_string()).filter(|h| !h.is_empty());
                out.push(Group { name, hof, depot: true, ..Default::default() });
                i += 3;
            }
            "[aigroup_depot_typgroup_2]" => {
                let file = lines.get(i + 1).unwrap_or(&"").trim().to_string();
                let mut tg = TypGroup { file, entries: vec![] };
                let mut j = i + 2;
                while j < lines.len() && !lines[j].trim().eq_ignore_ascii_case("[end]") {
                    if !lines[j].trim().is_empty() {
                        tg.entries.push(entry_of(lines[j]));
                    }
                    j += 1;
                }
                if let Some(d) = out.iter_mut().rev().find(|g| g.depot) {
                    d.typgroups.push(tg);
                }
                i = j;
            }
            _ => i += 1,
        }
    }
    out
}

/// The group as lines of an `ailists.cfg` (CR LF), which `parse` reads back as the same group.
pub fn to_cfg(g: &Group) -> String {
    let clean = |s: &str| s.replace(['\r', '\n', '\t'], " ");
    if g.depot {
        let mut o = format!("[aigroup_depot]\r\n{}\r\n{}\r\n", clean(&g.name), clean(g.hof.as_deref().unwrap_or("")));
        for tg in &g.typgroups {
            o.push_str(&format!("[aigroup_depot_typgroup_2]\r\n{}\r\n", clean(&tg.file)));
            for e in &tg.entries {
                let date = |d: Option<i32>| d.map(|d| d.to_string()).unwrap_or_default();
                let mut line = vec![clean(&e.number), clean(&e.registration), clean(&e.paint), date(e.from), date(e.to)];
                // (trailing empty fields are left out, as the original's files do)
                while line.len() > 1 && line.last().is_some_and(|l| l.is_empty()) {
                    line.pop();
                }
                o.push_str(&line.join("\t"));
                o.push_str("\r\n");
            }
            o.push_str("[end]\r\n");
        }
        return o;
    }
    let mut o = format!("[aigroup_2]\r\n{}\r\n{}\r\n", clean(&g.name), clean(g.hof.as_deref().unwrap_or("")));
    for v in &g.vehicles {
        o.push_str(&format!("{}\t{}\r\n", clean(&v.file), v.weight));
    }
    o.push_str("[end]\r\n");
    o
}

fn read_text(path: &Path) -> Option<String> {
    crate::omsi::read_cfg(path)
}

/// The groups of the map's own `ailists.cfg`.
pub fn load_map(omsi: &Path, folder: &str) -> Vec<Group> {
    read_text(&omsi.join("maps").join(folder).join("ailists.cfg")).map(|t| parse(&t)).unwrap_or_default()
}

pub fn user_file(content: &Path, folder: &str) -> PathBuf {
    content.join("maps").join(folder).join(USER_FILE)
}

/// The groups the map has with the player's own on top: a group of the same name takes the place of the map's, a new one is added.
pub fn effective(base: &[Group], user: Vec<Group>) -> Vec<Group> {
    let mut all = base.to_vec();
    for g in user {
        match all.iter().position(|b| b.depot == g.depot && b.name.trim().eq_ignore_ascii_case(g.name.trim())) {
            Some(i) => all[i] = g,
            None => all.push(g),
        }
    }
    all
}

pub fn load(omsi: &Path, content: Option<&Path>, folder: &str) -> (Vec<Group>, Vec<Group>) {
    let base = load_map(omsi, folder);
    let user = content.and_then(|c| read_text(&user_file(c, folder))).map(|t| parse(&t)).unwrap_or_default();
    let edited = effective(&base, user);
    (base, edited)
}

/// Writes the groups that differ from the map's own; with none, the player's file is removed.
pub fn save(content: &Path, folder: &str, base: &[Group], edited: &[Group]) -> Result<(), String> {
    let path = user_file(content, folder);
    let mut text = String::new();
    for g in edited {
        let same = base.iter().find(|b| b.depot == g.depot && b.name.trim().eq_ignore_ascii_case(g.name.trim())).is_some_and(|b| b == g);
        if !same {
            text.push_str(&to_cfg(g));
        }
    }
    if text.is_empty() {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Back to what the map gives: the player's file and line rules deleted.
pub fn reset_map(content: &Path, folder: &str) {
    let _ = std::fs::remove_file(user_file(content, folder));
    for f in rule_files(content, folder) {
        let _ = std::fs::remove_file(f);
    }
}

// --- line rules: which bus types run on a line (`car_use/*.ocu`) ---

/// What the launcher's rule files are called, so they can be told from the map's own.
const RULE_PREFIX: &str = "!openomsi-ai-lists-";

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub line: String,
    /// Only these types run (true), or most tours get one of them (false).
    pub only: bool,
    /// With `only` false: the share of tours that get one of the types, from 0 to 1.
    pub factor: f32,
    pub types: Vec<String>,
}

/// A line of the map: a timetable line file with tours.
#[derive(Clone, Debug)]
pub struct Line {
    pub name: String,
    pub tours: usize,
    /// The AI groups (depots) of its tours.
    pub groups: Vec<String>,
    /// The map has rules of its own for this line.
    pub map_rule: bool,
}

fn rules_dir(content: &Path, folder: &str) -> PathBuf {
    content.join("maps").join(folder).join("car_use")
}

fn rule_files(content: &Path, folder: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(rules_dir(content, folder)).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ocu")) && p.file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with(RULE_PREFIX))).collect();
    v.sort();
    v
}

/// The rule a car-use file holds, if it is a whole-line one of the kinds the launcher writes.
pub fn parse_rule(text: &str) -> Option<Rule> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut rule = Rule { line: String::new(), only: true, factor: 1.0, types: vec![] };
    let mut i = 0;
    let mut found = false;
    while i < lines.len() {
        match lines[i].to_ascii_lowercase().as_str() {
            "[line]" => {
                rule.line = lines[i + 1..].iter().find(|l| !l.is_empty()).unwrap_or(&"").to_string();
            }
            "[onlytypes]" | "[types_prefered]" => {
                let only = lines[i].eq_ignore_ascii_case("[onlytypes]");
                let mut j = i + 1;
                let mut factor = 1.0;
                if !only {
                    factor = lines.get(j).and_then(|v| v.replace(',', ".").parse::<f32>().ok()).unwrap_or(1.0);
                    j += 1;
                }
                let mut types = vec![];
                while j < lines.len() && !lines[j].eq_ignore_ascii_case("[end]") {
                    if !lines[j].is_empty() {
                        types.push(lines[j].to_string());
                    }
                    j += 1;
                }
                rule.only = only;
                rule.factor = factor;
                rule.types = types;
                found = true;
                i = j;
            }
            _ => {}
        }
        i += 1;
    }
    (found && !rule.line.is_empty()).then_some(rule)
}

pub fn rule_text(r: &Rule) -> String {
    let clean = |s: &str| s.replace(['\r', '\n'], " ").trim().to_string();
    let types: String = r.types.iter().map(|t| format!("{}\r\n", clean(t))).collect();
    let body = if r.only { format!("[onlytypes]\r\n{types}[end]\r\n") } else { format!("[types_prefered]\r\n{}\r\n{types}[end]\r\n", r.factor.clamp(0.0, 1.0)) };
    format!("[valid]\r\n10000101\r\n99991231\r\n\r\n[line]\r\n{}\r\n\r\n{body}", clean(&r.line))
}

pub fn load_rules(content: &Path, folder: &str) -> Vec<Rule> {
    rule_files(content, folder).iter().filter_map(|f| read_text(f)).filter_map(|t| parse_rule(&t)).collect()
}

/// Writes the rules (a rule with no types is not one) and removes the launcher's old rule files.
pub fn save_rules(content: &Path, folder: &str, rules: &[Rule]) -> Result<(), String> {
    let dir = rules_dir(content, folder);
    let valid: Vec<&Rule> = rules.iter().filter(|r| !r.line.trim().is_empty() && !r.types.is_empty()).collect();
    if !valid.is_empty() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let mut staged = vec![];
    for (n, r) in valid.iter().enumerate() {
        let to = dir.join(format!("{RULE_PREFIX}{:03}.ocu", n + 1));
        let tmp = to.with_extension("tmp");
        if let Err(e) = std::fs::write(&tmp, rule_text(r)) {
            for (t, _) in &staged {
                let _ = std::fs::remove_file(t);
            }
            return Err(e.to_string());
        }
        staged.push((tmp, to));
    }
    for old in rule_files(content, folder) {
        let _ = std::fs::remove_file(old);
    }
    for (tmp, to) in staged {
        std::fs::rename(&tmp, &to).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The lines of a map: its timetable line files (`TTData/*.ttl`) that have tours.
pub fn lines(omsi: &Path, folder: &str) -> Vec<Line> {
    let map = omsi.join("maps").join(folder);
    let rules_of_map: Vec<String> = std::fs::read_dir(map.join("car_use")).into_iter().flatten().flatten().filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("ocu"))).filter_map(|e| read_text(&e.path())).filter_map(|t| {
        let l: Vec<&str> = t.lines().map(str::trim).collect();
        let i = l.iter().position(|x| x.eq_ignore_ascii_case("[line]"))?;
        l[i + 1..].iter().find(|x| !x.is_empty()).map(|s| s.to_ascii_lowercase())
    }).collect();
    let mut out: Vec<Line> = std::fs::read_dir(map.join("TTData")).into_iter().flatten().flatten().filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("ttl"))).filter_map(|e| {
        let name = e.path().file_stem()?.to_string_lossy().into_owned();
        let text = read_text(&e.path())?;
        let l: Vec<&str> = text.lines().map(str::trim).collect();
        let mut tours = 0;
        let mut groups: Vec<String> = vec![];
        for (i, x) in l.iter().enumerate() {
            if x.eq_ignore_ascii_case("[newtour]") {
                tours += 1;
                if let Some(g) = l.get(i + 2).filter(|g| !g.is_empty()) {
                    if !groups.iter().any(|x| x.eq_ignore_ascii_case(g)) {
                        groups.push(g.to_string());
                    }
                }
            }
        }
        (tours > 0).then(|| Line { map_rule: rules_of_map.contains(&name.to_ascii_lowercase()), name, tours, groups })
    }).collect();
    out.sort_by(|a, b| match (a.name.parse::<u32>(), b.name.parse::<u32>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    out
}

/// A vehicle of the library of the installed content.
#[derive(Clone, Debug)]
pub struct LibVehicle {
    /// `Vehicles\Folder\file.bus`, as a list names it.
    pub file: String,
    pub name: String,
    pub folder: String,
}

/// A readable name from a file name: `ai_vw_golf_2` becomes `ai vw golf 2`.
pub fn pretty(stem: &str) -> String {
    stem.replace('_', " ")
}

/// Every `.bus` and `.ovh` in a folder of `Vehicles` of the installation or the content folder (one level of sub-folders too).
pub fn library(omsi: &Path, content: Option<&Path>) -> Vec<LibVehicle> {
    let mut out: Vec<LibVehicle> = vec![];
    let mut seen = std::collections::HashSet::new();
    let roots: Vec<PathBuf> = content.into_iter().chain(Some(omsi)).map(|r| r.join("Vehicles")).collect();
    for root in roots {
        for dir in std::fs::read_dir(&root).into_iter().flatten().flatten() {
            if !dir.path().is_dir() {
                continue;
            }
            let folder = dir.file_name().to_string_lossy().into_owned();
            let mut stack = vec![(dir.path(), String::new(), 2u32)];
            while let Some((d, rel, depth)) = stack.pop() {
                for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                    let p = e.path();
                    let name = e.file_name().to_string_lossy().into_owned();
                    if p.is_dir() {
                        if depth > 0 && !name.starts_with('.') && !matches!(name.to_ascii_lowercase().as_str(), "model" | "texture" | "sound" | "script") {
                            stack.push((p, if rel.is_empty() { name } else { format!("{rel}\\{name}") }, depth - 1));
                        }
                    } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bus") || x.eq_ignore_ascii_case("ovh")) {
                        let file = if rel.is_empty() { format!("Vehicles\\{folder}\\{name}") } else { format!("Vehicles\\{folder}\\{rel}\\{name}") };
                        if seen.insert(file.to_ascii_lowercase()) {
                            let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                            out.push(LibVehicle { file, name: pretty(&stem), folder: folder.clone() });
                        }
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.file.cmp(&b.file)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_are_written_and_read_back() {
        for r in [
            Rule { line: "92".into(), only: true, factor: 1.0, types: vec!["Vehicles\\MAN_SD200\\MAN_SD77.bus".into(), "Vehicles\\X\\y.bus".into()] },
            Rule { line: "13".into(), only: false, factor: 0.7, types: vec!["Vehicles\\A\\b.bus".into()] },
        ] {
            assert_eq!(parse_rule(&rule_text(&r)), Some(r));
        }
    }

    /// Berlin-Spandau's own rule files are not whole-line ones the launcher reads back, but its lines are found with their depots.
    #[test]
    fn the_lines_of_a_map_are_found() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        let l = lines(&omsi, "Berlin-Spandau");
        if l.is_empty() {
            return;
        }
        assert!(l.iter().any(|x| x.name == "13" && x.groups.iter().any(|g| g == "Hof Spandau")));
        assert!(l.iter().any(|x| x.name == "92" && x.map_rule));
    }

    #[test]
    fn a_group_is_written_and_read_back_the_same() {
        let g = Group { name: "Cars".into(), hof: None, depot: false, vehicles: vec![Vehicle { file: "Vehicles\\a\\b.bus".into(), weight: 7.0 }, Vehicle { file: "Vehicles\\c\\d.ovh".into(), weight: 0.5 }], typgroups: vec![] };
        assert_eq!(parse(&to_cfg(&g)), vec![g]);
    }

    #[test]
    fn only_changed_groups_are_saved_and_none_removes_the_file() {
        let dir = std::env::temp_dir().join(format!("classiclauncher-ai-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let a = Group { name: "A".into(), vehicles: vec![Vehicle { file: "x".into(), weight: 1.0 }], ..Default::default() };
        let b = Group { name: "B".into(), vehicles: vec![Vehicle { file: "y".into(), weight: 2.0 }], ..Default::default() };
        let base = vec![a.clone(), b.clone()];
        let mut edited = base.clone();
        edited[1].vehicles[0].weight = 9.0;
        save(&dir, "M", &base, &edited).unwrap();
        let written = parse(&std::fs::read_to_string(user_file(&dir, "M")).unwrap());
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].name, "B");
        assert_eq!(effective(&base, written)[1].vehicles[0].weight, 9.0);
        save(&dir, "M", &base, &base).unwrap();
        assert!(!user_file(&dir, "M").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Default content: Grundorf's NormalCars group has its ten vehicles with their weights.
    #[test]
    fn reads_the_traffic_of_grundorf() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        if !omsi.join("maps").join("Grundorf").join("ailists.cfg").is_file() {
            return;
        }
        let groups = load_map(&omsi, "Grundorf");
        let Some(cars) = groups.iter().find(|g| g.name == "NormalCars") else { return };
        assert!(!cars.vehicles.is_empty() && cars.vehicles.iter().all(|v| v.weight > 0.0 && v.file.to_lowercase().starts_with("vehicles")));
    }
}

#[cfg(test)]
mod depot_tests {
    use super::*;

    #[test]
    fn a_depot_with_its_types_is_written_and_read_back_the_same() {
        let g = Group {
            name: "Busses".into(),
            hof: Some("Grundorf".into()),
            depot: true,
            vehicles: vec![],
            typgroups: vec![TypGroup {
                file: "Vehicles\\a\\b.bus".into(),
                entries: vec![Entry { number: "2530".into(), registration: "X 1".into(), paint: "red".into(), from: Some(19900101), to: None }, Entry { number: "2535".into(), ..Default::default() }],
            }],
        };
        assert_eq!(parse(&to_cfg(&g)), vec![g]);
    }

    /// Default content: Grundorf's depot holds the type MAN_GN92_main with fleet numbers and plates.
    #[test]
    fn reads_the_depot_of_grundorf() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        if !omsi.join("maps").join("Grundorf").join("ailists.cfg").is_file() {
            return;
        }
        let groups = load_map(&omsi, "Grundorf");
        let Some(depot) = groups.iter().find(|g| g.depot) else { return };
        assert!(!depot.typgroups.is_empty() && depot.typgroups.iter().all(|t| !t.file.is_empty() && !t.entries.is_empty()));
        assert!(depot.typgroups[0].entries.iter().all(|e| !e.number.is_empty()));
    }
}

/// The repaints a vehicle's model offers, as OMSI finds them: the vehicle file names its model, the model's `[CTC]` blocks name folders,
/// and every `*.cti` file of those folders holds `[item]` entries, each with the repaint's name on its next line.
pub fn paints(omsi: &Path, content: Option<&Path>, file: &str) -> Vec<String> {
    let rel = file.replace('\\', "/");
    let mut names: Vec<String> = vec![];
    // (the content folder's copy and the installation's both count: a mod can add repaints to a vehicle the game has)
    let roots: Vec<PathBuf> = content.into_iter().chain(Some(omsi)).map(Path::to_path_buf).collect();
    for root in roots {
        let path = root.join(&rel);
        let Some(dir) = path.parent() else { continue };
        let Some(text) = read_text(&path) else { continue };
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let Some(i) = lines.iter().position(|l| l.eq_ignore_ascii_case("[model]")) else { continue };
        let Some(model) = lines[i + 1..].iter().find(|l| !l.is_empty()) else { continue };
        let model_path = dir.join(model.replace('\\', "/"));
        let Some(model_text) = read_text(&model_path) else { continue };
        let ml: Vec<&str> = model_text.lines().map(str::trim).collect();
        let mut folders: Vec<String> = vec![];
        for (k, l) in ml.iter().enumerate() {
            if l.eq_ignore_ascii_case("[CTC]") {
                if let Some(p) = ml.get(k + 2).filter(|p| !p.is_empty()) {
                    folders.push(p.replace('\\', "/"));
                }
            }
        }
        for f in folders {
            let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join(&f)).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("cti"))).collect();
            files.sort();
            for cti in files {
                let Some(t) = read_text(&cti) else { continue };
                let l: Vec<&str> = t.lines().map(str::trim).collect();
                for (k, x) in l.iter().enumerate() {
                    if x.eq_ignore_ascii_case("[item]") {
                        if let Some(n) = l.get(k + 1).filter(|n| !n.is_empty()) {
                            if !names.iter().any(|o| o.eq_ignore_ascii_case(n)) {
                                names.push(n.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    names
}

#[cfg(test)]
mod paint_tests {
    use super::*;

    /// A car of the default content: its repaints are the colours of its texture folder.
    #[test]
    fn a_vehicles_repaints_are_listed() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        if !omsi.join("Vehicles/407/407.ovh").is_file() {
            return;
        }
        let p = paints(&omsi, None, "Vehicles\\407\\407.ovh");
        assert!(p.iter().any(|n| n == "Black") && p.iter().any(|n| n == "Wine"), "{p:?}");
    }
}
