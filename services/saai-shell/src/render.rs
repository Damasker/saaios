use fontdue::{Font, FontSettings};
use saai_ui_core::{
    composite_gallery_fixtures, Button, ButtonVariant, ColorRole, ContextColor, ContextHeader,
    DataRow, DataRowVariant, DecisionOverlay, Disclosure, Divider, Field, FieldKind, FontFamily,
    FontWeight, Icon, IconGlyph, IconSize, LogicalUnit, Metric, MetricValue, NavigationItem,
    ObjectSummary, ObjectSummaryTrailing, Progress, Rect, Rgb, SemanticText, SpacingToken,
    StatusIndicator, StatusIndicatorVariant, StatusMark, StrokeToken, SurfacePattern, SurfaceScale,
    SystemSection, SystemSectionRow, SystemStatus, TextOverflow, TextRole, Theme, UniversalState,
    MIN_TOUCH_TARGET, TWO_LINE_ROW_HEIGHT,
};
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

/// S25: accessibility text scale, read inside `draw_text`/`draw_
/// text_centered` rather than threaded through this file's ~40
/// existing call sites as one more parameter each. This process only
/// ever renders one shell UI on one thread, so there's no
/// concurrency hazard from process-global state here -- the tradeoff
/// is one small piece of shared mutable state in exchange for not
/// touching every already-working draw call's signature. `AtomicU32`
/// holding the `f32`'s bits because `std` has no `AtomicF32`.
static TEXT_SCALE_BITS: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0f32.to_bits()

/// `scale_pct` is a percent of every literal `size` argument already
/// in this file (`100` = unchanged). Clamped to a sane range so a
/// corrupt settings file can't blow up glyph rasterization with a
/// zero or enormous size.
pub fn set_text_scale(scale_pct: u8) {
    let scale = (f32::from(scale_pct) / 100.0).clamp(0.5, 2.0);
    TEXT_SCALE_BITS.store(scale.to_bits(), Ordering::Relaxed);
}

fn text_scale() -> f32 {
    f32::from_bits(TEXT_SCALE_BITS.load(Ordering::Relaxed))
}

/// S25: a post-process contrast stretch applied once, from `main.rs`,
/// over an entire already-drawn frame's raw pixel bytes -- not a
/// second color palette threaded through this file's ~90 individual
/// `fill_rect`/`draw_text` call sites. `boost_pct` `0` is a byte-for-
/// byte no-op; higher values push every channel further from a
/// mid-grey pivot toward black or white. Operates on all 4 bytes per
/// pixel including the always-zero first byte (see `rgb`'s doc
/// comment) -- harmless, since stretching a value already at 0 away
/// from the 128 pivot only ever clamps back down to 0.
pub fn apply_contrast_boost(pixels: &mut [u8], boost_pct: u8) {
    if boost_pct == 0 {
        return;
    }
    let factor = 1.0 + (f32::from(boost_pct.min(100)) / 100.0) * 3.0;
    for channel in pixels.iter_mut() {
        let value = f32::from(*channel);
        let stretched = (value - 128.0) * factor + 128.0;
        *channel = stretched.clamp(0.0, 255.0) as u8;
    }
}

pub type Pixel = [u8; 4];

// Saai-displayd currently blits client pixels into the panel's native BGRX
// scanout buffer without conversion. The physically calibrated panel packing
// is [X, R, G, B] in little-endian memory (the same mapping as
// drm-splash.c::panel_color). Keep that quirk behind this backend boundary;
// product code deals only in saai-ui-core semantic roles.
const THEME: Theme = Theme::SAAIOS_DARK;

const fn panel_pixel(color: Rgb) -> Pixel {
    [0, color.red, color.green, color.blue]
}

pub const fn theme_color(role: ColorRole) -> Pixel {
    panel_pixel(THEME.color(role))
}

pub const fn context_color(color: ContextColor) -> Pixel {
    panel_pixel(THEME.context_color(color))
}

pub const fn state_color(state: UniversalState) -> Pixel {
    panel_pixel(THEME.state_color(state))
}

pub struct Fonts {
    regular: Font,
    semibold: Font,
    mono: Option<Font>,
    icons: Option<Font>,
}

#[derive(Clone)]
pub struct ActionCardView {
    pub label: String,
    pub status: String,
    pub action: String,
    pub selected: bool,
    pub indicator: Option<StatusIndicator>,
}

impl ActionCardView {
    pub fn new(
        label: impl Into<String>,
        status: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            status: status.into(),
            action: action.into(),
            selected: false,
            indicator: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn with_indicator(mut self, indicator: StatusIndicator) -> Self {
        self.indicator = Some(indicator);
        self
    }
}

impl Fonts {
    pub fn load_system() -> Result<Self, String> {
        // S13 Change 5: swapped from Inter, by explicit user request
        // after seeing Inter on-device. First tried Manrope, which
        // physically turned out to read as visually indistinguishable
        // from Inter at these render sizes -- Montserrat is the
        // second, confirmed-distinctive choice. Both Space Grotesk and
        // Sora were considered and rejected before that: neither ships
        // any Cyrillic glyphs at all (checked via `fontTools`'s cmap),
        // which would have broken every Russian label in this UI.
        // Montserrat ships as a variable font on Google Fonts now (no
        // separate static weight files) -- `fontdue` has no
        // variable-font support, so the two weights here are static
        // instances pre-generated with `fonttools varLib.instancer`
        // (wght=400, wght=600) rather than loaded from the variable
        // file directly.
        Self::load(
            "/saaios/fonts/Montserrat-Regular.ttf",
            "/saaios/fonts/Montserrat-SemiBold.ttf",
            "/saaios/fonts/IBMPlexMono-Regular.ttf",
            "/saaios/fonts/FeatherIcons.ttf",
        )
    }

    fn load(
        regular_path: &str,
        semibold_path: &str,
        mono_path: &str,
        icons_path: &str,
    ) -> Result<Self, String> {
        let regular =
            fs::read(regular_path).map_err(|error| format!("read {regular_path}: {error}"))?;
        let semibold =
            fs::read(semibold_path).map_err(|error| format!("read {semibold_path}: {error}"))?;
        let mono = match fs::read(mono_path) {
            Ok(bytes) => match Font::from_bytes(bytes, FontSettings::default()) {
                Ok(font) => {
                    eprintln!("saai-shell: loaded optional mono font {mono_path}");
                    Some(font)
                }
                Err(error) => {
                    eprintln!("saai-shell: parse {mono_path}: {error}; using sans fallback");
                    None
                }
            },
            Err(error) => {
                eprintln!("saai-shell: read {mono_path}: {error}; using sans fallback");
                None
            }
        };
        // ADR-100: unlike mono text, an icon has no sans fallback that means
        // anything -- a letter cannot substitute for a wifi/lock/chevron
        // mark. A missing or invalid icon font simply draws nothing for
        // whichever icon call sites exist (see `draw_keypad_label`), the
        // same fail-soft posture, applied at the one place it can actually
        // be honored instead of a fallback that would be misleading.
        let icons = match fs::read(icons_path) {
            Ok(bytes) => match Font::from_bytes(bytes, FontSettings::default()) {
                Ok(font) => {
                    eprintln!("saai-shell: loaded optional icon font {icons_path}");
                    Some(font)
                }
                Err(error) => {
                    eprintln!("saai-shell: parse {icons_path}: {error}; icons will not draw");
                    None
                }
            },
            Err(error) => {
                eprintln!("saai-shell: read {icons_path}: {error}; icons will not draw");
                None
            }
        };
        Ok(Self {
            regular: Font::from_bytes(regular, FontSettings::default())
                .map_err(|error| format!("parse {regular_path}: {error}"))?,
            semibold: Font::from_bytes(semibold, FontSettings::default())
                .map_err(|error| format!("parse {semibold_path}: {error}"))?,
            mono,
            icons,
        })
    }

    fn resolve(&self, role: TextRole) -> (&Font, f32) {
        let style = role.style();
        let font = match (style.family, style.weight) {
            (FontFamily::Mono, _) => self.mono.as_ref().unwrap_or(&self.regular),
            (FontFamily::Sans, FontWeight::Semibold) => &self.semibold,
            (FontFamily::Sans, FontWeight::Regular) => &self.regular,
        };
        let size = SurfaceScale::PIXEL_7.logical_to_physical(style.size) as f32;
        (font, size)
    }

    /// `None` when the optional icon font (ADR-100) failed to load --
    /// callers must skip drawing rather than fall back to a letter, the
    /// same "no fallback that means anything" posture
    /// `saai_ui_core::Icon`'s own doc comment states.
    pub fn icon(&self) -> Option<&Font> {
        self.icons.as_ref()
    }
}

pub struct Canvas<'a> {
    pixels: &'a mut [u8],
    width: u32,
    height: u32,
    clip: Option<Rect>,
}

impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u8], width: u32, height: u32) -> Self {
        assert_eq!(pixels.len(), width as usize * height as usize * 4);
        Self {
            pixels,
            width,
            height,
            clip: None,
        }
    }

    pub fn set_clip(&mut self, clip: Option<Rect>) {
        self.clip = clip;
    }

    pub fn fill(&mut self, color: Pixel) {
        if let Some(clip) = self.clip {
            self.fill_rect(clip, color);
            return;
        }
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Pixel) {
        let Some(rect) = self.clipped(rect) else {
            return;
        };
        let left = rect.x;
        let top = rect.y;
        let right = rect.x.saturating_add(rect.width);
        let bottom = rect.y.saturating_add(rect.height);
        for y in top..bottom {
            let start = (y as usize * self.width as usize + left as usize) * 4;
            let end = (y as usize * self.width as usize + right as usize) * 4;
            for pixel in self.pixels[start..end].chunks_exact_mut(4) {
                pixel.copy_from_slice(&color);
            }
        }
    }

    fn clipped(&self, rect: Rect) -> Option<Rect> {
        let bounds = Rect::new(0, 0, self.width, self.height);
        let clipped = match self.clip {
            Some(clip) => rect.intersection(clip)?,
            None => rect,
        };
        clipped.intersection(bounds)
    }

    fn blend(&mut self, x: i32, y: i32, color: Pixel, alpha: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 || alpha == 0 {
            return;
        }
        if let Some(clip) = self.clip {
            if !clip.contains(x as f64, y as f64) {
                return;
            }
        }
        let start = (y as usize * self.width as usize + x as usize) * 4;
        let inverse = 255 - alpha as u16;
        self.pixels[start] = 0;
        for (channel, &src) in color.iter().enumerate().skip(1) {
            self.pixels[start + channel] =
                ((src as u16 * alpha as u16 + self.pixels[start + channel] as u16 * inverse + 127)
                    / 255) as u8;
        }
    }

    #[cfg(test)]
    fn pixel(&self, x: u32, y: u32) -> Pixel {
        let start = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels[start..start + 4].try_into().unwrap()
    }
}

/// ADR-142: app-consent through `ContextHeader` and Static `DataRow`
/// cards. Accept/decline rects stay the `consent_view` hit targets.
/// No concatenated `draw_root` Surface bar, no accent-square bullets.
#[allow(clippy::too_many_arguments)]
pub fn draw_consent(
    canvas: &mut Canvas<'_>,
    content: Rect,
    header: &ContextHeader,
    rows: &[(Rect, ActionCardView)],
    accept_button: Rect,
    decline_button: Rect,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    canvas.set_clip(Some(content));
    if let Some(fonts) = fonts {
        paint_context_header(canvas, fonts, content, header);
    }
    for (rect, card) in rows {
        draw_action_card(canvas, *rect, card, fonts);
    }
    canvas.set_clip(None);

    canvas.fill_rect(accept_button, theme_color(ColorRole::Accent));
    canvas.fill_rect(decline_button, theme_color(ColorRole::Surface));
    let Some(fonts) = fonts else {
        return;
    };
    draw_text_centered(
        canvas,
        &fonts.semibold,
        "Разрешить",
        40.0,
        accept_button.x + accept_button.width / 2,
        accept_button.y + accept_button.height / 2 - 20,
        theme_color(ColorRole::Canvas),
    );
    draw_text_centered(
        canvas,
        &fonts.semibold,
        "Отклонить",
        40.0,
        decline_button.x + decline_button.width / 2,
        decline_button.y + decline_button.height / 2 - 20,
        theme_color(ColorRole::TextPrimary),
    );
}

/// ADR-161: Intent compose through `ContextHeader`. The Text `Field`
/// sits immediately above the docked QWERTY. Keys stay the ADR-029
/// rectangles from `intent_view()`. No Surface fill of the Fill slot.
pub fn draw_intent_input(
    canvas: &mut Canvas<'_>,
    content: Rect,
    header: &ContextHeader,
    field: &Field,
    field_rect: Rect,
    keys: &[(Rect, String)],
    pressed_key: Option<&str>,
    field_focused: bool,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    canvas.set_clip(Some(content));
    if let Some(fonts) = fonts {
        paint_context_header(canvas, fonts, content, header);
        draw_gallery_field(canvas, fonts, field, field_rect, field_focused);
    }
    canvas.set_clip(None);
    paint_keyboard_keys(canvas, fonts, keys, pressed_key);
}

/// ADR-161: Wi-Fi password is the same avoidance layout as Intent.
/// The Password `Field` still masks; SSID stays on the Field label.
pub fn draw_wifi_password(
    canvas: &mut Canvas<'_>,
    content: Rect,
    header: &ContextHeader,
    field: &Field,
    field_rect: Rect,
    keys: &[(Rect, String)],
    pressed_key: Option<&str>,
    field_focused: bool,
    fonts: Option<&Fonts>,
) {
    draw_intent_input(
        canvas,
        content,
        header,
        field,
        field_rect,
        keys,
        pressed_key,
        field_focused,
        fonts,
    );
}

/// HIA-07: one screen for any entity, instead of a dedicated view per
/// `entity_type` (this replaced the previous, `saaios.task`-only
/// `draw_task_confirm`, same header-plus-buttons shape and
/// fixed-offset text placement, generalized to 0-2 buttons and an
/// optional third "related" line instead of always exactly two).
/// `actions` is empty for an entity_type with no type-specific
/// behavior (HIA-ROADMAP.md's own negative scenario: still a real,
/// non-empty screen, just without a button row).
/// ADR-137: identity paint shared by NOW and Object View. Returns the
/// y just below the trailing status/value, matching the cursor math
/// `now_object_summary_rect` uses for hit-testing.
/// ADR-157: `waiting_confirmation` paints `DecisionOverlay` facts as
/// Body, not the same Caption dump as activity/OAM. Identity stays
/// `ObjectSummary`; overlay `heading()` is not painted.
fn draw_object_summary(
    canvas: &mut Canvas<'_>,
    fonts: &Fonts,
    object: &ObjectSummary,
    left: u32,
    top: u32,
    max_width: u32,
) -> u32 {
    let mut y = top;
    draw_semantic_text(canvas, fonts, &object.title_text(), left, y, max_width);
    y += scaled_line_height(TextRole::Body);
    draw_semantic_text(canvas, fonts, &object.meta_text(), left, y, max_width);
    y += scaled_line_height(TextRole::Caption);
    if let Some(ObjectSummaryTrailing::Status(status)) = &object.trailing {
        draw_status_indicator(canvas, fonts, status, left, y);
        y += scaled_line_height(TextRole::Body);
    } else if let Some(ObjectSummaryTrailing::Value(value)) = &object.trailing {
        let (value_font, value_size) = fonts.resolve(TextRole::Body);
        draw_text(
            canvas,
            value_font,
            value,
            value_size,
            left,
            y,
            theme_color(ColorRole::TextPrimary),
        );
        y += scaled_line_height(TextRole::Body);
    }
    y
}

/// Hit rect for the NOW `ObjectSummary`, same stacking as `draw_now`.
pub fn now_object_summary_rect(content: Rect, has_lifecycle: bool, object: &ObjectSummary) -> Rect {
    let margin = (content.width / 20).max(12);
    let top_inset = ((150_u64 * u64::from(content.height.max(1))) / 2400) as u32;
    let mut y = content.y + top_inset;
    y += scaled_line_height(TextRole::Title);
    if has_lifecycle {
        y += scaled_line_height(TextRole::Body);
    }
    y += physical(SpacingToken::Medium.value());
    let start = y;
    y += scaled_line_height(TextRole::Body);
    y += scaled_line_height(TextRole::Caption);
    if object.trailing.is_some() {
        y += scaled_line_height(TextRole::Body);
    }
    let height = y.saturating_sub(start).max(physical(MIN_TOUCH_TARGET));
    Rect::new(
        content.x + margin,
        start,
        content.width.saturating_sub(margin.saturating_mul(2)),
        height,
    )
}

