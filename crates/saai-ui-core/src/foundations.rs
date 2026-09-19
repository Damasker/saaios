use crate::ColorRole;

/// A density-independent unit used by component contracts and layout.
/// Backends convert it to physical pixels only at the rendering boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalUnit(u16);

impl LogicalUnit {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Rational surface scale. Integer arithmetic keeps layout deterministic
/// across host tests and the device renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceScale {
    physical: u16,
    logical: u16,
}

impl SurfaceScale {
    pub const PIXEL_7: Self = Self {
        physical: 3,
        logical: 1,
    };

    pub const fn new(physical: u16, logical: u16) -> Option<Self> {
        if physical == 0 || logical == 0 {
            None
        } else {
            Some(Self { physical, logical })
        }
    }

    pub const fn logical_to_physical(self, value: LogicalUnit) -> u32 {
        let numerator = value.get() as u32 * self.physical as u32;
        (numerator + self.logical as u32 / 2) / self.logical as u32
    }

    pub const fn physical_to_logical(self, value: u32) -> u32 {
        let numerator = value as u64 * self.logical as u64;
        let rounded = (numerator + self.physical as u64 / 2) / self.physical as u64;
        if rounded > u32::MAX as u64 {
            u32::MAX
        } else {
            rounded as u32
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontFamily {
    Sans,
    Mono,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeight {
    Regular,
    Semibold,
}

/// Stable product roles. Font filenames and rasterizer handles belong to the
/// backend and are intentionally absent here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    Display,
    Title,
    Section,
    Body,
    Label,
    Caption,
    MonoBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextStyle {
    pub family: FontFamily,
    pub weight: FontWeight,
    pub size: LogicalUnit,
    pub line_height: LogicalUnit,
}

impl TextRole {
    pub const fn style(self) -> TextStyle {
        match self {
            Self::Display => TextStyle::new(FontFamily::Sans, FontWeight::Semibold, 32, 40),
            Self::Title => TextStyle::new(FontFamily::Sans, FontWeight::Semibold, 24, 32),
            Self::Section => TextStyle::new(FontFamily::Sans, FontWeight::Semibold, 18, 24),
            Self::Body => TextStyle::new(FontFamily::Sans, FontWeight::Regular, 16, 24),
            Self::Label => TextStyle::new(FontFamily::Sans, FontWeight::Semibold, 14, 20),
            Self::Caption => TextStyle::new(FontFamily::Sans, FontWeight::Regular, 12, 16),
            Self::MonoBody => TextStyle::new(FontFamily::Mono, FontWeight::Regular, 14, 20),
        }
    }
}

impl TextStyle {
    const fn new(family: FontFamily, weight: FontWeight, size: u16, line_height: u16) -> Self {
        Self {
            family,
            weight,
            size: LogicalUnit::new(size),
            line_height: LogicalUnit::new(line_height),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpacingToken {
    None,
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
    XXLarge,
}

impl SpacingToken {
    pub const fn value(self) -> LogicalUnit {
        LogicalUnit::new(match self {
            Self::None => 0,
            Self::XSmall => 4,
            Self::Small => 8,
            Self::Medium => 12,
            Self::Large => 16,
            Self::XLarge => 24,
            Self::XXLarge => 32,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadiusToken {
    None,
    Small,
    Medium,
    Large,
}

impl RadiusToken {
    pub const fn value(self) -> LogicalUnit {
        LogicalUnit::new(match self {
            Self::None => 0,
            Self::Small => 4,
            Self::Medium => 8,
            Self::Large => 12,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeToken {
    Hairline,
    Focus,
}

impl StrokeToken {
    pub const fn value(self) -> LogicalUnit {
        LogicalUnit::new(match self {
            Self::Hairline => 1,
            Self::Focus => 2,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSize {
    Small,
    Medium,
    Large,
}

impl IconSize {
    pub const fn value(self) -> LogicalUnit {
        LogicalUnit::new(match self {
            Self::Small => 16,
            Self::Medium => 20,
            Self::Large => 24,
        })
    }
}

/// VUI-02's line-icon family (ADR-100): a Feather Icons webfont build,
/// loaded through the same `fontdue` path as the sans/mono text faces --
/// an icon is a glyph at a Private Use Area codepoint, drawn with the
/// renderer's existing text primitives. Product code names a semantic
/// icon, never a codepoint or the font file, the same boundary
/// `TextRole` already draws for typography.
///
/// Deliberately a small, curated set matched to a real near-term need
/// (the PIN keypad's backspace key labels itself with the Unicode erase
/// mark U+232B, which the sans face has no glyph for -- `fontdue` was
/// rendering its missing-glyph placeholder box there) rather than
/// importing the source webfont's full ~280-icon set upfront with no
/// call site yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconGlyph {
    Backspace,
    Wifi,
    WifiOff,
    Bluetooth,
    Battery,
    BatteryCharging,
    Lock,
    Check,
    X,
    ChevronRight,
    ChevronLeft,
    ChevronDown,
    AlertTriangle,
    Settings,
}

impl IconGlyph {
    /// The codepoint in `FeatherIcons.ttf`'s Private Use Area, taken
    /// directly from the webfont's own generated `content:` mapping (see
    /// `os/targets/panther/third_party/README.md`) -- not a SaaiOS
    /// choice, just where this build placed that glyph.
    pub const fn codepoint(self) -> char {
        match self {
            Self::AlertTriangle => '\u{f105}',
            Self::BatteryCharging => '\u{f11d}',
            Self::Battery => '\u{f11e}',
            Self::Bluetooth => '\u{f121}',
            Self::Check => '\u{f12e}',
            Self::ChevronDown => '\u{f12f}',
            Self::ChevronLeft => '\u{f130}',
            Self::ChevronRight => '\u{f131}',
            Self::Backspace => '\u{f156}',
            Self::Lock => '\u{f190}',
            Self::Settings => '\u{f1d0}',
            Self::WifiOff => '\u{f20d}',
            Self::Wifi => '\u{f20e}',
            Self::X => '\u{f213}',
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SafeInsets {
    pub top: LogicalUnit,
    pub right: LogicalUnit,
    pub bottom: LogicalUnit,
    pub left: LogicalUnit,
}

impl SafeInsets {
    pub const fn all(value: LogicalUnit) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub const fn symmetric(horizontal: LogicalUnit, vertical: LogicalUnit) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Pixel 7 portrait surface insets on the 360×800 logical canvas.
    /// Top is the status overlay (camera punch-hole plus clock row).
    /// Bottom is the navigation strip. Left/right stay 0 until a real
    /// side cutout is observed — components must not invent one.
    pub const PIXEL_7_PORTRAIT: Self = Self {
        top: CONTROL_VISUAL_HEIGHT,
        right: LogicalUnit::ZERO,
        bottom: LogicalUnit::new(100),
        left: LogicalUnit::ZERO,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceLevel {
    Canvas,
    Surface,
    Raised,
    Overlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceStyle {
    pub fill: ColorRole,
    pub border: Option<ColorRole>,
}

impl SurfaceLevel {
    pub const fn style(self) -> SurfaceStyle {
        match self {
            Self::Canvas => SurfaceStyle {
                fill: ColorRole::Canvas,
                border: None,
            },
            Self::Surface => SurfaceStyle {
                fill: ColorRole::Surface,
                border: Some(ColorRole::Grid),
            },
            Self::Raised | Self::Overlay => SurfaceStyle {
                fill: ColorRole::Elevated,
                border: Some(ColorRole::Border),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionToken {
    MicroFeedback,
    Selection,
    Context,
}

impl MotionToken {
    pub const fn milliseconds(self, reduced_motion: bool) -> u16 {
        if reduced_motion {
            return 0;
        }
        match self {
            Self::MicroFeedback => 120,
            Self::Selection => 180,
            Self::Context => 240,
        }
    }
}

/// One in-flight motion. Elapsed time is injected so host tests do not
/// depend on a wall clock. The shell copies `Instant` deltas into
/// `advance`. Reduced motion never asks for another frame.
/// ADR-170: `looping` wraps at two token windows so Orb activity can
/// pulse without a new duration number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionClock {
    token: MotionToken,
    reduced_motion: bool,
    elapsed_ms: u32,
    looping: bool,
}

impl MotionClock {
    pub fn one_shot(token: MotionToken, reduced_motion: bool) -> Self {
        Self {
            token,
            reduced_motion,
            elapsed_ms: 0,
            looping: false,
        }
    }

    pub fn looping(token: MotionToken, reduced_motion: bool) -> Self {
        Self {
            token,
            reduced_motion,
            elapsed_ms: 0,
            looping: true,
        }
    }

    pub fn duration_ms(self) -> u16 {
        self.token.milliseconds(self.reduced_motion)
    }

    pub fn token(self) -> MotionToken {
        self.token
    }

    pub fn is_looping(self) -> bool {
        self.looping
    }

    fn cycle_ms(self) -> u32 {
        u32::from(self.duration_ms()).saturating_mul(2)
    }

    pub fn advance(&mut self, dt_ms: u32) {
        if self.reduced_motion {
            return;
        }
        if self.looping {
            let cycle = self.cycle_ms();
            if cycle == 0 {
                return;
            }
            self.elapsed_ms = self.elapsed_ms.saturating_add(dt_ms) % cycle;
            return;
        }
        let cap = u32::from(self.duration_ms());
        self.elapsed_ms = self.elapsed_ms.saturating_add(dt_ms).min(cap);
    }

    pub fn progress_percent(self) -> u8 {
        let duration = u32::from(self.duration_ms());
        if duration == 0 {
            return 100;
        }
        if self.looping {
            return ((self.elapsed_ms.saturating_mul(100)) / duration).min(100) as u8;
        }
        ((self.elapsed_ms.saturating_mul(100)) / duration).min(100) as u8
    }

    pub fn needs_frame(self) -> bool {
        if self.reduced_motion {
            return false;
        }
        if self.looping {
            return self.cycle_ms() > 0;
        }
        self.elapsed_ms < u32::from(self.duration_ms())
    }

    /// ADR-170: inset on for the first token window, off for the second.
    pub fn pulse_visible(self) -> bool {
        if self.reduced_motion || !self.looping {
            return false;
        }
        self.elapsed_ms < u32::from(self.duration_ms())
    }
}

pub const MIN_TOUCH_TARGET: LogicalUnit = LogicalUnit::new(48);
pub const CONTROL_VISUAL_HEIGHT: LogicalUnit = LogicalUnit::new(40);
pub const TWO_LINE_ROW_HEIGHT: LogicalUnit = LogicalUnit::new(64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_7_reference_scale_is_three_physical_pixels_per_unit() {
        assert_eq!(
            SurfaceScale::PIXEL_7.logical_to_physical(LogicalUnit::new(360)),
            1080
        );
        assert_eq!(SurfaceScale::PIXEL_7.physical_to_logical(2400), 800);
        assert_eq!(SurfaceScale::new(0, 1), None);
        assert_eq!(SurfaceScale::new(1, 0), None);
        assert_eq!(
            SurfaceScale::new(1, u16::MAX)
                .unwrap()
                .physical_to_logical(u32::MAX),
            u32::MAX
        );
    }

    #[test]
    fn type_roles_are_semantic_and_have_valid_line_heights() {
        let roles = [
            TextRole::Display,
            TextRole::Title,
            TextRole::Section,
            TextRole::Body,
            TextRole::Label,
            TextRole::Caption,
            TextRole::MonoBody,
        ];
        for role in roles {
            let style = role.style();
            assert!(style.line_height >= style.size);
        }
        assert_eq!(TextRole::MonoBody.style().family, FontFamily::Mono);
        assert_eq!(TextRole::Body.style().family, FontFamily::Sans);
    }

    #[test]
    fn spacing_uses_the_four_unit_rhythm() {
        let tokens = [
            SpacingToken::XSmall,
            SpacingToken::Small,
            SpacingToken::Medium,
            SpacingToken::Large,
            SpacingToken::XLarge,
            SpacingToken::XXLarge,
        ];
        let mut previous = 0;
        for token in tokens {
            let value = token.value().get();
            assert_eq!(value % 4, 0);
            assert!(value > previous);
            previous = value;
        }
    }

    #[test]
    fn primary_touch_target_is_larger_than_the_visual_control() {
        assert_eq!(MIN_TOUCH_TARGET, LogicalUnit::new(48));
        assert!(MIN_TOUCH_TARGET > CONTROL_VISUAL_HEIGHT);
    }

    #[test]
    fn pixel_7_portrait_insets_match_the_reference_canvas() {
        let insets = SafeInsets::PIXEL_7_PORTRAIT;
        let scale = SurfaceScale::PIXEL_7;
        assert_eq!(scale.logical_to_physical(insets.top), 120);
        assert_eq!(scale.logical_to_physical(insets.bottom), 300);
        assert_eq!(scale.logical_to_physical(insets.left), 0);
        assert_eq!(scale.logical_to_physical(insets.right), 0);
        assert!(insets.bottom >= MIN_TOUCH_TARGET);
        assert_eq!(insets.top, CONTROL_VISUAL_HEIGHT);
    }

    #[test]
    fn surface_levels_only_reference_semantic_color_roles() {
        assert_eq!(SurfaceLevel::Canvas.style().fill, ColorRole::Canvas);
        assert_eq!(SurfaceLevel::Surface.style().border, Some(ColorRole::Grid));
        assert_eq!(SurfaceLevel::Raised.style().fill, ColorRole::Elevated);
        assert_eq!(
            SurfaceLevel::Overlay.style().border,
            Some(ColorRole::Border)
        );
    }

    #[test]
    fn reduced_motion_removes_transition_duration() {
        assert_eq!(MotionToken::MicroFeedback.milliseconds(false), 120);
        assert_eq!(MotionToken::Selection.milliseconds(false), 180);
        assert_eq!(MotionToken::Context.milliseconds(false), 240);
        assert_eq!(MotionToken::Context.milliseconds(true), 0);
    }

    #[test]
    fn motion_clock_one_shot_needs_a_frame_until_the_token_duration() {
        let mut clock = MotionClock::one_shot(MotionToken::MicroFeedback, false);
        assert!(clock.needs_frame());
        assert_eq!(clock.progress_percent(), 0);
        clock.advance(60);
        assert!(clock.needs_frame());
        assert_eq!(clock.progress_percent(), 50);
        clock.advance(60);
        assert!(!clock.needs_frame());
        assert_eq!(clock.progress_percent(), 100);
        clock.advance(40);
        assert_eq!(clock.progress_percent(), 100);
        assert_eq!(clock.token(), MotionToken::MicroFeedback);
    }

    #[test]
    fn motion_clock_reduced_motion_never_needs_a_frame() {
        let mut clock = MotionClock::one_shot(MotionToken::MicroFeedback, true);
        assert!(!clock.needs_frame());
        assert_eq!(clock.progress_percent(), 100);
        clock.advance(120);
        assert!(!clock.needs_frame());
    }

    #[test]
    fn motion_clock_looping_needs_a_frame_and_pulse_toggles_once_per_token() {
        let mut clock = MotionClock::looping(MotionToken::Context, false);
        assert!(clock.is_looping());
        assert!(clock.needs_frame());
        assert!(clock.pulse_visible());
        clock.advance(239);
        assert!(clock.pulse_visible());
        clock.advance(1);
        assert!(clock.needs_frame());
        assert!(!clock.pulse_visible());
        clock.advance(240);
        assert!(clock.needs_frame());
        assert!(clock.pulse_visible());
        let mut reduced = MotionClock::looping(MotionToken::Context, true);
        assert!(!reduced.needs_frame());
        assert!(!reduced.pulse_visible());
        reduced.advance(240);
        assert!(!reduced.needs_frame());
        assert!(!MotionClock::one_shot(MotionToken::Context, false).pulse_visible());
    }

    #[test]
    fn every_icon_glyph_is_a_distinct_private_use_area_codepoint() {
        let glyphs = [
            IconGlyph::Backspace,
            IconGlyph::Wifi,
            IconGlyph::WifiOff,
            IconGlyph::Bluetooth,
            IconGlyph::Battery,
            IconGlyph::BatteryCharging,
            IconGlyph::Lock,
            IconGlyph::Check,
            IconGlyph::X,
            IconGlyph::ChevronRight,
            IconGlyph::ChevronLeft,
            IconGlyph::ChevronDown,
            IconGlyph::AlertTriangle,
            IconGlyph::Settings,
        ];
        let mut seen = std::collections::BTreeSet::new();
        for glyph in glyphs {
            let codepoint = glyph.codepoint();
            assert!(
                ('\u{e000}'..='\u{f8ff}').contains(&codepoint),
                "{codepoint:?} is outside the Private Use Area"
            );
            assert!(seen.insert(codepoint), "{codepoint:?} used by two icons");
        }
    }
}
