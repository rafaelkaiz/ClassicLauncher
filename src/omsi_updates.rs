//! The versions of openOMSI published on GitHub (the project's releases, as the official launcher reads them): which is the latest on the
//! stable or the beta channel, and the list to choose a version from. This only looks and points to the release pages.

use serde_json::Value;
use std::process::Command;
use std::sync::{Arc, Mutex};

pub const REPO: &str = "openOMSI-org/openOMSI";
pub const REPO_URL: &str = "https://github.com/openOMSI-org/openOMSI";

#[derive(Clone, Debug, PartialEq)]
pub struct Rel {
    /// `0.3.46`, without the `v`.
    pub version: String,
    /// A beta: published before it is called stable.
    pub beta: bool,
    pub page: String,
    /// `2026-10-29`.
    pub date: String,
    /// The Windows archive of the release: its address, size and SHA-256 (as GitHub lists it).
    pub asset_url: String,
    pub size: u64,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    Done(Vec<Rel>),
    Failed,
}

pub type Shared = Arc<Mutex<State>>;

/// Reads the releases on a thread of its own; `done` runs when it ends.
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
        let r = fetch().map(State::Done).unwrap_or(State::Failed);
        *state.lock().unwrap() = r;
        done();
    });
}

fn fetch() -> Option<Vec<Rel>> {
    use std::os::windows::process::CommandExt;
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=60");
    let out = Command::new("curl.exe").args(["-s", "-L", "--max-time", "20", "-H", "User-Agent: ClassicLauncher", "-H", "Accept: application/vnd.github+json", &url]).creation_flags(0x0800_0000).output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse(&String::from_utf8_lossy(&out.stdout))
}

/// The releases of GitHub's answer, newest first (drafts left out).
pub fn parse(text: &str) -> Option<Vec<Rel>> {
    let v: Value = serde_json::from_str(text).ok()?;
    let mut list: Vec<Rel> = v
        .as_array()?
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?;
            Some(Rel {
                version: tag.trim_start_matches(['v', 'V']).to_string(),
                beta: r["prerelease"].as_bool().unwrap_or(false),
                page: r["html_url"].as_str().unwrap_or(REPO_URL).to_string(),
                date: r["published_at"].as_str().unwrap_or("").chars().take(10).collect(),
                asset_url: asset(r, tag).and_then(|a| a["browser_download_url"].as_str()).unwrap_or("").to_string(),
                size: asset(r, tag).and_then(|a| a["size"].as_u64()).unwrap_or(0),
                sha256: asset(r, tag).and_then(|a| a["digest"].as_str()).and_then(|d| d.strip_prefix("sha256:")).map(str::to_ascii_lowercase),
            })
        })
        .collect();
    list.sort_by(|a, b| if crate::update::is_newer(&a.version, &b.version) { std::cmp::Ordering::Less } else if crate::update::is_newer(&b.version, &a.version) { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Equal });
    Some(list)
}

/// The latest version of the channel: on the stable one, a beta is passed over.
pub fn latest(list: &[Rel], beta: bool) -> Option<&Rel> {
    list.iter().find(|r| beta || !r.beta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_latest_of_each_channel_is_found() {
        let text = r#"[
          {"tag_name":"v0.3.45","prerelease":false,"draft":false,"html_url":"https://x/45","published_at":"2026-10-20T10:00:00Z"},
          {"tag_name":"v0.3.46","prerelease":true,"draft":false,"html_url":"https://x/46","published_at":"2026-10-29T10:00:00Z"},
          {"tag_name":"v0.3.47","prerelease":true,"draft":true,"html_url":"https://x/47","published_at":"2026-10-30T10:00:00Z"}
        ]"#;
        let list = parse(text).unwrap();
        assert_eq!(list.len(), 2, "a draft is not a version");
        assert_eq!(latest(&list, false).unwrap().version, "0.3.45");
        assert_eq!(latest(&list, true).unwrap().version, "0.3.46");
        assert_eq!(list[0].date, "2026-10-29");
    }
}

