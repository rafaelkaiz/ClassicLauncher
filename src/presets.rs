//! Option presets for openOMSI's settings: the six quality levels the game's own launcher offers (Low to Ultra, ported from its
//! `graphics_presets`) and the launcher's own, saved as plain `key=value` files in `%APPDATA%\ClassicLauncher\presets`. Loading a
//! preset sets the keys it holds and leaves every other setting as it is; a saved file is replaced only when the user confirms it.

use crate::gamecfg::GameCfg;
use std::path::PathBuf;

pub struct Preset {
    pub name: String,
    /// A quality level of the game's launcher (not a file of the user's).
    pub builtin: bool,
    pub values: Vec<(String, String)>,
}

/// Settings that belong to this computer or this player, not to a look: a preset neither saves nor loads them.
const PERSONAL: [&str; 6] = ["version", "language", "machine_translation", "discord_status", "discord_app_id", "driver"];

pub fn dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("ClassicLauncher").join("presets")
}

fn b(v: bool) -> String {
    if v { "1" } else { "0" }.to_string()
}

/// Low, Medium, High, Extra, Max, Ultra.
pub fn builtin() -> Vec<Preset> {
    let graphics = ["vanilla", "vanilla_plus", "enhanced", "enhanced", "enhanced", "enhanced"];
    let msaa = ["1", "4", "2", "4", "4", "4"];
    let scale = ["0.75", "auto", "auto", "auto", "auto", "auto"];
    let anisotropy = ["2", "8", "8", "8", "16", "16"];
    let shadow = ["1024", "2048", "2048", "2048", "4096", "4096"];
    let mirror = ["128", "256", "256", "512", "512", "512"];
    let refresh = ["eco", "full", "full", "full", "full", "full"];
    let view = ["600", "auto", "auto", "auto", "2000", "2000"];
    let obj_size = ["0.03", "0.013", "0.013", "0.01", "0.005", "0.005"];
    let obj_dist = ["500", "auto", "auto", "auto", "1500", "1500"];
    let rain = ["low", "high", "high", "high", "high", "high"];
    let light_shadows = ["off", "street", "street", "street", "all", "all"];
    let street_light = ["off", "off", "off", "auto", "on", "on"];
    let puddles = ["off", "high", "low", "high", "high", "high"];
    let mirror_quality = ["standard", "standard", "standard", "standard", "enhanced", "enhanced"];
    ["Low", "Medium", "High", "Extra", "Max", "Ultra"]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let low = i == 0;
            let mut v: Vec<(&str, String)> = vec![
                ("graphics", graphics[i].into()),
                ("ray_tracing", b(i == 5)),
                ("msaa", msaa[i].into()),
                ("anisotropy", anisotropy[i].into()),
                ("shadow_size", shadow[i].into()),
                ("clouds", b(true)),
                ("rain_quality", rain[i].into()),
                ("windy_trees", b(!low)),
                ("view_distance", view[i].into()),
                ("min_obj_size", obj_size[i].into()),
                ("max_obj_dist", obj_dist[i].into()),
                ("mirror_size", mirror[i].into()),
                ("mirror_refresh", refresh[i].into()),
                ("mirror_quality", mirror_quality[i].into()),
                ("render_scale", scale[i].into()),
                ("reflections", "1".into()),
                ("light_shadows", light_shadows[i].into()),
                ("cloud_shadows", b(i >= 3)),
                ("light_glow", b(i >= 3)),
                ("street_light", street_light[i].into()),
                ("puddles", puddles[i].into()),
            ];
            // (Low is Vanilla, which draws none of these: it leaves them as they are, so going back to Vanilla+ does not find them off)
            if !low {
                v.extend([("detail_textures", b(true)), ("ssao", b(true)), ("shadows", b(true))]);
            }
            Preset { name: (*name).to_string(), builtin: true, values: v.into_iter().map(|(k, v)| (k.to_string(), v)).collect() }
        })
        .collect()
}

