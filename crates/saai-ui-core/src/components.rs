//! Component contracts from `docs/os/ui/component-library-v1.md` section 6
//! -- Experimental stability level per that document's section 2: each
//! contract exists and is tested, but promotion to Stable needs the
//! gallery, golden renders, and accessibility/migration tests section 9
//! requires, none of which exist yet.
//!
//! `SemanticText`, `Icon`, `Divider`, `StatusIndicator` (sections 6.1-6.4)
//! and `Progress`, `Button`, `Field`, `DataRow`, `Metric`, `Disclosure`
//! (sections 6.5-6.10) are pure content/semantic descriptors, not layout or
//! paint code -- matching section 8's ownership split ("`saai-ui-core`:
//! ... component contracts/state ... render backend: font loading, glyph
//! rasterization, icon tessellation, pixels"). None of these types
//! reference `Rect` or a physical pixel: a renderer measures and draws
//! them, this crate only says what they mean.
//!
//! Any display text that isn't already caller-supplied content (a status
//! label, a metric's "unknown" state) resolves through a translation key,
//! the same convention `UniversalState::style()`'s `label_key` already
//! established -- this crate never embeds a display-language string
//! directly.

use crate::{
    ColorRole, IconGlyph, IconSize, LogicalUnit, StatusMark, TextRole, UniversalState,
    MIN_TOUCH_TARGET, TWO_LINE_ROW_HEIGHT,
};

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

/// Section 6.5. `Determinate`'s value is always in 0..=100 -- constructed
/// only through `Progress::determinate`, which clamps, so an out-of-range
/// value can never exist. Deliberately has no method that derives a
/// `UniversalState` from reaching 100: "Completion changes to `COMPLETE`
/// only after the owning operation verifies it," not from the progress
/// value alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Determinate(u8),
    Indeterminate,
}

impl Progress {
    /// "Visual track: at least 4 logical units high."
    pub const MIN_TRACK_HEIGHT: LogicalUnit = LogicalUnit::new(4);

    /// "Determinate value is clamped to 0-100 and exposed numerically."
    pub fn determinate(value: u8) -> Self {
        Self::Determinate(value.min(100))
    }

    pub fn percent(&self) -> Option<u8> {
        match self {
            Self::Determinate(value) => Some(*value),
            Self::Indeterminate => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Quiet,
    /// "Destructive styling is allowed only for a genuinely destructive
    /// action and does not remove the confirmation/policy requirement" --
    /// a call-site decision this type records, not enforces (there is no
    /// way to know from here whether an action is genuinely destructive).
    Destructive,
}

/// Section 6.6. "Emits exactly one action after a valid press/release
/// sequence" -- `can_activate` is the single source of truth a renderer's
/// touch handler should check before honoring a release, so a disabled or
/// busy button can never fire twice or fire while doing nothing makes sense.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Button {
    pub label: String,
    pub action: String,
    pub variant: ButtonVariant,
    pub enabled: bool,
    pub busy: bool,
    pub icon: Option<IconGlyph>,
}

impl Button {
    pub fn new(label: impl Into<String>, action: impl Into<String>, variant: ButtonVariant) -> Self {
        Self {
            label: label.into(),
            action: action.into(),
            variant,
            enabled: true,
            busy: false,
            icon: None,
        }
    }

    pub fn with_icon(mut self, icon: IconGlyph) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn busy(mut self) -> Self {
        self.busy = true;
        self
    }

    pub fn can_activate(&self) -> bool {
        self.enabled && !self.busy
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Password,
}

/// Section 6.7.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub label: String,
    pub value: String,
    pub placeholder: Option<String>,
    pub help: Option<String>,
    pub error: Option<String>,
    pub leading_icon: Option<IconGlyph>,
    pub trailing_icon: Option<IconGlyph>,
    pub kind: FieldKind,
    /// Only meaningful for `FieldKind::Password` -- whether the user has
    /// explicitly asked to see the value.
    pub revealed: bool,
}

impl Field {
    pub fn new(label: impl Into<String>, kind: FieldKind) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            placeholder: None,
            help: None,
            error: None,
            leading_icon: None,
            trailing_icon: None,
            kind,
            revealed: false,
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn revealed(mut self, revealed: bool) -> Self {
        self.revealed = revealed;
        self
    }

