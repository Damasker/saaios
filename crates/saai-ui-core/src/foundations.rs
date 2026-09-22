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

pub const FRAME_PACE_CAP: usize = 32;
/// VUI-08 drag gate (ADR-173). Compared only to scroll samples.
pub const FRAME_PACE_P95_LIMIT_MS: u32 = 50;
/// VUI-08 first-visible gate (ADR-176). Compared to non-scroll
/// `input_to_commit_ms` remembered on the ring.
pub const FIRST_FEEDBACK_LIMIT_MS: u32 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameReason {
    Input,
    Motion,
    Scroll,
}

impl FrameReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Motion => "motion",
            Self::Scroll => "scroll",
        }
    }
}

/// Visible chrome that produced a main-surface commit (ADR-175).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameSurface {
    Now,
    Inbox,
    Spaces,
    Search,
    Me,
    List,
    Keyboard,
    Overlay,
    Orb,
    Lock,
}

impl FrameSurface {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Now => "now",
            Self::Inbox => "inbox",
            Self::Spaces => "spaces",
            Self::Search => "search",
            Self::Me => "me",
            Self::List => "list",
            Self::Keyboard => "keyboard",
            Self::Overlay => "overlay",
            Self::Orb => "orb",
            Self::Lock => "lock",
        }
    }
}

/// Staging path that attached the main-surface buffer (ADR-178).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameBackend {
    Dmabuf,
    Shm,
}

impl FrameBackend {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dmabuf => "dmabuf",
            Self::Shm => "shm",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSample {
    pub produce_ms: u32,
    pub input_to_commit_ms: Option<u32>,
    pub requested_frame: bool,
    pub pending_depth: u8,
    pub dropped: u32,
    pub coalesced: u32,
    pub reason: FrameReason,
    pub surface: FrameSurface,
    pub backend: FrameBackend,
}

/// Ring of recent main-surface commits. Elapsed times are injected.
/// Presentation timestamps are out of scope (compositor time is not
/// trusted).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FramePace {
    samples: [Option<FrameSample>; FRAME_PACE_CAP],
    next: usize,
    count: usize,
    dropped: u32,
    coalesced: u32,
    last_feedback_ms: Option<u32>,
    seq: u32,
}

impl Default for FramePace {
    fn default() -> Self {
        Self::new()
    }
}

impl FramePace {
    pub fn new() -> Self {
        Self {
            samples: [None; FRAME_PACE_CAP],
            next: 0,
            count: 0,
            dropped: 0,
            coalesced: 0,
            last_feedback_ms: None,
            seq: 0,
        }
    }

    pub fn note_dropped(&mut self) {
        self.dropped = self.dropped.saturating_add(1);
    }

    pub fn note_coalesced(&mut self) {
        self.coalesced = self.coalesced.saturating_add(1);
    }

    pub fn record(&mut self, mut sample: FrameSample) {
        sample.dropped = self.dropped;
        sample.coalesced = self.coalesced;
        if sample.reason != FrameReason::Scroll {
            if let Some(ms) = sample.input_to_commit_ms {
                self.last_feedback_ms = Some(ms);
            }
        }
        self.seq = self.seq.saturating_add(1);
        self.samples[self.next] = Some(sample);
        self.next = (self.next + 1) % FRAME_PACE_CAP;
        if self.count < FRAME_PACE_CAP {
            self.count += 1;
        }
    }

    pub fn last(&self) -> Option<FrameSample> {
        if self.count == 0 {
            return None;
        }
        let index = (self.next + FRAME_PACE_CAP - 1) % FRAME_PACE_CAP;
        self.samples[index]
    }

    /// Oldest to newest. Wrap-around starts at `next` once the ring is full.
    pub fn chronological(&self) -> Vec<FrameSample> {
        let start = if self.count < FRAME_PACE_CAP {
            0
        } else {
            self.next
        };
        (0..self.count)
            .filter_map(|i| self.samples[(start + i) % FRAME_PACE_CAP])
            .collect()
    }

    /// ADR-175: one line per sample, oldest first. Empty ring is empty.
    pub fn trace(&self) -> String {
        let mut out = String::new();
        for sample in self.chronological() {
            let input = sample
                .input_to_commit_ms
                .map(|ms| ms.to_string())
                .unwrap_or_else(|| "-".into());
            out.push_str(&format!(
                "surface={} reason={} backend={} produce_ms={} input_ms={} frame={} pending={} dropped={} coalesced={}\n",
                sample.surface.as_str(),
                sample.reason.as_str(),
                sample.backend.as_str(),
                sample.produce_ms,
                input,
                u8::from(sample.requested_frame),
                sample.pending_depth,
                sample.dropped,
                sample.coalesced,
            ));
        }
        out
    }