/// VUI-07 (ADR-137): identity is an `ObjectSummary` (title + type/
/// version meta + trailing status), not raw title/status `draw_text`.
/// Status layer is 120px; identity starts at `+140` like the other
/// migrated headers. Related/details stay optional lines below.
pub fn draw_object_view(
    canvas: &mut Canvas<'_>,
    summary: &ObjectSummary,
    related: Option<&str>,
    details: &[String],
    decision: Option<&DecisionOverlay>,
    permission: Option<&SurfacePattern>,
    header: Rect,
    actions: &[(Rect, &str)],
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    for (index, (rect, _label)) in actions.iter().enumerate() {
        // First button is the primary/accepting action -- holds
        // for a single-button screen too
        // (e.g. a notification's "Скрыть"), where it's the only, and
        // therefore primary, action.
        canvas.fill_rect(
            *rect,
            if index == 0 {
                theme_color(ColorRole::Accent)
            } else {
                theme_color(ColorRole::Surface)
            },
        );
    }

    let Some(fonts) = fonts else {
        return;
    };

    let margin = header.width / 22;
    let content_width = header.width.saturating_sub(margin.saturating_mul(2));
    let mut y = draw_object_summary(
        canvas,
        fonts,
        summary,
        header.x + margin,
        header.y + 140,
        content_width,
    );
    if let Some(related) = related {
        y = y.saturating_add(physical(SpacingToken::Medium.value()));
        draw_text(
            canvas,
            &fonts.regular,
            related,
            28.0,
            header.x + margin,
            y,
            theme_color(ColorRole::TextSecondary),
        );
        y = y.saturating_add(scaled_line_height(TextRole::Body));
    }
    if let Some(overlay) = decision {
        for fact in overlay.fact_lines() {
            if y + 40 >= header.y + header.height {
                break;
            }
            draw_semantic_text(
                canvas,
                fonts,
                &SemanticText::new(fact, TextRole::Body, ColorRole::TextPrimary),
                header.x + margin,
                y,
                content_width,
            );
            y = y.saturating_add(scaled_line_height(TextRole::Body));
        }
    }
    if let Some(pattern) = permission {
        if y + 40 < header.y + header.height {
            if pattern.paints_mark() {
                let mark_size = physical(IconSize::Medium.value());
                draw_calibration_mark(
                    canvas,
                    Rect::new(header.x + margin, y, mark_size, mark_size),
                    pattern.state.style().mark,
                    theme_color(pattern.state.style().color),
                );
            }
            draw_semantic_text(
                canvas,
                fonts,
                &pattern.message_text(),
                header.x + margin,
                y,
                content_width,
            );
            y = y.saturating_add(scaled_line_height(TextRole::Body));
        }
    }
    for detail in details {
        if y + 40 >= header.y + header.height {
            break;
        }
        draw_text(
            canvas,
            &fonts.regular,
            detail,
            28.0,
            header.x + margin,
            y,
            theme_color(ColorRole::TextSecondary),
        );
        y = y.saturating_add(scaled_line_height(TextRole::Body));
    }

    let overlay_labels: Option<[&str; 2]> = decision.map(|overlay| {
        [
            overlay.accept.label.as_str(),
            overlay.decline.label.as_str(),
        ]
    });
    for (index, (rect, label)) in actions.iter().enumerate() {
        let text = overlay_labels
            .and_then(|labels| labels.get(index).copied())
            .unwrap_or(*label);
        draw_text_centered(
            canvas,
            &fonts.semibold,
            text,
            40.0,
            rect.x + rect.width / 2,
            rect.y + rect.height / 2 - 20,
            if index == 0 {
                theme_color(ColorRole::Canvas)
            } else {
                theme_color(ColorRole::TextPrimary)
            },
        );
    }
}

/// HIA-04b: drawn last, unconditionally, on top of whatever
/// `Frame::Root` just rendered -- not a modal, coexists with the
/// tab-bar/cards underneath it (see `orb_zone_rect`'s own doc comment
/// in `main.rs` for why it never overlaps their hit-test space).
/// `menu_rows` is empty in `Idle`/`Attention`; two rows in `Menu`.
/// HIA-16: `is_attention` draws a real shape difference, not just a
/// different fill color -- a hollow ring-square (outer `dot_color`
/// frame, `canvas background`-colored center) instead of the solid square
/// every other state uses. `ATTENTION`'s own color (a fixed alert
/// red, `main.rs`'s `build_orb_frame`) already told a sighted user
/// something needs them; this is the same signal for anyone who
/// can't rely on color alone (HIA-ROADMAP.md's own acceptance line,
/// document section 52) -- a colorblind user, or a photo/screen-
/// share that's lost its color fidelity, still sees "hollow" as
/// distinct from "solid" regardless of hue.
/// VUI-04 (ADR-116): `mark` is the Orb's own `OrbHost::mark()` --
/// `draw_calibration_mark` already renders a distinct shape per
/// `StatusMark` variant (used by `StatusIndicator`'s own compact mark
/// and the calibration fixture), so every real Orb state
/// (Idle/Active/Running/Attention/Offline) now gets a shape of its own
/// instead of the old binary filled-square-or-hollow-ring. Non-color by
/// construction: `dot_color` and `mark` are computed independently by
/// the caller, so a state is legible even for a viewer who cannot use
/// `dot_color` at all.
#[allow(clippy::too_many_arguments)]
pub fn draw_orb(
    canvas: &mut Canvas<'_>,
    dot_rect: Rect,
    dot_color: Pixel,
    mark: StatusMark,
    attention_ring: bool,
    quantity: Option<u8>,
    activity_pulse: bool,
    menu_rows: &[(Rect, &str)],
    fonts: Option<&Fonts>,
) {
    for (rect, label) in menu_rows {
        canvas.fill_rect(*rect, theme_color(ColorRole::Elevated));
        if let Some(fonts) = fonts {
            draw_text(
                canvas,
                &fonts.regular,
                label,
                32.0,
                rect.x + 24,
                rect.y + rect.height / 2 - 16,
                theme_color(ColorRole::TextPrimary),
            );
        }
    }
    draw_calibration_mark(canvas, dot_rect, mark, dot_color);
    if activity_pulse {
        // ADR-170: inset hairline is the on-phase of the activity loop.
        let inset = physical(StrokeToken::Focus.value()).max(1);
        if dot_rect.width > inset * 2 && dot_rect.height > inset * 2 {
            draw_square_ring(
                canvas,
                Rect::new(
                    dot_rect.x + inset,
                    dot_rect.y + inset,
                    dot_rect.width - inset * 2,
                    dot_rect.height - inset * 2,
                ),
                physical(StrokeToken::Hairline.value()).max(1),
                theme_color(ColorRole::TextSecondary),
            );
        }
    }
    if attention_ring {
        draw_square_ring(
            canvas,
            dot_rect,
            physical(StrokeToken::Focus.value()).max(1),
            dot_color,
        );
    }
    if let Some(percent) = quantity {
        draw_quantity_fill(canvas, dot_rect, percent);
    }
}

fn draw_quantity_fill(canvas: &mut Canvas<'_>, rect: Rect, percent: u8) {
    let track_height = physical(Progress::MIN_TRACK_HEIGHT).max(1);
    if rect.height <= track_height {
        return;
    }
    let filled_width = (u64::from(rect.width) * u64::from(percent.min(100)) / 100) as u32;
    if filled_width == 0 {
        return;
    }
    canvas.fill_rect(
        Rect::new(
            rect.x,
            rect.y + rect.height.saturating_sub(track_height),
            filled_width,
            track_height,
        ),
        theme_color(ColorRole::Border),
    );
}

fn draw_square_ring(canvas: &mut Canvas<'_>, rect: Rect, thickness: u32, color: Pixel) {
    let thickness = thickness.max(1);
    if rect.width <= thickness * 2 || rect.height <= thickness * 2 {
        return;
    }
    canvas.fill_rect(Rect::new(rect.x, rect.y, rect.width, thickness), color);
    canvas.fill_rect(
        Rect::new(
            rect.x,
            rect.y + rect.height.saturating_sub(thickness),
            rect.width,
            thickness,
        ),
        color,
    );
    canvas.fill_rect(Rect::new(rect.x, rect.y, thickness, rect.height), color);
    canvas.fill_rect(
        Rect::new(
            rect.x + rect.width.saturating_sub(thickness),
            rect.y,
            thickness,
            rect.height,
        ),
        color,
    );
}

/// ADR-144: SSH pairing through `ContextHeader`. The live client name
/// is a Static `DataRow`; the fingerprint stays wrapped mono text so
/// the full `SHA256:` string remains readable. Buttons stay
/// `task_confirm_view`. Lock unlock stays `draw_lock_pin_entry`.
pub fn draw_remote_pair(
    canvas: &mut Canvas<'_>,
    content: Rect,
    header: &ContextHeader,
    rows: &[(Rect, ActionCardView)],
    fingerprint: &str,
    accept_button: Rect,
    decline_button: Rect,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    canvas.set_clip(Some(content));
    if let Some(fonts) = fonts {
        paint_context_header(canvas, fonts, content, header);
    }
    for (rect, card) in rows {
        draw_action_card(canvas, *rect, card, fonts);
    }
    if let Some(fonts) = fonts {
        let margin = (content.width / 22).max(12);
        let top = rows
            .first()
            .map(|(rect, _)| rect.y + rect.height + 24)
            .unwrap_or(content.y + 430);
        let (mono, mono_size) = fonts.resolve(TextRole::MonoBody);
        let scaled_size = mono_size * text_scale();
        let glyph_width = mono.metrics('0', scaled_size).advance_width.max(1.0);
        let available_width = content.width.saturating_sub(margin * 2) as f32;
        let chars_per_line = (available_width / glyph_width).floor().max(1.0) as usize;
        let line_height = SurfaceScale::PIXEL_7
            .logical_to_physical(TextRole::MonoBody.style().line_height)
            as f32
            * text_scale();
        for (line, chunk) in fingerprint
            .chars()
            .collect::<Vec<_>>()
            .chunks(chars_per_line)
            .enumerate()
        {
            let chunk = chunk.iter().collect::<String>();
            draw_text(
                canvas,
                mono,
                &chunk,
                mono_size,
                content.x + margin,
                top + (line as f32 * line_height).round() as u32,
                theme_color(ColorRole::TextSecondary),
            );
        }
    }
    canvas.set_clip(None);

    canvas.fill_rect(accept_button, theme_color(ColorRole::Accent));
    canvas.fill_rect(decline_button, theme_color(ColorRole::Surface));
    let Some(fonts) = fonts else {
        return;
    };
    draw_text_centered(
        canvas,
        &fonts.semibold,
        "Разрешить",
        40.0,
        accept_button.x + accept_button.width / 2,
        accept_button.y + accept_button.height / 2 - 20,
        theme_color(ColorRole::Canvas),
    );
    draw_text_centered(
        canvas,
        &fonts.semibold,
        "Отклонить",
        40.0,
        decline_button.x + decline_button.width / 2,
        decline_button.y + decline_button.height / 2 - 20,
        theme_color(ColorRole::TextPrimary),
    );
}

fn draw_action_card(
    canvas: &mut Canvas<'_>,
    rect: Rect,
    card: &ActionCardView,
    fonts: Option<&Fonts>,
) {
    canvas.fill_rect(
        rect,
        if card.selected {
            theme_color(ColorRole::Elevated)
        } else {
            theme_color(ColorRole::Surface)
        },
    );
    let has_action = !card.action.is_empty();
    let text_left = if has_action {
        canvas.fill_rect(
            Rect::new(rect.x + 34, rect.y + 52, 104, 104),
            theme_color(ColorRole::Accent),
        );
        rect.x + 174
    } else if let Some(indicator) = &card.indicator {
        let mark_size = physical(IconSize::Medium.value());
        draw_calibration_mark(
            canvas,
            Rect::new(rect.x + 34, rect.y + 72, mark_size, mark_size),
            indicator.mark(),
            theme_color(indicator.color()),
        );
        rect.x + 34 + mark_size + physical(SpacingToken::Small.value())
    } else {
        rect.x + 34
    };
    let button = has_action.then(|| {
        let button_width = 250.min(rect.width / 3);
        let button = Rect::new(
            rect.x + rect.width.saturating_sub(button_width + 34),
            rect.y + 58,
            button_width,
            88,
        );
        canvas.fill_rect(button, theme_color(ColorRole::Accent));
        button
    });
    let Some(fonts) = fonts else {
        return;
    };
    draw_text(
        canvas,
        &fonts.semibold,
        &card.label,
        38.0,
        text_left,
        rect.y + 48,
        theme_color(ColorRole::TextPrimary),
    );
    draw_text(
        canvas,
        &fonts.regular,
        &card.status,
        27.0,
        text_left,
        rect.y + 108,
        theme_color(ColorRole::TextSecondary),
    );
    if let Some(button) = button {
        draw_text_centered(
            canvas,
            &fonts.semibold,
            &card.action,
            25.0,
            button.x + button.width / 2,
            button.y + 24,
            theme_color(ColorRole::Canvas),
        );
    }
}

/// S24: "Изменить PIN" on "Я" -- same header-plus-keys shape as
/// `draw_intent_input`, but the preview is masked (a PIN is a secret,
/// same reasoning as `WifiPasswordInput`'s masked preview) and the
/// ADR-100: the PIN keypad's backspace key used to label itself with the
/// Unicode erase mark U+232B ("⌫"), which the sans face has no glyph
/// for -- `fontdue` silently drew its own missing-glyph placeholder box
/// there (found via `screencap`, ADR-099's real device screenshot). Draws
/// through the icon font instead for that one key; every other key (a
/// plain digit) is unaffected and keeps using the sans face.
fn draw_keypad_label(
    canvas: &mut Canvas<'_>,
    fonts: &Fonts,
    label: &str,
    size: f32,
    center_x: u32,
    top: u32,
    color: Pixel,
) {
    if label == "⌫" {
        if let Some(icons) = &fonts.icons {
            let glyph = IconGlyph::Backspace.codepoint().to_string();
            draw_text_centered(canvas, icons, &glyph, size, center_x, top, color);
            return;
        }
    }
    draw_text_centered(canvas, &fonts.semibold, label, size, center_x, top, color);
}

/// ADR-143/149: PIN setup through `ContextHeader`. The Password `Field`
/// sits in the first stacked row below the status layer. Keys are the
/// same ADR-029 keyboard paint as Intent and Wi-Fi. No Surface header bar.
pub fn draw_pin_setup(
    canvas: &mut Canvas<'_>,
    content: Rect,
    header: &ContextHeader,
    field: &Field,
    field_rect: Rect,
    keys: &[(Rect, String)],
    pressed_key: Option<&str>,
    field_focused: bool,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    canvas.set_clip(Some(content));
    if let Some(fonts) = fonts {
        paint_context_header(canvas, fonts, content, header);
        draw_gallery_field(canvas, fonts, field, field_rect, field_focused);
    }
    canvas.set_clip(None);
    paint_keyboard_keys(canvas, fonts, keys, pressed_key);
}

/// ADR-149: lock unlock. Password `Field` occupancy in the keyboard
/// header slot, then the shared key paint. Never the secret. PIN
/// setup stays `draw_pin_setup`.
pub fn draw_lock_pin_entry(
    canvas: &mut Canvas<'_>,
    field: &Field,
    field_rect: Rect,
    keys: &[(Rect, String)],
    pressed_key: Option<&str>,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    if let Some(fonts) = fonts {
        draw_gallery_field(canvas, fonts, field, field_rect, false);
    }
    paint_keyboard_keys(canvas, fonts, keys, pressed_key);
}

/// One key painter for Intent, Wi-Fi password, PIN setup, and lock
/// unlock. Rects come from `layout()`; this only fills them.
/// ADR-151: the live finger's key uses `ColorRole::Pressed`.
fn paint_keyboard_keys(
    canvas: &mut Canvas<'_>,
    fonts: Option<&Fonts>,
    keys: &[(Rect, String)],
    pressed_key: Option<&str>,
) {
    let Some(fonts) = fonts else {
        for (rect, label) in keys {
            let role = if pressed_key == Some(label.as_str()) {
                ColorRole::Pressed
            } else {
                ColorRole::Elevated
            };
            canvas.fill_rect(*rect, theme_color(role));
        }
        return;
    };
    for (rect, label) in keys {
        let key = Rect::new(
            rect.x.saturating_add(4),
            rect.y.saturating_add(4),
            rect.width.saturating_sub(8),
            rect.height.saturating_sub(8),
        );
        let role = if pressed_key == Some(label.as_str()) {
            ColorRole::Pressed
        } else {
            ColorRole::Surface
        };
        canvas.fill_rect(key, theme_color(role));
        draw_keypad_label(
            canvas,
            fonts,
            label,
            32.0,
            key.x + key.width / 2,
            key.y + key.height / 2 - 18,
            theme_color(ColorRole::TextPrimary),
        );
    }
}

