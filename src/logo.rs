//! The launcher's logo: openOMSI's own wordmark (`assets/logos`, copied from the openOMSI project). Its "open" is black for a light
//! background and white for a dark one; on the original orange theme, where the orange OMSI would vanish, the black one-colour mark.

use crate::theme::{Palette, Theme};

/// The SVG text of the wordmark for `theme`.
pub fn svg(theme: Theme, _p: &Palette) -> String {
    match theme {
        Theme::Light => include_str!("../assets/logos/openomsi-wordmark-dark.svg"),
        Theme::Dark => include_str!("../assets/logos/openomsi-wordmark-light.svg"),
        Theme::Original => include_str!("../assets/logos/openomsi-wordmark-mono-dark.svg"),
    }
    .to_string()
}

/// The wordmark's shape: width over height.
pub const ASPECT: f32 = 1142.0 / 303.0;