    pub fn p95_produce_ms(&self) -> Option<u32> {
        self.p95_produce_ms_matching(|_| true)
    }

    pub fn p95_produce_ms_for(&self, reason: FrameReason) -> Option<u32> {
        self.p95_produce_ms_matching(|sample| sample.reason == reason)
    }

    /// `None` when the ring has no scroll commit yet.
    pub fn scroll_p95_within_limit(&self) -> Option<bool> {
        self.p95_produce_ms_for(FrameReason::Scroll)
            .map(|ms| ms <= FRAME_PACE_P95_LIMIT_MS)
    }

    /// ADR-176: last non-scroll `input_to_commit_ms`. Scroll coalescing
    /// does not count as first visible Pressed.
    pub fn first_feedback_within_limit(&self) -> Option<bool> {
        self.last_feedback_ms
            .map(|ms| ms <= FIRST_FEEDBACK_LIMIT_MS)
    }

    /// ADR-177: total main-surface commits, including those that wrapped
    /// out of the ring.
    pub fn seq(&self) -> u32 {
        self.seq
    }

    /// `None` with no samples. `Some(true)` when the last commit did
    /// not request `wl_surface.frame`.
    pub fn idle_ok(&self) -> Option<bool> {
        self.last().map(|sample| !sample.requested_frame)
    }

    fn p95_produce_ms_matching(&self, keep: impl Fn(&FrameSample) -> bool) -> Option<u32> {
        let mut values = [0u32; FRAME_PACE_CAP];
        let mut n = 0usize;
        for sample in self.samples.iter().flatten() {
            if keep(sample) {
                values[n] = sample.produce_ms;
                n += 1;
            }
        }
        if n == 0 {
            return None;
        }
        values[..n].sort_unstable();
        Some(values[(n - 1) * 95 / 100])
    }

    pub fn line(&self) -> Option<String> {
        let sample = self.last()?;
        let input = sample
            .input_to_commit_ms
            .map(|ms| ms.to_string())
            .unwrap_or_else(|| "-".into());
        let p95_scroll = self
            .p95_produce_ms_for(FrameReason::Scroll)
            .map(|ms| ms.to_string())
            .unwrap_or_else(|| "-".into());
        let p95_ok = match self.scroll_p95_within_limit() {
            Some(true) => "1",
            Some(false) => "0",
            None => "-",
        };
        let input_ok = match self.first_feedback_within_limit() {
            Some(true) => "1",
            Some(false) => "0",
            None => "-",
        };
        let idle_ok = match self.idle_ok() {
            Some(true) => "1",
            Some(false) => "0",
            None => "-",
        };
        Some(format!(
            "produce_ms={} input_ms={} frame={} pending={} dropped={} coalesced={} reason={} surface={} backend={} p95_scroll={} p95_ok={} input_ok={} seq={} idle_ok={}",
            sample.produce_ms,
            input,
            u8::from(sample.requested_frame),
            sample.pending_depth,
            sample.dropped,
            sample.coalesced,
            sample.reason.as_str(),
            sample.surface.as_str(),
            sample.backend.as_str(),
            p95_scroll,
            p95_ok,
            input_ok,
            self.seq,
            idle_ok,
        ))
    }
}

pub fn frame_reason(scroll: bool, motion: bool) -> FrameReason {
    if scroll {
        FrameReason::Scroll
    } else if motion {
        FrameReason::Motion
    } else {
        FrameReason::Input
    }
}

