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
//!
//! Every primitive exposes an `accessibility()` method returning
//! `Option<AccessibilityInfo>` (`None` for a primitive that must not be
//! independently exposed at all, e.g. a decorative `Icon` or an unnamed
//! `Divider`) -- role, name, value, and disabled/busy state in one place
//! per component, matching component-library-v1.md section 4's shared
//! state contract and each primitive's own accessibility note in section
//! 6. Non-color cues are not a separate mechanism here: every primitive
//! that carries meaning already carries it as text/enum data (`StatusMark`,
//! `MetricValue`, `Field::error`, ...), never color alone, so there is
//! nothing additional to add for that part of this task.

use crate::{
    ColorRole, IconGlyph, IconSize, LogicalUnit, StatusMark, TextRole, UniversalState,
    MIN_TOUCH_TARGET, TWO_LINE_ROW_HEIGHT,
};

/// What assistive technology would announce this primitive as. Not a
/// complete platform accessibility API (SaaiOS has no screen reader
/// integration yet) -- the stable, backend-independent shape that one
/// exists to be built against later, matching this crate's "component
/// contracts/state" ownership (component-library-v1.md section 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRole {
    Text,
    /// A composite's own titling text (`ContextHeader`, `SystemSection`) --
    /// distinct from `Text` so a renderer's accessibility tree can group a
    /// section's children under it the way a real heading would.
    Heading,
    Image,
    Button,
    TextField,
    ListItem,
    Disclosure,
    ProgressIndicator,
    Status,
    /// A confirmation/decision surface that names the choice, not a
    /// generic list row (`DecisionOverlay`).
    Dialog,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessibilityInfo {
    pub role: AccessibilityRole,
    pub name: Option<String>,
    /// Already-resolved display text when the primitive has one; a
    /// translation key (this crate's `label_key` convention) when it does
    /// not, e.g. `MetricValue::Unknown`. Which one it is follows from the
    /// component's own value type -- see each `accessibility()` method's
    /// doc comment.
    pub value: Option<String>,
    pub disabled: bool,
    pub busy: bool,
}

impl AccessibilityInfo {
    /// `pub(crate)`, not private: section 7's composites live in a sibling
    /// module (`composites.rs`) and build their own `AccessibilityInfo`
    /// values the same way every primitive here does.
    pub(crate) fn new(role: AccessibilityRole) -> Self {
        Self {
            role,
            name: None,
            value: None,
            disabled: false,
            busy: false,
        }
    }
}

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

    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            value: Some(self.accessible_value().to_string()),
            ..AccessibilityInfo::new(AccessibilityRole::Text)
        }
    }
}

/// Section 6.2. `size` defaults to 24 logical units ("24 is the normal
/// control size").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Icon {
    pub glyph: IconGlyph,
    pub size: IconSize,
    pub color: ColorRole,
    /// Section 6.2: "decorative when paired with text; otherwise requires
    /// a name." `None` (the default) means decorative -- `accessibility()`
    /// returns `None` for it, since the text it sits beside already
    /// carries the meaning.
    pub name: Option<String>,
}

impl Icon {
    pub const fn new(glyph: IconGlyph, color: ColorRole) -> Self {
        Self {
            glyph,
            size: IconSize::Large,
            color,
            name: None,
        }
    }

    pub const fn with_size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn is_decorative(&self) -> bool {
        self.name.is_none()
    }

    pub fn accessibility(&self) -> Option<AccessibilityInfo> {
        let name = self.name.clone()?;
        Some(AccessibilityInfo {
            name: Some(name),
            ..AccessibilityInfo::new(AccessibilityRole::Image)
        })
    }
}

/// Section 6.3: "Uses `Separator` geometry and the border token" --
/// `crate::Node::separator` already supplies the geometry (a
/// `StrokeToken::Hairline`-thick leaf); this supplies which semantic color
/// role paints it and an optional leading inset so it can align with a
/// neighboring row's text edge instead of running the full container width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Divider {
    pub color: ColorRole,
    pub inset_start: Option<LogicalUnit>,
    /// Section 6.3: "Does not receive focus or accessibility exposure
    /// unless it represents a named boundary." `None` (the default) means
    /// not exposed -- `accessibility()` returns `None` for it.
    pub name: Option<String>,
}

impl Divider {
    /// `color.border.default` in the spec's naming.
    pub const fn new() -> Self {
        Self {
            color: ColorRole::Border,
            inset_start: None,
            name: None,
        }
    }

    /// `color.grid.subtle` in the spec's naming -- a quieter boundary than
    /// `new()`, for a division that matters less than a section edge.
    pub const fn subtle() -> Self {
        Self {
            color: ColorRole::Grid,
            inset_start: None,
            name: None,
        }
    }

