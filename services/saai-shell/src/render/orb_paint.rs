//! Painting for the Orb space (ADR-430): the sphere, its graticule, the
//! objects on it, a search trail, and the resting point that carries the
//! Context Light. Everything here is opaque-first: no blur, no glow.

use super::{
    draw_calibration_mark, draw_square_ring, draw_text, draw_text_centered, physical, role_px,
    text_scale, text_width, theme_color, Canvas, Fonts, Pixel,
};
use saai_orb::{Item, Primitive, Prominence};
use saai_ui_core::{ColorRole, LogicalUnit, Progress, Rect, StatusMark, StrokeToken, TextRole};
use std::f32::consts::{FRAC_1_SQRT_2, TAU};

/// The resting point and everything it says about the system. The geometry
/// is the sphere at rest, so pulling up grows exactly this shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbPoint {
    pub disc: (f32, f32, f32),
    /// Half the angle of the exposed rim, measured from 12 o'clock.
    pub half_angle: f32,
    pub color: Pixel,
    pub mark: StatusMark,
    pub attention_ring: bool,
    pub quantity: Option<u8>,
    pub activity_pulse: bool,
}

pub struct OrbPaint<'a> {
    pub viewport: Rect,
    pub rise: f32,
    pub disc: (f32, f32, f32),
    pub graticule: &'a [Vec<(f32, f32)>],
    pub trail: &'a [(f32, f32)],
    pub items: &'a [Item],
    pub selected: Option<&'a str>,
    pub point: OrbPoint,
}

/// Pixels per logical unit on this panel.
pub fn orb_unit() -> f32 {
    physical(LogicalUnit::new(1)) as f32
}

fn smooth(edge0: f32, edge1: f32, x: f32) -> f32 {
    saai_orb::smoothstep(edge0, edge1, x)
}

fn mix(a: Pixel, b: Pixel, t: f32) -> Pixel {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (f32::from(x) * t + f32::from(y) * (1.0 - t)).round() as u8;
    [0, m(a[1], b[1]), m(a[2], b[2]), m(a[3], b[3])]
}

