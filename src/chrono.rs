//! "Información sobre el tiempo seleccionado": OMSI shows, for the selected date, the descriptions of the map's timetable and tariff
//! periods that cover it. They live in `maps\<map>\Chrono\<yyyymmdd>_<name>\` (`Chrono.cfg` with `[startdate]`/`[enddate]`, and
//! `Chrono_<LANG>.dsc` with `[name]` and `[description]`). Read-only.

use std::path::Path;

pub type Date = (i64, u32, u32);

/// The date and time of day of a saved situation (`.osn`): its `[time]` block is year, day of the year, hour, minute, second.
pub fn situation_datetime(osn: &Path) -> Option<(Date, (u32, u32, u32))> {
    let b = std::fs::read(osn).ok()?;
    let text = if let Some(rest) = b.strip_prefix(&[0xFF, 0xFE]) {
        String::from_utf16_lossy(&rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>())
    } else {
        b.iter().map(|&c| c as char).collect()
    };
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let i = lines.iter().position(|l| l.eq_ignore_ascii_case("[time]"))?;
    let year: i64 = lines.get(i + 1)?.parse().ok()?;
    let day: u32 = lines.get(i + 2)?.parse().ok()?;
    let part = |k: usize| lines.get(i + k).and_then(|s| s.replace(',', ".").parse::<f64>().ok()).map(|v| v as u32);
    let time = (part(3).unwrap_or(0).min(23), part(4).unwrap_or(0).min(59), part(5).unwrap_or(0).min(59));
    Some((from_year_day(year, day)?, time))
}

/// The date of a saved situation.
pub fn situation_date(osn: &Path) -> Option<Date> {
    situation_datetime(osn).map(|(d, _)| d)
}


