//! Opaque, CPU-canvas implementations of the graphical vocabulary shown on
//! the SaaiOS concept boards. Geometry is deterministic and every state has a
//! non-colour cue; blur, glow, and invented runtime values are intentionally
//! absent.

use saai_ui_core::{ColorRole, IconGlyph, IconSize, OrbVisualState, Rect, UniversalState};

use super::{draw_gallery_icon, state_color, theme_color, Canvas, Fonts, Icon, Pixel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrbVisual {
    pub bounds: Rect,
    pub state: OrbVisualState,
    /// Verified execution progress. `None` paints an indeterminate segmented
    /// ring; callers must not invent a percentage.
    pub progress: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AvatarVisual {
    pub bounds: Rect,
    pub status: Option<UniversalState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SliderVisual {
    pub bounds: Rect,
    pub value: u8,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceBarVisual {
    pub bounds: Rect,
    /// `None` is unavailable and paints an outlined track, never zero.
    pub value: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RingProgressVisual {
    pub bounds: Rect,
    /// `None` paints a segmented indeterminate cue.
    pub value: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectIconVisual {
    pub bounds: Rect,
    pub glyph: IconGlyph,
    pub color: ColorRole,
}

fn center(rect: Rect) -> (i32, i32) {
    (
        (rect.x + rect.width / 2) as i32,
        (rect.y + rect.height / 2) as i32,
    )
}

fn fill_disc(canvas: &mut Canvas<'_>, cx: i32, cy: i32, radius: i32, color: Pixel) {
    if radius <= 0 {
        return;
    }
    let inner = (radius - 1).max(0);
    let outer2 = radius * radius;
    let inner2 = inner * inner;
    for y in -radius..=radius {
        for x in -radius..=radius {
            let d2 = x * x + y * y;
            if d2 <= inner2 {
                canvas.blend(cx + x, cy + y, color, 255);
            } else if d2 <= outer2 {
                canvas.blend(cx + x, cy + y, color, 128);
            }
        }
    }
}

fn stroke_ring(canvas: &mut Canvas<'_>, cx: i32, cy: i32, radius: i32, width: i32, color: Pixel) {
    let outer2 = radius * radius;
    let inner = (radius - width.max(1)).max(0);
    let inner2 = inner * inner;
    for y in -radius..=radius {
        for x in -radius..=radius {
            let d2 = x * x + y * y;
            if d2 <= outer2 && d2 >= inner2 {
                canvas.blend(cx + x, cy + y, color, 255);
            }
        }
    }
}

fn line(canvas: &mut Canvas<'_>, from: (i32, i32), to: (i32, i32), width: i32, color: Pixel) {
    let (mut x, mut y) = from;
    let dx = (to.0 - x).abs();
    let sx = if x < to.0 { 1 } else { -1 };
    let dy = -(to.1 - y).abs();
    let sy = if y < to.1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        fill_disc(canvas, x, y, width.max(1) / 2, color);
        if (x, y) == to {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
}

fn arc(
    canvas: &mut Canvas<'_>,
    cx: i32,
    cy: i32,
    radius: i32,
    width: i32,
    percent: u8,
    color: Pixel,
) {
    let steps = (radius.max(1) * 6) as usize;
    let count = steps * usize::from(percent.min(100)) / 100;
    for step in 0..count {
        let angle =
            -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * step as f32 / steps as f32;
        let x = cx + (angle.cos() * radius as f32).round() as i32;
        let y = cy + (angle.sin() * radius as f32).round() as i32;
        fill_disc(canvas, x, y, width.max(1) / 2, color);
    }
}

fn diamond(canvas: &mut Canvas<'_>, cx: i32, cy: i32, radius: i32, color: Pixel) {
    for y in -radius..=radius {
        let half = radius - y.abs();
        line(canvas, (cx - half, cy + y), (cx + half, cy + y), 1, color);
    }
}

pub fn draw_orb_visual(canvas: &mut Canvas<'_>, visual: OrbVisual) {
    let (cx, cy) = center(visual.bounds);
    let radius = (visual.bounds.width.min(visual.bounds.height) as i32 / 2).max(8);
    let edge = (radius / 12).max(2);
    let foreground = state_color(visual.state.universal_state());
    let quiet = theme_color(ColorRole::Elevated);
    let ink = theme_color(ColorRole::HighContrastText);

    fill_disc(canvas, cx, cy, radius, quiet);
    stroke_ring(canvas, cx, cy, radius - edge / 2, edge, foreground);

    match visual.state {
        OrbVisualState::Waiting => {
            stroke_ring(canvas, cx, cy, radius / 3, edge, foreground);
            fill_disc(canvas, cx, cy, edge, foreground);
        }
        OrbVisualState::Listening => {
            let spacing = (radius / 5).max(2);
            for (index, height) in [2, 4, 6, 4, 2].iter().enumerate() {
                let x = cx + (index as i32 - 2) * spacing;
                let half = radius * height / 14;
                line(canvas, (x, cy - half), (x, cy + half), edge, foreground);
            }
        }
        OrbVisualState::Analyzing => {
            let nodes = [
                (cx, cy - radius / 3),
                (cx - radius / 3, cy + radius / 4),
                (cx + radius / 3, cy + radius / 4),
                (cx, cy),
            ];
            for pair in [(0, 1), (0, 2), (1, 2), (0, 3), (1, 3), (2, 3)] {
                line(canvas, nodes[pair.0], nodes[pair.1], edge / 2, foreground);
            }
            for node in nodes {
                fill_disc(canvas, node.0, node.1, edge * 2, ink);
            }
        }
        OrbVisualState::Planning => {
            for step in 0..72 {
                let angle = std::f32::consts::TAU * step as f32 / 72.0;
                let x = (angle.cos() * radius as f32 * 0.58) as i32;
                let y = (angle.sin() * radius as f32 * 0.24) as i32;
                fill_disc(canvas, cx + x, cy + y, edge / 2, foreground);
                fill_disc(canvas, cx + y, cy + x, edge / 2, ink);
            }
            fill_disc(canvas, cx, cy, edge * 2, foreground);
        }
        OrbVisualState::Confirmation => {
            diamond(canvas, cx, cy, radius / 3, foreground);
            diamond(canvas, cx, cy, radius / 7, quiet);
        }
        OrbVisualState::Executing => {
            let progress = visual.progress.unwrap_or(72).min(100);
            arc(canvas, cx, cy, radius * 2 / 3, edge, progress, foreground);
            // A square means "running operation" even without colour.
            canvas.fill_rect(
                Rect::new(
                    (cx - edge * 2) as u32,
                    (cy - edge * 2) as u32,
                    (edge * 4) as u32,
                    (edge * 4) as u32,
                ),
                ink,
            );
        }
        OrbVisualState::Result => {
            line(
                canvas,
                (cx - radius / 3, cy),
                (cx - radius / 10, cy + radius / 4),
                edge,
                ink,
            );
            line(
                canvas,
                (cx - radius / 10, cy + radius / 4),
                (cx + radius / 3, cy - radius / 4),
                edge,
                ink,
            );
        }
    }
}

pub fn draw_avatar(canvas: &mut Canvas<'_>, visual: AvatarVisual) {
    let (cx, cy) = center(visual.bounds);
    let radius = visual.bounds.width.min(visual.bounds.height) as i32 / 2;
    let ink = theme_color(ColorRole::TextSecondary);
    fill_disc(canvas, cx, cy, radius, theme_color(ColorRole::Surface));
    fill_disc(canvas, cx, cy - radius / 4, radius / 4, ink);
    // Shoulders are deliberately geometric: no portrait is invented.
    for y in 0..radius / 2 {
        let half = radius / 2 - y / 2;
        line(canvas, (cx - half, cy + y), (cx + half, cy + y), 1, ink);
    }
    if let Some(status) = visual.status {
        let dot_radius = (radius / 5).max(2);
        fill_disc(
            canvas,
            cx + radius * 3 / 4,
            cy + radius * 3 / 4,
            dot_radius + 2,
            theme_color(ColorRole::Canvas),
        );
        fill_disc(
            canvas,
            cx + radius * 3 / 4,
            cy + radius * 3 / 4,
            dot_radius,
            state_color(status),
        );
    }
}

pub fn draw_slider(canvas: &mut Canvas<'_>, visual: SliderVisual) {
    let height = (visual.bounds.height / 6).max(2);
    let y = visual.bounds.y + (visual.bounds.height - height) / 2;
    canvas.fill_rect(
        Rect::new(visual.bounds.x, y, visual.bounds.width, height),
        theme_color(if visual.enabled {
            ColorRole::Grid
        } else {
            ColorRole::DisabledSurface
        }),
    );
    let filled = visual.bounds.width * u32::from(visual.value.min(100)) / 100;
    if visual.enabled {
        canvas.fill_rect(
            Rect::new(visual.bounds.x, y, filled, height),
            theme_color(ColorRole::Accent),
        );
    }
    let knob_radius = (visual.bounds.height as i32 / 3).max(3);
    fill_disc(
        canvas,
        (visual.bounds.x + filled) as i32,
        (visual.bounds.y + visual.bounds.height / 2) as i32,
        knob_radius,
        theme_color(if visual.enabled {
            ColorRole::TextPrimary
        } else {
            ColorRole::DisabledText
        }),
    );
}

pub fn draw_resource_bar(canvas: &mut Canvas<'_>, visual: ResourceBarVisual) {
    canvas.fill_rect(visual.bounds, theme_color(ColorRole::Grid));
    match visual.value {
        Some(value) => {
            let width = visual.bounds.width * u32::from(value.min(100)) / 100;
            canvas.fill_rect(
                Rect::new(
                    visual.bounds.x,
                    visual.bounds.y,
                    width,
                    visual.bounds.height,
                ),
                theme_color(ColorRole::Accent),
            );
        }
        None => {
            let border = visual.bounds.height.min(3).max(1);
            canvas.fill_rect(
                Rect::new(
                    visual.bounds.x,
                    visual.bounds.y,
                    visual.bounds.width,
                    border,
                ),
                theme_color(ColorRole::DisabledText),
            );
        }
    }
}

pub fn draw_ring_progress(canvas: &mut Canvas<'_>, visual: RingProgressVisual) {
    let (cx, cy) = center(visual.bounds);
    let radius = visual.bounds.width.min(visual.bounds.height) as i32 / 2;
    let width = (radius / 8).max(2);
    stroke_ring(canvas, cx, cy, radius, width, theme_color(ColorRole::Grid));
    if let Some(value) = visual.value {
        arc(
            canvas,
            cx,
            cy,
            radius,
            width,
            value.min(100),
            theme_color(ColorRole::Accent),
        );
    } else {
        for start in [0_u8, 25, 50, 75] {
            let steps = 12;
            for offset in 0..steps {
                let percent = start.saturating_add(offset);
                let angle = -std::f32::consts::FRAC_PI_2
                    + std::f32::consts::TAU * f32::from(percent) / 100.0;
                fill_disc(
                    canvas,
                    cx + (angle.cos() * radius as f32).round() as i32,
                    cy + (angle.sin() * radius as f32).round() as i32,
                    width / 2,
                    theme_color(ColorRole::Accent),
                );
            }
        }
    }
}

pub fn draw_object_icon(canvas: &mut Canvas<'_>, fonts: Option<&Fonts>, visual: ObjectIconVisual) {
    canvas.fill_rect(visual.bounds, theme_color(visual.color));
    let Some(fonts) = fonts else {
        return;
    };
    let size = IconSize::Large.value().get() as u32 * 3;
    let left = visual.bounds.x + visual.bounds.width.saturating_sub(size) / 2;
    let top = visual.bounds.y + visual.bounds.height.saturating_sub(size) / 2;
    let icon = Icon::new(visual.glyph, ColorRole::HighContrastText);
    draw_gallery_icon(canvas, fonts, &icon, left, top);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(width: u32, height: u32) -> Vec<u8> {
        vec![0; width as usize * height as usize * 4]
    }

    #[test]
    fn every_orb_phase_has_distinct_non_empty_pixels() {
        let mut checksums = std::collections::BTreeSet::new();
        for state in OrbVisualState::ALL {
            let mut bytes = pixels(96, 96);
            let mut canvas = Canvas::new(&mut bytes, 96, 96);
            canvas.fill(theme_color(ColorRole::Canvas));
            draw_orb_visual(
                &mut canvas,
                OrbVisual {
                    bounds: Rect::new(8, 8, 80, 80),
                    state,
                    progress: Some(40),
                },
            );
            let checksum = bytes.chunks_exact(4).fold(0_u64, |sum, pixel| {
                sum.wrapping_add(
                    u64::from(pixel[1]) * 3 + u64::from(pixel[2]) * 5 + u64::from(pixel[3]) * 7,
                )
            });
            assert!(checksums.insert(checksum), "{state:?} reused another phase");
        }
    }

    #[test]
    fn unavailable_resource_is_not_painted_as_zero() {
        let mut zero = pixels(100, 20);
        let mut unavailable = pixels(100, 20);
        draw_resource_bar(
            &mut Canvas::new(&mut zero, 100, 20),
            ResourceBarVisual {
                bounds: Rect::new(0, 5, 100, 10),
                value: Some(0),
            },
        );
        draw_resource_bar(
            &mut Canvas::new(&mut unavailable, 100, 20),
            ResourceBarVisual {
                bounds: Rect::new(0, 5, 100, 10),
                value: None,
            },
        );
        assert_ne!(zero, unavailable);
    }

    #[test]
    fn slider_clamps_out_of_range_values() {
        let mut at_hundred = pixels(120, 32);
        let mut above = pixels(120, 32);
        draw_slider(
            &mut Canvas::new(&mut at_hundred, 120, 32),
            SliderVisual {
                bounds: Rect::new(8, 4, 100, 24),
                value: 100,
                enabled: true,
            },
        );
        draw_slider(
            &mut Canvas::new(&mut above, 120, 32),
            SliderVisual {
                bounds: Rect::new(8, 4, 100, 24),
                value: 255,
                enabled: true,
            },
        );
        assert_eq!(at_hundred, above);
    }
}
