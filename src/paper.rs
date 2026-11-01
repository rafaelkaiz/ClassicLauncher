//! The sheet of squared paper the staff file is written on, like the one OMSI shows: drawn here, so no OMSI picture is needed.

use eframe::egui::{self, Color32, Mesh, Painter, Pos2, Rect, Stroke, vec2};

/// Size of the sheet at scale 1.
pub const WIDTH: f32 = 440.0;
pub const HEIGHT: f32 = 700.0;

/// Heights (from the top of the sheet) of the punched holes on its left edge.
const HOLES: [f32; 6] = [68.0, 190.0, 215.0, 432.0, 457.0, 581.0];

/// Paints the paper (squares, worn edges, specks and the binder holes) into `rect`; `k` is the scale.
pub fn paint(p: &Painter, rect: Rect, k: f32) {
    p.rect_filled(rect, 0.0, Color32::from_rgb(239, 232, 214));

    // the squares
    let step = 11.0 * k;
    let line = Stroke::new((1.0 * k).max(0.7), Color32::from_rgba_unmultiplied(168, 160, 138, 120));
    let mut x = rect.left() + step;
    while x < rect.right() {
        p.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], line);
        x += step;
    }
    let mut y = rect.top() + step;
    while y < rect.bottom() {
        p.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], line);
        y += step;
    }

    // worn, darker edges
    let edge = Color32::from_rgba_unmultiplied(120, 98, 56, 80);
    let none = Color32::from_rgba_unmultiplied(120, 98, 56, 0);
    let w = 16.0 * k;
    let mut mesh = Mesh::default();
    let mut band = |a: Pos2, b: Pos2, c: Pos2, d: Pos2| {
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(a, edge);
        mesh.colored_vertex(b, edge);
        mesh.colored_vertex(c, none);
        mesh.colored_vertex(d, none);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    };
    band(rect.left_top(), rect.left_bottom(), rect.left_bottom() + vec2(w, 0.0), rect.left_top() + vec2(w, 0.0));
    band(rect.right_top(), rect.right_bottom(), rect.right_bottom() - vec2(w, 0.0), rect.right_top() - vec2(w, 0.0));
    band(rect.left_top(), rect.right_top(), rect.right_top() + vec2(0.0, w), rect.left_top() + vec2(0.0, w));
    band(rect.left_bottom(), rect.right_bottom(), rect.right_bottom() - vec2(0.0, w), rect.left_bottom() - vec2(0.0, w));
    p.add(egui::Shape::mesh(mesh));

    // a few specks, always in the same places
    let mut seed: u32 = 0x2F6B_1A93;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / 16_777_216.0
    };
    for _ in 0..70 {
        let at = rect.left_top() + vec2(next() * rect.width(), next() * rect.height());
        let r = (0.5 + next() * 1.2) * k;
        p.circle_filled(at, r, Color32::from_rgba_unmultiplied(70, 55, 30, 70));
    }

    // the binder holes
    for h in HOLES {
        let c = rect.left_top() + vec2(26.0 * k, h * k);
        p.circle_filled(c, 8.5 * k, Color32::from_rgb(34, 28, 22));
        p.circle_stroke(c, 8.5 * k, Stroke::new(1.2 * k, Color32::from_rgb(150, 138, 118)));
        p.circle_filled(c + vec2(-1.5 * k, -1.5 * k), 3.6 * k, Color32::from_rgb(105, 95, 82));
    }
}
