//! Creating, editing and deleting a driver's file (`Drivers\<name>.odr`): the only thing this launcher writes into the OMSI 2 folder.
//! Every change first copies the current file to `%APPDATA%\ClassicLauncher\backups` (the 3 newest copies of each file are kept),
//! and nothing is written while the game runs, because it adds each run to the driver's file.

use crate::drivers::Driver;
use crate::i18n::{tr, trf};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn backup_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("ClassicLauncher").join("backups")
}

/// Copies `file` into the backup folder as `<millis>-<file name>` and prunes the old copies.
pub fn backup(file: &Path) -> Result<PathBuf, String> {
    let dir = backup_dir();
    std::fs::create_dir_all(&dir).map_err(|e| trf("Could not create the backup folder: {}", &[&e.to_string()]))?;
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let dest = dir.join(format!("{millis:013}-{name}"));
    std::fs::copy(file, &dest).map_err(|e| trf("Could not make the backup: {}", &[&e.to_string()]))?;
    prune(&dir, 3);
    Ok(dest)
}

/// Keeps the newest `keep` backups of each file. Only touches the launcher's own backup folder.
fn prune(dir: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut groups: std::collections::HashMap<String, Vec<(u64, PathBuf)>> = std::collections::HashMap::new();
    for e in rd.flatten() {
        let fname = e.file_name().to_string_lossy().into_owned();
        let Some((stamp, name)) = fname.split_once('-') else { continue };
        let Ok(stamp) = stamp.parse::<u64>() else { continue };
        groups.entry(name.to_string()).or_default().push((stamp, e.path()));
    }
    for (_, mut files) in groups {
        files.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, path) in files.into_iter().skip(keep) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn write_utf16(path: &Path, text: &str) -> Result<(), String> {
    let mut b = vec![0xFF, 0xFE];
    for u in text.encode_utf16() {
        b.extend(u.to_le_bytes());
    }
    std::fs::write(path, b).map_err(|e| trf("Could not write the driver: {}", &[&e.to_string()]))
}

fn read_utf16(path: &Path) -> Option<String> {
    let b = std::fs::read(path).ok()?;
    let b = b.strip_prefix(&[0xFF, 0xFE]).unwrap_or(&b);
    Some(String::from_utf16_lossy(&b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>()))
}

pub fn valid_birth(s: &str) -> bool {
    s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) && {
        let (m, d) = (s[4..6].parse::<u32>().unwrap_or(0), s[6..8].parse::<u32>().unwrap_or(0));
        (1..=12).contains(&m) && (1..=31).contains(&d)
    }
}

pub fn valid_name(n: &str) -> bool {
    !n.trim().is_empty() && !n.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|', '[', ']'])
}

fn sex_letter(female: bool) -> &'static str {
    if female { "W" } else { "M" }
}

fn check(name: &str, birth: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err(tr("Invalid name (it cannot be empty or contain \\ / : * ? \" < > | [ ]).").into());
    }
    if !valid_birth(birth) {
        return Err(tr("The birth date must be yyyymmdd (for example 19950606).").into());
    }
    Ok(())
}

/// Creates `Drivers\<name>.odr` with empty statistics, like OMSI does for a new driver.
pub fn create(omsi: &Path, name: &str, female: bool, birth: &str) -> Result<(), String> {
    let name = name.trim();
    check(name, birth)?;
    let path = omsi.join("Drivers").join(format!("{name}.odr"));
    if path.exists() {
        return Err(tr("A driver with that name exists already.").into());
    }
    let (y, m, d) = today();
    let text = format!(
        "-----------------------\r\nDriver File\r\n-----------------------\r\n\r\nCreated with Omsi Version 2.3.004\r\nDate: {d:02}/{m:02}/{y}\r\n\r\n\
         [ident]\r\n{name}\r\n{}\r\n{birth}\r\n{y}{m:02}{d:02}\r\n\r\n[busstops]\r\n0\r\n0\r\n0\r\n\r\n[hektom]\r\n0\r\n\r\n\
         [crashs]\r\n0\r\n0\r\n0\r\n0\r\n\r\n[tickets]\r\n0\r\n0\r\n\r\n[rating]\r\n0\r\n0\r\n0\r\n0\r\n0\r\n\r\n",
        sex_letter(female)
    );
    write_utf16(&path, &text)
}

