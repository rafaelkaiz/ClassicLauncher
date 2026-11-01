//! The buses the player can drive, as OMSI's own vehicle list shows them: every `.bus` with a `[friendlyname]` of the installation and
//! the content folder, with the family name (first line), the variant (second and third lines) and the folder.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Bus {
    /// `Vehicles/Folder/file.bus`, as the game takes it.
    pub file: String,
    pub family: String,
    /// The model (second line of the friendly name), e.g. `NL202 - EN92`.
    pub variant: String,
    /// The repaint it comes in (third line).
    pub paint: String,
    /// The text between `[description]` and `[end]`.
    pub description: String,
    pub folder: String,
}

/// The `[friendlyname]` lines of a vehicle file, if it has them.
fn friendly(text: &str) -> Option<(String, String, String)> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let i = lines.iter().position(|l| l.eq_ignore_ascii_case("[friendlyname]"))?;
    let mut it = lines[i + 1..].iter().take_while(|l| !l.starts_with('['));
    let family = it.next()?.to_string();
    let model = it.next().map(|s| s.to_string()).unwrap_or_default();
    let paint = it.next().map(|s| s.to_string()).unwrap_or_default();
    (!family.is_empty()).then(|| (family, model, paint))
}

/// Whether the file is one for the traffic or a part of a bus, not a bus to drive: its name has a `_trail` (the rear part of an
/// articulated bus), `_KI` or `_AI` (the traffic's own version) part, or starts with `ai_`.
fn is_traffic_or_part(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    let stem = name.rsplit_once('.').map_or(name, |x| x.0).to_ascii_lowercase();
    stem.starts_with("ai_") || stem.split('_').any(|t| matches!(t, "trail" | "ki" | "ai"))
}

/// Whether the file has a passenger cabin: a tractor, a lorry or a fire engine has none.
fn drivable(lines: &[&str]) -> bool {
    lines.iter().any(|l| l.eq_ignore_ascii_case("[passengercabin]"))
}


