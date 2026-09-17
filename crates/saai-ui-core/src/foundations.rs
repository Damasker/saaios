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
}