/// Changes name, sex and birth date of a driver, keeping its statistics. The file is renamed when the name changes.
pub fn update(old: &Driver, name: &str, female: bool, birth: &str) -> Result<(), String> {
    let name = name.trim();
    check(name, birth)?;
    // the same change in every copy of the file (the game's own and the OMSI 2 one)
    for copy in &old.copies {
        let text = read_utf16(copy).ok_or_else(|| tr("Could not read the driver.").to_string())?;
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        let i = lines.iter().position(|l| l.trim() == "[ident]").filter(|i| lines.len() >= i + 4).ok_or_else(|| tr("Invalid driver file.").to_string())?;
        let new_path = copy.with_file_name(format!("{name}.odr"));
        let same_file = new_path.to_string_lossy().eq_ignore_ascii_case(&copy.to_string_lossy());
        if !same_file && new_path.exists() {
            return Err(tr("A driver with that name exists already.").into());
        }
        backup(copy)?;
        lines[i + 1] = name.to_string();
        lines[i + 2] = sex_letter(female).to_string();
        lines[i + 3] = birth.to_string();
        write_utf16(&new_path, &(lines.join("
") + "
"))?;
        if new_path != *copy && !same_file {
            std::fs::remove_file(copy).map_err(|e| trf("Could not remove the old file: {}", &[&e.to_string()]))?;
        }
    }
    Ok(())
}

/// Deletes a driver's file after keeping a copy in the backup folder. The driver's photo, if any, stays where it is.
pub fn delete(d: &Driver) -> Result<PathBuf, String> {
    let mut kept = None;
    for copy in &d.copies {
        let b = backup(copy)?;
        kept.get_or_insert(b);
        std::fs::remove_file(copy).map_err(|e| trf("Could not remove the old file: {}", &[&e.to_string()]))?;
    }
    kept.ok_or_else(|| tr("Could not read the driver.").to_string())
}

/// Is the game (or OMSI 2 itself) running? Then the driver files are not touched.
pub fn game_running() -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap.is_null() || snap as isize == -1 {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut ok = Process32FirstW(snap, &mut entry) != 0;
        while ok {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if name.eq_ignore_ascii_case("openomsi.exe") || name.eq_ignore_ascii_case("Omsi.exe") {
                found = true;
                break;
            }
            ok = Process32NextW(snap, &mut entry) != 0;
        }
        CloseHandle(snap);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("classiclauncher-{tag}-{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(d.join("Drivers")).unwrap();
        d
    }

    #[test]
    fn a_created_driver_reads_back_and_edits_keep_its_file_readable() {
        let omsi = temp("drv");
        create(&omsi, "Test Driver", true, "19950606").unwrap();
        assert!(create(&omsi, "Test Driver", true, "19950606").is_err());
        let d = crate::drivers::list_drivers(&omsi, None).pop().unwrap();
        assert_eq!((d.name.as_str(), d.female, d.birth.as_str()), ("Test Driver", true, "19950606"));
        update(&d, "Renamed", false, "19800101").unwrap();
        let d = crate::drivers::list_drivers(&omsi, None).pop().unwrap();
        assert_eq!((d.name.as_str(), d.file.as_str(), d.female), ("Renamed", "Renamed", false));
        let _ = std::fs::remove_dir_all(omsi);
    }

    #[test]
    fn names_and_dates_are_checked() {
        assert!(valid_name("Ana") && !valid_name("a/b") && !valid_name(" "));
        assert!(valid_birth("19950606") && !valid_birth("19951306") && !valid_birth("1995"));
    }
}

/// (year, month, day) in UTC from the system clock.
fn today() -> (i64, u32, u32) {
    let days = (SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}
