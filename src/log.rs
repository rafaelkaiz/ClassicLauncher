//! A small log of what the launcher did (`%APPDATA%\ClassicLauncher\launcher.log`), so a failed launch can be diagnosed afterwards.
//! It never leaves the machine and is trimmed when it grows past 200 KB.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

fn path() -> std::path::PathBuf {
    let base = std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("ClassicLauncher").join("launcher.log")
}

pub fn line(msg: &str) {
    let p = path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::metadata(&p).is_ok_and(|m| m.len() > 200_000) {
        let _ = std::fs::remove_file(&p);
    }
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = writeln!(f, "{h:02}:{m:02}:{s:02} UTC  {}", msg.replace('\n', " "));
    }
}