    /// "Empty value is distinct from placeholder" -- a renderer checks this,
    /// not whether the value happens to equal the placeholder text.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// "Password/PIN variants never expose their value through logs or
    /// accessibility unless the user explicitly reveals it" -- the one
    /// place that decision is made, mirroring `SemanticText::
    /// accessible_value`'s role for truncation.
    pub fn accessible_value(&self) -> String {
        match self.kind {
            FieldKind::Text => self.value.clone(),
            FieldKind::Password if self.revealed => self.value.clone(),
            FieldKind::Password => "\u{2022}".repeat(self.value.chars().count()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataRowVariant {
    /// Not styled as clickable, per section 6.8.
    Static,
    Navigation,
    Toggle,
    Status,
}

/// Section 6.8.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataRow {
    pub icon: Option<IconGlyph>,
    pub primary: String,
    pub secondary: Option<String>,
    pub value: Option<String>,
    pub variant: DataRowVariant,
    pub action: Option<String>,
}

impl DataRow {
    pub fn new(primary: impl Into<String>, variant: DataRowVariant) -> Self {
        Self {
            icon: None,
            primary: primary.into(),
            secondary: None,
            value: None,
            variant,
            action: None,
        }
    }

    pub fn with_secondary(mut self, secondary: impl Into<String>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// "Static data is not styled as clickable" -- true only for a
    /// non-`Static` variant that actually carries an action, so a
    /// `Navigation`/`Toggle`/`Status` row built without one still reads as
    /// inert rather than a dead tap target.
    pub fn is_actionable(&self) -> bool {
        !matches!(self.variant, DataRowVariant::Static) && self.action.is_some()
    }

    /// "Minimum row hit height: 48; normal two-line row: 64 logical units."
    pub fn min_hit_height(&self) -> LogicalUnit {
        if self.secondary.is_some() {
            TWO_LINE_ROW_HEIGHT
        } else {
            MIN_TOUCH_TARGET
        }
    }
}

/// "Unknown and unavailable are text states, not zero" -- a caller cannot
/// substitute a numeric zero for missing data; it must choose one of these
/// three variants explicitly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetricValue {
    Known(String),
    Unknown,
    Unavailable,
}

impl MetricValue {
    /// Non-`Known` variants resolve through a translation key -- see this
    /// module's own doc comment on why display language never lives here.
    pub fn label_key(&self) -> Option<&'static str> {
        match self {
            Self::Known(_) => None,
            Self::Unknown => Some("metric.unknown"),
            Self::Unavailable => Some("metric.unavailable"),
        }
    }
}

/// Section 6.9.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metric {
    pub label: String,
    pub value: MetricValue,
    pub unit: Option<String>,
    /// Opaque, already-formatted timestamp text -- this crate does no time
    /// handling of its own.
    pub verified_at: Option<String>,
}

impl Metric {
    pub fn new(label: impl Into<String>, value: MetricValue) -> Self {
        Self {
            label: label.into(),
            value,
            unit: None,
            verified_at: None,
        }
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn with_verified_at(mut self, timestamp: impl Into<String>) -> Self {
        self.verified_at = Some(timestamp.into());
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisclosureState {
    #[default]
    Collapsed,
    Expanded,
}

/// Section 6.10. "Hit region is at least 48x48 even when the chevron is
/// 16-20 units" -- a layout/renderer concern (`MIN_TOUCH_TARGET`), not
/// something this data-only type enforces itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disclosure {
    pub label: String,
    /// The section/row this disclosure shows or hides -- "explicit
    /// collapsed/expanded state with a named target," not an anonymous
    /// toggle.
    pub target: String,
    pub state: DisclosureState,
}

impl Disclosure {
    pub fn new(label: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            target: target.into(),
            state: DisclosureState::Collapsed,
        }
    }

    pub fn expanded(mut self) -> Self {
        self.state = DisclosureState::Expanded;
        self
    }

    pub fn toggled(&self) -> Self {
        let mut next = self.clone();
        next.state = match self.state {
            DisclosureState::Collapsed => DisclosureState::Expanded,
            DisclosureState::Expanded => DisclosureState::Collapsed,
        };
        next
    }

    /// `IconGlyph::ChevronRight` collapsed, `IconGlyph::ChevronDown`
    /// expanded -- the one place this mapping is decided.
    pub fn chevron(&self) -> IconGlyph {
        match self.state {
            DisclosureState::Collapsed => IconGlyph::ChevronRight,
            DisclosureState::Expanded => IconGlyph::ChevronDown,
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

    #[test]
    fn progress_determinate_clamps_to_one_hundred() {
        assert_eq!(Progress::determinate(150).percent(), Some(100));
        assert_eq!(Progress::determinate(42).percent(), Some(42));
        assert_eq!(Progress::Indeterminate.percent(), None);
    }

    #[test]
    fn button_cannot_activate_when_disabled_or_busy() {
        let button = Button::new("Сохранить", "save", ButtonVariant::Primary);
        assert!(button.can_activate());
        assert!(!button.clone().disabled().can_activate());
        assert!(!button.busy().can_activate());
    }

    #[test]
    fn field_masks_password_value_unless_revealed() {
        let field = Field::new("PIN", FieldKind::Password).with_value("4269");
        assert_eq!(field.accessible_value(), "\u{2022}\u{2022}\u{2022}\u{2022}");
        assert_eq!(field.revealed(true).accessible_value(), "4269");

        let text = Field::new("Имя", FieldKind::Text).with_value("Аня");
        assert_eq!(text.accessible_value(), "Аня");
    }

    #[test]
    fn field_empty_is_distinct_from_placeholder() {
        let field = Field::new("SSID", FieldKind::Text).with_placeholder("Не выбрано");
        assert!(field.is_empty());
        assert_eq!(field.value, "");
        assert_eq!(field.placeholder.as_deref(), Some("Не выбрано"));
    }

    #[test]
    fn static_data_row_is_never_actionable() {
        let row = DataRow::new("Версия", DataRowVariant::Static).with_action("noop");
        assert!(!row.is_actionable());

        let nav = DataRow::new("Wi-Fi", DataRowVariant::Navigation).with_action("open_wifi");
        assert!(nav.is_actionable());

        let dead_nav = DataRow::new("Wi-Fi", DataRowVariant::Navigation);
        assert!(!dead_nav.is_actionable());
    }

    #[test]
    fn data_row_hit_height_grows_with_a_second_line() {
        let one_line = DataRow::new("Яркость", DataRowVariant::Navigation);
        assert_eq!(one_line.min_hit_height(), MIN_TOUCH_TARGET);

        let two_line =
            DataRow::new("Яркость", DataRowVariant::Navigation).with_secondary("50%");
        assert_eq!(two_line.min_hit_height(), TWO_LINE_ROW_HEIGHT);
    }

    #[test]
    fn metric_unknown_and_unavailable_are_not_zero() {
        let known = Metric::new("Батарея", MetricValue::Known("87".to_string()));
        assert_eq!(known.value.label_key(), None);

        let unknown = Metric::new("Батарея", MetricValue::Unknown);
        assert_eq!(unknown.value.label_key(), Some("metric.unknown"));

        let unavailable = Metric::new("Батарея", MetricValue::Unavailable);
        assert_eq!(unavailable.value.label_key(), Some("metric.unavailable"));
    }

    #[test]
    fn disclosure_toggle_flips_state_and_chevron() {
        let collapsed = Disclosure::new("Подробности", "diagnostics-panel");
        assert_eq!(collapsed.state, DisclosureState::Collapsed);
        assert_eq!(collapsed.chevron(), IconGlyph::ChevronRight);

        let expanded = collapsed.toggled();
        assert_eq!(expanded.state, DisclosureState::Expanded);
        assert_eq!(expanded.chevron(), IconGlyph::ChevronDown);

        let back = expanded.toggled();
        assert_eq!(back.state, DisclosureState::Collapsed);
    }
}