/// The release's file for Windows: `openOMSI-<version>-windows-x64.zip`.
fn asset<'a>(r: &'a Value, tag: &str) -> Option<&'a Value> {
    let want = format!("openOMSI-{}-windows-x64.zip", tag.trim_start_matches(['v', 'V']));
    r["assets"].as_array()?.iter().find(|a| a["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(&want)))
}

// --- installing -------------------------------------------------------------------------------------------------------------------

/// Where an installation stands.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Install {
    #[default]
    Idle,
    Downloading { done: u64, total: u64 },
    Checking,
    Unpacking,
    Testing,
    Replacing,
    Done(String),
    Failed(String),
}

pub type Progress = Arc<Mutex<Install>>;

const STAGING: &str = ".openomsi-update";
const MANIFEST: &str = ".openomsi-files";
const PREVIOUS: &str = ".openomsi-previous";
const OLD: &str = ".old-update";

fn set(p: &Progress, s: Install) {
    *p.lock().unwrap() = s;
}

fn hidden(c: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    c.creation_flags(0x0800_0000)
}

/// Downloads `rel`, checks it, tries the new program and puts its files in the place of the old ones in the folder of `exe`, as the
/// official launcher does: every top-level item the archive holds replaces the one of the same name, the old one staying beside it
/// as `*.old-update` (the game itself deletes them once the new version has started well, and goes back to them if it has not). Any
/// failure puts everything back as it was. Run on a thread of its own.
pub fn install(rel: Rel, exe: std::path::PathBuf, progress: &Progress, done: impl Fn() + Send + 'static) {
    {
        let mut s = progress.lock().unwrap();
        if matches!(*s, Install::Downloading { .. } | Install::Checking | Install::Unpacking | Install::Testing | Install::Replacing) {
            return;
        }
        *s = Install::Downloading { done: 0, total: rel.size };
    }
    let progress = progress.clone();
    std::thread::spawn(move || {
        let r = run(&rel, &exe, &progress);
        set(&progress, match r {
            Ok(()) => Install::Done(rel.version.clone()),
            Err(e) => Install::Failed(e),
        });
        done();
    });
}

