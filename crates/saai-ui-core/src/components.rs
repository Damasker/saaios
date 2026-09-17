//! First primitive layer from `docs/os/ui/component-library-v1.md` section 6
//! (`SemanticText`, `Icon`, `Divider`, `StatusIndicator`) -- Experimental
//! stability level per that document's section 2: the contract exists and is
//! tested, but promotion to Stable needs the gallery, golden renders, and
//! accessibility/migration tests section 9 requires, none of which exist yet.
//!
//! These are pure content/semantic descriptors, not layout or paint code --
//! matching section 8's ownership split ("`saai-ui-core`: ... component
//! contracts/state ... render backend: font loading, glyph rasterization,
//! icon tessellation, pixels"). None of these types reference `Rect` or a
//! physical pixel: a renderer measures and draws them, this crate only says
//! what they mean.

use crate::{ColorRole, IconGlyph, IconSize, LogicalUnit, StatusMark, TextRole, UniversalState};

/// Section 6.1: "wrap by default for prose; ellipsis only when a full-value
/// route exists" -- a call-site decision this type records, not enforces
/// (there is no way to know from here whether a full-value route exists).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextOverflow {
    #[default]
    Wrap,
    Ellipsis,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticText {
    pub content: String,
    pub role: TextRole,
    pub color: ColorRole,
    pub max_lines: Option<u32>,
    pub overflow: TextOverflow,
}

impl SemanticText {
    pub fn new(content: impl Into<String>, role: TextRole, color: ColorRole) -> Self {
        Self {
            content: content.into(),
            role,
            color,
            max_lines: None,
            overflow: TextOverflow::Wrap,
        }
    }

    pub fn with_max_lines(mut self, max_lines: u32) -> Self {
        self.max_lines = Some(max_lines);
        self
    }

    pub fn with_overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Section 6.1: "Accessibility: preserves the full untruncated string" --
    /// `max_lines`/`overflow` are a visual truncation only.
    pub fn accessible_value(&self) -> &str {
        &self.content
    }
}

/// Section 6.2. `size` defaults to 24 logical units ("24 is the normal
/// control size").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon {
    pub glyph: IconGlyph,
    pub size: IconSize,
    pub color: ColorRole,
}

impl Icon {
    pub const fn new(glyph: IconGlyph, color: ColorRole) -> Self {
        Self {
            glyph,
            size: IconSize::Large,
            color,
        }
    }

    pub const fn with_size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }
}

/// Section 6.3: "Uses `Separator` geometry and the border token" --
/// `crate::Node::separator` already supplies the geometry (a
/// `StrokeToken::Hairline`-thick leaf); this supplies which semantic color
/// role paints it and an optional leading inset so it can align with a
/// neighboring row's text edge instead of running the full container width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Divider {
    pub color: ColorRole,
    pub inset_start: Option<LogicalUnit>,
}

impl Divider {
    /// `color.border.default` in the spec's naming.
    pub const fn new() -> Self {
        Self {
            color: ColorRole::Border,
            inset_start: None,
        }
    }

    /// `color.grid.subtle` in the spec's naming -- a quieter boundary than
    /// `new()`, for a division that matters less than a section edge.
    pub const fn subtle() -> Self {
        Self {
            color: ColorRole::Grid,
            inset_start: None,
        }
    }

    pub const fn with_inset_start(mut self, inset: LogicalUnit) -> Self {
        self.inset_start = Some(inset);
        self
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusIndicatorVariant {
    #[default]
    Compact,
    Normal,
}

/// Section 6.4. Mark and color are derived from `UniversalState::style()`,
/// never chosen independently -- a `StatusIndicator` cannot invent a local
/// meaning for a universal state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusIndicator {
    pub state: UniversalState,
    pub label: String,
    pub reason: Option<String>,
    pub variant: StatusIndicatorVariant,
}

impl StatusIndicator {
    pub fn new(state: UniversalState, label: impl Into<String>) -> Self {
        Self {
            state,
            label: label.into(),
            reason: None,
            variant: StatusIndicatorVariant::Compact,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn with_variant(mut self, variant: StatusIndicatorVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn mark(&self) -> StatusMark {
        self.state.style().mark
    }

    pub fn color(&self) -> ColorRole {
        self.state.style().color
    }

    /// "Compact variant: mark plus label; normal variant may add reason" --
    /// the single source of truth for whether a set reason actually shows,
    /// so a caller cannot accidentally leak a reason into the compact
    /// layout by forgetting to check the variant itself.
    pub fn visible_reason(&self) -> Option<&str> {
        match self.variant {
            StatusIndicatorVariant::Compact => None,
            StatusIndicatorVariant::Normal => self.reason.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_text_accessible_value_ignores_truncation_settings() {
        let text = SemanticText::new("Очень длинная строка", TextRole::Body, ColorRole::TextPrimary)
            .with_max_lines(1)
            .with_overflow(TextOverflow::Ellipsis);
        assert_eq!(text.accessible_value(), "Очень длинная строка");
    }

    #[test]
    fn icon_defaults_to_the_normal_control_size() {
        let icon = Icon::new(IconGlyph::Wifi, ColorRole::TextPrimary);
        assert_eq!(icon.size, IconSize::Large);
        let small = icon.with_size(IconSize::Small);
        assert_eq!(small.size, IconSize::Small);
    }

    #[test]
    fn divider_variants_use_the_named_semantic_tokens() {
        assert_eq!(Divider::new().color, ColorRole::Border);
        assert_eq!(Divider::subtle().color, ColorRole::Grid);
        assert_eq!(Divider::default(), Divider::new());
        let inset = Divider::new().with_inset_start(LogicalUnit::new(16));
        assert_eq!(inset.inset_start, Some(LogicalUnit::new(16)));
    }

    #[test]
    fn status_indicator_mark_and_color_come_from_universal_state_style() {
        let indicator = StatusIndicator::new(UniversalState::Failed, "Отключено");
        let style = UniversalState::Failed.style();
        assert_eq!(indicator.mark(), style.mark);
        assert_eq!(indicator.color(), style.color);
    }

    #[test]
    fn compact_variant_never_shows_a_reason_even_when_one_is_set() {
        let indicator =
            StatusIndicator::new(UniversalState::Blocked, "Заблокировано").with_reason("Нет сети");
        assert_eq!(indicator.visible_reason(), None);

        let normal = indicator.with_variant(StatusIndicatorVariant::Normal);
        assert_eq!(normal.visible_reason(), Some("Нет сети"));
    }
}