fn leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Day of the year (1-based) to (year, month, day).
pub fn from_year_day(year: i64, day: u32) -> Option<Date> {
    let months = [31, if leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut left = day;
    if left == 0 {
        return None;
    }
    for (i, m) in months.iter().enumerate() {
        if left <= *m {
            return Some((year, i as u32 + 1, left));
        }
        left -= m;
    }
    None
}

fn key(d: Date) -> u32 {
    (d.0 as u32) * 10_000 + d.1 * 100 + d.2
}

fn read_any(p: &Path) -> Option<String> {
    let b = std::fs::read(p).ok()?;
    Some(if let Some(rest) = b.strip_prefix(&[0xFF, 0xFE]) {
        String::from_utf16_lossy(&rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>())
    } else {
        b.iter().map(|&c| c as char).collect()
    })
}

fn value_after(text: &str, tag: &str) -> Option<String> {
    let mut lines = text.lines().map(str::trim);
    lines.find(|l| l.eq_ignore_ascii_case(tag))?;
    lines.next().map(str::to_string)
}

fn description(text: &str) -> String {
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    if lines.find(|l| l.trim().eq_ignore_ascii_case("[description]")).is_none() {
        return String::new();
    }
    lines.take_while(|l| !l.trim().eq_ignore_ascii_case("[end]")).collect::<Vec<_>>().join("\n").trim().to_string()
}

/// The text OMSI shows for `date` on the map in `map_dir`, in `lang` ("ESP", "DEU"...), falling back to English then German.
pub fn info(map_dir: &Path, date: Date, lang: &str) -> String {
    let Ok(rd) = std::fs::read_dir(map_dir.join("Chrono")) else { return String::new() };
    let mut periods: Vec<(u32, String)> = vec![];
    for e in rd.flatten().filter(|e| e.path().is_dir()) {
        let dir = e.path();
        let Some(cfg) = read_any(&dir.join("Chrono.cfg")) else { continue };
        let parse = |tag: &str| value_after(&cfg, tag).and_then(|v| v.parse::<u32>().ok());
        let (Some(start), Some(end)) = (parse("[startdate]"), parse("[enddate]")) else { continue };
        if !(start <= key(date) && key(date) <= end) {
            continue;
        }
        let dsc = [lang, "ENG", "DEU"].iter().find_map(|l| read_any(&dir.join(format!("Chrono_{l}.dsc"))));
        let Some(dsc) = dsc else { continue };
        let name = value_after(&dsc, "[name]").unwrap_or_default();
        let desc = description(&dsc);
        if name.is_empty() && desc.is_empty() {
            continue;
        }
        let rule = "-".repeat(44);
        periods.push((start, format!("{name}\n{rule}\n{desc}")));
    }
    periods.sort_by_key(|p| p.0);
    if periods.is_empty() {
        return String::new();
    }
    let bar = "=".repeat(44);
    let body: Vec<String> = periods.into_iter().map(|p| p.1).collect();
    format!("{bar}\n\n{}\n\n{bar}", body.join(&format!("\n\n{bar}\n\n")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_of_year_conversion() {
        assert_eq!(from_year_day(2019, 302), Some((2019, 10, 29)));
        assert_eq!(from_year_day(2020, 60), Some((2020, 2, 29)));
        assert_eq!(from_year_day(2019, 366), None);
    }
}

/// Day of the week of a date: 0 is Monday, 6 is Sunday.
pub fn weekday((y, m, d): Date) -> usize {
    // days since 1970-01-01 (a Thursday)
    let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 as i64 + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (days + 3).rem_euclid(7) as usize
}

fn number(s: &str) -> Option<i64> {
    let s = s.trim();
    (s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten()
}

/// The names of the holidays of the map that fall on `date`, from `Holidays_<LANG>.txt` (or `Holidays.txt`) in the map folder:
/// `[holiday]` is one day (date, name) and `[holidays]` a period (first day, last day, name). Read-only.
pub fn holidays(map_dir: &Path, lang: &str, (y, m, d): Date) -> Vec<String> {
    let text = [format!("Holidays_{lang}.txt"), "Holidays.txt".to_string()]
        .iter()
        .find_map(|n| std::fs::read(map_dir.join(n)).ok())
        .map(|b| b.iter().map(|&c| c as char).collect::<String>())
        .unwrap_or_default();
    let day = y * 10_000 + m as i64 * 100 + d as i64;
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut found: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let (span, has_range) = match lines[i].to_ascii_lowercase().as_str() {
            "[holiday]" => (2, false),
            "[holidays]" => (3, true),
            _ => {
                i += 1;
                continue;
            }
        };
        if i + span < lines.len() {
            let first = number(lines[i + 1]);
            let last = if has_range { number(lines[i + 2]) } else { first };
            let name = lines[i + span];
            if let (Some(a), Some(b)) = (first, last) {
                // the file is read as Latin-1 text; names are shown as OMSI wrote them
                if (a..=b).contains(&day) && !name.is_empty() && !found.iter().any(|n| n == name) {
                    found.push(name.to_string());
                }
            }
        }
        i += span + 1;
    }
    found
}

#[cfg(test)]
mod day_tests {
    use super::*;

    #[test]
    fn weekdays_are_right() {
        assert_eq!(weekday((2014, 1, 7)), 1); // a Tuesday, as OMSI's own dialog shows
        assert_eq!(weekday((1970, 1, 1)), 3); // Thursday
        assert_eq!(weekday((2000, 2, 29)), 1);
        assert_eq!(weekday((2026, 10, 31)), 5);
    }
}

#[cfg(test)]
mod holiday_tests {
    use super::*;

    #[test]
    fn holidays_of_a_map_are_read() {
        let Some(omsi) = crate::omsi::find_omsi_dir(None) else { return };
        let dir = omsi.join("maps").join("Grundorf");
        if !dir.join("Holidays_ENG.txt").is_file() {
            return;
        }
        assert!(holidays(&dir, "ENG", (1980, 4, 4)).iter().any(|n| n == "Good Friday"));
        // a school holiday period covers every day of it
        assert!(holidays(&dir, "ENG", (1980, 3, 30)).iter().any(|n| n == "Easter"));
        assert!(holidays(&dir, "ENG", (1980, 6, 10)).is_empty());
    }
}