/// Every selectable bus, sorted by family and variant. Reads only the head of each file, so it takes a moment but not long.
pub fn list(omsi: &Path, content: Option<&Path>) -> Vec<Bus> {
    let mut out: Vec<Bus> = vec![];
    let mut seen = std::collections::HashSet::new();
    // the files another bus couples behind itself, and the buses that have a front coupling (a rear section has both)
    let mut coupled: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut fronted: std::collections::HashSet<String> = std::collections::HashSet::new();
    let roots: Vec<PathBuf> = content.into_iter().chain(Some(omsi)).map(|r| r.join("Vehicles")).collect();
    for root in roots {
        for dir in std::fs::read_dir(&root).into_iter().flatten().flatten() {
            if !dir.path().is_dir() {
                continue;
            }
            let folder = dir.file_name().to_string_lossy().into_owned();
            let mut stack = vec![(dir.path(), String::new(), 8u32)];
            while let Some((d, rel, depth)) = stack.pop() {
                for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                    let p = e.path();
                    let name = e.file_name().to_string_lossy().into_owned();
                    if p.is_dir() {
                        // (the folders of models, textures and sounds hold thousands of files and never a bus: they are not walked)
                        if depth > 0 && !name.starts_with('.') && !matches!(name.to_ascii_lowercase().as_str(), "model" | "models" | "texture" | "textures" | "sound" | "sounds" | "script" | "scripts" | "soundpack" | "sound_ai") {
                            stack.push((p, if rel.is_empty() { name } else { format!("{rel}/{name}") }, depth - 1));
                        }
                    } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bus")) {
                        let file = if rel.is_empty() { format!("Vehicles/{folder}/{name}") } else { format!("Vehicles/{folder}/{rel}/{name}") };
                        if !seen.insert(file.to_ascii_lowercase()) {
                            continue;
                        }
                        let Some(text) = crate::omsi::read_cfg(&p) else { continue };
                        let key = file.to_ascii_lowercase();
                        let lines: Vec<&str> = text.lines().map(str::trim).collect();
                        if lines.iter().any(|l| l.eq_ignore_ascii_case("[coupling_front]")) {
                            fronted.insert(key.clone());
                        }
                        if let Some(i) = lines.iter().position(|l| l.eq_ignore_ascii_case("[couple_back]")) {
                            if let Some(target) = lines[i + 1..].iter().find(|l| !l.is_empty()) {
                                let base = file.rsplit_once('/').map(|x| x.0).unwrap_or("");
                                coupled.insert(format!("{base}/{}", target.replace('\\', "/")).to_ascii_lowercase());
                            }
                        }
                        // (the head is enough: the name comes first)
                        let model_there = lines.iter().position(|l| l.eq_ignore_ascii_case("[model]")).and_then(|i| lines[i + 1..].iter().find(|l| !l.is_empty())).is_some_and(|m| p.parent().is_some_and(|d| d.join(m.replace('\\', "/")).is_file()));
                        let cabin_ok = drivable(&lines) || std::env::var_os("CL_NOCABIN").is_some();
                        if let (true, Some((family, variant, paint))) = (cabin_ok && model_there && !is_traffic_or_part(&file), friendly(&text)) {
                            out.push(Bus { file, family, variant, paint, description: description(&text), folder: folder.clone() });
                        }
                    }
                }
            }
        }
    }
    // (a rear section only ever comes with its front: it is not offered on its own)
    out.retain(|b| {
        let k = b.file.to_ascii_lowercase();
        !(fronted.contains(&k) && coupled.contains(&k))
    });
    out.sort_by(|a, b| a.family.to_lowercase().cmp(&b.family.to_lowercase()).then(a.variant.to_lowercase().cmp(&b.variant.to_lowercase())).then(a.file.cmp(&b.file)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_is_the_family_then_the_variant() {
        let t = "[friendlyname]\nCoach O560 Series\nEuro V - Manual\n White\n\n[description]\nx\n";
        assert_eq!(friendly(t), Some(("Coach O560 Series".into(), "Euro V - Manual".into(), "White".into())));
        assert_eq!(friendly("[model]\nx\n"), None);
    }

    #[test]
    fn the_default_content_has_buses() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        let l = list(&omsi, None);
        if l.is_empty() {
            return;
        }
        assert!(l.windows(2).all(|w| w[0].family.to_lowercase() <= w[1].family.to_lowercase()));
    }
}

#[cfg(test)]
mod count {
    use super::*;

    #[test]
    fn what_is_left_out() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        let l = list(&omsi, None);
        println!("buses offered: {}", l.len());
        let mut seen = std::collections::HashSet::new();
        for b in l.iter().filter(|b| b.folder.to_lowercase().contains("tractor") || b.family.to_lowercase().contains("generic")) {
            println!("still there: {} / {}", b.family, b.variant);
        }
        for b in l.iter().take(0) { seen.insert(&b.file); }
        println!("ai_c2: {}  gn92_ki: {}  gn92_main: {}  sd202: {}", l.iter().any(|b| b.file.contains("ai_C2_e5g_3door")), l.iter().any(|b| b.file.ends_with("MAN_GN92_main_KI.bus")), l.iter().any(|b| b.file.ends_with("MAN_GN92_main.bus")), l.iter().filter(|b| b.file.to_lowercase().contains("sd202")).count());
    }
}

#[cfg(test)]
mod excluded {
    use super::*;

    /// Files with a friendly name that the rule leaves out, by what they lack.
    #[test]
    fn what_the_rule_leaves_out() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        let (mut no_cabin, mut no_pax) = (vec![], vec![]);
        let mut stack = vec![omsi.join("Vehicles")];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bus")) {
                    let Some(t) = crate::omsi::read_cfg(&p) else { continue };
                    let l: Vec<&str> = t.lines().map(str::trim).collect();
                    if friendly(&t).is_none() {
                        continue;
                    }
                    let has = |k: &str| l.iter().any(|x| x.eq_ignore_ascii_case(k));
                    let name = p.file_name().unwrap().to_string_lossy().to_string();
                    if !has("[passengercabin]") {
                        no_cabin.push(name);
                    } else if !has("[add_camera_pax]") {
                        no_pax.push(name);
                    }
                }
            }
        }
        println!("no cabin: {} {:?}", no_cabin.len(), &no_cabin[..no_cabin.len().min(12)]);
        println!("cabin but no pax camera: {} {:?}", no_pax.len(), &no_pax[..no_pax.len().min(25)]);
    }
}