/// VUI-07 (ADR-134 / ADR-148 / ADR-153): no-PIN lock. Canvas instead of
/// the S04 diagnostic red fill. Time and hint are passed in; device
/// state is a Caption of the live fuel-gauge; essential attention is a
/// compact `StatusIndicator` whose type cannot hold Inbox titles or
/// bodies. PIN unlock stays `draw_lock_pin_entry`.
pub fn draw_lock_idle(
    canvas: &mut Canvas<'_>,
    width: u32,
    height: u32,
    time: &str,
    hint: &str,
    device: Option<&str>,
    attention: Option<&StatusIndicator>,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    let time_y = ((height as u64 * 480) / 2400) as u32;
    let hint_y = time_y + physical_line_height(TextRole::Display) + physical(LogicalUnit::new(16));
    let left = width / 22;
    if let Some(fonts) = fonts {
        let time_size = physical(TextRole::Display.style().size) as f32;
        let hint_size = physical(TextRole::Body.style().size) as f32;
        draw_text_centered(
            canvas,
            &fonts.semibold,
            time,
            time_size,
            width / 2,
            time_y,
            theme_color(ColorRole::TextPrimary),
        );
        draw_text_centered(
            canvas,
            &fonts.regular,
            hint,
            hint_size,
            width / 2,
            hint_y,
            theme_color(ColorRole::TextSecondary),
        );
    }
    let mut below_hint =
        hint_y + physical_line_height(TextRole::Body) + physical(LogicalUnit::new(24));
    if let Some(label) = device {
        if let Some(fonts) = fonts {
            draw_semantic_text(
                canvas,
                fonts,
                &SemanticText::new(label, TextRole::Caption, ColorRole::TextSecondary),
                left,
                below_hint,
                width.saturating_sub(left.saturating_mul(2)),
            );
        }
        below_hint += physical_line_height(TextRole::Caption) + physical(LogicalUnit::new(24));
    }
    if let Some(indicator) = attention {
        if let Some(fonts) = fonts {
            draw_status_indicator(canvas, fonts, indicator, left, below_hint);
        } else {
            let mark_size = physical(IconSize::Medium.value());
            draw_calibration_mark(
                canvas,
                Rect::new(left, below_hint, mark_size, mark_size),
                indicator.mark(),
                theme_color(indicator.color()),
            );
        }
    }
}

/// ADR-154: HIA-38 AOD. Canvas and the Display-role clock at the same
/// `time_y` as lock idle, so wake does not jump the digits. No hint,
/// no battery, no Inbox. PIN unlock stays `draw_lock_pin_entry`.
pub fn draw_lock_sleep(
    canvas: &mut Canvas<'_>,
    width: u32,
    height: u32,
    time: &str,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    let time_y = ((height as u64 * 480) / 2400) as u32;
    if let Some(fonts) = fonts {
        let time_size = physical(TextRole::Display.style().size) as f32;
        draw_text_centered(
            canvas,
            &fonts.semibold,
            time,
            time_size,
            width / 2,
            time_y,
            theme_color(ColorRole::TextPrimary),
        );
    }
}

/// VUI-01's deterministic, device-runnable calibration fixture. It is selected
/// only by the explicit `SAAIOS_UI_CALIBRATION=1` developer environment switch
/// or volatile `/run/saaios/ui-calibration` marker in `main.rs`; normal
/// navigation and stored settings cannot open it.
///
/// Every swatch comes through the same semantic-role/context/state APIs as the
/// production shell. The state rows also draw a distinct geometric mark, so a
/// photograph can verify both physical color and the non-color state channel.
pub fn draw_calibration(canvas: &mut Canvas<'_>, width: u32, height: u32, fonts: Option<&Fonts>) {
    const PALETTE: [(ColorRole, &str); 17] = [
        (ColorRole::Canvas, "CANVAS 071011"),
        (ColorRole::Surface, "SURFACE 0D181A"),
        (ColorRole::Elevated, "ELEVATED 142326"),
        (ColorRole::Accent, "ACCENT 63D4D6"),
        (ColorRole::AccentHighlight, "HIGHLIGHT A1EEF0"),
        (ColorRole::TextPrimary, "TEXT D7E2DF"),
        (ColorRole::TextSecondary, "SECONDARY 829796"),
        (ColorRole::Success, "SUCCESS 6FB79A"),
        (ColorRole::Attention, "ATTENTION D4B658"),
        (ColorRole::Critical, "CRITICAL C7514B"),
        (ColorRole::Border, "BORDER 315054"),
        (ColorRole::Grid, "GRID 20383A"),
        (ColorRole::Pressed, "PRESSED A1EEF0"),
        (ColorRole::Focus, "FOCUS A1EEF0"),
        (ColorRole::DisabledSurface, "DISABLED BG"),
        (ColorRole::DisabledText, "DISABLED TEXT"),
        (ColorRole::HighContrastText, "HIGH CONTRAST"),
    ];
    const CONTEXTS: [(ContextColor, &str); 6] = [
        (ContextColor::Default, "DEFAULT"),
        (ContextColor::Blue, "BLUE"),
        (ContextColor::Green, "GREEN"),
        (ContextColor::Orange, "ORANGE"),
        (ContextColor::Purple, "PURPLE"),
        (ContextColor::Pink, "PINK"),
    ];
    const STATES: [(UniversalState, &str); 9] = [
        (UniversalState::Idle, "IDLE"),
        (UniversalState::Active, "ACTIVE"),
        (UniversalState::Running, "RUNNING"),
        (UniversalState::Waiting, "WAITING"),
        (UniversalState::Blocked, "BLOCKED"),
        (UniversalState::Attention, "ATTENTION"),
        (UniversalState::Failed, "FAILED"),
        (UniversalState::Complete, "COMPLETE"),
        (UniversalState::Offline, "OFFLINE"),
    ];

    canvas.fill(theme_color(ColorRole::Canvas));
    let margin = (width / 20).max(12);
    let gap = (width / 60).max(6);
    let columns = 3u32;
    let cell_width = width
        .saturating_sub(margin * 2)
        .saturating_sub(gap * (columns - 1))
        / columns;
    let cell_height = (height / 24).max(72);
    let palette_top = height / 10;
    let palette_rows = PALETTE.len().div_ceil(columns as usize) as u32;

    if let Some(fonts) = fonts {
        draw_text(
            canvas,
            &fonts.semibold,
            "SaaiOS Visual v1 · VUI-01",
            42.0,
            margin,
            height / 24,
            theme_color(ColorRole::TextPrimary),
        );
        draw_text(
            canvas,
            &fonts.regular,
            "SEMANTIC PALETTE",
            24.0,
            margin,
            palette_top.saturating_sub(42),
            theme_color(ColorRole::TextSecondary),
        );
    }

    for (index, (role, label)) in PALETTE.iter().copied().enumerate() {
        let column = index as u32 % columns;
        let row = index as u32 / columns;
        let x = margin + column * (cell_width + gap);
        let y = palette_top + row * cell_height;
        let swatch_height = cell_height * 3 / 5;
        canvas.fill_rect(
            Rect::new(x, y, cell_width, swatch_height),
            theme_color(ColorRole::Border),
        );
        canvas.fill_rect(
            Rect::new(
                x.saturating_add(4),
                y.saturating_add(4),
                cell_width.saturating_sub(8),
                swatch_height.saturating_sub(8),
            ),
            theme_color(role),
        );
        if let Some(fonts) = fonts {
            draw_text(
                canvas,
                &fonts.regular,
                label,
                18.0,
                x,
                y + swatch_height + 8,
                theme_color(ColorRole::TextSecondary),
            );
        }
    }

    let context_top = palette_top + palette_rows * cell_height + height / 30;
    if let Some(fonts) = fonts {
        draw_text(
            canvas,
            &fonts.regular,
            "CONTEXT COLOR · NOT STATUS",
            24.0,
            margin,
            context_top.saturating_sub(42),
            theme_color(ColorRole::TextSecondary),
        );
    }
    for (index, (context, label)) in CONTEXTS.iter().copied().enumerate() {
        let column = index as u32 % columns;
        let row = index as u32 / columns;
        let x = margin + column * (cell_width + gap);
        let y = context_top + row * cell_height;
        let swatch_height = cell_height * 3 / 5;
        canvas.fill_rect(
            Rect::new(x, y, cell_width, swatch_height),
            context_color(context),
        );
        if let Some(fonts) = fonts {
            draw_text(
                canvas,
                &fonts.regular,
                label,
                18.0,
                x,
                y + swatch_height + 8,
                theme_color(ColorRole::TextSecondary),
            );
        }
    }

    let context_rows = CONTEXTS.len().div_ceil(columns as usize) as u32;
    let state_top = context_top + context_rows * cell_height + height / 30;
    let state_height =
        height.saturating_sub(state_top).saturating_sub(margin) / STATES.len() as u32;
    if let Some(fonts) = fonts {
        draw_text(
            canvas,
            &fonts.regular,
            "UNIVERSAL STATE · COLOR + MARK",
            24.0,
            margin,
            state_top.saturating_sub(42),
            theme_color(ColorRole::TextSecondary),
        );
    }
    for (index, (state, label)) in STATES.iter().copied().enumerate() {
        let style = state.style();
        let y = state_top + index as u32 * state_height;
        let row = Rect::new(
            margin,
            y,
            width.saturating_sub(margin * 2),
            state_height.saturating_sub(8),
        );
        canvas.fill_rect(row, theme_color(ColorRole::Surface));
        canvas.fill_rect(
            Rect::new(row.x, row.y + 8, 10, row.height.saturating_sub(16)),
            state_color(state),
        );
        let mark_size = row.height.saturating_sub(24).min(64);
        let mark = Rect::new(row.x + 28, row.y + 12, mark_size, mark_size);
        draw_calibration_mark(canvas, mark, style.mark, state_color(state));
        if let Some(fonts) = fonts {
            let description = format!("{label} · {}", style.label_key);
            draw_text(
                canvas,
                &fonts.semibold,
                &description,
                24.0,
                mark.x + mark.width + 28,
                row.y + row.height / 2 - 14,
                theme_color(ColorRole::TextPrimary),
            );
        }
    }
}

/// Summed glyph advance width -- the measurement half of "measure, then
/// place" that `draw_text_centered` does inline and a right-aligned or
/// custom-positioned label (the status bar, the gallery) needs to do
/// itself first.
fn text_width(font: &Font, text: &str, size: f32) -> f32 {
    text.chars()
        .map(|character| font.metrics(character, size).advance_width)
        .sum()
}

fn draw_calibration_mark(canvas: &mut Canvas<'_>, rect: Rect, mark: StatusMark, color: Pixel) {
    let quarter = (rect.width / 4).max(2);
    let half = rect.width / 2;
    match mark {
        StatusMark::Outline => {
            canvas.fill_rect(rect, color);
            canvas.fill_rect(
                Rect::new(
                    rect.x + quarter / 2,
                    rect.y + quarter / 2,
                    rect.width.saturating_sub(quarter),
                    rect.height.saturating_sub(quarter),
                ),
                theme_color(ColorRole::Surface),
            );
        }
        StatusMark::ActiveDot => canvas.fill_rect(
            Rect::new(rect.x + quarter, rect.y + quarter, half, half),
            color,
        ),
        StatusMark::Activity => {
            for index in 0..3 {
                let bar_height = rect.height * (index + 2) / 4;
                canvas.fill_rect(
                    Rect::new(
                        rect.x + index * quarter,
                        rect.y + rect.height.saturating_sub(bar_height),
                        quarter / 2,
                        bar_height,
                    ),
                    color,
                );
            }
        }
        StatusMark::Waiting => {
            canvas.fill_rect(
                Rect::new(rect.x, rect.y + quarter, rect.width, quarter / 2),
                color,
            );
            canvas.fill_rect(
                Rect::new(rect.x + quarter, rect.y + half, half, quarter / 2),
                color,
            );
        }
        StatusMark::Blocked => {
            canvas.fill_rect(Rect::new(rect.x, rect.y, quarter / 2, rect.height), color);
            canvas.fill_rect(
                Rect::new(
                    rect.x + half - quarter / 4,
                    rect.y,
                    quarter / 2,
                    rect.height,
                ),
                color,
            );
            canvas.fill_rect(
                Rect::new(
                    rect.x + rect.width - quarter / 2,
                    rect.y,
                    quarter / 2,
                    rect.height,
                ),
                color,
            );
        }
        StatusMark::Alert => {
            canvas.fill_rect(
                Rect::new(rect.x + half - quarter / 4, rect.y, quarter / 2, half),
                color,
            );
            canvas.fill_rect(
                Rect::new(
                    rect.x + half - quarter / 4,
                    rect.y + rect.height - quarter / 2,
                    quarter / 2,
                    quarter / 2,
                ),
                color,
            );
        }
        StatusMark::Failure => {
            canvas.fill_rect(
                Rect::new(
                    rect.x + half - quarter / 4,
                    rect.y,
                    quarter / 2,
                    rect.height,
                ),
                color,
            );
            canvas.fill_rect(
                Rect::new(rect.x, rect.y + half - quarter / 4, rect.width, quarter / 2),
                color,
            );
        }
        StatusMark::Complete => {
            canvas.fill_rect(Rect::new(rect.x, rect.y + half, quarter / 2, half), color);
            canvas.fill_rect(
                Rect::new(
                    rect.x,
                    rect.y + rect.height - quarter / 2,
                    rect.width,
                    quarter / 2,
                ),
                color,
            );
        }
        StatusMark::Offline => {
            canvas.fill_rect(
                Rect::new(rect.x, rect.y + quarter, half - quarter / 2, half),
                color,
            );
            canvas.fill_rect(
                Rect::new(
                    rect.x + half + quarter / 2,
                    rect.y + quarter,
                    half - quarter / 2,
                    half,
                ),
                color,
            );
        }
    }
}

/// VUI-02's own translation-key convention (`saai_ui_core`'s
/// `label_key`/`MetricValue::label_key`/`StatusIndicator`'s state key,
/// `Disclosure`'s state key) resolves here, at the shell layer -- the one
/// place component-library-v1.md section 8 actually assigns display
/// language ownership to. Deliberately small and gallery-scoped, not a
/// general i18n system: covers exactly the keys this file's own
/// `draw_gallery` produces.
fn resolve_label_key(key: &str) -> &'static str {
    match key {
        "state.idle" => "Ожидание",
        "state.active" => "Активно",
        "state.running" => "Выполняется",
        "state.waiting" => "В очереди",
        "state.blocked" => "Заблокировано",
        "state.attention" => "Внимание",
        "state.failed" => "Ошибка",
        "state.complete" => "Готово",
        "state.offline" => "Нет связи",
        "metric.unknown" => "Неизвестно",
        "metric.unavailable" => "Недоступно",
        "disclosure.collapsed" => "Свёрнуто",
        "disclosure.expanded" => "Развёрнуто",
        _ => "?",
    }
}

/// Physical pixels for the logical-unit line height a `TextRole` declares
/// -- every gallery helper below that stacks a second line under a first
/// one needs this, not a guessed constant. Missing this exact conversion
/// (using a logical value like `28` directly as physical pixels) was a
/// real bug caught by the first physical screenshot of this screen: the
/// reason line under `StatusIndicator`'s label, and `DataRow`'s secondary
/// line, both collided with the line above them.
fn physical_line_height(role: TextRole) -> u32 {
    SurfaceScale::PIXEL_7.logical_to_physical(role.style().line_height)
}

/// `physical_line_height` scaled by the user's current accessibility text
/// scale -- for actually stacking a second rendered line under a first
/// one. Kept a separate function rather than folding `text_scale()` into
/// `physical_line_height` itself: that function backs a pinned host test
/// (`physical_line_height_matches_the_pixel_7_scale`) that must stay
/// deterministic regardless of the process-global, mutable
/// `TEXT_SCALE_BITS` (`render::set_text_scale`) -- and `gallery_row_
/// positions`'s own between-row budget is intentionally scale-independent
/// (a fixed fraction of screen height), so it has no reason to call this.
/// Found missing, not designed in from the start: a real 150% text-scale
/// screenshot showed `SemanticText`'s wrapped second line clipping off the
/// right edge of the screen, because `wrap_text` was measuring against the
/// unscaled size while `draw_text` renders at the scaled one.
fn scaled_line_height(role: TextRole) -> u32 {
    (physical_line_height(role) as f32 * text_scale()).round() as u32
}

fn physical(value: LogicalUnit) -> u32 {
    SurfaceScale::PIXEL_7.logical_to_physical(value)
}