/// What a graphics profile holds: the settings of the Graphics tab, as the game's own launcher keeps them.
pub const GRAPHICS_KEYS: [&str; 43] = [
    "graphics", "msaa", "render_scale", "upscaler", "fsr_quality", "fsr_sharpness", "anisotropy", "shadow_size", "ssao", "shadows", "shadow_casters",
    "detail_textures", "night_brightness", "led_glow", "led_mips", "shadow_blobs", "lamp_shadows", "reflections", "clouds", "cloud_quality",
    "street_light", "rain_quality", "vehicle_smoke", "puddles", "windy_trees", "road_snow", "ray_tracing", "light_shadows", "cloud_shadows",
    "light_glow", "vsync", "max_fps", "view_distance", "max_obj_dist", "min_obj_size", "mirror_size", "mirror_refresh", "mirror_quality",
    "mirror_hud_every_frame", "video_memory", "texture_compression", "gpu_texture_compression", "mirror_shadows",
];

/// A name that is safe as a file name.
pub fn valid_name(n: &str) -> bool {
    let n = n.trim();
    !n.is_empty() && n.len() <= 60 && !n.chars().any(|c| "\\/:*?\"<>|".contains(c) || c.is_control()) && !n.starts_with('.')
}

pub fn user_presets() -> Vec<Preset> {
    let mut v: Vec<Preset> = std::fs::read_dir(dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.extension()?.eq_ignore_ascii_case("cfg") {
                return None;
            }
            let text = std::fs::read_to_string(&p).ok()?;
            let values = text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).filter(|(k, _)| !PERSONAL.contains(&k.as_str())).collect();
            Some(Preset { name: p.file_stem()?.to_string_lossy().into_owned(), builtin: false, values })
        })
        .collect();
    v.sort_by_key(|p| p.name.to_lowercase());
    v
}

pub fn all() -> Vec<Preset> {
    let mut v = builtin();
    v.extend(user_presets());
    v
}

pub fn apply(cfg: &mut GameCfg, p: &Preset) {
    for (k, v) in &p.values {
        cfg.set(k, v);
    }
}

/// Saves the current settings as a new preset. An existing preset of that name is never replaced.
pub fn save_new(cfg: &GameCfg, name: &str) -> Result<(), String> {
    write(cfg, name, false)
}

/// Is there a profile of the user's with that name?
pub fn exists(name: &str) -> bool {
    valid_name(name) && dir().join(format!("{}.cfg", name.trim())).exists()
}

/// Saves the current settings as a profile, replacing a profile of the user's of that name (a quality level is never replaced).
pub fn save_over(cfg: &GameCfg, name: &str) -> Result<(), String> {
    write(cfg, name, true)
}

fn write(cfg: &GameCfg, name: &str, overwrite: bool) -> Result<(), String> {
    let name = name.trim();
    if !valid_name(name) {
        return Err(crate::i18n::tr("That name cannot be used.").to_string());
    }
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let path = dir().join(format!("{name}.cfg"));
    if (path.exists() && !overwrite) || builtin().iter().any(|p| p.name.eq_ignore_ascii_case(name)) {
        return Err(crate::i18n::tr("A preset with that name exists already.").to_string());
    }
    let keys: Vec<&str> = GRAPHICS_KEYS.to_vec();
    let mut text = String::from("# ClassicLauncher graphics profile of openOMSI's settings\n");
    for k in keys {
        if PERSONAL.contains(&k) {
            continue;
        }
        if let Some(v) = cfg.get(k) {
            text.push_str(&format!("{k}={v}\n"));
        }
    }
    std::fs::write(path, text).map_err(|e| e.to_string())
}

/// Deletes a profile of the user's (a file of the profiles folder, never a quality level).
pub fn delete(name: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err(crate::i18n::tr("That name cannot be used.").to_string());
    }
    std::fs::remove_file(dir().join(format!("{name}.cfg"))).map_err(|e| e.to_string())
}

/// Does every value of the preset equal the file's? (a level of the game's launcher is "current" when so)
pub fn matches(cfg: &GameCfg, p: &Preset) -> bool {
    p.values.iter().all(|(k, v)| cfg.get(k).as_deref() == Some(v.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_levels_and_loading_sets_only_their_keys() {
        let levels = builtin();
        assert_eq!(levels.len(), 6);
        let mut c = GameCfg::parse("x".into(), "vsync=0\nmsaa=8\n");
        apply(&mut c, &levels[0]);
        assert_eq!(c.get("msaa").as_deref(), Some("1"));
        assert_eq!(c.get("vsync").as_deref(), Some("0"));
        assert!(matches(&c, &levels[0]) && !matches(&c, &levels[5]));
    }

    #[test]
    fn names_are_checked() {
        assert!(valid_name("My level 1"));
        assert!(!valid_name("a/b") && !valid_name("  ") && !valid_name("..x"));
    }
}