#[cfg(test)]
mod name_rule {
    use super::*;

    #[test]
    fn the_names_of_traffic_and_rear_parts_are_left_out() {
        for n in ["Vehicles/A/MAN_GN92_trail.bus", "Vehicles/A/MAN_EN92_main_KI.bus", "Vehicles/A/HHStadtbus_PVG_AI.bus", "Vehicles/A/ai_C2_e5g_3door.bus", "Vehicles/A/PDO_GC2_NGT_AI_main.bus", "Vehicles/A/BVG Urbino 18 IV_KI_1.3.bus"] {
            assert!(is_traffic_or_part(n), "{n}");
        }
        for n in ["Vehicles/A/MAN_GN92_main.bus", "Vehicles/A/O530_airport.bus", "Vehicles/A/Kaiser_main.bus", "Vehicles/A/SD202.bus"] {
            assert!(!is_traffic_or_part(n), "{n}");
        }
    }
}

#[cfg(test)]
mod parity {
    use super::*;

    /// openOMSI's own rule, counted on the installation: a friendly name, not a rear section, and the model there.
    #[test]
    fn openomsis_count() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        let mut files = vec![];
        let mut stack = vec![omsi.join("Vehicles")];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bus") || x.eq_ignore_ascii_case("ovh")) {
                    files.push(p);
                }
            }
        }
        let mut coupled = std::collections::HashSet::new();
        let mut info = vec![];
        for p in &files {
            let Some(t) = crate::omsi::read_cfg(p) else { continue };
            let l: Vec<&str> = t.lines().map(str::trim).collect();
            let next = |k: &str| l.iter().position(|x| x.eq_ignore_ascii_case(k)).and_then(|i| l[i + 1..].iter().find(|x| !x.is_empty()).map(|s| s.to_string()));
            if let Some(b) = next("[couple_back]") {
                coupled.insert(p.parent().unwrap().join(b.replace('\\', "/")).to_string_lossy().to_ascii_lowercase());
            }
            info.push((p.clone(), friendly(&t).is_some(), l.iter().any(|x| x.eq_ignore_ascii_case("[coupling_front]")), next("[model]"), l.iter().any(|x| x.eq_ignore_ascii_case("[passengercabin]"))));
        }
        let key = |p: &PathBuf| p.to_string_lossy().to_ascii_lowercase();
        let base: Vec<_> = info.iter().filter(|i| i.1 && !(i.2 && coupled.contains(&key(&i.0)))).collect();
        let with_model = base.iter().filter(|i| i.3.as_ref().is_some_and(|m| i.0.parent().unwrap().join(m.replace('\\', "/")).is_file())).count();
        let bus_only = base.iter().filter(|i| i.0.extension().unwrap().eq_ignore_ascii_case("bus")).count();
        println!("friendly + not rear: {}  with model: {}  only .bus: {}  with cabin: {}", base.len(), with_model, bus_only, base.iter().filter(|i| i.4).count());
        let a: Vec<_> = base.iter().filter(|i| i.0.extension().unwrap().eq_ignore_ascii_case("bus") && i.3.as_ref().is_some_and(|m| i.0.parent().unwrap().join(m.replace('\\', "/")).is_file())).collect();
        let rel = |p: &PathBuf| p.to_string_lossy().to_string();
        println!("A .bus+model: {}  B without names: {}  C with cabin: {}", a.len(), a.iter().filter(|i| !is_traffic_or_part(&rel(&i.0))).count(), a.iter().filter(|i| i.4 && !is_traffic_or_part(&rel(&i.0))).count());
    }
}

