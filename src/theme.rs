//! Colour themes: OMSI 2's own orange, a dark one and a light one.

use crate::i18n::tr;
use eframe::egui::{self, Color32};
use std::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Theme {
    Original,
    Dark,
    Light,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Original, Theme::Dark, Theme::Light];

    pub fn label(self) -> &'static str {
        match self {
            Theme::Original => tr("Original (OMSI 2 orange)"),
            Theme::Dark => tr("Dark"),
            Theme::Light => tr("Light"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Theme::Original => "original",
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    pub fn from_key(s: &str) -> Theme {
        Theme::ALL.into_iter().find(|t| t.key() == s).unwrap_or(Theme::Light)
    }

    fn palette(self) -> Palette {
        let c = Color32::from_rgb;
        match self {
            Theme::Original => Palette {
                bg: c(255, 170, 0),
                field: c(255, 222, 148),
                hover: c(255, 236, 188),
                line: c(255, 120, 16),
                text: c(150, 40, 0),
                border: c(150, 40, 0),
                dark: c(150, 40, 0),
                light: c(255, 150, 30),
            },
            Theme::Dark => Palette {
                bg: c(30, 30, 35),
                field: c(46, 46, 54),
                hover: c(64, 64, 76),
                line: c(150, 80, 14),
                text: c(228, 228, 234),
                border: c(96, 96, 108),
                dark: c(205, 105, 10),
                light: c(255, 255, 255),
            },
            Theme::Light => Palette {
                bg: c(244, 245, 247),
                field: c(255, 255, 255),
                hover: c(232, 236, 242),
                line: c(255, 160, 50),
                text: c(32, 36, 44),
                border: c(176, 182, 192),
                dark: c(205, 105, 10),
                light: c(255, 255, 255),
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub field: Color32,
    pub hover: Color32,
    /// The thick rules between the panels.
    pub line: Color32,
    pub text: Color32,
    pub border: Color32,
    /// Big buttons and selections.
    pub dark: Color32,
    /// Text on the big buttons.
    pub light: Color32,
}

static CURRENT: RwLock<Option<Palette>> = RwLock::new(None);

pub fn pal() -> Palette {
    CURRENT.read().ok().and_then(|p| *p).unwrap_or_else(|| Theme::Original.palette())
}

pub fn apply(ctx: &egui::Context, theme: Theme) {
    let p = theme.palette();
    if let Ok(mut w) = CURRENT.write() {
        *w = Some(p);
    }
    let mut v = if theme == Theme::Dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.bg;
    v.window_fill = p.bg;
    v.extreme_bg_color = p.field;
    v.hyperlink_color = if theme == Theme::Original { p.text } else { p.dark };
    // No global text colour override: selected items switch to white (selection.stroke) on the accent fill.
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, p.text);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.line);
    v.selection.bg_fill = p.dark;
    v.selection.stroke = egui::Stroke::new(1.0, Color32::WHITE);
    v.window_stroke = egui::Stroke::new(2.0, p.border);
    let stroke = egui::Stroke::new(1.0, p.border);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.bg_stroke = stroke;
        w.corner_radius = if theme == Theme::Original { 0.into() } else { 4.into() };
        w.fg_stroke = egui::Stroke::new(1.0, p.text);
    }
    v.widgets.inactive.weak_bg_fill = p.field;
    v.widgets.inactive.bg_fill = p.field;
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.active.weak_bg_fill = p.hover;
    if theme == Theme::Original {
        // The main window keeps OMSI 2's orange; the windows over it (options, pickers, dialogs) are a pale cream so they are not all
        // orange, and a chosen item is amber with the brown text still readable on it.
        v.window_fill = Color32::from_rgb(255, 248, 236);
        v.faint_bg_color = Color32::from_rgb(255, 240, 214);
        v.selection.bg_fill = Color32::from_rgb(255, 196, 70);
        v.selection.stroke = egui::Stroke::new(1.0, p.text);
    }
    ctx.set_visuals(v);
}
