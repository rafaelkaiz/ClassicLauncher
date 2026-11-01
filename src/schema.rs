//! What the options window shows: openOMSI's own settings (`~/.openomsi/settings.cfg`), tab by tab, ported from the game's launcher
//! (`launcher/ui/js/pages/settings-schema.js`). A row names the key, how it is edited and when it is shown. Keys and values are the
//! ones the file holds, so a value picked here is written as it is.

pub type Opt = (&'static str, &'static str);

#[derive(Clone, Copy, PartialEq)]
pub enum Fmt {
    Pct,
    /// Percent, and 100 % reads "OMSI".
    Omsi,
    Deg,
    Mm,
    Cm,
    Int,
    OffMs,
    OffPct,
    /// A percentage already (0 .. 100) with "Off" at 0.
    OffPct100,
    Db,
    Metre,
    Secs,
    /// "Default" below 20, degrees above.
    Fov,
    /// "OMSI" below 45, degrees above.
    LockDeg,
    /// "Automatic" below 90, degrees above.
    AutoDeg,
    Num2,
    /// A value in seconds shown in milliseconds.
    MsOfSecs,
}

impl Fmt {
    pub fn show(self, v: f64) -> String {
        let t = crate::i18n::tr;
        match self {
            Fmt::Pct => format!("{:.0}%", v * 100.0),
            Fmt::Omsi => if (v - 1.0).abs() < 0.01 { t("OMSI").to_string() } else { format!("{:.0}%", v * 100.0) },
            Fmt::Deg => format!("{v:.0}°"),
            Fmt::Mm => format!("{v:.0} mm"),
            Fmt::Cm => format!("{:+.0} cm", v * 100.0),
            Fmt::Int => format!("{v:.0}"),
            Fmt::OffMs => if v <= 0.0 { t("Off").to_string() } else { format!("{v:.0} ms") },
            Fmt::OffPct => if v <= 0.0 { t("Off").to_string() } else { format!("{:.0}%", v * 100.0) },
            Fmt::OffPct100 => if v <= 0.0 { t("Off").to_string() } else { format!("{v:.0}%") },
            Fmt::Db => format!("{v:.0} dB"),
            Fmt::Metre => format!("{v:.0} m"),
            Fmt::Secs => format!("{v:.1} s"),
            Fmt::Fov => if v < 20.0 { t("Default").to_string() } else { format!("{v:.0}°") },
            Fmt::LockDeg => if v < 45.0 { t("OMSI").to_string() } else { format!("{v:.0}°") },
            Fmt::AutoDeg => if v < 90.0 { t("Automatic").to_string() } else { format!("{v:.0}°") },
            Fmt::Num2 => format!("{v:.2}"),
            Fmt::MsOfSecs => format!("{:.0} ms", v * 1000.0),
        }
    }
}

/// When a row is shown, from the values of the other rows.
#[derive(Clone, Copy, PartialEq)]
pub enum When {
    Always,
    /// Anything but the "vanilla" graphics mode.
    NotClassic,
    Enhanced,
    NotTraced,
    NotClassicNotTraced,
    EnhancedNotTraced,
    EnhancedClouds,
    ReflectionsEnhanced,
    ReflectionsPlain,
    Fsr,
    Voice,
    VoiceOpen,
    HeadTracking,
    HeadTrackingTobii,
    Triple,
    Vr,
}