impl Canvas<'_> {
    /// Opaque filled disc; edges are not anti-aliased (draw a rim over it).
    pub(super) fn fill_disc(&mut self, cx: f32, cy: f32, r: f32, color: Pixel) {
        if r <= 0.0 || !cx.is_finite() || !cy.is_finite() || !r.is_finite() {
            return;
        }
        let Some(b) = self.clipped(Rect::new(0, 0, self.width, self.height)) else {
            return;
        };
        let y0 = (cy - r).floor().max(b.y as f32) as i32;
        let y1 = (cy + r).ceil().min((b.y + b.height) as f32) as i32;
        for y in y0..y1 {
            let dy = y as f32 + 0.5 - cy;
            let h2 = r * r - dy * dy;
            if h2 <= 0.0 {
                continue;
            }
            let half = h2.sqrt();
            let x0 = (cx - half).round().max(b.x as f32);
            let x1 = (cx + half).round().min((b.x + b.width) as f32);
            if x1 > x0 {
                self.fill_rect(Rect::new(x0 as u32, y as u32, (x1 - x0) as u32, 1), color);
            }
        }
    }

    /// Circle outline by row spans, so a sphere far larger than the screen
    /// costs one pass over the visible rows.
    pub(super) fn stroke_circle(&mut self, cx: f32, cy: f32, r: f32, thickness: f32, color: Pixel) {
        if r <= 0.0 || thickness <= 0.0 || !(cx.is_finite() && cy.is_finite() && r.is_finite()) {
            return;
        }
        let Some(b) = self.clipped(Rect::new(0, 0, self.width, self.height)) else {
            return;
        };
        let inner_r = (r - thickness).max(0.0);
        let y0 = (cy - r).floor().max(b.y as f32) as i32;
        let y1 = (cy + r).ceil().min((b.y + b.height) as f32) as i32;
        let (lo, hi) = (b.x as f32, (b.x + b.width) as f32);
        let span = |canvas: &mut Self, y: i32, a: f32, z: f32| {
            let (a, z) = (a.round().max(lo), z.round().min(hi));
            if z > a {
                canvas.fill_rect(Rect::new(a as u32, y as u32, (z - a) as u32, 1), color);
            }
        };
        for y in y0..y1 {
            let dy = y as f32 + 0.5 - cy;
            let outer2 = r * r - dy * dy;
            if outer2 <= 0.0 {
                continue;
            }
            let outer = outer2.sqrt();
            let inner2 = inner_r * inner_r - dy * dy;
            if inner2 <= 0.0 {
                span(self, y, cx - outer, cx + outer);
            } else {
                let inner = inner2.sqrt();
                span(self, y, cx - outer, cx - inner);
                span(self, y, cx + inner, cx + outer);
            }
        }
    }

    /// Alpha-blended rectangle (`alpha` 0..=255); used for the backdrop
    /// that dims the page while the sphere rises.
    pub(super) fn tint_rect(&mut self, rect: Rect, color: Pixel, alpha: u8) {
        if alpha == 0 {
            return;
        }
        let Some(rect) = self.clipped(rect) else {
            return;
        };
        let a = u16::from(alpha);
        for y in rect.y..rect.y + rect.height {
            let start = (y as usize * self.width as usize + rect.x as usize) * 4;
            let end = start + rect.width as usize * 4;
            for px in self.pixels[start..end].chunks_exact_mut(4) {
                px[0] = 0;
                for c in 1..4 {
                    px[c] = ((u16::from(color[c]) * a + u16::from(px[c]) * (255 - a) + 127) / 255)
                        as u8;
                }
            }
        }
    }

    /// One-pixel line with alpha, stepped at half-pixel increments.
    pub(super) fn line(&mut self, a: (f32, f32), b: (f32, f32), color: Pixel, alpha: u8) {
        if !(a.0.is_finite() && a.1.is_finite() && b.0.is_finite() && b.1.is_finite()) {
            return;
        }
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len > 20_000.0 {
            return;
        }
        let steps = (len * 2.0).ceil().max(1.0) as i32;
        let mut last = (i32::MIN, i32::MIN);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let p = ((a.0 + dx * t).floor() as i32, (a.1 + dy * t).floor() as i32);
            if p != last {
                self.blend(p.0, p.1, color, alpha);
                last = p;
            }
        }
    }

    /// Arc stroke between two angles measured clockwise from 12 o'clock
    /// (`from` may be negative); anti-aliased on the radial edges only.
    pub(super) fn arc_span(
        &mut self,
        center: (f32, f32),
        outer: f32,
        thickness: f32,
        from: f32,
        to: f32,
        color: Pixel,
    ) {
        let sweep = to - from;
        if sweep <= 0.0 || outer <= 0.0 || thickness <= 0.0 {
            return;
        }
        let sweep = sweep.min(TAU);
        let inner = (outer - thickness).max(0.0);
        let reach = outer.ceil() as i32 + 1;
        let (cx, cy) = center;
        for y in (cy as i32 - reach)..=(cy as i32 + reach) {
            for x in (cx as i32 - reach)..=(cx as i32 + reach) {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let distance = (dx * dx + dy * dy).sqrt();
                let radial = (distance - (inner - 0.5))
                    .min((outer + 0.5) - distance)
                    .clamp(0.0, 1.0);
                if radial <= 0.0 {
                    continue;
                }
                let angle = dx.atan2(-dy);
                if (angle - from).rem_euclid(TAU) > sweep {
                    continue;
                }
                self.blend(x, y, color, (radial * 255.0).round() as u8);
            }
        }
    }
}

/// Signed-distance coverage of a primitive of size `r` at offset `(dx, dy)`.
fn coverage(prim: Primitive, dx: f32, dy: f32, r: f32) -> f32 {
    let d = match prim {
        Primitive::Disc | Primitive::Ring => dx.hypot(dy) - r,
        Primitive::Square => dx.abs().max(dy.abs()) - r * 0.86,
        Primitive::Diamond => (dx.abs() + dy.abs()) * FRAC_1_SQRT_2 - r * 0.95,
    };
    (0.5 - d).clamp(0.0, 1.0)
}