/// ADR-175: lock, overlay, keyboard, list, then Orb activity, then the tab.
pub fn frame_surface(
    locked: bool,
    overlay: bool,
    keyboard: bool,
    list: bool,
    orb: bool,
    tab: FrameSurface,
) -> FrameSurface {
    if locked {
        FrameSurface::Lock
    } else if overlay {
        FrameSurface::Overlay
    } else if keyboard {
        FrameSurface::Keyboard
    } else if list {
        FrameSurface::List
    } else if orb {
        FrameSurface::Orb
    } else {
        tab
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

    fn sample(produce_ms: u32, reason: FrameReason, surface: FrameSurface) -> FrameSample {
        FrameSample {
            produce_ms,
            input_to_commit_ms: None,
            requested_frame: false,
            pending_depth: 0,
            dropped: 0,
            coalesced: 0,
            reason,
            surface,
            backend: FrameBackend::Dmabuf,
        }
    }

    #[test]
    fn frame_pace_p95_and_last_line_use_injected_samples() {
        let mut pace = FramePace::new();
        assert!(pace.last().is_none());
        assert!(pace.p95_produce_ms().is_none());
        for _ in 0..18 {
            let mut input = sample(10, FrameReason::Input, FrameSurface::Now);
            input.input_to_commit_ms = Some(20);
            input.pending_depth = 1;
            pace.record(input);
        }
        let mut high = sample(50, FrameReason::Input, FrameSurface::Now);
        high.input_to_commit_ms = Some(20);
        high.requested_frame = true;
        high.pending_depth = 1;
        pace.record(high);
        pace.record(high);
        assert_eq!(pace.last().map(|sample| sample.produce_ms), Some(50));
        assert_eq!(pace.p95_produce_ms(), Some(50));
        pace.note_dropped();
        pace.note_coalesced();
        pace.note_coalesced();
        pace.record(sample(8, FrameReason::Motion, FrameSurface::Orb));
        let line = pace.line().expect("recorded");
        assert!(line.contains("produce_ms=8"));
        assert!(line.contains("input_ms=-"));
        assert!(line.contains("frame=0"));
        assert!(line.contains("dropped=1"));
        assert!(line.contains("coalesced=2"));
        assert!(line.contains("reason=motion"));
        assert!(line.contains("surface=orb"));
        assert!(line.contains("p95_scroll=-"));
        assert!(line.contains("p95_ok=-"));
        assert!(line.contains("input_ok=1"));
        assert!(line.contains("seq="));
        assert!(line.contains("idle_ok=1"));
        assert_eq!(frame_reason(true, true), FrameReason::Scroll);
        assert_eq!(frame_reason(false, true), FrameReason::Motion);
        assert_eq!(frame_reason(false, false), FrameReason::Input);
    }

    fn record_scroll(pace: &mut FramePace, produce_ms: u32) {
        let mut scroll = sample(produce_ms, FrameReason::Scroll, FrameSurface::Me);
        scroll.pending_depth = 1;
        pace.record(scroll);
    }

    #[test]
    fn scroll_p95_gate_ignores_input_frames_and_uses_the_50ms_limit() {
        let mut pace = FramePace::new();
        assert_eq!(pace.scroll_p95_within_limit(), None);
        for _ in 0..18 {
            let mut input = sample(80, FrameReason::Input, FrameSurface::Inbox);
            input.input_to_commit_ms = Some(20);
            pace.record(input);
            record_scroll(&mut pace, 10);
        }
        record_scroll(&mut pace, 40);
        record_scroll(&mut pace, 40);
        assert_eq!(pace.p95_produce_ms_for(FrameReason::Scroll), Some(40));
        assert_eq!(pace.scroll_p95_within_limit(), Some(true));
        assert!(pace.line().expect("recorded").contains("p95_ok=1"));
        assert!(pace.line().expect("recorded").contains("surface=me"));

        let mut slow = FramePace::new();
        for _ in 0..18 {
            record_scroll(&mut slow, 10);
        }
        record_scroll(&mut slow, 80);
        record_scroll(&mut slow, 80);
        assert_eq!(slow.p95_produce_ms_for(FrameReason::Scroll), Some(80));
        assert_eq!(slow.scroll_p95_within_limit(), Some(false));
        assert!(slow.line().expect("recorded").contains("p95_ok=0"));
        assert_eq!(FRAME_PACE_P95_LIMIT_MS, 50);
    }

    #[test]
    fn frame_surface_prefers_lock_then_overlay_keyboard_list_orb_then_tab() {
        assert_eq!(
            frame_surface(true, true, true, true, true, FrameSurface::Me),
            FrameSurface::Lock
        );
        assert_eq!(
            frame_surface(false, true, true, true, true, FrameSurface::Me),
            FrameSurface::Overlay
        );
        assert_eq!(
            frame_surface(false, false, true, true, true, FrameSurface::Me),
            FrameSurface::Keyboard
        );
        assert_eq!(
            frame_surface(false, false, false, true, true, FrameSurface::Me),
            FrameSurface::List
        );
        assert_eq!(
            frame_surface(false, false, false, false, true, FrameSurface::Now),
            FrameSurface::Orb
        );
        assert_eq!(
            frame_surface(false, false, false, false, false, FrameSurface::Inbox),
            FrameSurface::Inbox
        );
    }

    #[test]
    fn frame_pace_trace_is_chronological_and_names_each_surface() {
        let mut pace = FramePace::new();
        assert!(pace.trace().is_empty());
        pace.record(sample(12, FrameReason::Scroll, FrameSurface::Me));
        pace.record(sample(6, FrameReason::Input, FrameSurface::Inbox));
        pace.record(sample(7, FrameReason::Input, FrameSurface::List));
        pace.record(sample(9, FrameReason::Motion, FrameSurface::Keyboard));
        pace.record(sample(5, FrameReason::Motion, FrameSurface::Overlay));
        pace.record(sample(4, FrameReason::Motion, FrameSurface::Orb));
        let trace = pace.trace();
        let me = trace.find("surface=me ").expect("me");
        let inbox = trace.find("surface=inbox ").expect("inbox");
        let list = trace.find("surface=list ").expect("list");
        let keyboard = trace.find("surface=keyboard ").expect("keyboard");
        let overlay = trace.find("surface=overlay ").expect("overlay");
        let orb = trace.find("surface=orb ").expect("orb");
        assert!(
            me < inbox && inbox < list && list < keyboard && keyboard < overlay && overlay < orb
        );
        assert!(trace.contains("reason=scroll backend=dmabuf produce_ms=12"));
        assert!(trace.contains("reason=input backend=dmabuf produce_ms=6"));
    }

    #[test]
    fn first_feedback_gate_ignores_scroll_and_uses_the_50ms_limit() {
        let mut pace = FramePace::new();
        assert_eq!(pace.first_feedback_within_limit(), None);
        let mut drag = sample(14, FrameReason::Scroll, FrameSurface::Me);
        drag.input_to_commit_ms = Some(100);
        pace.record(drag);
        assert_eq!(pace.first_feedback_within_limit(), None);
        assert!(pace.line().expect("recorded").contains("input_ok=-"));

        let mut tap = sample(8, FrameReason::Motion, FrameSurface::Inbox);
        tap.input_to_commit_ms = Some(20);
        pace.record(tap);
        assert_eq!(pace.first_feedback_within_limit(), Some(true));
        pace.record(sample(6, FrameReason::Motion, FrameSurface::Inbox));
        assert_eq!(pace.first_feedback_within_limit(), Some(true));
        assert!(pace.line().expect("recorded").contains("input_ok=1"));

        let mut slow = FramePace::new();
        let mut late = sample(8, FrameReason::Input, FrameSurface::Now);
        late.input_to_commit_ms = Some(80);
        slow.record(late);
        assert_eq!(slow.first_feedback_within_limit(), Some(false));
        assert!(slow.line().expect("recorded").contains("input_ok=0"));
        assert_eq!(FIRST_FEEDBACK_LIMIT_MS, 50);
    }

    #[test]
    fn idle_ok_follows_requested_frame_and_seq_counts_wraps() {
        let mut pace = FramePace::new();
        assert_eq!(pace.seq(), 0);
        assert_eq!(pace.idle_ok(), None);
        let mut clock = sample(8, FrameReason::Motion, FrameSurface::Now);
        clock.requested_frame = true;
        pace.record(clock);
        assert_eq!(pace.seq(), 1);
        assert_eq!(pace.idle_ok(), Some(false));
        assert!(pace.line().expect("recorded").contains("idle_ok=0"));
        pace.record(sample(5, FrameReason::Input, FrameSurface::Now));
        assert_eq!(pace.seq(), 2);
        assert_eq!(pace.idle_ok(), Some(true));
        let line = pace.line().expect("recorded");
        assert!(line.contains("seq=2"));
        assert!(line.contains("idle_ok=1"));
    }

    #[test]
    fn frame_backend_names_dmabuf_and_shm_on_the_line() {
        let mut pace = FramePace::new();
        pace.record(sample(6, FrameReason::Input, FrameSurface::Now));
        assert!(pace.line().expect("recorded").contains("backend=dmabuf"));
        let mut shm = sample(9, FrameReason::Input, FrameSurface::Now);
        shm.backend = FrameBackend::Shm;
        pace.record(shm);
        let line = pace.line().expect("recorded");
        assert!(line.contains("backend=shm"));
        let trace = pace.trace();
        assert!(trace.contains("backend=dmabuf"));
        assert!(trace.contains("backend=shm"));
        assert_eq!(FrameBackend::Dmabuf.as_str(), "dmabuf");
        assert_eq!(FrameBackend::Shm.as_str(), "shm");
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