    pub const fn with_inset_start(mut self, inset: LogicalUnit) -> Self {
        self.inset_start = Some(inset);
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn accessibility(&self) -> Option<AccessibilityInfo> {
        let name = self.name.clone()?;
        Some(AccessibilityInfo {
            name: Some(name),
            ..AccessibilityInfo::new(AccessibilityRole::Text)
        })
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
}

/// VUI-07 (ADR-155): whole-surface empty / loading / offline. The
/// message is caller-supplied (this crate still does not embed a
/// display-language string). Idle empty paints no mark; Waiting and
/// Offline keep color from being the only cue via `StatusMark`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfacePattern {
    pub state: UniversalState,
    pub message: String,
}

impl SurfacePattern {
    pub fn empty(message: impl Into<String>) -> Self {
        Self {
            state: UniversalState::Idle,
            message: message.into(),
        }
    }

    pub fn loading(message: impl Into<String>) -> Self {
        Self {
            state: UniversalState::Waiting,
            message: message.into(),
        }
    }

    pub fn offline(message: impl Into<String>) -> Self {
        Self {
            state: UniversalState::Offline,
            message: message.into(),
        }
    }

    pub fn paints_mark(&self) -> bool {
        !matches!(self.state, UniversalState::Idle)
    }

    pub fn message_text(&self) -> SemanticText {
        SemanticText::new(
            self.message.clone(),
            TextRole::Body,
            self.state.style().color,
        )
    }

    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.message.clone()),
            value: Some(self.state.style().label_key.to_string()),
            busy: matches!(self.state, UniversalState::Waiting),
            ..AccessibilityInfo::new(AccessibilityRole::Status)
        }
    }
}

impl StatusIndicator {
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