fn alpha_u8(a: f32) -> u8 {
    (a.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[allow(clippy::too_many_arguments)]
fn draw_primitive(
    canvas: &mut Canvas<'_>,
    prim: Primitive,
    (cx, cy): (f32, f32),
    r: f32,
    edge: f32,
    fill: Option<Pixel>,
    outline: Pixel,
    alpha: f32,
) {
    let reach = (r * 1.25).ceil() as i32 + 2;
    for y in (cy as i32 - reach)..=(cy as i32 + reach) {
        for x in (cx as i32 - reach)..=(cx as i32 + reach) {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let outer = coverage(prim, dx, dy, r);
            if outer <= 0.0 {
                continue;
            }
            let inner = coverage(prim, dx, dy, (r - edge).max(0.0));
            if let Some(fill) = fill {
                if inner > 0.0 {
                    canvas.blend(x, y, fill, alpha_u8(alpha * inner));
                }
            }
            let ring = (outer - inner).max(0.0);
            if ring > 0.0 {
                canvas.blend(x, y, outline, alpha_u8(alpha * ring));
            }
        }
    }
}

fn shorten(label: &str, max: usize) -> String {
    let count = label.chars().count();
    if count <= max {
        return label.to_string();
    }
    let mut s: String = label.chars().take(max.saturating_sub(1)).collect();
    s.push('…');
    s
}

fn draw_item(canvas: &mut Canvas<'_>, item: &Item, selected: bool, fonts: Option<&Fonts>) {
    let unit = physical(LogicalUnit::new(1)) as f32;
    let hollow = !item.availability.is_available() || item.ghost;
    let alpha = item.alpha * if hollow { 0.7 } else { 1.0 };
    let surface = theme_color(ColorRole::Surface);
    let (fill, outline) = match item.prominence {
        Prominence::Now => (
            theme_color(ColorRole::Elevated),
            theme_color(ColorRole::Accent),
        ),
        _ => (
            theme_color(ColorRole::Elevated),
            theme_color(ColorRole::Border),
        ),
    };
    let edge = match item.prominence {
        Prominence::Now => (unit * 2.5).max(3.0),
        _ => (unit * 1.2).max(2.0),
    }
    .max(if item.primitive == Primitive::Ring {
        item.radius * 0.2
    } else {
        0.0
    });
    let fill = (!hollow && item.primitive != Primitive::Ring).then_some(fill);
    let at = (item.x, item.y);
    if selected {
        draw_primitive(
            canvas,
            item.primitive,
            at,
            item.radius + unit * 4.0,
            unit * 2.0,
            None,
            theme_color(ColorRole::Focus),
            item.alpha,
        );
    }
    draw_primitive(
        canvas,
        item.primitive,
        at,
        item.radius,
        edge,
        fill,
        outline,
        alpha,
    );
    let Some(fonts) = fonts else { return };
    let text_color = if hollow {
        theme_color(ColorRole::TextSecondary)
    } else {
        theme_color(ColorRole::TextPrimary)
    };
    let letter: String = item
        .label
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    if !letter.is_empty() {
        let size = (item.radius * 0.95).clamp(18.0, 64.0);
        let top = (item.y - size * 0.5).max(0.0) as u32;
        draw_text_centered(
            canvas,
            &fonts.regular,
            &letter,
            size,
            item.x.max(0.0) as u32,
            top,
            mix(text_color, surface, alpha),
        );
    }
    if item.show_label {
        let size = role_px(TextRole::Caption) * 0.8;
        let top = (item.y + item.radius + unit * 4.0).max(0.0) as u32;
        draw_text_centered(
            canvas,
            &fonts.regular,
            &shorten(&item.label, 16),
            size,
            item.x.max(0.0) as u32,
            top,
            mix(theme_color(ColorRole::TextPrimary), surface, alpha),
        );
    }
}

fn draw_point(canvas: &mut Canvas<'_>, p: &OrbPoint, bottom: f32) {
    let unit = physical(LogicalUnit::new(1)) as f32;
    let (cx, cy, r) = p.disc;
    canvas.fill_disc(cx, cy, r, theme_color(ColorRole::Elevated));
    let hairline = physical(StrokeToken::Hairline.value()).max(1) as f32;
    canvas.stroke_circle(cx, cy, r, hairline * 2.0, theme_color(ColorRole::Border));
    let top = cy - r;
    let exposure = (bottom - top).max(0.0);
    let side = (exposure * 0.5).floor().max(4.0) as u32;
    let mark = Rect::new(
        (cx - side as f32 / 2.0).max(0.0) as u32,
        (top + exposure * 0.5 - side as f32 / 2.0 + unit).max(0.0) as u32,
        side,
        side,
    );
    draw_calibration_mark(canvas, mark, p.mark, p.color);
    let focus = physical(StrokeToken::Focus.value()).max(1) as f32;
    if p.attention_ring {
        canvas.arc_span((cx, cy), r, focus, -p.half_angle, p.half_angle, p.color);
    }
    if p.activity_pulse {
        canvas.arc_span(
            (cx, cy),
            r - focus - unit * 3.0,
            hairline,
            -p.half_angle,
            p.half_angle,
            theme_color(ColorRole::TextSecondary),
        );
    }
    if let Some(percent) = p.quantity.filter(|v| *v > 0) {
        let t = physical(Progress::MIN_TRACK_HEIGHT).max(2) as f32;
        let span = p.half_angle * 2.0 * (f32::from(percent.min(100)) / 100.0);
        canvas.arc_span(
            (cx, cy),
            r - focus - unit * 6.0,
            t,
            -p.half_angle,
            -p.half_angle + span,
            theme_color(ColorRole::Border),
        );
    }
}

/// One result under the search field.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchRowView {
    pub rect: Rect,
    pub label: String,
    /// Where it is from here, e.g. "СВ · 42°". Empty when it is under you.
    pub hint: String,
    pub primitive: Primitive,
    pub focused: bool,
    /// Offline or only remembered: still listed, drawn hollow.
    pub dim: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchView {
    pub backdrop: Rect,
    pub field: Rect,
    pub text: String,
    pub placeholder: String,
    pub rows: Vec<SearchRowView>,
    /// Said in the first row when the text matches nothing.
    pub note: Option<String>,
}

fn draw_search_row(canvas: &mut Canvas<'_>, row: &SearchRowView, fonts: &Fonts, unit: f32) {
    let r = row.rect;
    let fill = if row.focused {
        theme_color(ColorRole::Elevated)
    } else {
        theme_color(ColorRole::Surface)
    };
    canvas.fill_rect(r, fill);
    if row.focused {
        let bar = (unit * 3.0).round() as u32;
        canvas.fill_rect(
            Rect::new(r.x, r.y, bar, r.height),
            theme_color(ColorRole::AccentHighlight),
        );
    }
    let inset = unit * 14.0;
    let glyph_r = (r.height as f32 * 0.24).max(6.0);
    let hollow = row.dim;
    draw_primitive(
        canvas,
        row.primitive,
        (
            r.x as f32 + inset + glyph_r,
            r.y as f32 + r.height as f32 / 2.0,
        ),
        glyph_r,
        (unit * 1.4).max(2.0),
        (!hollow && row.primitive != Primitive::Ring).then(|| theme_color(ColorRole::Elevated)),
        theme_color(ColorRole::Border),
        1.0,
    );
    let (font, size) = fonts.resolve(TextRole::Body);
    let text_x = r.x as f32 + inset * 2.0 + glyph_r * 2.0;
    let color = if row.dim {
        ColorRole::TextSecondary
    } else {
        ColorRole::TextPrimary
    };
    let line = size * text_scale();
    let top = (r.y as f32 + (r.height as f32 - line) / 2.0).max(0.0) as u32;
    let (hint_font, hint_size) = fonts.resolve(TextRole::Caption);
    let hint_w = if row.hint.is_empty() {
        0.0
    } else {
        text_width(hint_font, &row.hint, hint_size * text_scale())
    };
    let room = (r.x + r.width) as f32 - inset - hint_w - text_x - inset;
    let mut label = row.label.clone();
    while !label.is_empty() && text_width(font, &label, line) > room {
        label = shorten(&label, label.chars().count().saturating_sub(1));
    }
    draw_text(
        canvas,
        font,
        &label,
        size,
        text_x as u32,
        top,
        theme_color(color),
    );
    if hint_w > 0.0 {
        let hint_top = (r.y as f32 + (r.height as f32 - hint_size * text_scale()) / 2.0) as u32;
        draw_text(
            canvas,
            hint_font,
            &row.hint,
            hint_size,
            ((r.x + r.width) as f32 - inset - hint_w).max(0.0) as u32,
            hint_top,
            theme_color(ColorRole::TextSecondary),
        );
    }
}

/// The field and the list of where things are. Opaque, drawn over whatever
/// page is underneath, above the sphere's own window.
pub fn draw_search_panel(canvas: &mut Canvas<'_>, view: &SearchView, fonts: Option<&Fonts>) {
    let unit = physical(LogicalUnit::new(1)) as f32;
    canvas.fill_rect(view.backdrop, theme_color(ColorRole::Canvas));
    canvas.fill_rect(view.field, theme_color(ColorRole::Surface));
    draw_square_ring(
        canvas,
        view.field,
        physical(StrokeToken::Focus.value()).max(1),
        theme_color(ColorRole::Focus),
    );
    let Some(fonts) = fonts else { return };
    let (font, size) = fonts.resolve(TextRole::Body);
    let line = size * text_scale();
    let top = (view.field.y as f32 + (view.field.height as f32 - line) / 2.0).max(0.0) as u32;
    let left = view.field.x as f32 + unit * 14.0;
    if view.text.is_empty() {
        draw_text(
            canvas,
            font,
            &view.placeholder,
            size,
            left as u32,
            top,
            theme_color(ColorRole::TextSecondary),
        );
    } else {
        draw_text(
            canvas,
            font,
            &view.text,
            size,
            left as u32,
            top,
            theme_color(ColorRole::TextPrimary),
        );
    }
    let caret_x = left + text_width(font, &view.text, line) + unit * 2.0;
    canvas.fill_rect(
        Rect::new(
            caret_x as u32,
            top,
            (unit * 2.0).round().max(2.0) as u32,
            line as u32,
        ),
        theme_color(ColorRole::AccentHighlight),
    );
    for row in &view.rows {
        draw_search_row(canvas, row, fonts, unit);
    }
    if let (Some(note), Some(first)) = (&view.note, view.rows.first()) {
        canvas.fill_rect(first.rect, theme_color(ColorRole::Surface));
        let (cap, cap_size) = fonts.resolve(TextRole::Caption);
        let top = (first.rect.y as f32 + (first.rect.height as f32 - cap_size * text_scale()) / 2.0)
            as u32;
        draw_text(
            canvas,
            cap,
            note,
            cap_size,
            first.rect.x + (unit * 14.0) as u32,
            top,
            theme_color(ColorRole::TextSecondary),
        );
    }
}

pub fn draw_orb_space(canvas: &mut Canvas<'_>, paint: &OrbPaint<'_>, fonts: Option<&Fonts>) {
    canvas.set_clip(Some(paint.viewport));
    let bottom = (paint.viewport.y + paint.viewport.height) as f32;
    let rise = paint.rise.clamp(0.0, 1.0);
    if rise > 0.35 {
        canvas.tint_rect(
            paint.viewport,
            theme_color(ColorRole::Canvas),
            alpha_u8(smooth(0.35, 0.85, rise)),
        );
    }
    let (cx, cy, r) = paint.disc;
    canvas.fill_disc(cx, cy, r, theme_color(ColorRole::Surface));
    let hairline = physical(StrokeToken::Hairline.value()).max(1) as f32;
    if rise > 0.5 {
        let fade = alpha_u8(smooth(0.5, 0.95, rise) * 0.7);
        let grid = theme_color(ColorRole::Grid);
        for polyline in paint.graticule {
            for pair in polyline.windows(2) {
                canvas.line(pair[0], pair[1], grid, fade);
            }
        }
    }
    canvas.stroke_circle(cx, cy, r, hairline * 2.0, theme_color(ColorRole::Border));
    if paint.trail.len() >= 2 {
        let focus = theme_color(ColorRole::AccentHighlight);
        for pair in paint.trail.windows(2) {
            canvas.line(pair[0], pair[1], focus, 220);
            canvas.line(
                (pair[0].0, pair[0].1 + 1.0),
                (pair[1].0, pair[1].1 + 1.0),
                focus,
                220,
            );
        }
    }
    for item in paint.items {
        draw_item(
            canvas,
            item,
            paint.selected == Some(item.id.as_str()),
            fonts,
        );
    }
    draw_point(canvas, &paint.point, bottom);
    canvas.set_clip(None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use saai_orb::{Availability, ObjectClass, Tier, Unavailable};

    const W: u32 = 400;
    const H: u32 = 600;

    fn blank() -> Vec<u8> {
        vec![0u8; (W * H * 4) as usize]
    }

    fn px(buf: &mut [u8], x: u32, y: u32) -> Pixel {
        Canvas::new(buf, W, H).pixel(x, y)
    }

    fn point(mark: StatusMark, ring: bool, quantity: Option<u8>, pulse: bool) -> OrbPoint {
        let unit = physical(LogicalUnit::new(1)) as f32;
        let r = saai_orb::camera::REST_RADIUS_UNITS * unit;
        let exposure = saai_orb::camera::REST_EXPOSURE_UNITS * unit;
        OrbPoint {
            disc: (W as f32 / 2.0, H as f32 + r - exposure, r),
            half_angle: ((r - exposure) / r).acos(),
            color: theme_color(ColorRole::Accent),
            mark,
            attention_ring: ring,
            quantity,
            activity_pulse: pulse,
        }
    }

    fn paint_point(p: OrbPoint) -> Vec<u8> {
        let mut buf = blank();
        let mut canvas = Canvas::new(&mut buf, W, H);
        draw_orb_space(
            &mut canvas,
            &OrbPaint {
                viewport: Rect::new(0, 0, W, H),
                rise: 0.0,
                disc: p.disc,
                graticule: &[],
                trail: &[],
                items: &[],
                selected: None,
                point: p,
            },
            None,
        );
        buf
    }

    fn item(prim_class: ObjectClass, availability: Availability, ghost: bool) -> Item {
        Item {
            id: "x".into(),
            primitive: prim_class.primitive(),
            class: prim_class,
            label: "Mail".into(),
            echelon: 1,
            x: 200.0,
            y: 300.0,
            radius: 40.0,
            facing: 1.0,
            alpha: 1.0,
            prominence: Prominence::Nearby,
            availability,
            tier: Tier::Persistent,
            ghost,
            show_label: false,
        }
    }

    fn paint_item(it: &Item, selected: bool) -> Vec<u8> {
        let mut buf = blank();
        let mut canvas = Canvas::new(&mut buf, W, H);
        draw_item(&mut canvas, it, selected, None);
        buf
    }

    #[test]
    fn at_rest_only_a_small_dome_is_painted_at_the_bottom_centre() {
        let mut buf = paint_point(point(StatusMark::ActiveDot, false, None, false));
        let unit = physical(LogicalUnit::new(1));
        let exposure = (saai_orb::camera::REST_EXPOSURE_UNITS as u32) * unit;
        assert_ne!(px(&mut buf, W / 2, H - 4), [0, 0, 0, 0]);
        assert_eq!(px(&mut buf, 8, H - 4), [0, 0, 0, 0], "outside the dome");
        assert_eq!(
            px(&mut buf, W / 2, H - exposure - 6),
            [0, 0, 0, 0],
            "above it"
        );
        let painted = buf.chunks_exact(4).filter(|p| p[1..] != [0, 0, 0]).count();
        assert!(
            (painted as u32) < W * H / 10,
            "a point, not a screen: {painted}"
        );
    }

    #[test]
    fn marks_differ_by_shape_not_only_colour() {
        let idle = paint_point(point(StatusMark::Outline, false, None, false));
        let alert = paint_point(point(StatusMark::Alert, false, None, false));
        let offline = paint_point(point(StatusMark::Offline, false, None, false));
        assert_ne!(idle, alert);
        assert_ne!(idle, offline);
        assert_ne!(alert, offline);
    }

    #[test]
    fn attention_ring_and_activity_pulse_are_distinct_signals() {
        let base = paint_point(point(StatusMark::Activity, false, None, false));
        let ring = paint_point(point(StatusMark::Activity, true, None, false));
        let pulse = paint_point(point(StatusMark::Activity, false, None, true));
        assert_ne!(base, ring);
        assert_ne!(base, pulse);
        assert_ne!(ring, pulse);
    }

    #[test]
    fn quantity_gauge_fills_left_to_right_along_the_rim_in_the_border_token() {
        let border = theme_color(ColorRole::Border);
        let count = |q: Option<u8>| {
            paint_point(point(StatusMark::ActiveDot, false, q, false))
                .chunks_exact(4)
                .filter(|p| p[1..] == border[1..])
                .count()
        };
        let none = count(None);
        assert_eq!(none, count(Some(0)), "unknown and zero draw nothing extra");
        let (q, h, f) = (count(Some(25)), count(Some(50)), count(Some(100)));
        assert!(none < q && q < h && h < f, "{none} {q} {h} {f}");
        assert_eq!(count(Some(250)), f, "clamped, never wraps");
        assert_ne!(border, theme_color(ColorRole::Attention));
    }

    #[test]
    fn gauge_starts_at_the_left_end_of_the_rim() {
        let p = point(StatusMark::ActiveDot, false, Some(30), false);
        let (cx, cy, r) = p.disc;
        let focus = physical(StrokeToken::Focus.value()).max(1) as f32;
        let unit = physical(LogicalUnit::new(1)) as f32;
        let t = physical(Progress::MIN_TRACK_HEIGHT).max(2) as f32;
        let mid = r - focus - unit * 6.0 - t / 2.0;
        let probe = |angle: f32| {
            (
                (cx + mid * angle.sin()) as u32,
                (cy - mid * angle.cos()) as u32,
            )
        };
        let mut buf = paint_point(p);
        let border = theme_color(ColorRole::Border);
        let (lx, ly) = probe(-p.half_angle * 0.8);
        let (rx, ry) = probe(p.half_angle * 0.8);
        assert_eq!(px(&mut buf, lx, ly), border, "left end is filled at 30%");
        assert_ne!(px(&mut buf, rx, ry), border, "right end is still empty");
    }

    #[test]
    fn an_unavailable_object_is_hollow_and_a_ghost_is_too_so_it_reads_without_colour() {
        let on = paint_item(
            &item(ObjectClass::Device, Availability::Available, false),
            false,
        );
        let off = paint_item(
            &item(
                ObjectClass::Device,
                Availability::Unavailable(Unavailable::Offline),
                false,
            ),
            false,
        );
        let ghost = paint_item(
            &item(
                ObjectClass::Device,
                Availability::Unavailable(Unavailable::Missing),
                true,
            ),
            false,
        );
        assert_ne!(on, off);
        let mut on_b = on.clone();
        let mut off_b = off.clone();
        assert_ne!(px(&mut on_b, 200, 300), [0, 0, 0, 0], "available is filled");
        assert_eq!(
            px(&mut off_b, 200, 300),
            [0, 0, 0, 0],
            "unavailable is hollow"
        );
        let mut g = ghost;
        assert_eq!(px(&mut g, 200, 300), [0, 0, 0, 0]);
    }

    #[test]
    fn every_primitive_has_its_own_silhouette_and_selection_adds_a_ring() {
        let shapes: Vec<Vec<u8>> = [
            ObjectClass::Person,
            ObjectClass::Application,
            ObjectClass::Action,
            ObjectClass::Service,
        ]
        .into_iter()
        .map(|c| paint_item(&item(c, Availability::Available, false), false))
        .collect();
        for i in 0..shapes.len() {
            for j in i + 1..shapes.len() {
                assert_ne!(shapes[i], shapes[j], "{i} vs {j}");
            }
        }
        let plain = item(ObjectClass::Person, Availability::Available, false);
        assert_ne!(paint_item(&plain, false), paint_item(&plain, true));
    }

    #[test]
    fn unknown_classes_are_still_drawn_with_the_generic_primitive() {
        let it = item(
            ObjectClass::Unknown("quantum_toaster".into()),
            Availability::Available,
            false,
        );
        let mut buf = paint_item(&it, false);
        assert_ne!(px(&mut buf, 200, 300), [0, 0, 0, 0]);
        assert_eq!(it.primitive, Primitive::Disc);
    }

    #[test]
    fn risen_sphere_fills_the_viewport_and_never_paints_outside_it() {
        let mut buf = blank();
        let mut canvas = Canvas::new(&mut buf, W, H);
        let viewport = Rect::new(0, 0, W, 500);
        draw_orb_space(
            &mut canvas,
            &OrbPaint {
                viewport,
                rise: 1.0,
                disc: (200.0, 250.0, 700.0),
                graticule: &[vec![(0.0, 100.0), (399.0, 120.0)]],
                trail: &[],
                items: &[item(ObjectClass::Person, Availability::Available, false)],
                selected: None,
                point: point(StatusMark::ActiveDot, false, None, false),
            },
            None,
        );
        for y in 0..H {
            for x in 0..W {
                let p = Canvas::new(&mut buf, W, H).pixel(x, y);
                if y >= 500 {
                    assert_eq!(p, [0, 0, 0, 0], "painted below the viewport at ({x},{y})");
                }
            }
        }
        assert_eq!(px(&mut buf, 3, 3), theme_color(ColorRole::Surface));
    }

    #[test]
    fn backdrop_dims_the_page_progressively_while_rising() {
        let sample = |rise: f32| {
            let mut buf = blank();
            Canvas::new(&mut buf, W, H).fill(theme_color(ColorRole::Accent));
            let mut canvas = Canvas::new(&mut buf, W, H);
            draw_orb_space(
                &mut canvas,
                &OrbPaint {
                    viewport: Rect::new(0, 0, W, H),
                    rise,
                    disc: (200.0, 10_000.0, 40.0),
                    graticule: &[],
                    trail: &[],
                    items: &[],
                    selected: None,
                    point: point(StatusMark::ActiveDot, false, None, false),
                },
                None,
            );
            px(&mut buf, 5, 5)
        };
        let page = theme_color(ColorRole::Accent);
        assert_eq!(sample(0.2), page, "no dimming at the start");
        assert_ne!(sample(0.6), page);
        assert_eq!(
            sample(1.0),
            theme_color(ColorRole::Canvas),
            "fully opaque when risen"
        );
    }

    #[test]
    fn canvas_arc_span_covers_exactly_the_requested_sector() {
        let mut buf = blank();
        let mut canvas = Canvas::new(&mut buf, W, H);
        canvas.arc_span((200.0, 300.0), 50.0, 6.0, -1.0, 1.0, [0, 9, 9, 9]);
        let probe = |a: f32| {
            (
                (200.0 + 47.0 * a.sin()) as u32,
                (300.0 - 47.0 * a.cos()) as u32,
            )
        };
        let inside = probe(0.0);
        let left = probe(-0.8);
        let outside = probe(1.4);
        let opposite = probe(3.1);
        assert_eq!(px(&mut buf, inside.0, inside.1), [0, 9, 9, 9]);
        assert_eq!(px(&mut buf, left.0, left.1), [0, 9, 9, 9]);
        assert_eq!(px(&mut buf, outside.0, outside.1), [0, 0, 0, 0]);
        assert_eq!(px(&mut buf, opposite.0, opposite.1), [0, 0, 0, 0]);
    }

    #[test]
    fn huge_circles_are_cheap_and_clipped() {
        let mut buf = blank();
        let mut canvas = Canvas::new(&mut buf, W, H);
        canvas.fill_disc(200.0, 300.0, 50_000.0, [0, 1, 2, 3]);
        canvas.stroke_circle(200.0, 300.0, 10.0, 2.0, [0, 7, 7, 7]);
        assert_eq!(px(&mut buf, 0, 0), [0, 1, 2, 3]);
        assert_eq!(px(&mut buf, 200, 300), [0, 1, 2, 3]);
        assert_eq!(px(&mut buf, 200, 291), [0, 7, 7, 7]);
    }

    #[test]
    fn labels_are_shortened_with_an_ellipsis() {
        assert_eq!(shorten("Почта", 16), "Почта");
        assert_eq!(shorten("Очень длинное название объекта", 10), "Очень дли…");
    }

    #[test]
    fn the_search_panel_is_opaque_over_the_page_and_the_field_shows_focus() {
        let mut buf = blank();
        Canvas::new(&mut buf, W, H).fill(theme_color(ColorRole::Accent));
        let view = SearchView {
            backdrop: Rect::new(0, 0, W, 200),
            field: Rect::new(16, 20, W - 32, 52),
            text: String::new(),
            placeholder: "Что найти?".into(),
            rows: Vec::new(),
            note: None,
        };
        draw_search_panel(&mut Canvas::new(&mut buf, W, H), &view, None);
        assert_eq!(px(&mut buf, 5, 150), theme_color(ColorRole::Canvas));
        assert_eq!(px(&mut buf, 200, 46), theme_color(ColorRole::Surface));
        assert_eq!(px(&mut buf, 16, 40), theme_color(ColorRole::Focus));
        assert_eq!(
            px(&mut buf, 5, 300),
            theme_color(ColorRole::Accent),
            "nothing below the panel is touched"
        );
    }
}