fn run(rel: &Rel, exe: &std::path::Path, progress: &Progress) -> Result<(), String> {
    let dir = exe.parent().ok_or("the game's folder is not known")?.to_path_buf();
    if rel.asset_url.is_empty() {
        return Err("this release has no file for Windows".into());
    }
    let downloads = std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir).join("ClassicLauncher").join("downloads");
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let zip = downloads.join(format!("openOMSI-{}-windows-x64.zip", rel.version));
    // the download, which goes on where an earlier one stopped
    let mut child = hidden(Command::new("curl.exe").args(["-L", "-f", "-s", "--retry", "3", "-C", "-", "-o"]).arg(&zip).arg(&rel.asset_url)).spawn().map_err(|e| format!("curl: {e}"))?;
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(s) if s.success() => break,
            Some(s) => return Err(format!("the download failed ({s})")),
            None => {
                let done = std::fs::metadata(&zip).map(|m| m.len()).unwrap_or(0);
                set(progress, Install::Downloading { done, total: rel.size });
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        }
    }
    set(progress, Install::Checking);
    if let Some(want) = &rel.sha256 {
        let out = hidden(Command::new("certutil.exe").args(["-hashfile"]).arg(&zip).arg("SHA256")).output().map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        let got = text.lines().nth(1).map(|l| l.replace(' ', "").to_ascii_lowercase()).unwrap_or_default();
        if got != *want {
            let _ = std::fs::remove_file(&zip);
            return Err("the file does not match the checksum GitHub gives for it: deleted, try again".into());
        }
    }
    set(progress, Install::Unpacking);
    let staging = dir.join(STAGING);
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| format!("{}: {e}", staging.display()))?;
    let out = hidden(Command::new("tar.exe").arg("-xf").arg(&zip).arg("-C").arg(&staging)).output().map_err(|e| format!("tar: {e}"))?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(format!("the archive could not be unpacked: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let mut items: Vec<String> = std::fs::read_dir(&staging).map_err(|e| e.to_string())?.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    items.sort();
    if !items.iter().any(|n| n.eq_ignore_ascii_case("openomsi.exe")) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err("the archive holds no openOMSI program: nothing was replaced".into());
    }
    // the new program must start before it replaces the old one
    set(progress, Install::Testing);
    let mut test = hidden(Command::new(staging.join("openomsi.exe")).arg("--version").current_dir(&staging)).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e| format!("the new program cannot start: {e}"))?;
    let started = std::time::Instant::now();
    loop {
        match test.try_wait().map_err(|e| e.to_string())? {
            Some(s) if s.success() => break,
            Some(s) => {
                let _ = std::fs::remove_dir_all(&staging);
                return Err(format!("the new program does not start ({s}): nothing was replaced"));
            }
            None if started.elapsed() > std::time::Duration::from_secs(30) => {
                let _ = test.kill();
                let _ = std::fs::remove_dir_all(&staging);
                return Err("the new program did not answer in 30 seconds: nothing was replaced".into());
            }
            None => std::thread::sleep(std::time::Duration::from_millis(200)),
        }
    }
    set(progress, Install::Replacing);
    let before: Vec<String> = std::fs::read_to_string(dir.join(MANIFEST)).map(|t| t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty() && !l.contains(['/', '\\']) && l != ".." && l != ".").collect()).unwrap_or_default();
    let gone: Vec<String> = before.into_iter().filter(|n| !items.iter().any(|i| i.eq_ignore_ascii_case(n))).collect();
    // the version these files are, for the game to go back to it if the new one does not start well
    let current = crate::game::version_of(exe).unwrap_or_default();
    let _ = std::fs::write(dir.join(PREVIOUS), &current);
    let manifest = dir.join(MANIFEST);
    let manifest_aside = if manifest.exists() {
        let aside = aside_of(&manifest);
        std::fs::rename(&manifest, &aside).ok().map(|_| aside)
    } else {
        None
    };
    let mut moved: Vec<(std::path::PathBuf, Option<std::path::PathBuf>)> = vec![];
    let result = (|| -> Result<(), String> {
        for name in &items {
            let target = dir.join(name);
            let old = aside_of(&target);
            let had = target.exists();
            if had {
                std::fs::rename(&target, &old).map_err(|e| format!("{} cannot be replaced ({e}): is the game closed?", target.display()))?;
            }
            moved.push((target.clone(), had.then_some(old)));
            std::fs::rename(staging.join(name), &target).map_err(|e| format!("{} cannot be written ({e})", target.display()))?;
        }
        for name in &gone {
            let target = dir.join(name);
            if target.exists() {
                let old = old_of(&target);
                remove_any(&old);
                std::fs::rename(&target, &old).map_err(|e| e.to_string())?;
                moved.push((target, None));
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        for (target, old) in moved.into_iter().rev() {
            match old {
                Some(old) => {
                    remove_any(&target);
                    let _ = std::fs::rename(&old, &target);
                }
                None => {
                    let o = old_of(&target);
                    if o.exists() {
                        let _ = std::fs::rename(&o, &target);
                    } else {
                        remove_any(&target);
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&staging);
        if let Some(aside) = manifest_aside {
            let _ = std::fs::rename(aside, &manifest);
        }
        let _ = std::fs::remove_file(dir.join(PREVIOUS));
        return Err(format!("{e}: everything is back as it was"));
    }
    let _ = std::fs::write(dir.join(MANIFEST), items.join("\n") + "\n");
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_file(&zip);
    Ok(())
}

fn old_of(p: &std::path::Path) -> std::path::PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(OLD);
    s.into()
}

fn remove_any(p: &std::path::Path) {
    if p.is_dir() {
        let _ = std::fs::remove_dir_all(p);
    } else {
        let _ = std::fs::remove_file(p);
    }
}

/// Where `target` is put aside: its `*.old-update`, or `*.<n>.old-update` while an earlier one cannot be deleted.
fn aside_of(target: &std::path::Path) -> std::path::PathBuf {
    let first = old_of(target);
    remove_any(&first);
    if !first.exists() {
        return first;
    }
    for n in 2..100 {
        let mut s = target.as_os_str().to_os_string();
        s.push(format!(".{n}{OLD}"));
        let p = std::path::PathBuf::from(s);
        remove_any(&p);
        if !p.exists() {
            return p;
        }
    }
    first
}

// --- the card and windows of the settings ------------------------------------------------------------------------------------------

use crate::i18n::{tr, trf};
use eframe::egui::{self, RichText};

/// What the updates card and its windows keep between frames.
#[derive(Default)]
pub struct Upd {
    pub rel: Shared,
    pub install: Progress,
    pub versions_open: bool,
    pub confirm: Option<Rel>,
}

impl Upd {
    /// The card of openOMSI's own updates: the version installed, the channel, the latest version and the list to choose from. Installing
    /// is asked for with a window of its own that says what it does.
    pub fn card(&mut self, ui: &mut egui::Ui, beta_channel: &mut bool, installed: Option<String>, open_url: &mut Option<String>) -> bool {
        let mut changed = false;
        ui.label(RichText::new(tr("openOMSI")).strong().size(15.0));
        ui.label(match &installed {
            Some(v) => trf("Installed: openOMSI {}", &[v]),
            None => tr("Installed: openOMSI (version not known)").to_string(),
        });
        ui.horizontal(|ui| {
            ui.label(tr("Update channel"));
            let mut beta = *beta_channel;
            egui::ComboBox::from_id_salt("omsi-channel").selected_text(tr(if beta { "Beta" } else { "Stable" })).show_ui(ui, |ui| {
                ui.selectable_value(&mut beta, false, tr("Stable"));
                ui.selectable_value(&mut beta, true, tr("Beta"));
            });
            if beta != *beta_channel {
                *beta_channel = beta;
                changed = true;
            }
        });
        let state = self.rel.lock().unwrap().clone();
        let progress = self.install.lock().unwrap().clone();
        let busy = matches!(progress, Install::Downloading { .. } | Install::Checking | Install::Unpacking | Install::Testing | Install::Replacing);
        ui.horizontal(|ui| {
            if ui.add_enabled(state != State::Checking && !busy, egui::Button::new(tr("Check now"))).clicked() {
                let c = ui.ctx().clone();
                start(&self.rel, move || c.request_repaint());
            }
            if ui.add_enabled(matches!(state, State::Done(_)) && !busy, egui::Button::new(tr("Choose a version..."))).clicked() {
                self.versions_open = true;
            }
            ui.hyperlink_to(RichText::new("github.com/openOMSI-org/openOMSI").size(12.0), REPO_URL);
        });
        match &state {
            State::Idle => {}
            State::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr("Checking..."));
                });
            }
            State::Failed => {
                ui.label(tr("Could not check (no connection?)."));
            }
            State::Done(list) => {
                match (latest(list, *beta_channel), &installed) {
                    (Some(l), Some(v)) if crate::update::is_newer(&l.version, v) => {
                        ui.label(RichText::new(trf("A newer version is available: {}", &[&l.version])).strong());
                        ui.horizontal(|ui| {
                            if ui.add_enabled(!busy && !l.asset_url.is_empty(), egui::Button::new(trf("Update to {}...", &[&l.version]))).clicked() {
                                self.confirm = Some(l.clone());
                            }
                            if ui.small_button(tr("Release page")).clicked() {
                                *open_url = Some(l.page.clone());
                            }
                        });
                    }
                    (Some(_), Some(v)) => {
                        ui.label(trf("openOMSI {} is the latest version.", &[v]));
                    }
                    (Some(l), None) => {
                        ui.label(trf("The latest version is {}.", &[&l.version]));
                    }
                    (None, _) => {
                        ui.label(tr("No version found."));
                    }
                }
            }
        }
        match &progress {
            Install::Idle => {}
            Install::Downloading { done, total } => {
                let f = if *total > 0 { *done as f32 / *total as f32 } else { 0.0 };
                ui.add(egui::ProgressBar::new(f.clamp(0.0, 1.0)).text(trf("Downloading... {} of {} MB", &[&(done / 1_000_000).to_string(), &(total / 1_000_000).to_string()])));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
            Install::Checking => {
                ui.label(tr("Checking the download..."));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
            Install::Unpacking => {
                ui.label(tr("Unpacking..."));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
            Install::Testing => {
                ui.label(tr("Trying the new version..."));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
            Install::Replacing => {
                ui.label(tr("Replacing the files..."));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
            Install::Done(v) => {
                ui.colored_label(egui::Color32::from_rgb(40, 140, 70), trf("openOMSI {} is installed. Start the game once: it removes the old files when the new version has started well.", &[v]));
            }
            Install::Failed(e) => {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), tr(e));
            }
        }
        changed
    }

    /// The list of versions to choose from, and the question before installing one.
    pub fn windows(&mut self, ctx: &egui::Context, installed: Option<String>, game: Option<&std::path::Path>) {
        if self.versions_open {
            let list = match &*self.rel.lock().unwrap() {
                State::Done(l) => l.clone(),
                _ => vec![],
            };
            let busy = matches!(*self.install.lock().unwrap(), Install::Downloading { .. } | Install::Checking | Install::Unpacking | Install::Testing | Install::Replacing);
            let mut pick: Option<Rel> = None;
            let mut page: Option<String> = None;
            let open = crate::free::window(ctx, "omsi-versions", tr("Choose a version"), egui::vec2(520.0, 460.0), |ui| {
                ui.label(RichText::new(tr("Any version can be installed, an older one too. The files being replaced are kept until the new one has started well.")).weak().size(12.0));
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(360.0).auto_shrink([false, true]).show(ui, |ui| {
                    for r in &list {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&r.version).strong());
                            if r.beta {
                                ui.label(RichText::new(tr("Beta")).color(egui::Color32::from_rgb(200, 120, 20)).size(12.0));
                            }
                            if installed.as_deref() == Some(r.version.as_str()) {
                                ui.label(RichText::new(tr("installed")).color(egui::Color32::from_rgb(40, 140, 70)).size(12.0));
                            }
                            ui.label(RichText::new(&r.date).weak().size(12.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let same = installed.as_deref() == Some(r.version.as_str());
                                if ui.add_enabled(!busy && !same && !r.asset_url.is_empty(), egui::Button::new(tr("Install"))).clicked() {
                                    pick = Some(r.clone());
                                }
                                if ui.small_button(tr("Release page")).clicked() {
                                    page = Some(r.page.clone());
                                }
                            });
                        });
                        ui.separator();
                    }
                });
            });
            if !open {
                self.versions_open = false;
            }
            if let Some(p) = page {
                crate::omsi::open_folder(std::path::Path::new(&p));
            }
            if let Some(r) = pick {
                self.confirm = Some(r);
            }
        }
        // the question before replacing the game's files
        if let Some(r) = self.confirm.clone() {
            let running = crate::driver_files::game_running();
            let mut go = false;
            let mut cancel = false;
            let folder = game.and_then(|g| g.parent().map(|p| p.display().to_string())).unwrap_or_default();
            let open = crate::free::window(ctx, "omsi-confirm", &trf("Install openOMSI {}?", &[&r.version]), egui::vec2(520.0, 230.0), |ui| {
                ui.add(egui::Label::new(trf("The program files of openOMSI in {} are replaced by those of version {}. The files being replaced stay beside the new ones, as *.old-update, until the new version has started well; if anything fails, everything is put back as it was. Your maps, vehicles and settings are not touched.", &[&folder, &r.version])).wrap());
                if running {
                    ui.add_space(4.0);
                    ui.colored_label(egui::Color32::from_rgb(200, 60, 60), tr("Close the game first."));
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!running, egui::Button::new(tr("Install"))).clicked() {
                        go = true;
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        cancel = true;
                    }
                });
            });
            if go {
                if let Some(exe) = game.map(std::path::Path::to_path_buf) {
                    let c = ctx.clone();
                    install(r, exe, &self.install, move || c.request_repaint());
                }
                self.confirm = None;
                self.versions_open = false;
            } else if cancel || !open {
                self.confirm = None;
            }
        }
    }

}
