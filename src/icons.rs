//! Small vector icons drawn with the painter (no icon font needed), used on the big buttons.

use eframe::egui::{Color32, Painter, Pos2, Shape, Stroke, pos2};

#[derive(Clone, Copy)]
pub enum Icon {
    Play,
    Book,
    Sliders,
    Close,
    Clock,
    Bus,
    People,
    Fleet,
    Wheel,
    Gear,
    Cube,
    Puzzle,
}

/// Draws `icon` centred on `c`, fitting in a square of side `2 * r`.
pub fn paint(p: &Painter, icon: Icon, c: Pos2, r: f32, color: Color32) {
    let w = (r * 0.18).max(1.5);
    let stroke = Stroke::new(w, color);
    let line = |a: Pos2, b: Pos2| p.line_segment([a, b], stroke);
    match icon {
        Icon::Play => {
            let pts = vec![pos2(c.x - r * 0.55, c.y - r * 0.85), pos2(c.x - r * 0.55, c.y + r * 0.85), pos2(c.x + r * 0.85, c.y)];
            p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        Icon::Book => {
            let (l, rt, t, b) = (c.x - r * 0.9, c.x + r * 0.9, c.y - r * 0.7, c.y + r * 0.7);
            line(pos2(l, t), pos2(rt, t));
            line(pos2(l, b), pos2(rt, b));
            line(pos2(l, t), pos2(l, b));
            line(pos2(rt, t), pos2(rt, b));
            line(pos2(c.x, t), pos2(c.x, b));
            for k in [-0.25, 0.15] {
                line(pos2(l + r * 0.25, c.y + r * k), pos2(c.x - r * 0.25, c.y + r * k));
                line(pos2(c.x + r * 0.25, c.y + r * k), pos2(rt - r * 0.25, c.y + r * k));
            }
        }
        Icon::Sliders => {
            for (i, knob) in [-0.4_f32, 0.5, -0.1].into_iter().enumerate() {
                let y = c.y + r * (i as f32 - 1.0) * 0.7;
                line(pos2(c.x - r * 0.9, y), pos2(c.x + r * 0.9, y));
                p.circle_filled(pos2(c.x + r * knob, y), r * 0.26, color);
            }
        }
        Icon::Close => {
            line(pos2(c.x - r * 0.7, c.y - r * 0.7), pos2(c.x + r * 0.7, c.y + r * 0.7));
            line(pos2(c.x - r * 0.7, c.y + r * 0.7), pos2(c.x + r * 0.7, c.y - r * 0.7));
        }
        // Tutorial I: views, time and weather -> a clock
        Icon::Clock => {
            p.circle_stroke(c, r * 0.85, stroke);
            line(c, pos2(c.x, c.y - r * 0.55));
            line(c, pos2(c.x + r * 0.4, c.y + r * 0.2));
        }
        // Tutorial II: the bus and its passengers
        Icon::Bus => bus(p, c, r, color, stroke),
        // Tutorial III: passengers at the stops
        Icon::People => {
            for dx in [-0.55_f32, 0.0, 0.55] {
                let x = c.x + r * dx;
                p.circle_filled(pos2(x, c.y - r * 0.4), r * 0.22, color);
                line(pos2(x - r * 0.25, c.y + r * 0.6), pos2(x, c.y - r * 0.05));
                line(pos2(x + r * 0.25, c.y + r * 0.6), pos2(x, c.y - r * 0.05));
            }
        }
        // Tutorial IV: several buses
        // controls: a steering wheel
        Icon::Wheel => {
            p.circle_stroke(c, r * 0.9, stroke);
            p.circle_filled(c, r * 0.2, color);
            line(pos2(c.x - r * 0.9, c.y), pos2(c.x - r * 0.2, c.y));
            line(pos2(c.x + r * 0.2, c.y), pos2(c.x + r * 0.9, c.y));
            line(pos2(c.x, c.y + r * 0.2), pos2(c.x, c.y + r * 0.9));
        }
        // setup: a gear
        Icon::Gear => {
            p.circle_stroke(c, r * 0.55, stroke);
            p.circle_stroke(c, r * 0.2, stroke);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                let d = pos2(a.cos(), a.sin());
                line(pos2(c.x + d.x * r * 0.55, c.y + d.y * r * 0.55), pos2(c.x + d.x * r * 0.9, c.y + d.y * r * 0.9));
            }
        }
        // add-ons: a box
        Icon::Cube => {
            let pts = [pos2(c.x, c.y - r * 0.9), pos2(c.x + r * 0.8, c.y - r * 0.45), pos2(c.x + r * 0.8, c.y + r * 0.45), pos2(c.x, c.y + r * 0.9), pos2(c.x - r * 0.8, c.y + r * 0.45), pos2(c.x - r * 0.8, c.y - r * 0.45)];
            for i in 0..6 {
                line(pts[i], pts[(i + 1) % 6]);
            }
            line(c, pts[1]);
            line(c, pts[5]);
            line(c, pts[3]);
        }
        // mods: a puzzle piece
        Icon::Puzzle => {
            let (l, rt, t, b) = (c.x - r * 0.6, c.x + r * 0.6, c.y - r * 0.5, c.y + r * 0.7);
            line(pos2(l, t), pos2(rt, t));
            line(pos2(l, b), pos2(rt, b));
            line(pos2(l, t), pos2(l, b));
            line(pos2(rt, t), pos2(rt, b));
            p.circle_filled(pos2(c.x, t - r * 0.22), r * 0.26, color);
            p.circle_filled(pos2(rt + r * 0.22, c.y + r * 0.1), r * 0.26, color);
        }
        Icon::Fleet => {
            bus(p, pos2(c.x - r * 0.35, c.y - r * 0.35), r * 0.62, color, stroke);
            bus(p, pos2(c.x + r * 0.4, c.y + r * 0.4), r * 0.62, color, stroke);
        }
    }
}

fn bus(p: &Painter, c: Pos2, r: f32, color: Color32, stroke: Stroke) {
    let (l, rt, t, b) = (c.x - r * 0.85, c.x + r * 0.85, c.y - r * 0.7, c.y + r * 0.5);
    let line = |a: Pos2, b: Pos2| p.line_segment([a, b], stroke);
    line(pos2(l, t), pos2(rt, t));
    line(pos2(l, b), pos2(rt, b));
    line(pos2(l, t), pos2(l, b));
    line(pos2(rt, t), pos2(rt, b));
    line(pos2(l, c.y - r * 0.1), pos2(rt, c.y - r * 0.1)); // windows line
    for x in [-0.5_f32, 0.0, 0.5] {
        p.circle_filled(pos2(c.x + r * x, c.y - r * 0.4), r * 0.1, color);
    }
    for x in [-0.5_f32, 0.5] {
        p.circle_filled(pos2(c.x + r * x, b + r * 0.15), r * 0.2, color);
    }
}
