//! The drivers of OMSI 2 (`Drivers\*.odr`, read-only) and what the staff-file window shows of each.

use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq)]
pub struct Driver {
    pub name: String,
    pub female: bool,
    /// `yyyymmdd`
    pub birth: String,
    pub path: PathBuf,
    /// File name without extension: what `--driver Drivers/<file>.odr` takes.
    pub file: String,
    /// Distance driven, in hectometres (65 is 6.5 km).
    pub hektometers: u64,
    pub stops: u64,
    /// Every copy of the file: the game keeps its own in its content folder, with the runs it has added; the OMSI 2 one stays as it was.
    /// `path` (the first copy) is the one that counts: the game's when there is one.
    pub copies: Vec<PathBuf>,
}

fn read_utf16(path: &Path) -> Option<String> {
    let b = std::fs::read(path).ok()?;
    let b = b.strip_prefix(&[0xFF, 0xFE]).unwrap_or(&b);
    let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    Some(String::from_utf16_lossy(&u))
}

fn tag_line(lines: &[&str], tag: &str) -> Option<usize> {
    lines.iter().position(|l| l.trim() == tag)
}

pub fn read_driver(path: &Path) -> Option<Driver> {
    let text = read_utf16(path)?;
    let lines: Vec<&str> = text.lines().collect();
    let i = tag_line(&lines, "[ident]")?;
    let sex = lines.get(i + 2)?.trim();
    let num = |tag: &str| tag_line(&lines, tag).and_then(|p| lines.get(p + 1)?.trim().parse().ok()).unwrap_or(0);
    Some(Driver {
        name: lines.get(i + 1)?.trim().to_string(),
        female: sex.eq_ignore_ascii_case("W") || sex.eq_ignore_ascii_case("F"),
        birth: lines.get(i + 3)?.trim().to_string(),
        path: path.to_path_buf(),
        file: path.file_stem()?.to_string_lossy().into_owned(),
        hektometers: num("[hektom]"),
        stops: num("[busstops]"),
        copies: vec![path.to_path_buf()],
    })
}

pub fn list_drivers(omsi: &Path, content: Option<&Path>) -> Vec<Driver> {
    let in_dir = |dir: PathBuf| -> Vec<PathBuf> {
        std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("odr"))).collect()
    };
    let mut v: Vec<Driver> = vec![];
    let mut files = in_dir(omsi.join("Drivers"));
    if let Some(c) = content {
        files.extend(in_dir(c.join("Drivers")));
    }
    for p in files {
        let Some(stem) = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()) else { continue };
        match v.iter_mut().find(|d| d.file.to_lowercase() == stem) {
            // the game's copy comes first: it has the runs
            Some(d) => {
                if content.is_some_and(|c| p.starts_with(c)) {
                    if let Some(r) = read_driver(&p) {
                        let others = std::mem::take(&mut d.copies);
                        *d = r;
                        d.copies.extend(others);
                    }
                } else {
                    d.copies.push(p);
                }
            }
            None => {
                if let Some(r) = read_driver(&p) {
                    v.push(r);
                }
            }
        }
    }
    v.sort_by_key(|d| d.name.to_lowercase());
    v
}

/// Everything OMSI keeps in a driver's file, for the staff-file window.
#[derive(Default)]
pub struct Record {
    pub hired: String,
    pub stops: u64,
    pub late: u64,
    pub early: u64,
    pub hektometers: u64,
    /// accidents, hit and runs, heavy hit and runs, injured pedestrians
    pub crashes: [u64; 4],
    pub tickets_sold: u64,
    pub cash: f64,
    pub rating: [i64; 5],
    /// (bus, hectometres driven with it: 65 is 6.5 km)
    pub buses: Vec<(String, u64)>,
}

impl Record {
    /// Passenger comfort rating in percent (comfort points obtained / possible), if there is one.
    pub fn comfort(&self) -> Option<(f64, i64, i64)> {
        (self.rating[0] > 0).then(|| (self.rating[1] as f64 * 100.0 / self.rating[0] as f64, self.rating[1], self.rating[0]))
    }

    /// 100 % when the driver has no accidents or hit and runs at all; otherwise OMSI's formula is not known here.
    pub fn driving(&self) -> Option<f64> {
        (self.crashes.iter().all(|&c| c == 0) && self.rating[2..].iter().all(|&r| r == 0)).then_some(100.0)
    }
}

/// The lines under `tag` up to the next blank line or tag.
fn block<'a>(lines: &[&'a str], tag: &str) -> Vec<&'a str> {
    let Some(i) = lines.iter().position(|l| l.trim() == tag) else { return vec![] };
    lines[i + 1..].iter().map(|l| l.trim()).take_while(|l| !l.is_empty() && !l.starts_with('[')).collect()
}

pub fn read_record(path: &Path) -> Option<Record> {
    let text = read_utf16(path)?;
    let lines: Vec<&str> = text.lines().collect();
    let num = |tag: &str, n: usize| block(&lines, tag).get(n).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    let ident = block(&lines, "[ident]");
    let mut rec = Record {
        hired: ident.get(3).map(|s| s.to_string()).unwrap_or_default(),
        stops: num("[busstops]", 0),
        late: num("[busstops]", 1),
        early: num("[busstops]", 2),
        hektometers: num("[hektom]", 0),
        crashes: [num("[crashs]", 0), num("[crashs]", 1), num("[crashs]", 2), num("[crashs]", 3)],
        tickets_sold: num("[tickets]", 0),
        cash: block(&lines, "[tickets]").get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        ..Record::default()
    };
    for (n, r) in rec.rating.iter_mut().enumerate() {
        *r = block(&lines, "[rating]").get(n).and_then(|s| s.parse().ok()).unwrap_or(0);
    }
    // each [perbusinfo] is the bus name (one or more lines) followed by how many times it was driven
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "[perbusinfo]" {
            let body: Vec<&str> = lines[i + 1..].iter().map(|l| l.trim()).take_while(|l| !l.is_empty() && !l.starts_with('[')).collect();
            if let Some((count, name)) = body.split_last() {
                rec.buses.push((name.join(" "), count.parse().unwrap_or(0)));
            }
        }
        i += 1;
    }
    Some(rec)
}
