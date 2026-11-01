//! OMSI's tutorial menu data (`Tutorials\menu_N.jpg` and `menu_N_<LANG>.html`), read-only. `menu_0` is the introduction shown
//! before a tutorial is picked, `menu_1`..`menu_4` are tutorials I to IV.

use std::path::{Path, PathBuf};

pub struct Entry {
    pub title: String,
    pub text: String,
    pub picture: Option<PathBuf>,
}

fn decode(bytes: &[u8]) -> String {
    let b = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => b.iter().map(|&c| c as char).collect(), // ISO-8859-1, as the files declare
    }
}

fn plain_text(html: &str) -> (String, String) {
    let body = html.split_once("<body>").map_or(html, |(_, b)| b);
    let mut title = String::new();
    let mut out = String::new();
    let mut tag = String::new();
    let mut in_tag = false;
    let mut in_h2 = false;
    for c in body.chars() {
        match c {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let t = tag.trim().to_lowercase();
                match t.as_str() {
                    "h2" => in_h2 = true,
                    "/h2" => in_h2 = false,
                    "/p" | "br" | "br/" | "/li" | "/ul" => out.push('\n'),
                    "li" => out.push_str("• "),
                    _ => {}
                }
            }
            _ if in_tag => tag.push(c),
            _ if in_h2 => title.push(c),
            _ => out.push(c),
        }
    }
    let clean = |s: &str| {
        s.replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
    };
    let text: Vec<String> = clean(&out).lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect();
    let mut text = text.join("\n");
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    (clean(title.trim()), text.trim().to_string())
}

/// The five menu entries in OMSI's `lang` (e.g. "DEU"), falling back to English for languages it has no text for.
pub fn load(omsi: &Path, lang: &str) -> Vec<Entry> {
    let dir = omsi.join("Tutorials");
    (0..=4)
        .map(|n| {
            let pick = |l: &str| std::fs::read(dir.join(format!("menu_{n}_{l}.html"))).ok();
            let html = pick(lang).or_else(|| pick("ENG")).map(|b| decode(&b)).unwrap_or_default();
            let (title, text) = plain_text(&html);
            let picture = Some(dir.join(format!("menu_{n}.jpg"))).filter(|p| p.is_file());
            Entry { title, text, picture }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_real_tutorial_texts() {
        let omsi = Path::new(r"C:\Program Files (x86)\Steam\steamapps\common\OMSI 2");
        if !omsi.join("Tutorials").is_dir() {
            return;
        }
        let e = load(omsi, "ENG");
        assert_eq!(e.len(), 5);
        assert_eq!(e[0].title, "Select a tutorial");
        assert!(e[0].text.contains("Have fun!"));
        assert!(e.iter().all(|x| !x.text.contains('<') && x.picture.is_some()));
        let p = load(omsi, "POL");
        assert!(p[0].title.starts_with("Wybierz"));
    }
}