/// Greedy word-wrap: breaks `text` into lines whose rendered width (in
/// `font` at `size`) fits within `max_width`. A single word wider than
/// `max_width` on its own is not split further -- the practical limit any
/// word-wrap without hyphenation has, and better than an infinite loop.
/// Component-library-v1.md section 6.1: "wrap by default for prose."
fn wrap_text(font: &Font, text: &str, size: f32, max_width: u32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if current.is_empty() || text_width(font, &candidate, size) as u32 <= max_width {
            current = candidate;
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Section 6.1's full contract: wraps by default, respects `max_lines`
/// when set, and appends an ellipsis to the last visible line only when
/// content was actually cut *and* the caller asked for
/// `TextOverflow::Ellipsis` -- never invented on a line that already fit.
fn draw_semantic_text(
    canvas: &mut Canvas<'_>,
    fonts: &Fonts,
    text: &SemanticText,
    left: u32,
    top: u32,
    max_width: u32,
) {
    let (font, size) = fonts.resolve(text.role);
    let lines = wrap_text(font, &text.content, size * text_scale(), max_width);
    let visible_count = text
        .max_lines
        .map(|max| (max as usize).min(lines.len()).max(1))
        .unwrap_or(lines.len());
    let truncated = lines.len() > visible_count;
    let line_height = scaled_line_height(text.role);
    for (index, line) in lines.iter().take(visible_count).enumerate() {
        let is_last_visible = index + 1 == visible_count;
        let display = if is_last_visible && truncated && text.overflow == TextOverflow::Ellipsis {
            format!("{line}\u{2026}")
        } else {
            line.clone()
        };
        draw_text(
            canvas,
            font,
            &display,
            size,
            left,
            top + index as u32 * line_height,
            theme_color(text.color),
        );
    }
}

fn draw_gallery_icon(canvas: &mut Canvas<'_>, fonts: &Fonts, icon: &Icon, left: u32, top: u32) {
    let Some(icon_font) = fonts.icon() else {
        return;
    };
    let size = physical(icon.size.value()) as f32;
    let glyph = icon.glyph.codepoint().to_string();
    draw_text(
        canvas,
        icon_font,
        &glyph,
        size,
        left,
        top,
        theme_color(icon.color),
    );
}

fn draw_divider(canvas: &mut Canvas<'_>, divider: &Divider, rect: Rect) {
    canvas.fill_rect(rect, theme_color(divider.color));
}

fn draw_status_indicator(
    canvas: &mut Canvas<'_>,
    fonts: &Fonts,
    indicator: &StatusIndicator,
    left: u32,
    top: u32,
) {
    let style = indicator.state.style();
    let mark_size = physical(IconSize::Medium.value());
    draw_calibration_mark(
        canvas,
        Rect::new(left, top, mark_size, mark_size),
        indicator.mark(),
        theme_color(style.color),
    );
    let (font, size) = fonts.resolve(TextRole::Body);
    let text_left = left + mark_size + physical(SpacingToken::Small.value());
    draw_text(
        canvas,
        font,
        &indicator.label,
        size,
        text_left,
        top,
        theme_color(style.color),
    );
    if let Some(reason) = indicator.visible_reason() {
        let (reason_font, reason_size) = fonts.resolve(TextRole::Caption);
        draw_text(
            canvas,
            reason_font,
            reason,
            reason_size,
            text_left,
            top + scaled_line_height(TextRole::Body),
            theme_color(ColorRole::TextSecondary),
        );
    }
}

fn draw_gallery_progress(canvas: &mut Canvas<'_>, progress: &Progress, rect: Rect) {
    canvas.fill_rect(rect, theme_color(ColorRole::Grid));
    let filled_width = match progress.percent() {
        Some(percent) => (u64::from(rect.width) * u64::from(percent) / 100) as u32,
        // Indeterminate: a static snapshot of the restrained motion section
        // 6.5 describes -- this gallery draws one still frame, not the
        // animation itself (VUI-08's job).
        None => rect.width / 4,
    };
    canvas.fill_rect(
        Rect::new(rect.x, rect.y, filled_width, rect.height),
        theme_color(ColorRole::Accent),
    );
}

fn draw_gallery_button(canvas: &mut Canvas<'_>, fonts: &Fonts, button: &Button, rect: Rect) {
    let (fill, text_color) = match (button.variant, button.enabled) {
        (_, false) => (ColorRole::DisabledSurface, ColorRole::DisabledText),
        (ButtonVariant::Primary, true) => (ColorRole::Accent, ColorRole::HighContrastText),
        (ButtonVariant::Destructive, true) => (ColorRole::Critical, ColorRole::HighContrastText),
        (ButtonVariant::Secondary, true) => (ColorRole::Elevated, ColorRole::TextPrimary),
        (ButtonVariant::Quiet, true) => (ColorRole::Surface, ColorRole::TextPrimary),
    };
    canvas.fill_rect(rect, theme_color(fill));
    let (font, size) = fonts.resolve(TextRole::Label);
    draw_text_centered(
        canvas,
        font,
        &button.label,
        size,
        rect.x + rect.width / 2,
        rect.y + rect.height / 2 - (size * 0.38) as u32,
        theme_color(text_color),
    );
}

fn draw_gallery_field(
    canvas: &mut Canvas<'_>,
    fonts: &Fonts,
    field: &Field,
    rect: Rect,
    focused: bool,
) {
    let (label_font, label_size) = fonts.resolve(TextRole::Caption);
    draw_text(
        canvas,
        label_font,
        &field.label,
        label_size,
        rect.x,
        rect.y,
        theme_color(ColorRole::TextSecondary),
    );
    let box_top = rect.y + physical_line_height(TextRole::Caption);
    let box_rect = Rect::new(
        rect.x,
        box_top,
        rect.width,
        rect.height.saturating_sub(box_top - rect.y),
    );
    let box_fill = if field.error.is_some() {
        ColorRole::Critical
    } else {
        ColorRole::Surface
    };
    canvas.fill_rect(box_rect, theme_color(box_fill));
    if focused {
        draw_square_ring(
            canvas,
            box_rect,
            physical(StrokeToken::Focus.value()).max(1),
            theme_color(ColorRole::Focus),
        );
    }
    let (value_font, value_size) = fonts.resolve(TextRole::Body);
    let inset = physical(SpacingToken::Small.value());
    let (display, color): (String, ColorRole) = if field.is_empty() {
        (
            field.placeholder.clone().unwrap_or_default(),
            ColorRole::TextSecondary,
        )
    } else {
        (field.accessible_value(), ColorRole::TextPrimary)
    };
    draw_text(
        canvas,
        value_font,
        &display,
        value_size,
        box_rect.x + inset,
        box_rect.y + inset / 2,
        theme_color(color),
    );
}

fn draw_data_row(canvas: &mut Canvas<'_>, fonts: &Fonts, row: &DataRow, rect: Rect) {
    canvas.fill_rect(rect, theme_color(ColorRole::Surface));
    let inset = physical(SpacingToken::Small.value());
    let top_inset = physical(SpacingToken::XSmall.value());
    let mut cursor_x = rect.x + inset;
    if let Some(glyph) = row.icon {
        if let Some(icon_font) = fonts.icon() {
            let icon_size = physical(IconSize::Large.value());
            draw_text(
                canvas,
                icon_font,
                &glyph.codepoint().to_string(),
                icon_size as f32,
                cursor_x,
                rect.y + top_inset,
                theme_color(ColorRole::TextSecondary),
            );
            cursor_x += icon_size + inset;
        }
    }
    let (primary_font, primary_size) = fonts.resolve(TextRole::Body);
    draw_text(
        canvas,
        primary_font,
        &row.primary,
        primary_size,
        cursor_x,
        rect.y + top_inset,
        theme_color(ColorRole::TextPrimary),
    );
    if let Some(secondary) = &row.secondary {
        let (secondary_font, secondary_size) = fonts.resolve(TextRole::Caption);
        draw_text(
            canvas,
            secondary_font,
            secondary,
            secondary_size,
            cursor_x,
            rect.y + top_inset + scaled_line_height(TextRole::Body),
            theme_color(ColorRole::TextSecondary),
        );
    }
    if let Some(value) = &row.value {
        let (value_font, value_size) = fonts.resolve(TextRole::Body);
        let value_width = text_width(value_font, value, value_size) as u32;
        let value_left = (rect.x + rect.width)
            .saturating_sub(inset)
            .saturating_sub(value_width);
        draw_text(
            canvas,
            value_font,
            value,
            value_size,
            value_left,
            rect.y + top_inset,
            theme_color(ColorRole::TextSecondary),
        );
    }
}

fn draw_gallery_metric(
    canvas: &mut Canvas<'_>,
    fonts: &Fonts,
    metric: &Metric,
    left: u32,
    top: u32,
) {
    let (label_font, label_size) = fonts.resolve(TextRole::Caption);
    draw_text(
        canvas,
        label_font,
        &metric.label,
        label_size,
        left,
        top,
        theme_color(ColorRole::TextSecondary),
    );
    let display = match &metric.value {
        MetricValue::Known(text) => match &metric.unit {
            Some(unit) => format!("{text} {unit}"),
            None => text.clone(),
        },
        other => resolve_label_key(other.label_key().unwrap_or("")).to_string(),
    };
    let (value_font, value_size) = fonts.resolve(TextRole::Title);
    draw_text(
        canvas,
        value_font,
        &display,
        value_size,
        left,
        top + physical_line_height(TextRole::Caption),
        theme_color(ColorRole::TextPrimary),
    );
}

fn draw_gallery_disclosure(
    canvas: &mut Canvas<'_>,
    fonts: &Fonts,
    disclosure: &Disclosure,
    rect: Rect,
) {
    let (font, size) = fonts.resolve(TextRole::Body);
    draw_text(
        canvas,
        font,
        &disclosure.label,
        size,
        rect.x,
        rect.y,
        theme_color(ColorRole::TextPrimary),
    );
    if let Some(icon_font) = fonts.icon() {
        let icon_size = physical(IconSize::Medium.value()) as f32;
        let glyph = disclosure.chevron().codepoint().to_string();
        draw_text(
            canvas,
            icon_font,
            &glyph,
            icon_size,
            (rect.x + rect.width)
                .saturating_sub(icon_size as u32 + physical(SpacingToken::Small.value())),
            rect.y,
            theme_color(ColorRole::TextSecondary),
        );
    }
}

/// One `cursor_y` per row `draw_gallery` paints, in draw order (0 is the
/// title). Pulled out of `draw_gallery` itself so it is testable without a
/// loaded font or a device -- ADR-105's own bug (two rows' text colliding)
/// was a layout-math error a host test on exactly this function would have
/// caught before a physical screenshot had to.
const GALLERY_ROW_COUNT: usize = 11;
const COMPOSITE_GALLERY_ROW_COUNT: usize = 13;

fn stacked_gallery_row_positions<const N: usize>(height: u32) -> [u32; N] {
    // Same clearance `draw_calibration`'s own `palette_top` uses -- the
    // separately-composited status bar overlay sits above the main
    // surface regardless of which fixture this surface draws, so both
    // fixtures need the same top clearance to avoid it.
    let mut cursor_y = height / 10;
    let mut rows = [0u32; N];
    rows[0] = cursor_y;
    // Same minimum clearance every other row gets below it (one Body line
    // height plus one Caption line height) -- the title is smaller than a
    // full row's own content, but there is no reason its own gap should be
    // held to a looser standard than every row after it.
    cursor_y += physical_line_height(TextRole::Body) + physical_line_height(TextRole::Caption);
    let remaining = (N - 1) as u32;
    let row_height = (height.saturating_sub(cursor_y) / remaining).max(140);
    for slot in rows.iter_mut().skip(1) {
        *slot = cursor_y;
        cursor_y += row_height;
    }
    rows
}

fn gallery_row_positions(height: u32) -> [u32; GALLERY_ROW_COUNT] {
    stacked_gallery_row_positions(height)
}

fn composite_gallery_row_positions(height: u32) -> [u32; COMPOSITE_GALLERY_ROW_COUNT] {
    stacked_gallery_row_positions(height)
}

fn composite_gallery_decision_buttons(width: u32, height: u32) -> (Rect, Rect) {
    let rows = composite_gallery_row_positions(height);
    let margin = (width / 20).max(12);
    let content_width = width.saturating_sub(margin * 2);
    let gap = physical(SpacingToken::Small.value());
    let button_width = content_width.saturating_sub(gap) / 2;
    let button_height = physical(MIN_TOUCH_TARGET);
    let top = rows[8];
    (
        Rect::new(margin, top, button_width, button_height),
        Rect::new(
            margin + button_width + gap,
            top,
            button_width,
            button_height,
        ),
    )
}

/// VUI-02's first device component gallery (component-library-v1.md
/// section 7): one instance of each of the ten primitives from ADR-102/
/// ADR-103, default state only -- pressed/focused/disabled/busy variants,
/// compact vs normal, long Russian labels, and scaled-text coverage are
/// explicit follow-up (see ADR-105's "Not verified by this ADR"), not
/// claimed here. Reached only through the same developer-only gate VUI-01's
/// calibration fixture uses (`SAAIOS_UI_GALLERY`/the runtime marker file) --
/// never a normal navigation destination.
///
/// `Divider` and `Progress` need no font to draw at all, so they are drawn
/// before the `fonts` check and stay visible even if the font asset failed
/// to load -- the same "still shows something, not a blank screen" posture
/// `draw_lock_pin_entry`'s own no-fonts path already has, and the reason
/// this crate's host tests can assert on them without a real font file.
/// Every other primitive here needs real glyphs to mean anything, so it
/// stays behind the check.
pub fn draw_gallery(canvas: &mut Canvas<'_>, width: u32, height: u32, fonts: Option<&Fonts>) {
    canvas.fill(theme_color(ColorRole::Canvas));
    let margin = (width / 20).max(12);
    let content_width = width.saturating_sub(margin * 2);
    let hairline = physical(StrokeToken::Hairline.value()).max(1);
    let rows = gallery_row_positions(height);

    let divider = Divider::new();
    draw_divider(
        canvas,
        &divider,
        Rect::new(margin, rows[3], content_width, hairline),
    );

    let progress = Progress::determinate(62);
    draw_gallery_progress(
        canvas,
        &progress,
        Rect::new(
            margin,
            rows[5],
            content_width,
            physical(Progress::MIN_TRACK_HEIGHT),
        ),
    );

    let Some(fonts) = fonts else {
        return;
    };

    draw_text(
        canvas,
        &fonts.semibold,
        "SaaiOS Component Gallery · VUI-02",
        32.0,
        margin,
        rows[0],
        theme_color(ColorRole::TextPrimary),
    );

    // Deliberately longer than one line fits -- demonstrates section
    // 6.1's full contract (wrap, then truncate at `max_lines` with an
    // ellipsis) instead of a short string that would never exercise it.
    // Fixes a real gap this gallery had until now: the untruncated demo
    // string used to run straight off the right edge of the screen,
    // violating the "long labels wrap or reflow; they do not clip" VUI-02
    // acceptance criterion its own gallery is supposed to demonstrate.
    let semantic_text = SemanticText::new(
        "Пример SemanticText -- обычный текст тела, достаточно длинный, чтобы перенестись на новую строку и показать многоточие",
        TextRole::Body,
        ColorRole::TextPrimary,
    )
    .with_max_lines(2)
    .with_overflow(TextOverflow::Ellipsis);
    draw_semantic_text(
        canvas,
        fonts,
        &semantic_text,
        margin,
        rows[1],
        content_width,
    );

    let icon = Icon::new(IconGlyph::Wifi, ColorRole::Accent).with_name("Wi-Fi");
    draw_gallery_icon(canvas, fonts, &icon, margin, rows[2]);

    let status = StatusIndicator::new(UniversalState::Blocked, "Заблокировано")
        .with_reason("Нет сети")
        .with_variant(StatusIndicatorVariant::Normal);
    draw_status_indicator(canvas, fonts, &status, margin, rows[4]);

    let button = Button::new("Сохранить", "gallery:save", ButtonVariant::Primary);
    draw_gallery_button(
        canvas,
        fonts,
        &button,
        Rect::new(
            margin,
            rows[6],
            physical(LogicalUnit::new(160)),
            physical(MIN_TOUCH_TARGET),
        ),
    );

    let field = Field::new("PIN", FieldKind::Password).with_value("4269");
    // Section 6.7: "Minimum hit height: 48 logical units" -- explicit,
    // not an incidental leftover from the row's own budget.
    let field_height = (rows[8] - rows[7])
        .min(physical(LogicalUnit::new(90)))
        .max(physical(MIN_TOUCH_TARGET));
    draw_gallery_field(
        canvas,
        fonts,
        &field,
        Rect::new(margin, rows[7], content_width, field_height),
        false,
    );

    let data_row = DataRow::new("Wi-Fi", DataRowVariant::Navigation)
        .with_secondary("Подключено: Wallbox")
        .with_value("99%");
    draw_data_row(
        canvas,
        fonts,
        &data_row,
        Rect::new(
            margin,
            rows[8],
            content_width,
            physical(TWO_LINE_ROW_HEIGHT),
        ),
    );

    let metric = Metric::new("Батарея", MetricValue::Known("87".to_string())).with_unit("%");
    draw_gallery_metric(canvas, fonts, &metric, margin, rows[9]);

    let disclosure = Disclosure::new("Подробности", "diagnostics-panel").expanded();
    // Section 6.10: "Hit region is at least 48x48 even when the chevron
    // is 16-20 units" -- was 32 logical units here, short of that floor.
    draw_gallery_disclosure(
        canvas,
        fonts,
        &disclosure,
        Rect::new(margin, rows[10], content_width, physical(MIN_TOUCH_TARGET)),
    );
}

/// VUI-05 composite page of the same developer gallery. Labelled fixture
/// data from `saai-ui-core` -- no live telemetry. `Divider` is skipped
/// here; state marks and the two decision buttons still draw without a
/// font so host tests can see geometry.
pub fn draw_composite_gallery(
    canvas: &mut Canvas<'_>,
    width: u32,
    height: u32,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    let margin = (width / 20).max(12);
    let content_width = width.saturating_sub(margin * 2);
    let rows = composite_gallery_row_positions(height);
    let fixtures = composite_gallery_fixtures();
    let (accept, decline) = composite_gallery_decision_buttons(width, height);

    canvas.fill_rect(accept, theme_color(ColorRole::Accent));
    canvas.fill_rect(decline, theme_color(ColorRole::Elevated));

    let mark_size = physical(IconSize::Medium.value());
    let cell_width = content_width / 3;
    for (index, indicator) in fixtures.states.iter().enumerate() {
        let row = 9 + index / 3;
        let column = index % 3;
        let left = margin + column as u32 * cell_width;
        draw_calibration_mark(
            canvas,
            Rect::new(left, rows[row], mark_size, mark_size),
            indicator.mark(),
            theme_color(indicator.state.style().color),
        );
    }

    let pattern_cell = content_width / fixtures.patterns.len() as u32;
    for (index, pattern) in fixtures.patterns.iter().enumerate() {
        if !pattern.paints_mark() {
            continue;
        }
        let left = margin + index as u32 * pattern_cell;
        draw_calibration_mark(
            canvas,
            Rect::new(left, rows[12], mark_size, mark_size),
            pattern.state.style().mark,
            theme_color(pattern.state.style().color),
        );
    }

    let Some(fonts) = fonts else {
        return;
    };

    draw_text(
        canvas,
        &fonts.semibold,
        fixtures.title,
        32.0,
        margin,
        rows[0],
        theme_color(ColorRole::TextPrimary),
    );
    draw_semantic_text(
        canvas,
        fonts,
        &fixtures.header.heading(),
        margin,
        rows[1],
        content_width,
    );
    draw_semantic_text(
        canvas,
        fonts,
        &fixtures.object.title_text(),
        margin,
        rows[2],
        content_width,
    );
    draw_status_indicator(canvas, fonts, &fixtures.task.status(), margin, rows[3]);
    draw_semantic_text(
        canvas,
        fonts,
        &fixtures.intent.heading(),
        margin,
        rows[4],
        content_width,
    );
    if let Some(missing) = fixtures.intent.missing_task_text() {
        draw_semantic_text(
            canvas,
            fonts,
            &missing,
            margin,
            rows[4] + physical_line_height(TextRole::Caption),
            content_width,
        );
    }
    draw_semantic_text(
        canvas,
        fonts,
        &fixtures.agent_unassigned.caption(),
        margin,
        rows[5],
        content_width,
    );
    draw_semantic_text(
        canvas,
        fonts,
        &fixtures.agent_assigned.caption(),
        margin,
        rows[6],
        content_width,
    );
    let facts = fixtures.decision.fact_lines().join(" · ");
    draw_semantic_text(
        canvas,
        fonts,
        &SemanticText::new(facts, TextRole::Caption, ColorRole::TextSecondary),
        margin,
        rows[7],
        content_width,
    );
    draw_gallery_button(canvas, fonts, &fixtures.decision.accept, accept);
    draw_gallery_button(canvas, fonts, &fixtures.decision.decline, decline);
    for (index, indicator) in fixtures.states.iter().enumerate() {
        let row = 9 + index / 3;
        let column = index % 3;
        let left =
            margin + column as u32 * cell_width + mark_size + physical(SpacingToken::Small.value());
        let (font, size) = fonts.resolve(TextRole::Caption);
        draw_text(
            canvas,
            font,
            &indicator.label,
            size,
            left,
            rows[row],
            theme_color(indicator.state.style().color),
        );
    }
}
// S23 added `is_grid` as the 8th plain draw-time knob on an already
// data-only function (no behavior to extract into a struct without
// inventing one purely to appease this lint) -- same call shape as
// `draw_consent`/`draw_object_view`, just with one more page-shaped
// screen to describe.
#[allow(clippy::too_many_arguments)]
pub fn draw_root(
    canvas: &mut Canvas<'_>,
    content: Rect,
    tabs: &[(Rect, NavigationItem)],
    selected: usize,
    context_label: &str,
    fonts: Option<&Fonts>,
    content_actions: &[(Rect, ActionCardView)],
    is_grid: bool,
    paint_navigation: bool,
) {
    if paint_navigation {
        canvas.fill(theme_color(ColorRole::Canvas));
    } else {
        canvas.set_clip(Some(content));
        canvas.fill(theme_color(ColorRole::Canvas));
    }
    canvas.set_clip(Some(content));

    // A stable phone-like content surface. The number of rows changes per
    // root page so page transitions remain visible even if a display pipeline
    // maps two colors too similarly.
    let margin = content.width / 22;
    let card_width = content.width.saturating_sub(margin * 2);
    canvas.fill_rect(
        Rect::new(margin, 150, card_width, 190),
        theme_color(ColorRole::Surface),
    );
    canvas.fill_rect(
        Rect::new(margin, 150, 14, 190),
        theme_color(ColorRole::Accent),
    );
    if let (Some(fonts), Some((_, title))) = (fonts, tabs.get(selected)) {
        let header = format!("{context_label} · {}", title.label);
        draw_text_centered(
            canvas,
            &fonts.semibold,
            &header,
            54.0,
            content.x + content.width / 2,
            210,
            theme_color(ColorRole::TextPrimary),
        );
    }

    let row_count = selected.saturating_add(2).min(5);
    let first_placeholder = if content_actions.is_empty() {
        0
    } else {
        row_count
    };
    if !is_grid {
        for row in first_placeholder..row_count {
            let y = 430 + row as u32 * 230;
            if y >= content.height {
                break;
            }
            canvas.fill_rect(
                Rect::new(margin, y, card_width, 170),
                theme_color(ColorRole::Surface),
            );
            canvas.fill_rect(
                Rect::new(margin + 34, y + 42, 86, 86),
                theme_color(ColorRole::Border),
            );
            canvas.fill_rect(
                Rect::new(margin + 154, y + 52, card_width.saturating_sub(210), 24),
                theme_color(ColorRole::Border),
            );
            canvas.fill_rect(
                Rect::new(margin + 154, y + 96, card_width.saturating_sub(290), 18),
                theme_color(ColorRole::Border),
            );
        }
    }

    // ADR-138: the live apps grid paints through `draw_apps_grid`.
    // `is_grid` stays for the leftover `draw_root` path if a caller
    // still asks for letter-square tiles.
    if is_grid {
        draw_app_icon_grid(canvas, fonts, content_actions);
    } else {
        for (rect, card) in content_actions {
            draw_action_card(canvas, *rect, card, fonts);
        }
    }

    canvas.set_clip(None);
    if paint_navigation {
        draw_tab_bar(canvas, tabs, fonts);
    }
}

fn paint_context_header(
    canvas: &mut Canvas<'_>,
    fonts: &Fonts,
    content: Rect,
    header: &ContextHeader,
) {
    let margin = (content.width / 20).max(12);
    let content_width = content.width.saturating_sub(margin * 2);
    let top_inset = ((150_u64 * u64::from(content.height)) / 2400) as u32;
    let mut cursor_y = content.y + top_inset;
    draw_semantic_text(
        canvas,
        fonts,
        &header.heading(),
        content.x + margin,
        cursor_y,
        content_width,
    );
    cursor_y += scaled_line_height(TextRole::Title);
    if let Some(lifecycle) = &header.lifecycle {
        draw_status_indicator(canvas, fonts, lifecycle, content.x + margin, cursor_y);
    }
}

/// S23 letter-square tiles: no per-app icon asset exists, so the
/// "icon" is a colored square with the app's first letter.
fn draw_app_icon_grid(
    canvas: &mut Canvas<'_>,
    fonts: Option<&Fonts>,
    apps: &[(Rect, ActionCardView)],
) {
    for (rect, card) in apps {
        let icon_size = rect.width.min(rect.height.saturating_sub(70)).min(180);
        let icon_x = rect.x + rect.width.saturating_sub(icon_size) / 2;
        canvas.fill_rect(
            Rect::new(icon_x, rect.y, icon_size, icon_size),
            if card.selected {
                theme_color(ColorRole::Elevated)
            } else {
                theme_color(ColorRole::Accent)
            },
        );
        if let Some(fonts) = fonts {
            let initial = card
                .label
                .chars()
                .next()
                .map(|ch| ch.to_uppercase().to_string())
                .unwrap_or_default();
            draw_text_centered(
                canvas,
                &fonts.semibold,
                &initial,
                54.0,
                icon_x + icon_size / 2,
                rect.y + icon_size / 2 - 27,
                theme_color(ColorRole::Canvas),
            );
            draw_text_centered(
                canvas,
                &fonts.regular,
                &card.label,
                26.0,
                rect.x + rect.width / 2,
                rect.y + icon_size + 16,
                theme_color(ColorRole::TextPrimary),
            );
        }
    }
}

/// ADR-138: `Приложения` through `ContextHeader`, same status-layer
/// inset as `draw_now`. No concatenated `draw_root` Surface bar. No
/// skeleton tiles. Empty is a named `SurfacePattern`, not invented icons.
pub fn draw_apps_grid(
    canvas: &mut Canvas<'_>,
    content: Rect,
    tabs: &[(Rect, NavigationItem)],
    header: &ContextHeader,
    apps: &[(Rect, ActionCardView)],
    empty_pattern: Option<&SurfacePattern>,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    canvas.set_clip(Some(content));

    if let Some(fonts) = fonts {
        paint_context_header(canvas, fonts, content, header);
    }

    draw_app_icon_grid(canvas, fonts, apps);

    if apps.is_empty() {
        if let Some(pattern) = empty_pattern {
            draw_surface_pattern(canvas, fonts, content, pattern);
        }
    }

    canvas.set_clip(None);
    draw_tab_bar(canvas, tabs, fonts);
}

/// ADR-139/140/141: Inbox, Spaces, and Система through
/// `ContextHeader`, same status-layer inset as `draw_now` /
/// `draw_apps_grid`. Live cards keep their stacked rects. No
/// concatenated `draw_root` Surface bar. `paint_navigation` is false
/// on a content-only Me scroll frame so the tab strip is not redrawn.
pub fn draw_context_row_list(
    canvas: &mut Canvas<'_>,
    content: Rect,
    tabs: &[(Rect, NavigationItem)],
    header: &ContextHeader,
    rows: &[(Rect, ActionCardView)],
    paint_navigation: bool,
    fonts: Option<&Fonts>,
) {
    if paint_navigation {
        canvas.fill(theme_color(ColorRole::Canvas));
    } else {
        canvas.set_clip(Some(content));
        canvas.fill(theme_color(ColorRole::Canvas));
    }
    canvas.set_clip(Some(content));
    if let Some(fonts) = fonts {
        paint_context_header(canvas, fonts, content, header);
    }
    for (rect, card) in rows {
        draw_action_card(canvas, *rect, card, fonts);
    }
    canvas.set_clip(None);
    if paint_navigation {
        draw_tab_bar(canvas, tabs, fonts);
    }
}

/// Extracted from `draw_root` (VUI-03): the bottom navigation bar is the
/// same four-tab strip regardless of what a page draws above it, so
/// `draw_now` (a real composed screen, not `draw_root`'s diagnostic
/// scaffold) can share this exact drawing code instead of duplicating it.
/// VUI-04 (ADR-116): `tabs` now carries a real `NavigationItem` per
/// destination instead of a bare label -- `selected`/`disabled`/`badge`
/// all come from that item's own data, not a separate index parameter
/// (which `draw_root` still needs for its own unrelated title-lookup/
/// row-count logic, so it keeps its own `selected: usize`, just no
/// longer forwards it here). `pressed` is a real touch-down on that
/// tab (`Shell::pressed_tab`); it uses the pressed token and a bottom
/// hairline so color is never the only cue, and it does not change
/// icon size or move neighbors.
pub fn draw_tab_bar(
    canvas: &mut Canvas<'_>,
    tabs: &[(Rect, NavigationItem)],
    fonts: Option<&Fonts>,
) {
    if let Some(tab_bar) = tabs.first().and_then(|(first, _)| {
        tabs.last().map(|(last, _)| {
            Rect::new(
                first.x,
                first.y,
                last.x.saturating_add(last.width).saturating_sub(first.x),
                first.height.max(last.height),
            )
        })
    }) {
        canvas.fill_rect(tab_bar, theme_color(ColorRole::Surface));
    }

    for (rect, item) in tabs {
        let rect = *rect;
        let is_selected = item.selected;
        let inner = Rect::new(
            rect.x.saturating_add(12),
            rect.y.saturating_add(12),
            rect.width.saturating_sub(24),
            rect.height.saturating_sub(24),
        );
        if item.pressed && !item.disabled {
            canvas.fill_rect(inner, theme_color(ColorRole::Pressed));
            canvas.fill_rect(
                Rect::new(
                    inner.x,
                    inner.y + inner.height.saturating_sub(8),
                    inner.width,
                    8,
                ),
                theme_color(ColorRole::TextSecondary),
            );
        } else if is_selected {
            canvas.fill_rect(inner, theme_color(ColorRole::Elevated));
        }
        if is_selected {
            canvas.fill_rect(
                Rect::new(
                    rect.x.saturating_add(rect.width.saturating_sub(112) / 2),
                    rect.y.saturating_add(18),
                    112.min(rect.width),
                    14,
                ),
                theme_color(ColorRole::Accent),
            );
        }

        let icon_size = if is_selected { 76 } else { 54 };
        let icon_rect = Rect::new(
            rect.x
                .saturating_add(rect.width.saturating_sub(icon_size) / 2),
            rect.y.saturating_add(70),
            icon_size,
            icon_size,
        );
        canvas.fill_rect(
            icon_rect,
            if item.disabled {
                theme_color(ColorRole::DisabledSurface)
            } else if is_selected {
                theme_color(ColorRole::Accent)
            } else {
                theme_color(ColorRole::Border)
            },
        );

        if let Some(fonts) = fonts {
            let text_color = if item.disabled {
                theme_color(ColorRole::DisabledText)
            } else if is_selected {
                theme_color(ColorRole::TextPrimary)
            } else {
                theme_color(ColorRole::TextSecondary)
            };
            draw_text_centered(
                canvas,
                if is_selected {
                    &fonts.semibold
                } else {
                    &fonts.regular
                },
                &item.label,
                if is_selected { 31.0 } else { 27.0 },
                rect.x + rect.width / 2,
                rect.y + 172,
                text_color,
            );

            // Section 7.4: "a real count ... never a decorative dot" --
            // drawn only when `badge` actually carries one. `attention`
            // picks the alert color for it; a badge without `attention`
            // set (not exercised by any real call site yet) would still
            // show in the same neutral accent as a selected tab's own
            // accent mark.
            if let Some(count) = item.badge.filter(|count| *count > 0) {
                let badge_color = if item.attention {
                    theme_color(ColorRole::Attention)
                } else {
                    theme_color(ColorRole::Accent)
                };
                let badge_size = 40;
                let badge_rect = Rect::new(
                    icon_rect.x + icon_rect.width.saturating_sub(badge_size * 2 / 3),
                    icon_rect.y.saturating_sub(badge_size / 3),
                    badge_size,
                    badge_size,
                );
                canvas.fill_rect(badge_rect, badge_color);
                draw_text_centered(
                    canvas,
                    &fonts.semibold,
                    &count.to_string(),
                    24.0,
                    badge_rect.x + badge_rect.width / 2,
                    badge_rect.y + badge_rect.height / 2 - 12,
                    theme_color(ColorRole::HighContrastText),
                );
            }
        }
    }
}

/// VUI-03: the real `Сейчас` composition -- `ContextHeader` +
/// `SystemSection`s + an optional `ObjectSummary`, replacing `draw_root`'s
/// diagnostic scaffold (hardcoded rectangles, an ad hoc header string) for
/// this one page. Built entirely from the section 6/7 primitive/composite
/// draw functions already used by the gallery -- no new text-rendering
/// path, matching every ADR in this file since VUI-02.
#[allow(clippy::too_many_arguments)]
pub fn draw_now(
    canvas: &mut Canvas<'_>,
    content: Rect,
    tabs: &[(Rect, NavigationItem)],
    header: &ContextHeader,
    sections: &[SystemSection],
    object: Option<&ObjectSummary>,
    footer_actions: &[(Rect, DataRow)],
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    let margin = (content.width / 20).max(12);
    let content_width = content.width.saturating_sub(margin * 2);
    canvas.set_clip(Some(content));

    if let Some(fonts) = fonts {
        for (rect, row) in footer_actions {
            draw_data_row(canvas, fonts, row, *rect);
        }

        // `content` spans the full canvas from y=0 -- the status bar is a
        // separate, always-on-top compositor surface (`layer.set_size(0,
        // 120)` in `main.rs`), not a reserved inset inside this one. Content
        // drawn at `content.y` alone renders directly underneath it and is
        // invisible; `draw_root`'s own header/card rows avoid this with
        // hardcoded 150/430 (2400-scale) starting offsets -- this scales the
        // same 150 proportionally instead of repeating the literal, matching
        // `now_grid_rect`'s own scaling convention for its 2400-scale numbers.
        let top_inset = ((150_u64 * u64::from(content.height)) / 2400) as u32;
        let mut cursor_y = content.y + top_inset;
        draw_semantic_text(
            canvas,
            fonts,
            &header.heading(),
            content.x + margin,
            cursor_y,
            content_width,
        );
        cursor_y += scaled_line_height(TextRole::Title);

        // Section 7.1: "a non-default lifecycle is exposed through the
        // nested `StatusIndicator`'s own state, not a second accessible
        // string glued onto the header's name" -- drawn here as a real,
        // always-visible compact mark, not just consulted for the
        // whole-screen empty-state message below. Without this, an offline
        // signal (or an archived-space one) would be silently invisible
        // whenever `sections`/`object` still have real, possibly-stale
        // content to show.
        if let Some(lifecycle) = &header.lifecycle {
            draw_status_indicator(canvas, fonts, lifecycle, content.x + margin, cursor_y);
            cursor_y += scaled_line_height(TextRole::Body);
        }

        if let Some(object) = object {
            cursor_y += physical(SpacingToken::Medium.value());
            cursor_y = draw_object_summary(
                canvas,
                fonts,
                object,
                content.x + margin,
                cursor_y,
                content_width,
            );
        }

        if sections.is_empty() && object.is_none() {
            // HIA-13's own second mockup: "Ничего срочного", centered, no
            // section chrome at all -- an empty `Сейчас` is a normal, calm
            // state, not a broken one, and `SystemSection` itself never
            // invents a placeholder row to fill space (see its own doc
            // comment), so this is the one place that message can honestly
            // come from: the whole-screen empty state, not a per-section one.
            //
            // VUI-03 (ADR-114): that message is only true when this shell
            // actually knows there is nothing pending. When `ContextHeader`
            // itself is reporting `Offline` (no `saai-entityd` connection --
            // see `now_context_header`), an empty `sections`/`object` means
            // "cannot tell," not "confirmed calm," and saying otherwise
            // would be the exact dishonest empty state VUI-03's own
            // acceptance criteria rule out.
            let offline = matches!(
                header.lifecycle.as_ref().map(|status| status.state),
                Some(UniversalState::Offline)
            );
            draw_surface_pattern(canvas, Some(fonts), content, &now_empty_pattern(offline));
        } else {
            for section in sections {
                cursor_y += physical(SpacingToken::Medium.value());
                draw_semantic_text(
                    canvas,
                    fonts,
                    &section.heading(),
                    content.x + margin,
                    cursor_y,
                    content_width,
                );
                cursor_y += scaled_line_height(TextRole::Section);
                let hairline = physical(StrokeToken::Hairline.value()).max(1);
                draw_divider(
                    canvas,
                    &section.divider(),
                    Rect::new(content.x + margin, cursor_y, content_width, hairline),
                );
                cursor_y += physical(SpacingToken::Small.value());

                // "A section with no children renders only its title" -- title
                // and divider are already drawn above; nothing else to add for
                // an empty section, and never an invented filler row.
                if section.is_empty() {
                    continue;
                }

                for row in &section.rows {
                    match row {
                        SystemSectionRow::Data(data_row) => {
                            let row_height = physical(data_row.min_hit_height());
                            draw_data_row(
                                canvas,
                                fonts,
                                data_row,
                                Rect::new(content.x + margin, cursor_y, content_width, row_height),
                            );
                            cursor_y += row_height;
                        }
                        SystemSectionRow::Status(status) => {
                            draw_status_indicator(
                                canvas,
                                fonts,
                                status,
                                content.x + margin,
                                cursor_y,
                            );
                            cursor_y += scaled_line_height(TextRole::Body);
                            if status.visible_reason().is_some() {
                                cursor_y += scaled_line_height(TextRole::Caption);
                            }
                        }
                        SystemSectionRow::Task(task) => {
                            let status = task.status();
                            draw_status_indicator(
                                canvas,
                                fonts,
                                &status,
                                content.x + margin,
                                cursor_y,
                            );
                            cursor_y += scaled_line_height(TextRole::Body);
                            if status.visible_reason().is_some() {
                                cursor_y += scaled_line_height(TextRole::Caption);
                            }
                            if let Some(related) = task.related_text() {
                                draw_semantic_text(
                                    canvas,
                                    fonts,
                                    &related,
                                    content.x + margin,
                                    cursor_y,
                                    content_width,
                                );
                                cursor_y += scaled_line_height(TextRole::Caption);
                            }
                        }
                        SystemSectionRow::Intent(intent) => {
                            draw_semantic_text(
                                canvas,
                                fonts,
                                &intent.heading(),
                                content.x + margin,
                                cursor_y,
                                content_width,
                            );
                            cursor_y += scaled_line_height(TextRole::Body);
                            if let Some(task) = &intent.task {
                                let status = task.status();
                                draw_status_indicator(
                                    canvas,
                                    fonts,
                                    &status,
                                    content.x + margin,
                                    cursor_y,
                                );
                                cursor_y += scaled_line_height(TextRole::Body);
                                if status.visible_reason().is_some() {
                                    cursor_y += scaled_line_height(TextRole::Caption);
                                }
                            } else if let Some(missing) = intent.missing_task_text() {
                                draw_semantic_text(
                                    canvas,
                                    fonts,
                                    &missing,
                                    content.x + margin,
                                    cursor_y,
                                    content_width,
                                );
                                cursor_y += scaled_line_height(TextRole::Caption);
                            }
                        }
                        SystemSectionRow::Metric(metric) => {
                            let (label_font, label_size) = fonts.resolve(TextRole::Caption);
                            draw_text(
                                canvas,
                                label_font,
                                &metric.label,
                                label_size,
                                content.x + margin,
                                cursor_y,
                                theme_color(ColorRole::TextSecondary),
                            );
                            cursor_y += scaled_line_height(TextRole::Caption);
                            let value_text = match &metric.value {
                                MetricValue::Known(value) => match &metric.unit {
                                    Some(unit) => format!("{value} {unit}"),
                                    None => value.clone(),
                                },
                                MetricValue::Unknown => "—".to_string(),
                                MetricValue::Unavailable => "—".to_string(),
                            };
                            let (value_font, value_size) = fonts.resolve(TextRole::Body);
                            draw_text(
                                canvas,
                                value_font,
                                &value_text,
                                value_size,
                                content.x + margin,
                                cursor_y,
                                theme_color(ColorRole::TextPrimary),
                            );
                            cursor_y += scaled_line_height(TextRole::Body);
                        }
                    }
                    if cursor_y >= content.y + content.height {
                        break;
                    }
                }
            }
        }
    }

    canvas.set_clip(None);
    draw_tab_bar(canvas, tabs, fonts);
}

/// The permanent system layer's real content (S13 Change 1) -- time on
/// the left, network and battery state on the right. The facts come from
/// `SystemStatus` (VUI-04); this function only paints them. Context Light
/// is a square of the Space color, drawn before the font early-return so
/// the dot stays visible if `Fonts::load_system()` failed.
pub fn draw_status_bar(
    canvas: &mut Canvas<'_>,
    width: u32,
    height: u32,
    status: &SystemStatus,
    fonts: Option<&Fonts>,
) {
    canvas.fill(theme_color(ColorRole::Canvas));
    let margin = width / 30;
    let dot_size = 22;
    let dot_y = height / 2 - dot_size / 2;
    canvas.fill_rect(
        Rect::new(margin, dot_y, dot_size, dot_size),
        context_color(status.context),
    );
    let Some(fonts) = fonts else {
        return;
    };
    let baseline = height / 2 - 22;
    let time_x = margin + dot_size + 16;
    draw_text(
        canvas,
        &fonts.semibold,
        &status.time_text,
        44.0,
        time_x,
        baseline,
        theme_color(ColorRole::TextPrimary),
    );

    let wifi_label = status.network_label();
    let wifi_color = if status.network_up {
        theme_color(ColorRole::Accent)
    } else {
        theme_color(ColorRole::TextSecondary)
    };
    let battery_label = status.battery_label().unwrap_or_default();

    let gap = 40.0;
    let battery_width = text_width(&fonts.semibold, &battery_label, 40.0 * text_scale());
    let battery_left = width as f32 - margin as f32 - battery_width;
    if !battery_label.is_empty() {
        draw_text(
            canvas,
            &fonts.semibold,
            &battery_label,
            40.0,
            battery_left.round() as u32,
            baseline,
            theme_color(ColorRole::TextPrimary),
        );
    }
    let wifi_width = text_width(&fonts.regular, wifi_label, 36.0 * text_scale());
    let wifi_left = battery_left - gap - wifi_width;
    draw_text(
        canvas,
        &fonts.regular,
        wifi_label,
        36.0,
        wifi_left.round() as u32,
        baseline + 4,
        wifi_color,
    );
}

/// A short glyph (hyphen, period, colon, apostrophe...) has a bitmap only a
/// few pixels tall, cropped tight to its own ink by `fontdue::rasterize`.
/// Blitting every glyph's bitmap starting at the same `top` row (as this
/// code did before) top-aligns bitmaps instead of baseline-aligning them --
/// invisible for ordinary letters, whose bitmap height happens to span
/// close to a full line already, but a short glyph then renders far above
/// where it belongs (a hyphen appearing as a floating mark near cap-height
/// instead of sitting at mid-height -- found via `os/targets/panther/
/// tools/screencap.c`'s real device screenshot, "Wi-Fi"/"PIN-код").
///
/// `fontdue::Metrics::ymin` is the glyph bitmap's bottom edge, in whole
/// pixels above the baseline (fontdue's own doc comment). Every glyph in a
/// run shares one baseline, so the right anchor is the TALLEST glyph
/// actually present in this specific run: for that glyph,
/// `ymin + height` already equals what `top` alone used to (correctly)
/// place, so this returns 0 and every full-height call site keeps its
/// current pixel position unchanged; every shorter glyph gets pushed down
/// by exactly the difference, landing on the same baseline as its
/// neighbors instead of floating at the top.
fn glyph_baseline_offset(reference_height: i32, metrics: &fontdue::Metrics) -> i32 {
    reference_height - (metrics.ymin + metrics.height as i32)
}

fn reference_glyph_height(glyphs: &[(fontdue::Metrics, Vec<u8>)]) -> i32 {
    glyphs
        .iter()
        .map(|(metrics, _)| metrics.ymin + metrics.height as i32)
        .max()
        .unwrap_or(0)
}

fn now_empty_pattern(offline: bool) -> SurfacePattern {
    if offline {
        SurfacePattern::offline("Нет связи с пространствами")
    } else {
        SurfacePattern::empty("Ничего срочного")
    }
}

/// ADR-155: centered Body copy for empty / loading / offline. Idle
/// keeps the mark off so a calm empty stays text-only.
fn draw_surface_pattern(
    canvas: &mut Canvas<'_>,
    fonts: Option<&Fonts>,
    content: Rect,
    pattern: &SurfacePattern,
) {
    let center_x = content.x + content.width / 2;
    let center_y = content.y + content.height / 2;
    if pattern.paints_mark() {
        let mark_size = physical(IconSize::Medium.value());
        let spacing = physical(SpacingToken::Small.value());
        let mark_left = center_x.saturating_sub(mark_size / 2);
        let mark_top = center_y.saturating_sub(mark_size + spacing);
        draw_calibration_mark(
            canvas,
            Rect::new(mark_left, mark_top, mark_size, mark_size),
            pattern.state.style().mark,
            theme_color(pattern.state.style().color),
        );
    }
    if let Some(fonts) = fonts {
        let text = pattern.message_text();
        let (font, size) = fonts.resolve(text.role);
        draw_text_centered(
            canvas,
            font,
            &text.content,
            size,
            center_x,
            center_y,
            theme_color(text.color),
        );
    }
}

fn draw_text_centered(
    canvas: &mut Canvas<'_>,
    font: &Font,
    text: &str,
    size: f32,
    center_x: u32,
    top: u32,
    color: Pixel,
) {
    let size = size * text_scale();
    let glyphs: Vec<_> = text
        .chars()
        .map(|character| font.rasterize(character, size))
        .collect();
    let width = glyphs
        .iter()
        .map(|(metrics, _)| metrics.advance_width)
        .sum::<f32>();
    let reference_height = reference_glyph_height(&glyphs);
    let mut cursor = center_x as f32 - width / 2.0;
    for (metrics, bitmap) in &glyphs {
        let glyph_x = cursor.round() as i32 + metrics.xmin;
        let glyph_y = top as i32 + glyph_baseline_offset(reference_height, metrics);
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                canvas.blend(
                    glyph_x + column as i32,
                    glyph_y + row as i32,
                    color,
                    bitmap[row * metrics.width + column],
                );
            }
        }
        cursor += metrics.advance_width;
    }
}