    /// `value` is the state's `label_key` (e.g. `"state.blocked"`), not
    /// display text -- resolved the same way `MetricValue::label_key`
    /// already is.
    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.label.clone()),
            value: Some(self.state.style().label_key.to_string()),
            ..AccessibilityInfo::new(AccessibilityRole::Status)
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

    /// "Indeterminate progress uses restrained motion and a textual
    /// activity state" -- `busy: true` for `Indeterminate` is that textual
    /// activity state's accessibility half.
    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            value: self.percent().map(|value| value.to_string()),
            busy: matches!(self, Self::Indeterminate),
            ..AccessibilityInfo::new(AccessibilityRole::ProgressIndicator)
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
    pub fn new(
        label: impl Into<String>,
        action: impl Into<String>,
        variant: ButtonVariant,
    ) -> Self {
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

    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.label.clone()),
            disabled: !self.enabled,
            busy: self.busy,
            ..AccessibilityInfo::new(AccessibilityRole::Button)
        }
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
    pub disabled: bool,
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
            disabled: false,
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

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// `name` is the persistent label, never the placeholder -- section
    /// 6.7: "Placeholder is never the only accessible label." `value`
    /// reuses `accessible_value()`, so a masked password stays masked here
    /// too.
    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.label.clone()),
            value: Some(self.accessible_value()),
            disabled: self.disabled,
            ..AccessibilityInfo::new(AccessibilityRole::TextField)
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

    /// `Static` reads as `ListItem` (informational); every other variant
    /// reads as `Button`, matching `is_actionable()`'s own distinction --
    /// a row this crate marks non-clickable must not also announce itself
    /// as a button.
    pub fn accessibility(&self) -> AccessibilityInfo {
        let role = match self.variant {
            DataRowVariant::Static => AccessibilityRole::ListItem,
            DataRowVariant::Navigation | DataRowVariant::Toggle | DataRowVariant::Status => {
                AccessibilityRole::Button
            }
        };
        AccessibilityInfo {
            name: Some(self.primary.clone()),
            value: self.value.clone().or_else(|| self.secondary.clone()),
            disabled: !self.is_actionable() && self.variant != DataRowVariant::Static,
            ..AccessibilityInfo::new(role)
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

    /// `value` is the known text (plus unit, when set) for
    /// `MetricValue::Known`, or the same translation key
    /// `MetricValue::label_key()` already returns for `Unknown`/
    /// `Unavailable` -- one field, whose meaning follows the same
    /// text-vs-key split this module's own doc comment describes.
    pub fn accessibility(&self) -> AccessibilityInfo {
        let value = match (&self.value, &self.unit) {
            (MetricValue::Known(text), Some(unit)) => Some(format!("{text} {unit}")),
            (MetricValue::Known(text), None) => Some(text.clone()),
            (other, _) => other.label_key().map(str::to_string),
        };
        AccessibilityInfo {
            name: Some(self.label.clone()),
            value,
            ..AccessibilityInfo::new(AccessibilityRole::Text)
        }
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

    /// `value` is a `disclosure.collapsed`/`disclosure.expanded`
    /// translation key, this module's usual convention for a state that
    /// needs display language a caller supplies. Section 6.10:
    /// "Expansion updates focus order and accessibility state atomically
    /// with layout" is a call-site sequencing rule this data-only type
    /// cannot itself guarantee -- it can only make sure `toggled()`'s new
    /// state and this method's output are never out of sync with each
    /// other.
    pub fn accessibility(&self) -> AccessibilityInfo {
        let value = match self.state {
            DisclosureState::Collapsed => "disclosure.collapsed",
            DisclosureState::Expanded => "disclosure.expanded",
        };
        AccessibilityInfo {
            name: Some(self.label.clone()),
            value: Some(value.to_string()),
            ..AccessibilityInfo::new(AccessibilityRole::Disclosure)
        }
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
        let text = SemanticText::new(
            "Очень длинная строка",
            TextRole::Body,
            ColorRole::TextPrimary,
        )
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

        let two_line = DataRow::new("Яркость", DataRowVariant::Navigation).with_secondary("50%");
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

    #[test]
    fn decorative_icon_has_no_accessibility_exposure() {
        let decorative = Icon::new(IconGlyph::ChevronRight, ColorRole::TextSecondary);
        assert!(decorative.is_decorative());
        assert_eq!(decorative.accessibility(), None);

        let named = decorative.with_name("Раскрыть");
        assert!(!named.is_decorative());
        let info = named.accessibility().unwrap();
        assert_eq!(info.role, AccessibilityRole::Image);
        assert_eq!(info.name.as_deref(), Some("Раскрыть"));
    }

    #[test]
    fn unnamed_divider_has_no_accessibility_exposure() {
        assert_eq!(Divider::new().accessibility(), None);
        let named = Divider::new().with_name("Конец списка устройств");
        assert_eq!(
            named.accessibility().unwrap().name.as_deref(),
            Some("Конец списка устройств")
        );
    }

    #[test]
    fn button_accessibility_exposes_disabled_and_busy() {
        let button = Button::new("Сохранить", "save", ButtonVariant::Primary);
        let info = button.accessibility();
        assert_eq!(info.role, AccessibilityRole::Button);
        assert_eq!(info.name.as_deref(), Some("Сохранить"));
        assert!(!info.disabled);
        assert!(!info.busy);

        assert!(button.clone().disabled().accessibility().disabled);
        assert!(button.busy().accessibility().busy);
    }

    #[test]
    fn field_accessibility_reuses_the_masked_value() {
        let field = Field::new("PIN", FieldKind::Password).with_value("4269");
        let info = field.accessibility();
        assert_eq!(info.role, AccessibilityRole::TextField);
        assert_eq!(info.name.as_deref(), Some("PIN"));
        assert_eq!(
            info.value.as_deref(),
            Some("\u{2022}\u{2022}\u{2022}\u{2022}")
        );
        assert!(!info.disabled);
        assert!(field.disabled().accessibility().disabled);
    }

    #[test]
    fn data_row_accessibility_role_follows_actionability() {
        let static_row = DataRow::new("Версия", DataRowVariant::Static);
        assert_eq!(static_row.accessibility().role, AccessibilityRole::ListItem);
        assert!(!static_row.accessibility().disabled);

        let live_nav = DataRow::new("Wi-Fi", DataRowVariant::Navigation).with_action("open_wifi");
        let info = live_nav.accessibility();
        assert_eq!(info.role, AccessibilityRole::Button);
        assert!(!info.disabled);

        let dead_nav = DataRow::new("Wi-Fi", DataRowVariant::Navigation);
        assert!(dead_nav.accessibility().disabled);
    }

    #[test]
    fn metric_accessibility_value_is_text_when_known_and_a_key_otherwise() {
        let known = Metric::new("Батарея", MetricValue::Known("87".to_string())).with_unit("%");
        assert_eq!(known.accessibility().value.as_deref(), Some("87 %"));

        let unknown = Metric::new("Батарея", MetricValue::Unknown);
        assert_eq!(
            unknown.accessibility().value.as_deref(),
            Some("metric.unknown")
        );
    }

    #[test]
    fn disclosure_accessibility_value_tracks_state() {
        let collapsed = Disclosure::new("Подробности", "diagnostics-panel");
        assert_eq!(
            collapsed.accessibility().value.as_deref(),
            Some("disclosure.collapsed")
        );
        assert_eq!(
            collapsed.toggled().accessibility().value.as_deref(),
            Some("disclosure.expanded")
        );
    }

    #[test]
    fn surface_pattern_empty_has_no_mark_loading_and_offline_do() {
        let empty = SurfacePattern::empty("Ничего срочного");
        assert_eq!(empty.state, UniversalState::Idle);
        assert!(!empty.paints_mark());
        assert_eq!(empty.message_text().role, TextRole::Body);
        assert!(!empty.accessibility().busy);
        assert_eq!(empty.accessibility().role, AccessibilityRole::Status);

        let loading = SurfacePattern::loading("Сканирование…");
        assert_eq!(loading.state, UniversalState::Waiting);
        assert!(loading.paints_mark());
        assert!(loading.accessibility().busy);
        assert_eq!(loading.state.style().mark, StatusMark::Waiting);

        let offline = SurfacePattern::offline("Нет связи");
        assert_eq!(offline.state, UniversalState::Offline);
        assert!(offline.paints_mark());
        assert!(!offline.accessibility().busy);
        assert_eq!(offline.state.style().mark, StatusMark::Offline);
    }
}
