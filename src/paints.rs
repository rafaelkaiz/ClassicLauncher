//! The repaints of a vehicle, for the bus picker: as OMSI finds them, from its model.

use std::path::{Path, PathBuf};

fn read_text(path: &Path) -> Option<String> {
    crate::omsi::read_cfg(path)
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