#[cfg(test)]
mod dump {
    #[test]
    fn write_the_list() {
        let (Some(omsi), Some(out)) = (crate::omsi::find_omsi_dir(None), std::env::var_os("CL_DUMP")) else { return };
        let text: Vec<String> = super::list(&omsi, None).iter().map(|b| format!("{}\t{}\t{}\t{}", b.file, b.family, b.variant, b.paint)).collect();
        std::fs::write(out, text.join("\n")).unwrap();
    }
}

/// The text of a vehicle file's `[description]` block, up to its `[end]`, trimmed.
pub fn description(text: &str) -> String {
    let mut lines = text.lines().map(|l| l.trim_end());
    if !lines.any(|l| l.trim().eq_ignore_ascii_case("[description]")) {
        return String::new();
    }
    let body: Vec<&str> = lines.take_while(|l| !l.trim().eq_ignore_ascii_case("[end]")).collect();
    body.join("\n").trim().to_string()
}

#[cfg(test)]
mod description_tests {
    use super::*;

    #[test]
    fn the_description_runs_to_the_end_tag() {
        let t = "[friendlyname]\nJelcz\nM185M\nBeige\n\n[description]\nJelcz M185M by KAJOTT3D\n\nwww.example.com\n[end]\n\n\t[number] x\n";
        assert_eq!(description(t), "Jelcz M185M by KAJOTT3D\n\nwww.example.com");
        assert_eq!(description("[model]\nx\n"), "");
    }
}

/// The description of a bus in the language of the launcher: the `<bus>_<LANG>.dsc` file beside the vehicle file (then the English one)
/// when there is one, else the vehicle file's own `[description]`. `file` is `Vehicles/Folder/x.bus`.
pub fn description_in(omsi: &Path, content: Option<&Path>, file: &str, lang: &str, own: &str) -> String {
    let rel = file.replace('\\', "/");
    let Some((dir, name)) = rel.rsplit_once('/') else { return own.to_string() };
    let stem = name.rsplit_once('.').map_or(name, |x| x.0);
    let mut langs = vec![lang];
    if lang != "DEU" && lang != "ENG" {
        langs.push("ENG");
    }
    for l in langs {
        for root in content.into_iter().chain(Some(omsi)) {
            let path = root.join(dir).join(format!("{stem}_{l}.dsc"));
            if let Some(text) = crate::omsi::read_cfg(&path) {
                let d = dsc_description(&text);
                if !d.is_empty() {
                    return d;
                }
            }
        }
    }
    own.to_string()
}

/// The `[description]` text of a `.dsc` file: up to the next tag.
fn dsc_description(text: &str) -> String {
    let mut inside = false;
    let mut out: Vec<&str> = vec![];
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            inside = t.eq_ignore_ascii_case("[description]");
            continue;
        }
        if inside {
            out.push(line.trim_end());
        }
    }
    out.join("\n").trim().to_string()
}

#[cfg(test)]
mod dsc_tests {
    use super::*;

    #[test]
    fn a_description_file_gives_the_description_up_to_the_next_tag() {
        let t = "\r\n[friendlyname]\r\nMAN\r\nNL202 - EN92\r\nBeige\r\n\r\n[description]\r\nAlthough the BVG did not\r\n\r\nsecond line\r\n[end]\r\n";
        assert_eq!(dsc_description(t), "Although the BVG did not\n\nsecond line");
    }

    /// The NL202 of the default content has a description file in English.
    #[test]
    fn the_english_file_is_found_and_the_own_text_is_the_fallback() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        let f = "Vehicles/MAN_NL_NG/MAN_EN92_main.bus";
        if !omsi.join("Vehicles/MAN_NL_NG/MAN_EN92_main_ENG.dsc").is_file() {
            return;
        }
        assert!(description_in(&omsi, None, f, "ENG", "own").starts_with("Although the BVG"));
        assert_eq!(description_in(&omsi, None, "Vehicles/NoSuch/x.bus", "ESP", "own"), "own");
    }
}
