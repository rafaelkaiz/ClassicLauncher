//! Looks for a newer launcher release on GitHub. It only checks and points to the release page: nothing is downloaded or replaced.

use std::process::Command;
use std::sync::{Arc, Mutex};

/// GitHub repository whose releases are checked ("owner/name").
pub const REPO: &str = "rafaelkaiz/ClassicLauncher";

/// The version as people read it ("0.9-rc1"); `VERSION` is the same in the form Cargo and the version comparison use.
pub const LABEL: &str = "1.0-beta2";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, PartialEq)]
#[derive(Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Newer { tag: String, url: String },
    /// Nothing published yet, or no connection.
    Failed(String),
}

pub type Shared = Arc<Mutex<State>>;

/// Starts the check on its own thread; `done` runs when it ends (to wake the interface).
pub fn start(state: &Shared, done: impl Fn() + Send + 'static) {
    {
        let mut s = state.lock().unwrap();
        if *s == State::Checking {
            return;
        }
        *s = State::Checking;
    }
    let state = state.clone();
    std::thread::spawn(move || {
        let result = check();
        *state.lock().unwrap() = result;
        done();
    });
}

fn check() -> State {
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=1");
    let mut cmd = Command::new("curl.exe");
    cmd.args(["-s", "-L", "--max-time", "15", "-H", "User-Agent: ClassicLauncher", "-H", "Accept: application/vnd.github+json", &url]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // no console window
    }
    let out = match cmd.output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => return State::Failed("no connection".into()),
    };
    let Some(tag) = json_string(&out, "tag_name") else {
        return State::Failed("no release".into());
    };
    let page = json_string(&out, "html_url").unwrap_or_else(|| format!("https://github.com/{REPO}/releases"));
    if is_newer(&tag, VERSION) { State::Newer { tag, url: page } } else { State::UpToDate }
}

/// The value of `"key": "value"` in a JSON text (first occurrence; enough for GitHub's release reply).
fn json_string(json: &str, key: &str) -> Option<String> {
    let at = json.find(&format!("\"{key}\""))? + key.len() + 2;
    let rest = json[at..].trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_string())
}

/// Splits a version into its numbers and its pre-release identifiers ("0.9.0-rc.1" is [0, 9, 0] and ["rc", "1"]).
fn parts(v: &str) -> (Vec<u32>, Vec<String>) {
    let v = v.trim_start_matches(['v', 'V']);
    let (core, pre) = v.split_once('-').map_or((v, ""), |(c, p)| (c, p));
    let nums = core.split('.').map(|p| p.chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0)).collect();
    (nums, pre.split('.').filter(|p| !p.is_empty()).map(str::to_string).collect())
}

/// Order of two pre-release identifiers: numbers by value, words alphabetically (alpha < beta < rc), numbers before words.
fn identifier_order(a: &str, b: &str) -> std::cmp::Ordering {
    match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        _ => a.cmp(b),
    }
}

/// Semantic order: 0.9.0-beta.3 < 0.9.0-rc.1 < 0.9.0 < 0.10.0.
pub fn is_newer(tag: &str, current: &str) -> bool {
    let ((a, pa), (b, pb)) = (parts(tag), parts(current));
    if a != b {
        return a > b;
    }
    match (pa.is_empty(), pb.is_empty()) {
        (true, false) => true,
        (false, true) => false,
        (true, true) => false,
        (false, false) => {
            for (x, y) in pa.iter().zip(pb.iter()) {
                match identifier_order(x, y) {
                    std::cmp::Ordering::Equal => continue,
                    o => return o == std::cmp::Ordering::Greater,
                }
            }
            pa.len() > pb.len()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(is_newer("v0.9.0", "0.9.0-beta.1"));
        assert!(is_newer("v0.9.0-beta.2", "0.9.0-beta.1"));
        assert!(!is_newer("v0.9.0-beta.1", "0.9.0-beta.1"));
        assert!(!is_newer("v0.9.0-beta.1", "0.9.0"));
        // a release candidate is newer than every beta and older than the final version
        assert!(is_newer("v0.9.0-rc.1", "0.9.0-beta.3"));
        assert!(is_newer("v0.9.0-rc.2", "0.9.0-rc.1"));
        assert!(!is_newer("v0.9.0-beta.3", "0.9.0-rc.1"));
        assert!(is_newer("v0.9.0", "0.9.0-rc.1"));
        assert!(!is_newer("v0.9.0-rc.1", "0.9.0"));
        assert!(is_newer("v0.9.0-beta.10", "0.9.0-beta.9"));
    }

    #[test]
    fn reads_release_fields() {
        let j = r#"{"url":"x","html_url": "https://github.com/a/b/releases/tag/v1","tag_name":"v1.2.3"}"#;
        assert_eq!(json_string(j, "tag_name").as_deref(), Some("v1.2.3"));
        assert_eq!(json_string(j, "html_url").as_deref(), Some("https://github.com/a/b/releases/tag/v1"));
        assert_eq!(json_string(j, "missing"), None);
    }
}