fn draw_text(
    canvas: &mut Canvas<'_>,
    font: &Font,
    text: &str,
    size: f32,
    left: u32,
    top: u32,
    color: Pixel,
) {
    let size = size * text_scale();
    let glyphs: Vec<_> = text
        .chars()
        .map(|character| font.rasterize(character, size))
        .collect();
    let reference_height = reference_glyph_height(&glyphs);
    let mut cursor = left as f32;
    for (metrics, bitmap) in &glyphs {
        let glyph_x = cursor.round() as i32 + metrics.xmin;
        let glyph_y = top as i32 + glyph_baseline_offset(reference_height, metrics);
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                canvas.blend(
                    glyph_x + column as i32,
                    glyph_y + row as i32,
                    color,
                    bitmap[row * metrics.width + column],
                );
            }
        }
        cursor += metrics.advance_width;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_contrast_boost, composite_gallery_decision_buttons, composite_gallery_row_positions,
        context_color, draw_action_card, draw_apps_grid, draw_calibration, draw_composite_gallery,
        draw_consent, draw_context_row_list, draw_gallery, draw_intent_input, draw_lock_idle,
        draw_lock_pin_entry, draw_lock_sleep, draw_object_view, draw_orb, draw_pin_setup,
        draw_remote_pair, draw_root, draw_status_bar, draw_surface_pattern, draw_tab_bar,
        gallery_row_positions, now_empty_pattern, physical, physical_line_height, state_color,
        theme_color, ActionCardView, Canvas,
    };
    use saai_ui_core::{
        composite_gallery_fixtures, ColorRole, ContextColor, ContextHeader, DecisionOverlay, Field,
        FieldKind, IconSize, LogicalUnit, NavigationItem, ObjectSummary, Progress, Rect,
        SpacingToken, StatusIndicator, StatusMark, SurfacePattern, SystemStatus, TextRole,
        UniversalState, MIN_TOUCH_TARGET,
    };

    #[test]
    fn semantic_colors_use_the_physically_calibrated_panel_packing() {
        assert_eq!(theme_color(ColorRole::Canvas), [0, 0x07, 0x10, 0x11]);
        assert_eq!(theme_color(ColorRole::Accent), [0, 0x63, 0xD4, 0xD6]);
        assert_eq!(context_color(ContextColor::Blue), [0, 0x58, 0x9C, 0xE8]);
        assert_eq!(
            state_color(UniversalState::Attention),
            [0, 0xD4, 0xB6, 0x58]
        );
    }

    #[test]
    fn calibration_fixture_contains_palette_context_and_state_channels() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        draw_calibration(canvas, width, height, None);

        let margin = width / 20;
        let gap = width / 60;
        let cell_width = (width - margin * 2 - gap * 2) / 3;
        let cell_height = height / 24;
        let palette_top = height / 10;
        // Accent is palette entry 3: first column, second row.
        assert_eq!(
            canvas.pixel(margin + 4, palette_top + cell_height + 4),
            theme_color(ColorRole::Accent)
        );

        let context_top = palette_top + 6 * cell_height + height / 30;
        // Blue is context entry 1: second column, first row.
        assert_eq!(
            canvas.pixel(margin + cell_width + gap + 4, context_top + 4),
            context_color(ContextColor::Blue)
        );

        let state_top = context_top + 2 * cell_height + height / 30;
        let state_height = (height - state_top - margin) / 9;
        // Attention is state entry 5. The left band is its semantic color.
        assert_eq!(
            canvas.pixel(margin + 2, state_top + 5 * state_height + 12),
            state_color(UniversalState::Attention)
        );
    }

    #[test]
    fn contrast_boost_zero_is_a_byte_for_byte_no_op() {
        let original = vec![10u8, 90, 128, 200, 250, 0];
        let mut pixels = original.clone();
        apply_contrast_boost(&mut pixels, 0);
        assert_eq!(pixels, original);
    }

    #[test]
    fn contrast_boost_pushes_values_away_from_the_midpoint() {
        let mut pixels = vec![100u8, 128, 200];
        apply_contrast_boost(&mut pixels, 100);
        // Below the 128 pivot moves further down, above it moves
        // further up, exactly at the pivot stays put.
        assert!(pixels[0] < 100);
        assert_eq!(pixels[1], 128);
        assert!(pixels[2] > 200);
    }

    #[test]
    fn status_bar_paints_context_light_without_fonts_and_never_uses_severity() {
        let width = 1080;
        let height = 80;
        let mut pixels = vec![0; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let status = SystemStatus::new(ContextColor::Orange, "09:42");
        draw_status_bar(canvas, width, height, &status, None);
        let margin = width / 30;
        assert_eq!(
            canvas.pixel(margin + 2, height / 2),
            context_color(ContextColor::Orange)
        );
        assert_ne!(
            canvas.pixel(margin + 2, height / 2),
            theme_color(ColorRole::Attention)
        );
        assert_ne!(
            canvas.pixel(margin + 2, height / 2),
            theme_color(ColorRole::Critical)
        );
        assert_ne!(
            canvas.pixel(margin + 2, height / 2),
            context_color(ContextColor::Default)
        );
    }

    #[test]
    fn selected_indicator_moves_between_edge_tabs() {
        let mut pixels = vec![0; 1080 * 2400 * 4];
        let labels = ["Сейчас", "Входящие", "Пространства", "Система"];
        let make_tabs = |selected_index: usize| -> Vec<(Rect, NavigationItem)> {
            labels
                .iter()
                .enumerate()
                .map(|(index, label)| {
                    let mut item = NavigationItem::new(*label, *label);
                    if index == selected_index {
                        item = item.selected();
                    }
                    (Rect::new(index as u32 * 270, 2100, 270, 300), item)
                })
                .collect()
        };
        let mut canvas = Canvas::new(&mut pixels, 1080, 2400);
        draw_root(
            &mut canvas,
            Rect::new(0, 0, 1080, 2100),
            &make_tabs(0),
            0,
            "Дом",
            None,
            &[],
            true,
            true,
        );
        assert_eq!(canvas.pixel(135, 2125), theme_color(ColorRole::Accent));
        assert_eq!(canvas.pixel(945, 2125), theme_color(ColorRole::Surface));

        draw_root(
            &mut canvas,
            Rect::new(0, 0, 1080, 2100),
            &make_tabs(3),
            3,
            "Дом",
            None,
            &[],
            false,
            true,
        );
        assert_eq!(canvas.pixel(135, 2125), theme_color(ColorRole::Surface));
        assert_eq!(canvas.pixel(945, 2125), theme_color(ColorRole::Accent));
    }

    #[test]
    fn scroll_frame_does_not_repaint_navigation() {
        let mut pixels = vec![0; 1080 * 2400 * 4];
        let tabs = vec![(
            Rect::new(0, 2100, 270, 300),
            NavigationItem::new("me", "Система").selected(),
        )];
        let mut canvas = Canvas::new(&mut pixels, 1080, 2400);
        draw_root(
            &mut canvas,
            Rect::new(0, 0, 1080, 2100),
            &tabs,
            3,
            "Дом",
            None,
            &[],
            false,
            false,
        );
        assert_eq!(canvas.pixel(135, 2125), [0, 0, 0, 0]);
    }

    #[test]
    fn pressed_tab_uses_pressed_token_without_shifting_neighbors() {
        let tabs = |pressed_inbox: bool| -> Vec<(Rect, NavigationItem)> {
            let mut inbox = NavigationItem::new("inbox", "Входящие");
            if pressed_inbox {
                inbox = inbox.pressed();
            }
            vec![
                (
                    Rect::new(0, 2100, 270, 300),
                    NavigationItem::new("now", "Сейчас").selected(),
                ),
                (Rect::new(270, 2100, 270, 300), inbox),
            ]
        };
        let mut idle = vec![0; 1080 * 2400 * 4];
        let mut down = vec![0; 1080 * 2400 * 4];
        draw_tab_bar(&mut Canvas::new(&mut idle, 1080, 2400), &tabs(false), None);
        draw_tab_bar(&mut Canvas::new(&mut down, 1080, 2400), &tabs(true), None);
        assert_eq!(
            Canvas::new(&mut down, 1080, 2400).pixel(292, 2122),
            theme_color(ColorRole::Pressed)
        );
        assert_ne!(
            Canvas::new(&mut idle, 1080, 2400).pixel(292, 2122),
            theme_color(ColorRole::Pressed)
        );
        assert_eq!(
            Canvas::new(&mut down, 1080, 2400).pixel(135, 2125),
            theme_color(ColorRole::Accent)
        );
        assert_eq!(
            Canvas::new(&mut idle, 1080, 2400).pixel(135, 2125),
            theme_color(ColorRole::Accent)
        );
    }

    #[test]
    fn content_clip_cannot_paint_over_the_navigation_strip() {
        let mut pixels = vec![0; 1080 * 2400 * 4];
        let mut canvas = Canvas::new(&mut pixels, 1080, 2400);
        canvas.fill(theme_color(ColorRole::Surface));
        canvas.set_clip(Some(Rect::new(0, 0, 1080, 2100)));
        canvas.fill(theme_color(ColorRole::Canvas));
        canvas.fill_rect(
            Rect::new(0, 2000, 1080, 400),
            theme_color(ColorRole::Accent),
        );
        canvas.set_clip(None);
        assert_eq!(canvas.pixel(540, 1000), theme_color(ColorRole::Canvas));
        assert_eq!(canvas.pixel(540, 2200), theme_color(ColorRole::Surface));
        assert_ne!(canvas.pixel(540, 2050), theme_color(ColorRole::Surface));
    }

    #[test]
    fn orb_mark_shapes_differ_between_states() {
        // HIA-16's own acceptance line (HIA-ROADMAP.md), now carried by
        // VUI-04's real `StatusMark` per state (ADR-116) instead of a
        // binary solid-square-or-hollow-ring: every state must be
        // distinguishable by shape, not only by color. Compares whole
        // rendered buffers rather than hand-picked pixel coordinates,
        // since each `StatusMark` variant's exact geometry is
        // `draw_calibration_mark`'s own concern, not this test's.
        let render_mark = |mark: StatusMark| -> Vec<u8> {
            let mut pixels = vec![0u8; 200 * 200 * 4];
            let mut canvas = Canvas::new(&mut pixels, 200, 200);
            let dot_rect = Rect::new(50, 50, 100, 100);
            draw_orb(
                &mut canvas,
                dot_rect,
                theme_color(ColorRole::Accent),
                mark,
                false,
                None,
                false,
                &[],
                None,
            );
            pixels
        };
        let idle = render_mark(StatusMark::Outline);
        let attention = render_mark(StatusMark::Alert);
        let offline = render_mark(StatusMark::Offline);
        assert_ne!(idle, attention);
        assert_ne!(idle, offline);
        assert_ne!(attention, offline);
    }

    #[test]
    fn attention_ring_is_drawn_beyond_the_alert_mark() {
        let render = |ring: bool| -> Vec<u8> {
            let mut pixels = vec![0u8; 200 * 200 * 4];
            let mut canvas = Canvas::new(&mut pixels, 200, 200);
            draw_orb(
                &mut canvas,
                Rect::new(50, 50, 100, 100),
                theme_color(ColorRole::Accent),
                StatusMark::Alert,
                ring,
                None,
                false,
                &[],
                None,
            );
            pixels
        };
        assert_ne!(render(true), render(false));
    }

    #[test]
    fn quantity_fill_uses_border_not_severity_and_missing_stays_absent() {
        let rect = Rect::new(50, 50, 100, 100);
        let render = |quantity: Option<u8>| -> Vec<u8> {
            let mut pixels = vec![0u8; 200 * 200 * 4];
            let mut canvas = Canvas::new(&mut pixels, 200, 200);
            draw_orb(
                &mut canvas,
                rect,
                theme_color(ColorRole::Accent),
                StatusMark::Outline,
                false,
                quantity,
                false,
                &[],
                None,
            );
            pixels
        };
        let missing = render(None);
        let filled = render(Some(87));
        assert_ne!(missing, filled);
        let mut filled_pixels = filled;
        let canvas = Canvas::new(&mut filled_pixels, 200, 200);
        let track_y = rect.y + rect.height - physical(Progress::MIN_TRACK_HEIGHT).max(1);
        assert_eq!(
            canvas.pixel(rect.x + 10, track_y),
            theme_color(ColorRole::Border)
        );
        assert_ne!(
            canvas.pixel(rect.x + 10, track_y),
            theme_color(ColorRole::Attention)
        );
        assert_ne!(
            canvas.pixel(rect.x + 10, track_y),
            theme_color(ColorRole::Critical)
        );
        let mut missing_pixels = missing;
        let missing_canvas = Canvas::new(&mut missing_pixels, 200, 200);
        assert_ne!(
            missing_canvas.pixel(rect.x + 10, track_y),
            theme_color(ColorRole::Border)
        );
    }

    #[test]
    fn activity_pulse_inset_is_not_an_attention_ring() {
        let render = |pulse: bool| -> Vec<u8> {
            let mut pixels = vec![0u8; 200 * 200 * 4];
            let mut canvas = Canvas::new(&mut pixels, 200, 200);
            draw_orb(
                &mut canvas,
                Rect::new(50, 50, 100, 100),
                theme_color(ColorRole::Accent),
                StatusMark::Activity,
                false,
                None,
                pulse,
                &[],
                None,
            );
            pixels
        };
        assert_ne!(render(true), render(false));
        assert_ne!(render(true), {
            let mut pixels = vec![0u8; 200 * 200 * 4];
            let mut canvas = Canvas::new(&mut pixels, 200, 200);
            draw_orb(
                &mut canvas,
                Rect::new(50, 50, 100, 100),
                theme_color(ColorRole::Accent),
                StatusMark::Activity,
                true,
                None,
                false,
                &[],
                None,
            );
            pixels
        });
    }

    #[test]
    fn gallery_rows_never_overlap() {
        // ADR-105's own bug, pinned as a regression test: the gap between
        // consecutive rows must be at least one full Body line height, the
        // tallest single line any gallery primitive draws, plus room for a
        // second (Caption) line underneath it. A smaller gap is exactly
        // the class of error that let `StatusIndicator`'s reason line and
        // `DataRow`'s secondary line collide with the line above them.
        let rows = gallery_row_positions(2400);
        let min_gap =
            physical_line_height(TextRole::Body) + physical_line_height(TextRole::Caption);
        for pair in rows.windows(2) {
            let gap = pair[1] - pair[0];
            assert!(
                gap >= min_gap,
                "gap {gap} between rows at {} and {} is smaller than {min_gap}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn gallery_rows_stay_within_the_screen() {
        let rows = gallery_row_positions(2400);
        assert!(*rows.last().unwrap() < 2400);
    }

    #[test]
    fn physical_line_height_matches_the_pixel_7_scale() {
        // Pinned, not computed here -- a golden value. `TextRole::Body`'s
        // logical line height is 24 units; at Pixel 7's 3-physical-per-
        // logical scale that is exactly 72. If this ever changes,
        // `gallery_rows_never_overlap`'s own minimum gap changes with it
        // automatically, but this test makes the actual number visible
        // rather than only implied.
        assert_eq!(physical_line_height(TextRole::Body), 72);
        assert_eq!(physical_line_height(TextRole::Caption), 48);
    }

    #[test]
    fn now_object_summary_rect_sits_below_the_header_and_above_the_footer() {
        let content = Rect::new(0, 0, 1080, 2160);
        let object = ObjectSummary::new("vnnnmb", "saaios.intent · версия 1");
        let rect = super::now_object_summary_rect(content, false, &object);
        assert!(rect.y >= 150);
        assert!(rect.height >= physical(MIN_TOUCH_TARGET));
        assert!(rect.y + rect.height < 1800);
        let mid_y = rect.y + rect.height / 2;
        assert!(rect.contains(540.0, f64::from(mid_y)));
        assert!(!rect.contains(540.0, 2000.0));
    }

    #[test]
    fn apps_grid_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, 2160);
        let header = ContextHeader::new("Дом").with_section_title("Приложения");
        let cell = Rect::new(49, 430, 310, 300);
        let apps = vec![(cell, ActionCardView::new("Saai Demo", "", "Запустить"))];
        draw_apps_grid(canvas, content, &[], &header, &apps, None, None);
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(cell.x + cell.width / 2, cell.y + 40),
            theme_color(ColorRole::Accent)
        );
    }

    #[test]
    fn apps_grid_empty_does_not_invent_tiles() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, 2160);
        let header = ContextHeader::new("Дом").with_section_title("Приложения");
        draw_apps_grid(
            canvas,
            content,
            &[],
            &header,
            &[],
            Some(&SurfacePattern::empty("Нет приложений")),
            None,
        );
        assert_eq!(canvas.pixel(200, 430), theme_color(ColorRole::Canvas));
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
    }

    #[test]
    fn now_empty_pattern_is_idle_when_connected_and_offline_when_not() {
        let calm = now_empty_pattern(false);
        assert_eq!(calm.state, UniversalState::Idle);
        assert!(!calm.paints_mark());
        assert_eq!(calm.message, "Ничего срочного");
        let offline = now_empty_pattern(true);
        assert_eq!(offline.state, UniversalState::Offline);
        assert!(offline.paints_mark());
        assert_eq!(offline.message, "Нет связи с пространствами");
    }

    #[test]
    fn surface_pattern_loading_paints_a_mark_empty_does_not() {
        let width = 1080;
        let height = 2400;
        let content = Rect::new(0, 0, width, 2160);
        let mark_size = physical(IconSize::Medium.value());
        let spacing = physical(SpacingToken::Small.value());
        let mark_x = content.x + content.width / 2 - mark_size / 2;
        let mark_y = content.y + content.height / 2 - mark_size - spacing;
        let sample_x = mark_x;
        let sample_y = mark_y + (mark_size / 4).max(2);

        let mut empty_pixels = vec![0u8; width as usize * height as usize * 4];
        {
            let canvas = &mut Canvas::new(&mut empty_pixels, width, height);
            canvas.fill(theme_color(ColorRole::Canvas));
            draw_surface_pattern(
                canvas,
                None,
                content,
                &SurfacePattern::empty("Ничего срочного"),
            );
        }
        let empty = Canvas::new(&mut empty_pixels, width, height);
        assert_eq!(
            empty.pixel(sample_x, sample_y),
            theme_color(ColorRole::Canvas)
        );

        let mut loading_pixels = vec![0u8; width as usize * height as usize * 4];
        {
            let canvas = &mut Canvas::new(&mut loading_pixels, width, height);
            canvas.fill(theme_color(ColorRole::Canvas));
            draw_surface_pattern(
                canvas,
                None,
                content,
                &SurfacePattern::loading("Сканирование…"),
            );
        }
        let loading = Canvas::new(&mut loading_pixels, width, height);
        assert_eq!(
            loading.pixel(sample_x, sample_y),
            theme_color(ColorRole::TextSecondary)
        );
    }

    #[test]
    fn inbox_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, 2160);
        let header = ContextHeader::new("Дом").with_section_title("Входящие");
        let row = Rect::new(49, 430, 982, 190);
        let rows = vec![(
            row,
            ActionCardView::new("Нет новых задач и уведомлений", "", ""),
        )];
        draw_context_row_list(canvas, content, &[], &header, &rows, true, None);
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(row.x + 40, row.y + 40),
            theme_color(ColorRole::Surface)
        );
    }

    #[test]
    fn spaces_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, 2160);
        let header = ContextHeader::new("Дом").with_section_title("Пространства");
        let row = Rect::new(49, 430, 982, 190);
        let rows = vec![(
            row,
            ActionCardView::new("Дом", "Объектов: 1", "").selected(true),
        )];
        draw_context_row_list(canvas, content, &[], &header, &rows, true, None);
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(row.x + 40, row.y + 40),
            theme_color(ColorRole::Elevated)
        );
    }

    #[test]
    fn me_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, 2160);
        let header = ContextHeader::new("Работа").with_section_title("Система");
        let row = Rect::new(49, 430, 982, 190);
        let rows = vec![(row, ActionCardView::new("Pixel 7", "Это устройство", ""))];
        draw_context_row_list(canvas, content, &[], &header, &rows, true, None);
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(row.x + 40, row.y + 40),
            theme_color(ColorRole::Surface)
        );
    }

    #[test]
    fn me_scroll_does_not_repaint_navigation() {
        let mut pixels = vec![0; 1080 * 2400 * 4];
        let tabs = vec![(
            Rect::new(0, 2100, 270, 300),
            NavigationItem::new("me", "Система").selected(),
        )];
        let header = ContextHeader::new("Работа").with_section_title("Система");
        let mut canvas = Canvas::new(&mut pixels, 1080, 2400);
        draw_context_row_list(
            &mut canvas,
            Rect::new(0, 0, 1080, 2100),
            &tabs,
            &header,
            &[],
            false,
            None,
        );
        assert_eq!(canvas.pixel(135, 2125), [0, 0, 0, 0]);
    }

    #[test]
    fn consent_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, 2100);
        let header = ContextHeader::new("Работа").with_section_title("Разрешение");
        let row = Rect::new(49, 430, 982, 190);
        let rows = vec![(
            row,
            ActionCardView::new("Saai Demo", "запрашивает доступ", ""),
        )];
        draw_consent(
            canvas,
            content,
            &header,
            &rows,
            Rect::new(0, 2100, 540, 300),
            Rect::new(540, 2100, 540, 300),
            None,
        );
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(row.x + 40, row.y + 40),
            theme_color(ColorRole::Surface)
        );
        assert_eq!(canvas.pixel(270, 2250), theme_color(ColorRole::Accent));
        assert_eq!(canvas.pixel(810, 2250), theme_color(ColorRole::Surface));
    }

    #[test]
    fn pin_setup_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, height);
        let header = ContextHeader::new("Работа").with_section_title("PIN");
        let field = Field::new("Новый PIN-код", FieldKind::Password)
            .with_placeholder("Введите новый PIN (минимум 4 цифры)");
        let field_rect = Rect::new(49, 430, 982, 190);
        let keys = vec![(Rect::new(108, 900, 264, 240), "1".to_string())];
        draw_pin_setup(
            canvas, content, &header, &field, field_rect, &keys, None, false, None,
        );
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(canvas.pixel(240, 1020), theme_color(ColorRole::Elevated));
    }

    #[test]
    fn intent_input_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, height);
        let header = ContextHeader::new("Дом").with_section_title("Намерение");
        let field =
            Field::new("Новое намерение", FieldKind::Text).with_placeholder("Наберите текст…");
        let field_rect = Rect::new(49, 1490, 982, 190);
        let keys = vec![(Rect::new(108, 1700, 96, 144), "Q".to_string())];
        draw_intent_input(
            canvas, content, &header, &field, field_rect, &keys, None, false, None,
        );
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(canvas.pixel(156, 1772), theme_color(ColorRole::Elevated));
    }

    #[test]
    fn remote_pair_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, 2100);
        let header = ContextHeader::new("Работа").with_section_title("SSH");
        let row = Rect::new(49, 430, 982, 190);
        let rows = vec![(row, ActionCardView::new("test-client", "", ""))];
        draw_remote_pair(
            canvas,
            content,
            &header,
            &rows,
            "SHA256:OuaL+poCXsAtdU50sBMBwOUNL+faDqJqWTmiE3baoOI",
            Rect::new(0, 2100, 540, 300),
            Rect::new(540, 2100, 540, 300),
            None,
        );
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(row.x + 40, row.y + 40),
            theme_color(ColorRole::Surface)
        );
        assert_eq!(canvas.pixel(270, 2250), theme_color(ColorRole::Accent));
        assert_eq!(canvas.pixel(810, 2250), theme_color(ColorRole::Surface));
    }

    #[test]
    fn bluetooth_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, height);
        let header = ContextHeader::new("Дом").with_section_title("Bluetooth");
        let row = Rect::new(49, 430, 982, 190);
        let rows = vec![(row, ActionCardView::new("Нет устройств", "", ""))];
        draw_context_row_list(canvas, content, &[], &header, &rows, false, None);
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(row.x + 40, row.y + 40),
            theme_color(ColorRole::Surface)
        );
    }

    #[test]
    fn wifi_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, height);
        let header = ContextHeader::new("Дом").with_section_title("Wi-Fi");
        let row = Rect::new(49, 430, 982, 190);
        let rows = vec![(row, ActionCardView::new("Нет сетей", "", ""))];
        draw_context_row_list(canvas, content, &[], &header, &rows, false, None);
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(row.x + 40, row.y + 40),
            theme_color(ColorRole::Surface)
        );
    }

    #[test]
    fn trusted_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, height);
        let header = ContextHeader::new("Дом").with_section_title("Ключи");
        let row = Rect::new(49, 430, 982, 190);
        let rows = vec![(row, ActionCardView::new("Нет клиентов", "", ""))];
        draw_context_row_list(canvas, content, &[], &header, &rows, false, None);
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(row.x + 40, row.y + 40),
            theme_color(ColorRole::Surface)
        );
    }

    #[test]
    fn diagnostic_does_not_paint_the_root_surface_bar() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let content = Rect::new(0, 0, width, height);
        let header = ContextHeader::new("Дом").with_section_title("Диагностика");
        let row = Rect::new(49, 430, 982, 190);
        let rows = vec![(row, ActionCardView::new("Сборка", "abc123", ""))];
        draw_context_row_list(canvas, content, &[], &header, &rows, false, None);
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(
            canvas.pixel(row.x + 40, row.y + 40),
            theme_color(ColorRole::Surface)
        );
    }

    #[test]
    fn lock_idle_fill_is_canvas_not_the_diagnostic_red() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        draw_lock_idle(
            canvas,
            width,
            height,
            "22:46",
            "Коснитесь, чтобы разблокировать",
            None,
            None,
            None,
        );
        assert_eq!(canvas.pixel(540, 1200), theme_color(ColorRole::Canvas));
        assert_ne!(canvas.pixel(540, 1200), [0x00, 0xd0, 0x00, 0x00]);
    }

    #[test]
    fn lock_sleep_fill_is_canvas_not_the_diagnostic_red() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        draw_lock_sleep(canvas, width, height, "22:46", None);
        assert_eq!(canvas.pixel(540, 1200), theme_color(ColorRole::Canvas));
        assert_ne!(canvas.pixel(540, 1200), [0x00, 0xd0, 0x00, 0x00]);
        assert_ne!(canvas.pixel(540, 1200), [0x00, 0x00, 0x00, 0x00]);
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
    }

    #[test]
    fn lock_idle_attention_mark_uses_attention_color_not_a_surface_card() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let indicator = StatusIndicator::new(UniversalState::Attention, "Требует внимания");
        draw_lock_idle(
            canvas,
            width,
            height,
            "22:46",
            "Коснитесь, чтобы разблокировать",
            None,
            Some(&indicator),
            None,
        );
        let time_y = ((height as u64 * 480) / 2400) as u32;
        let hint_y =
            time_y + physical_line_height(TextRole::Display) + physical(LogicalUnit::new(16));
        let top = hint_y + physical_line_height(TextRole::Body) + physical(LogicalUnit::new(24));
        let left = width / 22;
        let mark_size = physical(IconSize::Medium.value());
        assert_eq!(
            canvas.pixel(left + mark_size / 2, top + mark_size / 4),
            theme_color(ColorRole::Attention)
        );
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(canvas.pixel(540, 1200), theme_color(ColorRole::Canvas));
    }

    #[test]
    fn lock_idle_device_caption_does_not_paint_a_surface_card() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        draw_lock_idle(
            canvas,
            width,
            height,
            "22:46",
            "Коснитесь, чтобы разблокировать",
            Some("Заряд 87%"),
            None,
            None,
        );
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_eq!(canvas.pixel(540, 1200), theme_color(ColorRole::Canvas));
        assert_ne!(canvas.pixel(540, 1200), theme_color(ColorRole::Surface));
    }

    #[test]
    fn lock_pin_entry_fill_is_canvas_and_keys_are_not_a_surface_header() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let field = Field::new("Введите PIN", FieldKind::Password).with_value("0000");
        let field_rect = Rect::new(49, 24, 982, 212);
        let key = Rect::new(108, 900, 264, 240);
        draw_lock_pin_entry(
            canvas,
            &field,
            field_rect,
            &[(key, "1".to_string())],
            None,
            None,
        );
        assert_eq!(canvas.pixel(540, 210), theme_color(ColorRole::Canvas));
        assert_ne!(canvas.pixel(540, 1200), [0x00, 0xd0, 0x00, 0x00]);
        assert_eq!(canvas.pixel(240, 1020), theme_color(ColorRole::Elevated));
    }

    #[test]
    fn pressed_keyboard_key_uses_pressed_token_without_shifting_neighbors() {
        let width = 1080;
        let height = 2400;
        let field = Field::new("Введите PIN", FieldKind::Password).with_value("0000");
        let field_rect = Rect::new(49, 24, 982, 212);
        let one = Rect::new(108, 900, 264, 240);
        let two = Rect::new(372, 900, 264, 240);
        let keys = vec![(one, "1".to_string()), (two, "2".to_string())];
        let mut idle = vec![0u8; width as usize * height as usize * 4];
        let mut down = vec![0u8; width as usize * height as usize * 4];
        draw_lock_pin_entry(
            &mut Canvas::new(&mut idle, width, height),
            &field,
            field_rect,
            &keys,
            None,
            None,
        );
        draw_lock_pin_entry(
            &mut Canvas::new(&mut down, width, height),
            &field,
            field_rect,
            &keys,
            Some("1"),
            None,
        );
        assert_eq!(
            Canvas::new(&mut down, width, height).pixel(240, 1020),
            theme_color(ColorRole::Pressed)
        );
        assert_eq!(
            Canvas::new(&mut idle, width, height).pixel(240, 1020),
            theme_color(ColorRole::Elevated)
        );
        assert_eq!(
            Canvas::new(&mut down, width, height).pixel(504, 1020),
            theme_color(ColorRole::Elevated)
        );
        assert_eq!(one, Rect::new(108, 900, 264, 240));
        assert_eq!(two, Rect::new(372, 900, 264, 240));
    }

    #[test]
    fn gallery_divider_and_progress_draw_without_a_loaded_font() {
        // Regression coverage for the "still shows something, not a blank
        // screen" contract `draw_gallery`'s own doc comment states for
        // these two primitives -- both must be visible even when `fonts`
        // is `None`, unlike every other row in this screen.
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        draw_gallery(canvas, width, height, None);

        let rows = gallery_row_positions(height);
        let margin = (width / 20).max(12);
        assert_eq!(
            canvas.pixel(margin + 4, rows[3]),
            theme_color(ColorRole::Border)
        );
        // Progress at 62%: left edge is the filled (Accent) portion, far
        // right edge is the unfilled (Grid) track.
        assert_eq!(
            canvas.pixel(margin + 4, rows[5] + 2),
            theme_color(ColorRole::Accent)
        );
        assert_eq!(
            canvas.pixel(width - margin - 4, rows[5] + 2),
            theme_color(ColorRole::Grid)
        );
    }

    #[test]
    fn composite_gallery_rows_never_overlap() {
        let rows = composite_gallery_row_positions(2400);
        let min_gap =
            physical_line_height(TextRole::Body) + physical_line_height(TextRole::Caption);
        for pair in rows.windows(2) {
            let gap = pair[1] - pair[0];
            assert!(
                gap >= min_gap,
                "gap {gap} between composite rows at {} and {} is smaller than {min_gap}",
                pair[0],
                pair[1]
            );
        }
        assert!(*rows.last().unwrap() < 2400);
    }

    #[test]
    fn composite_gallery_decision_buttons_meet_min_touch_and_do_not_overlap() {
        let (accept, decline) = composite_gallery_decision_buttons(1080, 2400);
        assert!(accept.width >= physical(MIN_TOUCH_TARGET));
        assert!(accept.height >= physical(MIN_TOUCH_TARGET));
        assert!(decline.width >= physical(MIN_TOUCH_TARGET));
        assert!(decline.height >= physical(MIN_TOUCH_TARGET));
        assert!(accept.x + accept.width <= decline.x);
        assert!(decline.x + decline.width <= 1080);
        assert!(accept.y + accept.height < 2400);
    }

    #[test]
    fn composite_gallery_state_marks_and_buttons_draw_without_a_loaded_font() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        draw_composite_gallery(canvas, width, height, None);

        let (accept, decline) = composite_gallery_decision_buttons(width, height);
        assert_eq!(
            canvas.pixel(accept.x + 4, accept.y + 4),
            theme_color(ColorRole::Accent)
        );
        assert_eq!(
            canvas.pixel(decline.x + 4, decline.y + 4),
            theme_color(ColorRole::Elevated)
        );

        let rows = composite_gallery_row_positions(height);
        let margin = (width / 20).max(12);
        let content_width = width.saturating_sub(margin * 2);
        assert_ne!(
            canvas.pixel(margin + 4, rows[9] + 4),
            theme_color(ColorRole::Canvas)
        );

        let fixtures = composite_gallery_fixtures();
        let pattern_cell = content_width / fixtures.patterns.len() as u32;
        let mark_size = physical(IconSize::Medium.value());
        let empty_x = margin + 4;
        let loading_x = margin + pattern_cell + mark_size / 2;
        let failed_x = margin + 4 * pattern_cell + mark_size / 2;
        let sample_y = rows[12] + mark_size / 2;
        assert_eq!(
            canvas.pixel(empty_x, sample_y),
            theme_color(ColorRole::Canvas)
        );
        assert_ne!(
            canvas.pixel(loading_x, sample_y),
            theme_color(ColorRole::Canvas)
        );
        assert_ne!(
            canvas.pixel(failed_x, sample_y),
            theme_color(ColorRole::Canvas)
        );
    }

    #[test]
    fn object_view_decision_buttons_use_accent_and_surface_without_a_font() {
        let width = 1080;
        let height = 2400;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let canvas = &mut Canvas::new(&mut pixels, width, height);
        let accept = Rect::new(0, 2200, 540, 200);
        let decline = Rect::new(540, 2200, 540, 200);
        let overlay = DecisionOverlay::new("Подтвердите: убить процесс")
            .with_actor("Система")
            .with_action("process.kill_request");
        draw_object_view(
            canvas,
            &ObjectSummary::new("Подтвердите: убить процесс", "saaios.task · версия 1"),
            None,
            &[],
            Some(&overlay),
            None,
            Rect::new(0, 0, width, 2200),
            &[(accept, "Подтвердить"), (decline, "Отклонить")],
            None,
        );
        assert_eq!(
            canvas.pixel(accept.x + 4, accept.y + 4),
            theme_color(ColorRole::Accent)
        );
        assert_eq!(
            canvas.pixel(decline.x + 4, decline.y + 4),
            theme_color(ColorRole::Surface)
        );
    }

    #[test]
    fn failed_action_card_paints_a_mark_and_is_not_a_pair_button() {
        let mut pixels = vec![0u8; 1080 * 2400 * 4];
        let canvas = &mut Canvas::new(&mut pixels, 1080, 2400);
        let rect = Rect::new(40, 430, 1000, 200);
        let card = ActionCardView::new("Ошибка сопряжения: timeout", "", "").with_indicator(
            StatusIndicator::new(UniversalState::Failed, "Ошибка сопряжения: timeout"),
        );
        draw_action_card(canvas, rect, &card, None);
        assert_ne!(
            canvas.pixel(rect.x + 40, rect.y + 80),
            theme_color(ColorRole::Canvas)
        );
        assert_ne!(
            canvas.pixel(rect.x + 40, rect.y + 80),
            theme_color(ColorRole::Accent)
        );
    }

    #[test]
    fn draw_root_reuses_draw_action_card() {
        let src = include_str!("render.rs");
        let draw_root = src
            .split("pub fn draw_root(")
            .nth(1)
            .expect("draw_root")
            .split("fn paint_context_header(")
            .next()
            .expect("paint_context_header");
        assert!(draw_root.contains("draw_action_card(canvas, *rect, card, fonts)"));
        assert!(!draw_root.contains("38.0"));
        assert!(!draw_root.contains("27.0"));
    }
}