#[derive(Clone)]
pub enum Kind {
    Toggle { def: bool },
    Select { def: &'static str, options: &'static [Opt] },
    Slider { min: f64, max: f64, step: f64, def: f64, fmt: Fmt },
    /// The languages OMSI and the launcher have.
    Language,
    Section,
    Note,
    /// A block the options window draws itself (quality preset, profiles, radio stations...).
    Custom(&'static str),
    /// A button that does something (reset, go to the controllers...); `label` is its text.
    Action(&'static str),
}

#[derive(Clone)]
pub struct Row {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    pub when: When,
    pub help: &'static str,
    /// A slider value below this is stored as 0 (the field of view's "Default").
    pub zero_below: f64,
}

impl Row {
    fn new(key: &'static str, label: &'static str, kind: Kind) -> Row {
        Row { key, label, kind, when: When::Always, help: "", zero_below: 0.0 }
    }
    fn w(mut self, when: When) -> Row {
        self.when = when;
        self
    }
    fn h(mut self, help: &'static str) -> Row {
        self.help = help;
        self
    }
    fn z(mut self, below: f64) -> Row {
        self.zero_below = below;
        self
    }
}

pub struct Tab {
    pub id: &'static str,
    pub label: &'static str,
    pub cols: Vec<Column>,
}

pub struct Column {
    pub title: &'static str,
    pub rows: Vec<Row>,
}

fn tog(key: &'static str, label: &'static str, def: bool) -> Row {
    Row::new(key, label, Kind::Toggle { def })
}
fn sel(key: &'static str, label: &'static str, def: &'static str, options: &'static [Opt]) -> Row {
    Row::new(key, label, Kind::Select { def, options })
}
fn sli(key: &'static str, label: &'static str, min: f64, max: f64, step: f64, def: f64, fmt: Fmt) -> Row {
    Row::new(key, label, Kind::Slider { min, max, step, def, fmt })
}
fn sec(title: &'static str) -> Row {
    Row::new("", title, Kind::Section)
}
fn custom(id: &'static str) -> Row {
    Row::new("", id, Kind::Custom(id))
}
fn action(id: &'static str, label: &'static str) -> Row {
    Row::new("", label, Kind::Action(id))
}
fn note(text: &'static str) -> Row {
    Row::new("", text, Kind::Note)
}
fn vol(key: &'static str, label: &'static str, def: f64, max: f64) -> Row {
    sli(key, label, 0.0, max, 0.05, def, Fmt::Pct)
}
fn track(key: &'static str, label: &'static str) -> Row {
    sli(key, label, 0.0, 100.0, 1.0, 100.0, Fmt::OffPct100)
}

const RESOLUTIONS: &[Opt] = &[
    ("auto", "Automatic"), ("1280x720", "1280 x 720"), ("1280x800", "1280 x 800 (Steam Deck)"), ("1366x768", "1366 x 768"),
    ("1600x900", "1600 x 900"), ("1920x1080", "1920 x 1080"), ("1920x1200", "1920 x 1200"), ("2560x1440", "2560 x 1440"), ("3840x2160", "3840 x 2160"),
];
const VIDEO_MEMORY: &[Opt] = &[
    ("0", "Automatic"), ("1000", "1 GB"), ("1500", "1.5 GB"), ("2000", "2 GB"), ("3000", "3 GB"), ("4000", "4 GB"), ("6000", "6 GB"),
    ("8000", "8 GB"), ("12000", "12 GB"), ("16000", "16 GB"),
];
const ON_OFF_AUTO: &[Opt] = &[("auto", "Auto"), ("on", "On"), ("off", "Off")];
const HIGH_LOW: &[Opt] = &[("high", "High"), ("low", "Low")];
const HIGH_MED_LOW: &[Opt] = &[("high", "High"), ("medium", "Medium"), ("low", "Low")];
const PUDDLES: &[Opt] = &[("high", "High"), ("low", "Low"), ("off", "Off")];

fn graphics() -> Tab {
    Tab {
        id: "graphics",
        label: "Graphics",
        cols: vec![
            Column {
                title: "Graphics",
                rows: vec![
                    sel("graphics", "Graphics", "vanilla_plus", &[("vanilla", "Vanilla (as OMSI 2)"), ("vanilla_plus", "Vanilla+"), ("enhanced", "Enhanced")]),
                    tog("ray_tracing", "Real-Time Ray Tracing", false)
                        .h("Enhanced: the sun's shadows, the ambient occlusion and the reflections traced with hardware ray queries, where the card has them; the heaviest option (takes effect when the game starts the next time)")
                        .w(When::Enhanced),
                    custom("quality"),
                    custom("optimise"),
                    sel("msaa", "Anti-aliasing", "4", &[("1", "Off"), ("2", "2x MSAA"), ("4", "4x MSAA"), ("8", "8x MSAA")]),
                    sel("upscaler", "Upscaling", "default", &[("default", "Standard (Catmull-Rom)"), ("fsr1", "AMD FSR 1 (experimental)")]).h("Standard Catmull-Rom or experimental FSR 1; changes show at once"),
                    sel("fsr_quality", "FSR quality", "quality", &[("ultra", "Ultra Quality (77%)"), ("quality", "Quality (67%)"), ("balanced", "Balanced (59%)"), ("performance", "Performance (50%)"), ("auto", "Automatic (keeps the frame rate)"), ("custom", "Use render scale")]).w(When::Fsr),
                    sli("fsr_sharpness", "FSR sharpness", 0.0, 1.0, 0.05, 0.8, Fmt::Pct).h("RCAS sharpening strength").w(When::Fsr),
                    sel("render_scale", "Render scale", "auto", &[("auto", "Auto"), ("1", "100%"), ("0.85", "85%"), ("0.75", "75%"), ("0.67", "67%"), ("0.5", "50%")]),
                    sel("anisotropy", "Anisotropic", "8", &[("1", "Off"), ("2", "2x"), ("4", "4x"), ("8", "8x"), ("16", "16x")]),
                    sel("shadow_size", "Shadow map", "2048", &[("1024", "1024"), ("2048", "2048"), ("4096", "4096")]).w(When::NotClassic),
                    tog("ssao", "Ambient occlusion", true).w(When::NotClassicNotTraced),
                    tog("shadows", "Sun shadows", true).w(When::NotClassicNotTraced),
                    sel("shadow_casters", "Shadows cast by", "all", &[("all", "Every solid mesh"), ("omsi", "[shadow] meshes, as OMSI")]).w(When::NotClassic),
                    tog("detail_textures", "Detail texturing up close", true).w(When::NotClassic),
                    sli("night_brightness", "Night brightness", 0.0, 3.0, 0.25, 0.0, Fmt::Num2).w(When::Enhanced),
                    sli("led_glow", "LED glow", 0.0, 15.0, 1.0, 6.0, Fmt::Int).w(When::Enhanced),
                    sli("led_mips", "LED mip strength", 0.0, 4.0, 0.05, 1.3, Fmt::Num2).w(When::Enhanced),
                    tog("shadow_blobs", "OMSI's shadow meshes (under vehicles)", true),
                    sli("reflections", "Reflective objects reflect", 0.0, 1.0, 0.05, 1.0, Fmt::OffPct).w(When::NotTraced),
                    tog("clouds", "Clouds", true),
                    sel("cloud_quality", "Cloud quality", "high", HIGH_LOW).w(When::EnhancedClouds),
                    sel("street_light", "Street light", "off", ON_OFF_AUTO).w(When::EnhancedNotTraced),
                    sel("light_shadows", "Light has shadows", "street", &[("off", "Off"), ("street", "Street lamps and vehicles"), ("all", "Every light")]).w(When::Enhanced),
                    tog("cloud_shadows", "Cloud shadows and light shafts", false).w(When::Enhanced),
                    tog("light_glow", "Glow of lights", false).w(When::Enhanced),
                    sel("rain_quality", "Rain quality", "high", HIGH_MED_LOW),
                    sel("vehicle_smoke", "Vehicle smoke", "all", &[("all", "All vehicles"), ("player", "Player only"), ("off", "Off")]).h("Controls the vehicles' smoke particles, not scenery effects or glowing particles."),
                    sel("puddles", "Puddles", "high", PUDDLES).w(When::ReflectionsPlain),
                    sel("puddles", "Real-Time Reflections", "high", PUDDLES).w(When::ReflectionsEnhanced),
                    tog("windy_trees", "Windy trees", true),
                    tog("road_snow", "Built-up snow on roads and roofs", true).h("While it snows, snow builds up on the roads and on the vehicles' roofs, with ruts and tyre tracks, and thaws again; off: the weather's own snow on road, as in OMSI 2").w(When::NotClassic),
                ],
            },
            Column {
                title: "Display",
                rows: vec![
                    tog("fullscreen", "Fullscreen", false),
                    sel("resolution", "Window size", "auto", RESOLUTIONS),
                    tog("vsync", "V-sync", true),
                    tog("sim_thread", "Simulation on its own thread", false),
                    sel("max_fps", "Frame limit", "0", &[("0", "Screen refresh rate"), ("30", "30 fps"), ("45", "45 fps"), ("60", "60 fps"), ("120", "120 fps"), ("144", "144 fps"), ("1000", "Unlimited")]),
                    sel("graphics_api", "Graphics API", "auto", &[("auto", "Automatic"), ("vulkan", "Vulkan"), ("dx12", "DirectX 12"), ("gl", "OpenGL"), ("angle", "ANGLE (DirectX 11)")]),
                    sec("World & memory"),
                    sel("view_distance", "View distance", "auto", &[("auto", "Default (1200 m)"), ("600", "600 m - fastest"), ("900", "900 m"), ("1200", "1200 m"), ("1500", "1500 m"), ("2000", "2000 m"), ("2500", "2500 m")]),
                    sel("max_obj_dist", "Object distance", "auto", &[("auto", "Automatic"), ("500", "500 m"), ("750", "750 m"), ("900", "900 m"), ("1500", "1500 m"), ("3000", "3000 m")]),
                    sel("min_obj_size", "Small objects", "0.013", &[("0.005", "All"), ("0.013", "Normal"), ("0.02", "Fewer (faster)"), ("0.03", "Few (fastest)")]),
                    sel("mirror_size", "Mirrors", "256", &[("0", "Off"), ("128", "Low (128)"), ("256", "Normal (256)"), ("512", "High (512)"), ("1024", "Very high (1024)")]),
                    sel("mirror_refresh", "Bus mirror refresh", "full", &[("off", "None (frozen picture)"), ("eco", "Economical"), ("full", "Full"), ("every", "Every frame (smoothest)")]),
                    sel("mirror_quality", "Mirror quality", "standard", &[("standard", "Standard"), ("enhanced", "As the picture (Enhanced)")]).w(When::Enhanced),
                    tog("mirror_hud_every_frame", "Mirror panels every frame", false),
                    tog("mirror_shadows", "Shadows in the mirrors", false),
                    tog("mirror_particles", "Smoke and spray in the mirrors", false),
                    sel("video_memory", "Video memory", "0", VIDEO_MEMORY).h("Everything the game keeps on the graphics card: textures, models and the picture itself. Automatic is nine tenths of the card."),
                    tog("texture_compression", "Compress textures on loading", true),
                    tog("gpu_texture_compression", "DXT/BC textures stay compressed on the GPU", true),
                    sec("Profiles"),
                    custom("profiles"),
                ],
            },
        ],
    }
}

fn driving() -> Tab {
    Tab {
        id: "driving",
        label: "Driving",
        cols: vec![
            Column {
                title: "Keyboard & mouse",
                rows: vec![
                    sel("drive_keys", "Driving keys", "simple", &[("omsi", "Custom controls (Controls page)"), ("simple", "W A S D + arrows"), ("wasd", "W A S D only"), ("arrows", "Arrow keys only")]),
                    tog("steering_linear", "Steering linearity (keys at OMSI's steady pace)", false),
                    tog("old_steering", "Old Steering (the wheel stays, turn it back yourself)", false),
                    tog("red_steer_spd", "Dynamic steering (slower keys at speed, OMSI's redSteerSpd)", false),
                    sli("mouse_sens", "Mouse steering sensitivity (O)", 0.1, 3.0, 0.05, 1.0, Fmt::Omsi),
                    sli("mouse_pedal_strength", "Mouse pedal strength", 0.5, 2.0, 0.05, 1.0, Fmt::Omsi),
                    tog("mouse_smooth", "Smooth mouse steering (off: the wheel follows the cursor at once, as in OMSI)", true),
                    tog("mouse_pedals", "The mouse works the pedals", true).h("Up the throttle, down the brake, as in OMSI; off: the mouse only steers, the pedals stay on the keyboard"),
                    tog("mouse_hold", "Hold the cursor while the mouse steers", true),
                    tog("mouse_right_off", "A right click ends the mouse steering (as in OMSI)", false),
                    tog("blinker_cancel", "Indicators cancel themselves (as the bus's script does)", true),
                    tog("brake_hold", "The keyboard brake stays on until the throttle (as in OMSI)", true),
                    sli("key_pedal_speed", "Pedal keys speed (100 % = OMSI)", 0.25, 2.0, 0.05, 1.0, Fmt::Omsi),
                    tog("auto_clutch", "Automatic clutch (manual gearboxes)", true),
                    tog("auto_shift", "Automated manual gearbox (shifts a manual gearbox for you by the engine speed)", false),
                ],
            },
            Column {
                title: "Game controllers",
                rows: vec![
                    action("configure_wheel", "Set up a wheel or pedals"),
                    sec("Wheel and pedals"),
                    sli("wheel_range", "Wheel rotation", 180.0, 1800.0, 30.0, 900.0, Fmt::Deg),
                    sli("wheel_lock", "Full lock at", 0.0, 1800.0, 30.0, 0.0, Fmt::LockDeg).z(45.0),
                    sli("cab_wheel_rotation", "Cab wheel rotation", 0.0, 1800.0, 30.0, 0.0, Fmt::AutoDeg).z(90.0).h("How far the steering wheel in the cab turns, lock to lock. Automatic = as the bus's own model; set it to your wheel's rotation so both turn the same. Some buses animate their wheel with a script of their own and keep its rotation."),
                    sli("ctrl_deadzone", "Dead zone", 0.0, 0.3, 0.01, 0.0, Fmt::OffPct),
                    sli("pedal_throttle", "Throttle pedal strength", 0.5, 2.0, 0.05, 1.0, Fmt::Omsi),
                    sli("pedal_brake", "Brake pedal strength", 0.5, 2.0, 0.05, 1.0, Fmt::Omsi),
                    tog("momentary_gears", "H-pattern shifter: return to neutral when the gear is released", false),
                    tog("arrows_switch_cams", "Arrow keys switch the cameras with a wheel too (no glance)", true),
                    tog("ctrl_background", "Keep driving in the background", false),
                    sec("Gamepad"),
                    sel("pad_type", "Gamepad type", "auto", &[("auto", "Automatic"), ("xbox", "Xbox"), ("ps4", "PlayStation 4"), ("ps5", "PlayStation 5")]),
                    tog("pad_buttons", "Default gamepad buttons (indicators, doors, views, gears, menu, pause)", true),
                    sli("pad_steer_speed", "Stick steering speed", 0.8, 5.0, 0.1, 2.0, Fmt::Secs),
                    sli("pad_steer_smooth", "Stick steering smoothing", 0.0, 300.0, 10.0, 120.0, Fmt::OffMs),
                    sli("pad_deadzone", "Stick dead zone", 0.0, 0.4, 0.01, 0.08, Fmt::Pct),
                    tog("pad_steer_linear", "Stick steers like a wheel (a wheel seen as a gamepad)", false),
                    sec("Force feedback"),
                    tog("ff_enabled", "Force feedback and vibration", true).h("Off: the game sends the wheel no forces; any resistance left comes from the wheel's own driver software"),
                    tog("ff_invert", "Invert force feedback by default", false),
                    note("Wheels with a saved direction use their own setting in Controllers."),
                    sli("ff_road_vib", "Road texture vibration", 0.0, 4.0, 0.05, 1.0, Fmt::OffPct),
                    sli("ff_engine_vib", "Engine vibration", 0.0, 4.0, 0.05, 1.0, Fmt::OffPct).h("The running engine buzzes in the wheel, felt even at a standstill"),
                    sli("ff_fade", "Vibration fade-out", 0.0, 1.5, 0.05, 0.28, Fmt::Num2),
                    action("reset_wheel", "Reset wheel settings"),
                ],
            },
        ],
    }
}

fn camera() -> Tab {
    Tab {
        id: "camera",
        label: "Camera",
        cols: vec![
            Column {
                title: "Driver's view",
                rows: vec![
                    sec("Seat position"),
                    sli("seat_y", "Seat forward / back", -0.6, 0.6, 0.01, 0.0, Fmt::Cm),
                    sli("seat_z", "Seat up / down", -0.6, 0.6, 0.01, 0.0, Fmt::Cm),
                    sli("seat_x", "Seat right / left", -0.6, 0.6, 0.01, 0.0, Fmt::Cm),
                    sli("seat_pitch_deg", "Head pitch", -45.0, 45.0, 1.0, 0.0, Fmt::Deg),
                    action("reset_seat", "Reset the seat position"),
                    sli("fov", "Field of view", 0.0, 120.0, 1.0, 0.0, Fmt::Fov).z(20.0),
                    sli("look_sens", "Mouse look sensitivity", 0.1, 2.0, 0.05, 1.0, Fmt::Omsi),
                    tog("right_stick_look", "Right stick turns the view", true),
                    sli("look_smoothing_ms", "Smooth the mouse look", 0.0, 200.0, 10.0, 0.0, Fmt::OffMs),
                    sec("A head at rest"),
                    sli("head_idle", "Head sway at a standstill", 0.0, 1.0, 0.05, 0.0, Fmt::OffPct),
                    sli("head_idle_pace", "Sway pace", 0.5, 2.0, 0.05, 1.0, Fmt::Pct),
                    tog("steer_look", "Driver's view turns with the steering", false),
                    sli("steer_look_angle", "Steering view angle", 0.0, 60.0, 1.0, 30.0, Fmt::Deg),
                    sli("steer_look_response", "Steering view response", 0.05, 1.0, 0.05, 0.25, Fmt::MsOfSecs),
                    tog("head_movement", "Head moves with the bus", true),
                    tog("driverview_smooth", "Camera glides between viewpoints", true),
                    tog("hands_in_cab", "Driver's hands in the cab view", false),
                    tog("alt_view", "Right mouse button turns the view, Shift+right zooms (off: right zooms as in OMSI, the wheel button turns)", true),
                    sel("mirror_hud", "Mirror panels on the screen", "0", &[("0", "Off"), ("2", "Left mirror"), ("1", "Right mirror"), ("3", "Left and right"), ("7", "Left, right and front")]),
                    tog("precision_zoom", "Precision mouse zoom (FOV curve instead of the linear way)", false),
                ],
            },
            Column {
                title: "Outside views",
                rows: vec![
                    tog("camera_collision", "Camera collisions (outside view)", true),
                    tog("driver", "Driver at the wheel (outside views)", true),
                    sec("Head tracking"),
                    tog("head_tracking", "Head tracking (native TrackIR / OpenTrack)", false),
                    sel("head_tracking_source", "Head tracking with", "auto", &[("auto", "Automatic"), ("tobii", "Tobii eye tracker"), ("opentrack", "opentrack (UDP 4242)")]).w(When::HeadTracking),
                    tog("tobii_edges", "Look further at the screen's edges (Tobii)", false).w(When::HeadTrackingTobii),
                    sec("Rotation"),
                    track("head_tracking_yaw_sens", "Yaw sensitivity (left / right)"),
                    track("head_tracking_pitch_sens", "Pitch sensitivity (up / down)"),
                    track("head_tracking_roll_sens", "Roll sensitivity"),
                    tog("head_tracking_invert_yaw", "Invert yaw", false),
                    tog("head_tracking_invert_pitch", "Invert pitch", false),
                    tog("head_tracking_invert_roll", "Invert roll", false),
                    sec("Position"),
                    track("head_tracking_x_sens", "X sensitivity (left / right)"),
                    track("head_tracking_y_sens", "Y sensitivity (up / down)"),
                    track("head_tracking_z_sens", "Z sensitivity (forward / back)"),
                    tog("head_tracking_invert_x", "Invert X", false),
                    tog("head_tracking_invert_y", "Invert Y", false),
                    tog("head_tracking_invert_z", "Invert Z", false),
                    action("reset_tracking", "Reset head-tracking axes"),
                    sec("Triple screen"),
                    tog("triple_screen", "Three screen projections", false),
                    tog("triple_span", "Span three monitors at startup", true),
                    tog("triple_hud_center", "HUD on centre screen", true),
                    note("Three equal screens in a horizontal row. OpenXR takes priority.").w(When::Triple),
                    sli("triple_width_mm", "Visible width of one panel", 200.0, 2000.0, 10.0, 600.0, Fmt::Mm).w(When::Triple),
                    sli("triple_distance_mm", "Eye to centre screen", 200.0, 3000.0, 10.0, 650.0, Fmt::Mm).w(When::Triple),
                    sli("triple_bezel_mm", "Both frames at each join", 0.0, 100.0, 1.0, 0.0, Fmt::Mm).w(When::Triple),
                    sli("triple_left_angle_deg", "Left screen inward angle", 0.0, 90.0, 1.0, 45.0, Fmt::Deg).w(When::Triple),
                    sli("triple_right_angle_deg", "Right screen inward angle", 0.0, 90.0, 1.0, 45.0, Fmt::Deg).w(When::Triple),
                    sli("triple_eye_height_mm", "Eye above screen centre", -500.0, 500.0, 1.0, 0.0, Fmt::Mm).w(When::Triple),
                    sec("Virtual reality"),
                    tog("vr", "Use OpenXR headset", false),
                    sel("vr_scale", "Eye resolution", "0.65", &[("0.5", "50%"), ("0.65", "65%"), ("0.8", "80%"), ("1", "100%")]).w(When::Vr),
                    sel("vr_head_smoothing_ms", "Head tracking smoothing", "0", &[("0", "Off"), ("5", "5 ms"), ("10", "10 ms"), ("20", "20 ms"), ("30", "30 ms")]).w(When::Vr),
                    sel("vr_mirror_rate", "Bus mirror refresh", "16", &[("0", "Off"), ("8", "8/s"), ("16", "16/s"), ("24", "24/s"), ("32", "32/s"), ("48", "48/s"), ("60", "60/s"), ("90", "90/s"), ("120", "120/s"), ("180", "180/s"), ("240", "240/s"), ("360", "360/s"), ("-1", "Every frame")]).w(When::Vr),
                    tog("vr_desktop_mirror", "Show headset picture on monitor", true).w(When::Vr),
                ],
            },
        ],
    }
}

fn sound() -> Tab {
    Tab {
        id: "sound",
        label: "Sound",
        cols: vec![Column {
            title: "Volume",
            rows: vec![
                vol("volume", "Volume", 0.6, 1.0),
                vol("vol_ai", "Traffic", 1.0, 1.0),
                vol("vol_scenery", "Surroundings", 1.0, 1.0),
                tog("doppler", "Doppler effect", true),
                tog("ambient", "Ambience (places, weather, nature, town)", true),
                vol("vol_ambient", "Ambience volume", 0.8, 1.0),
                sel("pax_voices", "Passenger voices", "all", &[("all", "Greetings and tickets"), ("tickets", "Only the ticket asked for"), ("off", "Silent")]),
                vol("radio_speaker", "Radio: small speaker", 0.0, 1.0),
                vol("radio_hiss", "Radio: FM hiss", 0.0, 1.0),
                vol("radio_dropouts", "Radio: signal dropouts", 0.0, 1.0),
                sec("Voice chat in multiplayer"),
                tog("voice_enabled", "Voice chat", false).h("Hold a talk key to speak: Nearby (V) is heard by players close to you, Radio (B) by everybody in the session. The keys are in Controls. The system asks for the microphone the first time. Nothing is recorded or stored; the microphone is used only while you hold a talk key (or always, in the open microphone mode)."),
                sel("voice_mode", "Microphone", "ptt", &[("ptt", "Push to talk"), ("open", "Open microphone (nearby)")]).w(When::Voice),
                sli("voice_mic_gain", "Microphone gain", 0.0, 4.0, 0.1, 1.0, Fmt::Pct).w(When::Voice),
                sli("voice_threshold_db", "Open microphone: speech threshold", -80.0, -10.0, 1.0, -45.0, Fmt::Db).w(When::VoiceOpen),
                vol("voice_volume", "Voices: volume", 1.0, 2.0).w(When::Voice),
                vol("voice_volume_nearby", "Voices: nearby players", 1.0, 2.0).w(When::Voice),
                vol("voice_volume_radio", "Voices: radio", 1.0, 2.0).w(When::Voice),
                sli("voice_range", "Nearby voice range when you host", 10.0, 300.0, 5.0, 75.0, Fmt::Metre).w(When::Voice),
                tog("voice_talker_list", "Show who is talking", true).w(When::Voice),
                tog("voice_all_off", "Everybody else's voice chat off", false).w(When::Voice),
            ],
        },
        Column {
            title: "Radio stations",
            rows: vec![
                note("A radio's station button n plays the n-th station, a cassette player the first; Shift+R steps through them. An address is an MP3, AAC or Ogg stream or an .m3u/.pls playlist."),
                custom("radio"),
            ],
        }],
    }
}

fn gameplay() -> Tab {
    Tab {
        id: "gameplay",
        label: "Gameplay",
        cols: vec![
            Column {
                title: "Passengers",
                rows: vec![
                    sel("boarding", "Boarding", "auto", &[("auto", "Pay and take the ticket"), ("pay", "The driver sells the ticket"), ("walk", "Just walk in")]),
                    sel("standing_chance", "Standing passengers", "5", &[("0", "Never"), ("5", "5%"), ("10", "10%"), ("25", "25%"), ("50", "50%"), ("100", "Always")]),
                    sel("passenger_animation", "Passenger animations", "original", &[("original", "Original OMSI"), ("enhanced", "Natural (procedural)")]),
                    sel("character_models", "Character models", "original", &[("original", "Original"), ("improved", "High quality (improved)")]).h("High quality draws the people with the Microsoft Rocketbox avatars and their living faces, on this computer only: servers and other players keep their own. They are downloaded once (about 310 MB)."),
                    tog("exact_fare", "Passengers pay the exact fare", true),
                    tog("pax_bad_driving", "Passengers react to bad driving", true),
                    tog("passenger_hailing", "Passengers signal approaching buses", false),
                    tog("auto_announce", "Arrival announcements at the stop", false).h("On depot files that keep two recordings per stop (the Chinese maps), the arrival one is played as the bus reaches the stop: the IBIS's next stop key, pressed as the driver would"),
                    sli("pax_density", "How many passengers", 0.0, 2.0, 0.1, 1.0, Fmt::Pct),
                    tog("get_up", "Ability to get up (Ctrl+Shift+G)", false),
                    tog("full_tank", "Start with a full tank", true),
                    tog("trip_summary", "Trip summary", true),
                    sec("Traffic"),
                    sel("ai_unsched_factor", "Random traffic", "100", &[("25", "25%"), ("50", "50%"), ("75", "75%"), ("100", "100%"), ("150", "150%"), ("200", "200%")]),
                    sel("ai_max_scheduled", "Timetable vehicles", "0", &[("0", "All"), ("10", "At most 10"), ("25", "At most 25"), ("50", "At most 50")]),
                    sel("ai_max_parked", "Parked cars", "0", &[("-1", "None"), ("0", "Every space"), ("35", "At most 35"), ("100", "At most 100"), ("250", "At most 250")]),
                    sel("ai_max_humans", "People at once", "200", &[("50", "50"), ("100", "100"), ("200", "200"), ("400", "400"), ("720", "720"), ("1000", "1000")]),
                    tog("ai_wait_timed_stops_only", "Timetable buses ahead of time wait only at timed stops", false),
                ],
            },
            Column {
                title: "Simulation",
                rows: vec![
                    sel("maintenance", "Maintenance", "0", &[("0", "Infinite (no wear)"), ("1", "Very bad"), ("2", "Bad"), ("3", "Normal"), ("4", "Good")]),
                    tog("collision_vehicles", "Collisions with vehicles", true),
                    tog("collision_objects", "Collisions with objects (walls, poles)", true),
                    tog("collision_pedestrians", "Collisions with people", true),
                    tog("use_real_time", "Start at the real time", false),
                    tog("use_real_date", "Start on today's date", false),
                    tog("time_sync", "Sync the clock with the real time (locks the time)", false),
                    tog("metar_sync", "Sync the weather with METAR (locks the weather)", false),
                    sel("time_speed", "Time speed (not in multiplayer or with the real-time sync)", "1", &[("1", "Real time"), ("2", "x2"), ("4", "x4"), ("8", "x8"), ("15", "x15"), ("30", "x30")]),
                ],
            },
        ],
    }
}

fn general() -> Tab {
    Tab {
        id: "general",
        label: "General",
        cols: vec![
            Column {
                title: "Interface & online",
                rows: vec![
                    Row::new("language", "Language", Kind::Language),
                    tog("discord_status", "Discord Rich Presence", true).h("Shows the launcher or your map, bus, line and multiplayer status in Discord."),
                    tog("disable_plugins", "Disable plugins", false).h("Starts the next game without Lua or legacy OMSI plugins, including vehicle plugins."),
                    sli("ui_scale", "Game interface size", 0.5, 2.0, 0.05, 1.0, Fmt::Pct),
                    tog("ui_scale_window", "Interface grows with the window", true),
                    sli("ui_opacity", "Interface opacity", 0.2, 1.0, 0.05, 0.85, Fmt::Pct),
                    tog("tooltips", "Name of the button under the mouse", true),
                    tog("show_fps", "Frame rate in the corner", false),
                    sel("perf_overlay", "Performance details", "off", &[("off", "Off"), ("compact", "Compact"), ("detailed", "Detailed")]),
                    tog("notes", "Notes in the top-left corner", true),
                    tog("chat", "Chat in online games", true),
                    sli("chat_size", "Chat size", 0.5, 3.0, 0.1, 1.0, Fmt::Pct),
                    tog("name_tags", "Other players' names above their buses", true),
                    sel("photo_mode", "Photo mode", "simple", &[("simple", "Simple"), ("advanced", "Advanced (for photographers)")]).h("Simple: the lens, the look and the frame. Advanced: a camera body - sensor size, P/A/S/M exposure with ISO and metering, white balance in kelvin, tilt-shift, histogram, focus peaking, bracketing and 16-bit PNG."),
                    sec("Navigator"),
                    tog("navigator", "Navigator (Shift+N: map, schedule, off)", true),
                    tog("nav_arrows", "Route arrows (as in OMSI 2)", false),
                    tog("nav_ai", "AI vehicles on the map", true),
                    tog("nav_surface", "Map surface under the roads", true),
                    tog("nav_classic_2d", "Classic 2D navigator (lightweight)", false),
                    tog("nav_3d", "3D buildings on the map", true),
                    tog("nav_widget", "Navigator as a widget (drag it anywhere, size it at its corner)", true),
                    sli("nav_size", "Navigator size", 0.6, 2.0, 0.05, 1.0, Fmt::Pct),
                    custom("corner"),
                ],
            },
            Column {
                title: "Maintenance",
                rows: vec![
                    sec("Reset"),
                    action("reset_all", "Reset all settings..."),
                    sec("Support"),
                    action("support_pack", "Support package"),
                ],
            },
        ],
    }
}

pub fn tabs() -> Vec<Tab> {
    vec![graphics(), driving(), camera(), sound(), gameplay(), general()]
}

/// Every text the schema shows, for the translation template.
pub fn labels() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = vec![];
    for t in tabs() {
        v.push(t.label);
        for c in t.cols {
            v.push(c.title);
            for r in c.rows {
                if matches!(r.kind, Kind::Custom(_)) {
                    continue;
                }
                v.push(r.label);
                if !r.help.is_empty() {
                    v.push(r.help);
                }
                if let Kind::Select { options, .. } = r.kind {
                    v.extend(options.iter().map(|o| o.1));
                }
            }
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_select_default_is_one_of_its_options() {
        for t in tabs() {
            for c in t.cols {
                for r in c.rows {
                    if let Kind::Select { def, options } = r.kind {
                        assert!(options.iter().any(|o| o.0 == def), "{}: default {def} is not an option", r.key);
                    }
                }
            }
        }
    }
}

/// Every key the options window edits (the ones a saved preset holds).
pub fn keys() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = vec![];
    for t in tabs() {
        for c in t.cols {
            for r in c.rows {
                if !r.key.is_empty() && !v.contains(&r.key) {
                    v.push(r.key);
                }
            }
        }
    }
    v
}

/// The rows of the given settings, taken out of the tabs (for the Controls page, which shows some of them beside the devices).
pub fn rows_of(keys: &[&str]) -> Vec<Row> {
    let mut out: Vec<Row> = tabs().into_iter().flat_map(|t| t.cols).flat_map(|c| c.rows).filter(|r| keys.contains(&r.key)).collect();
    out.sort_by_key(|r| keys.iter().position(|k| *k == r.key));
    out
}
