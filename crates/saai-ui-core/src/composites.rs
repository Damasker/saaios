//! Composite contracts from `docs/os/ui/component-library-v1.md` section 7
//! -- built entirely from section 6 primitives, per that section's own
//! rule: composites never draw their own text or own a rendering path a
//! primitive does not already provide. VUI-03's three
//! (`ContextHeader`, `SystemSection`, `ObjectSummary`) plus VUI-04's
//! (`BottomNavigation`, `OrbHost`, `SystemStatus`) per section 3's
//! inventory table; VUI-05 adds `IntentSummary`/`TaskSummary`/
//! `DecisionOverlay`/`AgentSummary`. VUI-06 adds `SettingRow`/
//! `CapabilityRow`. `EventRow` remains deferred.

use crate::{
    AccessibilityInfo, AccessibilityRole, Button, ButtonVariant, ColorRole, ContextColor, DataRow,
    DataRowVariant, Divider, IconGlyph, Metric, MotionCue, Progress, SemanticText, StatusIndicator,
    StatusIndicatorVariant, StatusMark, TextRole, UniversalState,
};

/// Section 7.1. Anatomy: an active-context label, an optional current-
/// section title, and an optional trailing `StatusIndicator` for a
/// non-default lifecycle -- nothing here is a new text-rendering path;
/// `heading()` hands the caller a `SemanticText` to draw exactly the way
/// any other primitive is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextHeader {
    pub context_name: String,
    pub section_title: Option<String>,
    pub lifecycle: Option<StatusIndicator>,
}

impl ContextHeader {
    pub fn new(context_name: impl Into<String>) -> Self {
        Self {
            context_name: context_name.into(),
            section_title: None,
            lifecycle: None,
        }
    }

    pub fn with_section_title(mut self, title: impl Into<String>) -> Self {
        self.section_title = Some(title.into());
        self
    }

    pub fn with_lifecycle(mut self, lifecycle: StatusIndicator) -> Self {
        self.lifecycle = Some(lifecycle);
        self
    }

    /// `"{context_name} · {section_title}"` when a section title is set --
    /// matches `saai-shell`'s current ad hoc `format!("{context_label} ·
    /// {title}")` in `draw_root` exactly, so this type is a drop-in
    /// replacement for that string, not a new information design.
    pub fn heading_text(&self) -> String {
        match &self.section_title {
            Some(title) => format!("{} · {}", self.context_name, title),
            None => self.context_name.clone(),
        }
    }

    pub fn heading(&self) -> SemanticText {
        SemanticText::new(self.heading_text(), TextRole::Title, ColorRole::TextPrimary)
    }

    /// Section 7.1: "name is the Space name plus section title; a
    /// non-default lifecycle is exposed through the nested
    /// `StatusIndicator`'s own state, not a second accessible string glued
    /// onto the header's name" -- `lifecycle_accessibility()` is that
    /// nested exposure, kept separate rather than merged in here.
    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.heading_text()),
            ..AccessibilityInfo::new(AccessibilityRole::Heading)
        }
    }

    pub fn lifecycle_accessibility(&self) -> Option<AccessibilityInfo> {
        Some(self.lifecycle.as_ref()?.accessibility())
    }
}

/// A `SystemSection` does not own the type of its own children -- section
/// 7.2: "any `DataRow`/`StatusIndicator`/`Metric`/`TaskSummary`/
/// `IntentSummary` a caller composes into it." This enum is that
/// composition boundary, not a fourth primitive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemSectionRow {
    Data(DataRow),
    Status(StatusIndicator),
    Metric(Metric),
    Task(TaskSummary),
    Intent(IntentSummary),
}

/// Section 7.2. Anatomy: a section title, a `Divider` immediately below
/// it, and zero or more `SystemSectionRow`s. Matches
/// `human-interface-architecture-v2.md` section 13's worked example
/// directly -- "Сегодня" / "Продолжается" / "Требует внимания" are three
/// `SystemSection`s, each with a different, possibly zero, row count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemSection {
    pub title: String,
    pub rows: Vec<SystemSectionRow>,
}

impl SystemSection {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
        }
    }

    pub fn with_row(mut self, row: SystemSectionRow) -> Self {
        self.rows.push(row);
        self
    }

    /// "A section with no children renders only its title (or is omitted
    /// entirely by the caller); `SystemSection` never invents a
    /// placeholder row to fill space." This is the check a caller makes
    /// to decide which of those two it wants -- `SystemSection` itself
    /// takes no position on which is correct for empty.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn heading(&self) -> SemanticText {
        SemanticText::new(
            self.title.clone(),
            TextRole::Section,
            ColorRole::TextPrimary,
        )
    }

    pub fn divider(&self) -> Divider {
        Divider::new()
    }

    /// Section 7.2: "title is exposed as a heading; children keep their
    /// own individual accessibility contracts -- `SystemSection` never
    /// flattens them into one combined string." Each `SystemSectionRow`'s
    /// own `accessibility()` (on the primitive it wraps) is what a
    /// renderer exposes for the children; this method only ever covers
    /// the title.
    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.title.clone()),
            ..AccessibilityInfo::new(AccessibilityRole::Heading)
        }
    }
}

/// Section 7.3: "optional trailing value/status" -- a plain value string,
/// or a `StatusIndicator` using the universal state mapping. Never a bare
/// color, matching every other primitive's own state-cue rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectSummaryTrailing {
    Value(String),
    Status(StatusIndicator),
}

/// Section 7.3. Distinct from `DataRow`: a `DataRow` is a generic list
/// item whose meaning a caller assembles by position; `ObjectSummary` has
/// one fixed semantic meaning -- "this is the object I am currently
/// working with" (`human-interface-architecture-v2.md` section 14's
/// OBJECT) -- so its meta line always reads as identity information
/// (object type and a distinguishing detail), never an arbitrary second
/// string a caller could repurpose for something else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectSummary {
    pub title: String,
    pub meta: String,
    pub trailing: Option<ObjectSummaryTrailing>,
}

impl ObjectSummary {
    pub fn new(title: impl Into<String>, meta: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            meta: meta.into(),
            trailing: None,
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.trailing = Some(ObjectSummaryTrailing::Value(value.into()));
        self
    }

    pub fn with_status(mut self, status: StatusIndicator) -> Self {
        self.trailing = Some(ObjectSummaryTrailing::Status(status));
        self
    }

    pub fn title_text(&self) -> SemanticText {
        SemanticText::new(self.title.clone(), TextRole::Body, ColorRole::TextPrimary)
    }

    pub fn meta_text(&self) -> SemanticText {
        SemanticText::new(
            self.meta.clone(),
            TextRole::Caption,
            ColorRole::TextSecondary,
        )
    }

    /// Section 7.3: "name is the object title; value is the meta line."
    /// A trailing status is exposed separately (`trailing_accessibility()`)
    /// rather than merged in, matching `SystemSection`'s own
    /// never-flatten rule.
    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.title.clone()),
            value: Some(self.meta.clone()),
            ..AccessibilityInfo::new(AccessibilityRole::ListItem)
        }
    }

    pub fn trailing_accessibility(&self) -> Option<AccessibilityInfo> {
        match self.trailing.as_ref()? {
            ObjectSummaryTrailing::Value(_) => None,
            ObjectSummaryTrailing::Status(status) => Some(status.accessibility()),
        }
    }
}

/// Section 7.6. Current work, not identity: title + universal-state
/// `StatusIndicator` + optional reason + optional originating Intent
/// caption. No worker count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSummary {
    pub title: String,
    pub state: UniversalState,
    pub reason: Option<String>,
    pub related: Option<String>,
}

impl TaskSummary {
    pub fn new(title: impl Into<String>, state: UniversalState) -> Self {
        Self {
            title: title.into(),
            state,
            reason: None,
            related: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn with_related(mut self, related: impl Into<String>) -> Self {
        self.related = Some(related.into());
        self
    }

    pub fn status(&self) -> StatusIndicator {
        let mut indicator = StatusIndicator::new(self.state, self.title.clone());
        if let Some(reason) = &self.reason {
            indicator = indicator
                .with_reason(reason.clone())
                .with_variant(StatusIndicatorVariant::Normal);
        }
        indicator
    }

    pub fn related_text(&self) -> Option<SemanticText> {
        Some(SemanticText::new(
            self.related.as_ref()?.clone(),
            TextRole::Caption,
            ColorRole::TextSecondary,
        ))
    }

    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.title.clone()),
            value: Some(self.state.style().label_key.to_string()),
            ..AccessibilityInfo::new(AccessibilityRole::ListItem)
        }
    }

    pub fn related_accessibility(&self) -> Option<AccessibilityInfo> {
        Some(AccessibilityInfo {
            name: Some(self.related.as_ref()?.clone()),
            ..AccessibilityInfo::new(AccessibilityRole::Text)
        })
    }
}

/// Section 7.7. Intent title plus at most one nested `TaskSummary`.
/// Missing work is «Нет задачи». No worker list, no Agent personality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntentSummary {
    pub title: String,
    pub task: Option<TaskSummary>,
}

impl IntentSummary {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            task: None,
        }
    }

    pub fn with_task(mut self, task: TaskSummary) -> Self {
        self.task = Some(task);
        self
    }

    pub fn heading(&self) -> SemanticText {
        SemanticText::new(self.title.clone(), TextRole::Body, ColorRole::TextPrimary)
    }

    pub fn missing_task_text(&self) -> Option<SemanticText> {
        if self.task.is_some() {
            None
        } else {
            Some(SemanticText::new(
                "Нет задачи",
                TextRole::Caption,
                ColorRole::TextSecondary,
            ))
        }
    }

    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.title.clone()),
            ..AccessibilityInfo::new(AccessibilityRole::ListItem)
        }
    }
}

/// Section 7.8. A decision names actor, intended action, affected
/// object, scope, optional consequence, and two reversible choices
/// (accept/decline). Missing facts are omitted. The overlay does not
/// invent whether the *action* can be undone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionOverlay {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub object: String,
    pub scope: Option<String>,
    pub consequence: Option<String>,
    pub accept: Button,
    pub decline: Button,
}

impl DecisionOverlay {
    pub fn new(object: impl Into<String>) -> Self {
        Self {
            actor: None,
            action: None,
            object: object.into(),
            scope: None,
            consequence: None,
            accept: Button::new("Подтвердить", "confirm", ButtonVariant::Primary),
            decline: Button::new("Отклонить", "decline", ButtonVariant::Secondary),
        }
    }

    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    pub fn with_consequence(mut self, consequence: impl Into<String>) -> Self {
        self.consequence = Some(consequence.into());
        self
    }

    pub fn fact_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(actor) = &self.actor {
            lines.push(format!("Актёр: {actor}"));
        }
        if let Some(action) = &self.action {
            lines.push(format!("Действие: {action}"));
        }
        lines.push(format!("Объект: {}", self.object));
        if let Some(scope) = &self.scope {
            lines.push(format!("Область: {scope}"));
        }
        if let Some(consequence) = &self.consequence {
            lines.push(format!("Последствие: {consequence}"));
        }
        lines
    }

    pub fn heading(&self) -> SemanticText {
        SemanticText::new(self.object.clone(), TextRole::Title, ColorRole::TextPrimary)
    }

    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.object.clone()),
            value: self.action.clone(),
            ..AccessibilityInfo::new(AccessibilityRole::Dialog)
        }
    }
}

/// Section 7.9. Disposable execution slot (ADR-121 Worker), not a
/// persistent Agent personality. Assigned only from a real `saaios.action`.
/// Unassigned / unavailable are captions, never a fake cluster or
/// `Воркеры (3)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentAssignment {
    Unassigned,
    Unavailable,
    Execution {
        title: String,
        state: UniversalState,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSummary {
    pub assignment: AgentAssignment,
}

impl AgentSummary {
    pub fn unassigned() -> Self {
        Self {
            assignment: AgentAssignment::Unassigned,
        }
    }

    pub fn unavailable() -> Self {
        Self {
            assignment: AgentAssignment::Unavailable,
        }
    }

    pub fn from_execution(title: impl Into<String>, state: UniversalState) -> Self {
        Self {
            assignment: AgentAssignment::Execution {
                title: title.into(),
                state,
            },
        }
    }

    pub fn caption(&self) -> SemanticText {
        match &self.assignment {
            AgentAssignment::Unassigned => SemanticText::new(
                "Нет исполнения",
                TextRole::Caption,
                ColorRole::TextSecondary,
            ),
            AgentAssignment::Unavailable => SemanticText::new(
                "Исполнение недоступно",
                TextRole::Caption,
                ColorRole::TextSecondary,
            ),
            AgentAssignment::Execution { title, .. } => {
                SemanticText::new(title.clone(), TextRole::Body, ColorRole::TextPrimary)
            }
        }
    }

    pub fn status(&self) -> Option<StatusIndicator> {
        match &self.assignment {
            AgentAssignment::Execution { title, state } => {
                Some(StatusIndicator::new(*state, title.clone()))
            }
            AgentAssignment::Unassigned | AgentAssignment::Unavailable => None,
        }
    }

    pub fn detail_line(&self) -> String {
        match &self.assignment {
            AgentAssignment::Unassigned => "Нет исполнения".to_string(),
            AgentAssignment::Unavailable => "Исполнение недоступно".to_string(),
            AgentAssignment::Execution { title, .. } => format!("Исполнение: {title}"),
        }
    }

    pub fn accessibility(&self) -> AccessibilityInfo {
        match &self.assignment {
            AgentAssignment::Unassigned => AccessibilityInfo {
                name: Some("Нет исполнения".into()),
                ..AccessibilityInfo::new(AccessibilityRole::Text)
            },
            AgentAssignment::Unavailable => AccessibilityInfo {
                name: Some("Исполнение недоступно".into()),
                ..AccessibilityInfo::new(AccessibilityRole::Text)
            },
            AgentAssignment::Execution { title, state } => AccessibilityInfo {
                name: Some(title.clone()),
                value: Some(state.style().label_key.to_string()),
                ..AccessibilityInfo::new(AccessibilityRole::ListItem)
            },
        }
    }
}

/// Section 7.10. A device setting as a `DataRow`: label, current value,
/// optional dispatch. No gauge. Silent rows look static but still carry
/// a hidden action (HIA-20 build-info).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingRow {
    pub row: DataRow,
}

impl SettingRow {
    pub fn readout(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            row: DataRow::new(label, DataRowVariant::Static).with_value(value),
        }
    }

    pub fn cycle(
        label: impl Into<String>,
        value: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            row: DataRow::new(label, DataRowVariant::Toggle)
                .with_value(value)
                .with_action(action),
        }
    }

    pub fn open(
        label: impl Into<String>,
        value: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            row: DataRow::new(label, DataRowVariant::Navigation)
                .with_value(value)
                .with_action(action),
        }
    }

    pub fn silent(
        label: impl Into<String>,
        value: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            row: DataRow::new(label, DataRowVariant::Static)
                .with_value(value)
                .with_action(action),
        }
    }

    pub fn accessibility(&self) -> AccessibilityInfo {
        self.row.accessibility()
    }
}

/// Section 7.11. One installed app and its granted capabilities.
/// Read-only: there is no revoke protocol on this surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityRow {
    pub row: DataRow,
}

impl CapabilityRow {
    pub fn new(
        name: impl Into<String>,
        state: impl Into<String>,
        grants: impl Into<String>,
    ) -> Self {
        Self {
            row: DataRow::new(name, DataRowVariant::Static)
                .with_value(state)
                .with_secondary(grants),
        }
    }

    pub fn accessibility(&self) -> AccessibilityInfo {
        self.row.accessibility()
    }
}

/// Section 7.4. One destination in the bottom navigation strip. `icon` is
/// optional rather than required: this project's current icon set
/// (`docs/os/architecture/../../os/targets/panther/third_party/README.md`'s
/// Feather build) has no glyph yet for any of these destinations, so a
/// renderer's existing placeholder mark stays honest instead of this type
/// pretending an icon exists when it does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationItem {
    pub id: String,
    pub label: String,
    pub icon: Option<IconGlyph>,
    pub selected: bool,
    pub pressed: bool,
    pub disabled: bool,
    pub attention: bool,
    /// A real count (e.g. unread items), never a decorative dot -- this
    /// crate's own top-level rule against invented data applies to
    /// composites as much as primitives.
    pub badge: Option<u32>,
}

impl NavigationItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            selected: false,
            pressed: false,
            disabled: false,
            attention: false,
            badge: None,
        }
    }

    pub fn with_icon(mut self, icon: IconGlyph) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }

    pub fn pressed(mut self) -> Self {
        self.pressed = true;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn with_attention(mut self) -> Self {
        self.attention = true;
        self
    }

    pub fn with_badge(mut self, count: u32) -> Self {
        self.badge = Some(count);
        self
    }

    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            name: Some(self.label.clone()),
            value: self.badge.map(|count| count.to_string()),
            disabled: self.disabled,
            ..AccessibilityInfo::new(AccessibilityRole::Button)
        }
    }
}

/// Section 7.4. A caller builds exactly one `NavigationItem` per real
/// destination; this type does not invent extras and does not enforce
/// "exactly one selected" itself (`selected_index` reports what it
/// actually finds, including zero or more than one, so a bug in the
/// caller's own state is visible instead of silently normalized away).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BottomNavigation {
    pub items: Vec<NavigationItem>,
}

impl BottomNavigation {
    pub fn new(items: Vec<NavigationItem>) -> Self {
        Self { items }
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.items.iter().position(|item| item.selected)
    }
}

/// Section 7.5. VUI-04's Orb host contract: quiet/active/progress/
/// attention/offline map directly onto the existing `UniversalState`
/// vocabulary (`Idle`/`Active`/`Running`/`Attention`/`Offline`) rather
/// than inventing a parallel enum -- section 4's own rule ("controls do
/// not invent local ... meanings") applies here as much as to any
/// primitive. `reduced_motion` is a separate flag, not a sixth state:
/// every other primitive in this crate already treats reduced motion as
/// a rendering modifier on top of a state, never a state of its own
/// (`Progress`'s "restrained motion" note, `StatusIndicator`'s "reduced
/// motion shows the static activity mark").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrbHost {
    pub state: UniversalState,
    pub reduced_motion: bool,
    /// Context Light quantity=arc/fill. `None` means the quantity is
    /// unknown, not `0`. Callers pass a determinate `Progress` from a
    /// real reading (battery today).
    pub quantity: Option<Progress>,
}

impl OrbHost {
    pub fn new(state: UniversalState) -> Self {
        Self {
            state,
            reduced_motion: false,
            quantity: None,
        }
    }

    pub fn with_reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = reduced;
        self
    }

    pub fn with_quantity(mut self, progress: Progress) -> Self {
        self.quantity = Some(progress);
        self
    }

    pub fn mark(&self) -> StatusMark {
        self.state.style().mark
    }

    /// Context Light attention=ring: a dedicated cue beyond `mark()`,
    /// only when the Orb's own state is Attention. Not a second
    /// notification model — the caller decides Attention from
    /// `saai-attention`.
    pub fn attention_ring(&self) -> bool {
        self.state == UniversalState::Attention
    }

    /// Context Light activity=motion. Reduced motion is a rendering
    /// modifier, not a sixth state: Running still exists, it just
    /// does not pulse.
    pub fn motion(&self) -> MotionCue {
        if self.reduced_motion {
            MotionCue::None
        } else {
            self.state.style().motion
        }
    }

    /// Determinate percent only. Missing quantity stays `None`.
    pub fn quantity_percent(&self) -> Option<u8> {
        self.quantity.and_then(|progress| progress.percent())
    }

    pub fn color(&self) -> ColorRole {
        self.state.style().color
    }

    /// Section 7.5: "understandable without animation" -- `value` always
    /// exposes the real state via its `label_key`, independent of
    /// whether a renderer happens to be animating anything right now.
    pub fn accessibility(&self) -> AccessibilityInfo {
        AccessibilityInfo {
            value: Some(self.state.style().label_key.to_string()),
            busy: self.state == UniversalState::Running && !self.reduced_motion,
            ..AccessibilityInfo::new(AccessibilityRole::Status)
        }
    }
}

/// VUI-04 system-status composite for the persistent status layer.
/// Describes the same facts `draw_status_bar` already paints: Context
/// Light (a Space color, never a severity color), clock, network, battery.
/// Missing battery is absent, not `0%`. A Space with no color is
/// `ContextColor::Default`, never an empty/missing dot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemStatus {
    pub context: ContextColor,
    pub time_text: String,
    pub network_up: bool,
    pub battery_percent: Option<u8>,
    pub battery_charging: bool,
}

impl SystemStatus {
    pub fn new(context: ContextColor, time_text: impl Into<String>) -> Self {
        Self {
            context,
            time_text: time_text.into(),
            network_up: false,
            battery_percent: None,
            battery_charging: false,
        }
    }

    pub fn with_network_up(mut self, up: bool) -> Self {
        self.network_up = up;
        self
    }

    pub fn with_battery(mut self, percent: u8, charging: bool) -> Self {
        self.battery_percent = Some(percent);
        self.battery_charging = charging;
        self
    }

    pub fn time(&self) -> SemanticText {
        SemanticText::new(
            self.time_text.clone(),
            TextRole::Title,
            ColorRole::TextPrimary,
        )
    }

    pub fn network_label(&self) -> &'static str {
        if self.network_up {
            "Wi-Fi"
        } else {
            "Нет сети"
        }
    }

    pub fn battery_label(&self) -> Option<String> {
        self.battery_percent.map(|percent| {
            if self.battery_charging {
                format!("{percent}% +")
            } else {
                format!("{percent}%")
            }
        })
    }

    pub fn accessibility(&self) -> AccessibilityInfo {
        let mut parts = vec![self.time_text.clone(), self.network_label().to_string()];
        if let Some(battery) = self.battery_label() {
            parts.push(battery);
        }
        AccessibilityInfo {
            name: Some("system status".into()),
            value: Some(parts.join(" · ")),
            ..AccessibilityInfo::new(AccessibilityRole::Status)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessibilityRole, DataRowVariant, UniversalState};

    #[test]
    fn context_header_heading_text_matches_draw_roots_current_format() {
        let bare = ContextHeader::new("Personal");
        assert_eq!(bare.heading_text(), "Personal");

        let with_section = ContextHeader::new("Personal").with_section_title("Сейчас");
        assert_eq!(with_section.heading_text(), "Personal · Сейчас");
        assert_eq!(
            with_section.accessibility().name.as_deref(),
            Some("Personal · Сейчас")
        );
    }

    #[test]
    fn context_header_lifecycle_accessibility_is_separate_from_the_heading_name() {
        let header = ContextHeader::new("Archive")
            .with_lifecycle(StatusIndicator::new(UniversalState::Blocked, "Архив"));
        assert_eq!(header.accessibility().name.as_deref(), Some("Archive"));
        assert!(header.lifecycle_accessibility().is_some());

        assert!(ContextHeader::new("Personal")
            .lifecycle_accessibility()
            .is_none());
    }

    #[test]
    fn system_section_is_empty_reflects_row_count_and_never_invents_placeholder_rows() {
        let empty = SystemSection::new("Требует внимания");
        assert!(empty.is_empty());
        assert_eq!(empty.rows.len(), 0);

        let filled = SystemSection::new("Сегодня").with_row(SystemSectionRow::Data(DataRow::new(
            "10:30 Daily",
            DataRowVariant::Static,
        )));
        assert!(!filled.is_empty());
        assert_eq!(filled.accessibility().name.as_deref(), Some("Сегодня"));
    }

    #[test]
    fn object_summary_accessibility_value_is_the_meta_line_not_the_trailing_status() {
        let summary = ObjectSummary::new("Отчёт", "saaios.task · версия 3")
            .with_status(StatusIndicator::new(UniversalState::Running, "Выполняется"));
        let info = summary.accessibility();
        assert_eq!(info.name.as_deref(), Some("Отчёт"));
        assert_eq!(info.value.as_deref(), Some("saaios.task · версия 3"));
        assert!(summary.trailing_accessibility().is_some());
    }

    #[test]
    fn object_summary_with_value_trailing_has_no_separate_trailing_accessibility() {
        let summary = ObjectSummary::new("Батарея", "Metric").with_value("87%");
        assert!(summary.trailing_accessibility().is_none());
    }

    #[test]
    fn task_summary_uses_universal_state_and_keeps_related_caption_separate() {
        let task = TaskSummary::new("Копирует файлы", UniversalState::Running)
            .with_reason("Выполняется")
            .with_related("Подготовить демо");
        let status = task.status();
        assert_eq!(status.state, UniversalState::Running);
        assert_eq!(status.label, "Копирует файлы");
        assert_eq!(status.visible_reason(), Some("Выполняется"));
        assert_eq!(
            task.related_text().map(|text| text.content),
            Some("Подготовить демо".into())
        );
        assert_eq!(task.accessibility().value.as_deref(), Some("state.running"));
        assert!(task.related_accessibility().is_some());
        assert!(TaskSummary::new("Ожидает", UniversalState::Waiting)
            .related_text()
            .is_none());
    }

    #[test]
    fn intent_summary_without_a_task_is_truthful_and_has_no_worker_count() {
        let empty = IntentSummary::new("Пустое намерение");
        assert!(empty.task.is_none());
        assert_eq!(
            empty.missing_task_text().map(|text| text.content),
            Some("Нет задачи".into())
        );
        let with_task = IntentSummary::new("Подготовить демо").with_task(TaskSummary::new(
            "Подтвердите: демо",
            UniversalState::Attention,
        ));
        assert!(with_task.missing_task_text().is_none());
        assert_eq!(
            with_task.task.as_ref().map(|task| task.title.as_str()),
            Some("Подтвердите: демо")
        );
    }

    #[test]
    fn bottom_navigation_selected_index_finds_the_one_real_selection() {
        let nav = BottomNavigation::new(vec![
            NavigationItem::new("now", "Сейчас").selected(),
            NavigationItem::new("inbox", "Входящие").with_badge(3),
            NavigationItem::new("spaces", "Пространства"),
            NavigationItem::new("system", "Система").disabled(),
        ]);
        assert_eq!(nav.selected_index(), Some(0));
        assert_eq!(nav.items[1].accessibility().value.as_deref(), Some("3"));
        assert!(nav.items[3].accessibility().disabled);
    }

    #[test]
    fn bottom_navigation_selected_index_is_none_when_nothing_is_selected() {
        let nav = BottomNavigation::new(vec![
            NavigationItem::new("now", "Сейчас"),
            NavigationItem::new("inbox", "Входящие"),
        ]);
        assert_eq!(nav.selected_index(), None);
    }

    #[test]
    fn orb_host_reuses_universal_state_and_is_busy_only_when_running_and_not_reduced() {
        let running = OrbHost::new(UniversalState::Running);
        assert!(running.accessibility().busy);

        let running_reduced = OrbHost::new(UniversalState::Running).with_reduced_motion(true);
        assert!(!running_reduced.accessibility().busy);

        let attention = OrbHost::new(UniversalState::Attention);
        assert!(!attention.accessibility().busy);
        assert_eq!(
            attention.accessibility().value.as_deref(),
            Some("state.attention")
        );
        assert!(attention.attention_ring());
        assert!(!OrbHost::new(UniversalState::Idle).attention_ring());
        assert!(!OrbHost::new(UniversalState::Running).attention_ring());
        assert!(!OrbHost::new(UniversalState::Offline).attention_ring());
        assert_eq!(
            OrbHost::new(UniversalState::Running).motion(),
            MotionCue::ActivityPulse
        );
        assert_eq!(
            OrbHost::new(UniversalState::Running)
                .with_reduced_motion(true)
                .motion(),
            MotionCue::None
        );
        assert_eq!(OrbHost::new(UniversalState::Idle).motion(), MotionCue::None);
        assert!(OrbHost::new(UniversalState::Idle)
            .quantity_percent()
            .is_none());
        assert_eq!(
            OrbHost::new(UniversalState::Idle)
                .with_quantity(Progress::determinate(87))
                .quantity_percent(),
            Some(87)
        );
    }

    #[test]
    fn system_status_does_not_invent_battery_or_a_missing_context_dot() {
        let status = SystemStatus::new(ContextColor::Default, "09:42");
        assert!(status.battery_label().is_none());
        assert!(!status.network_up);
        assert_eq!(status.network_label(), "Нет сети");
        assert_eq!(status.context, ContextColor::Default);
    }

    #[test]
    fn system_status_context_is_not_a_severity_color() {
        let status = SystemStatus::new(ContextColor::Orange, "09:42").with_network_up(true);
        assert_ne!(status.context, ContextColor::Default);
        assert_eq!(status.network_label(), "Wi-Fi");
        assert_eq!(
            status.with_battery(87, true).battery_label().as_deref(),
            Some("87% +")
        );
        assert_eq!(
            SystemStatus::new(ContextColor::Green, "09:42")
                .with_battery(12, false)
                .battery_label()
                .as_deref(),
            Some("12%")
        );
    }

    #[test]
    fn decision_overlay_names_facts_and_omits_missing_ones() {
        let overlay = DecisionOverlay::new("Подтвердите: убить процесс")
            .with_actor("Система")
            .with_action("process.kill_request")
            .with_scope("work")
            .with_consequence("Завершить процесс 999");
        assert_eq!(
            overlay.fact_lines(),
            vec![
                "Актёр: Система".to_string(),
                "Действие: process.kill_request".to_string(),
                "Объект: Подтвердите: убить процесс".to_string(),
                "Область: work".to_string(),
                "Последствие: Завершить процесс 999".to_string(),
            ]
        );
        assert_eq!(overlay.accept.label, "Подтвердить");
        assert_eq!(overlay.decline.label, "Отклонить");
        assert!(overlay.accept.can_activate());
        assert_eq!(overlay.accessibility().role, AccessibilityRole::Dialog);
        assert_eq!(
            overlay.accessibility().value.as_deref(),
            Some("process.kill_request")
        );

        let sparse = DecisionOverlay::new("Задача");
        assert_eq!(sparse.fact_lines(), vec!["Объект: Задача".to_string()]);
        assert!(sparse.action.is_none());
        assert!(sparse.consequence.is_none());
    }

    #[test]
    fn agent_summary_is_unassigned_or_unavailable_without_inventing_a_personality() {
        let empty = AgentSummary::unassigned();
        assert_eq!(empty.assignment, AgentAssignment::Unassigned);
        assert_eq!(empty.caption().content, "Нет исполнения");
        assert!(empty.status().is_none());
        assert_eq!(empty.detail_line(), "Нет исполнения");
        assert_eq!(
            empty.accessibility().name.as_deref(),
            Some("Нет исполнения")
        );

        let down = AgentSummary::unavailable();
        assert_eq!(down.assignment, AgentAssignment::Unavailable);
        assert_eq!(down.caption().content, "Исполнение недоступно");
        assert!(down.status().is_none());
        assert_eq!(down.detail_line(), "Исполнение недоступно");

        let running = AgentSummary::from_execution("Экспорт PDF", UniversalState::Running);
        assert_eq!(
            running.assignment,
            AgentAssignment::Execution {
                title: "Экспорт PDF".into(),
                state: UniversalState::Running,
            }
        );
        assert_eq!(running.caption().content, "Экспорт PDF");
        assert_eq!(
            running.status().map(|status| status.state),
            Some(UniversalState::Running)
        );
        assert_eq!(running.detail_line(), "Исполнение: Экспорт PDF");
        assert_eq!(
            running.accessibility().value.as_deref(),
            Some("state.running")
        );
        assert!(!running.detail_line().contains("Воркеры"));
        assert!(!running.caption().content.contains("ResearchAgent"));
    }

    #[test]
    fn setting_row_variants_do_not_invent_a_gauge() {
        let readout = SettingRow::readout("Хранилище", "12 ГБ свободно");
        assert_eq!(readout.row.variant, DataRowVariant::Static);
        assert!(readout.row.action.is_none());
        assert!(!readout.row.is_actionable());

        let cycle = SettingRow::cycle("Яркость экрана", "50%", "cycle_brightness");
        assert_eq!(cycle.row.variant, DataRowVariant::Toggle);
        assert_eq!(cycle.row.action.as_deref(), Some("cycle_brightness"));
        assert!(cycle.row.is_actionable());

        let open = SettingRow::open("Wi-Fi", "Wallbox", "open_wifi_list");
        assert_eq!(open.row.variant, DataRowVariant::Navigation);
        assert_eq!(open.row.value.as_deref(), Some("Wallbox"));

        let silent = SettingRow::silent("SaaiOS · сборка abc", "panther", "tap_build_info");
        assert_eq!(silent.row.variant, DataRowVariant::Static);
        assert_eq!(silent.row.action.as_deref(), Some("tap_build_info"));
        assert!(!silent.row.is_actionable());
        assert_eq!(
            silent.accessibility().name.as_deref(),
            Some("SaaiOS · сборка abc")
        );
    }

    #[test]
    fn capability_row_is_read_only_and_keeps_empty_grants_honest() {
        let row = CapabilityRow::new("Камера", "остановлено", "без разрешений");
        assert_eq!(row.row.variant, DataRowVariant::Static);
        assert!(row.row.action.is_none());
        assert_eq!(row.row.value.as_deref(), Some("остановлено"));
        assert_eq!(row.row.secondary.as_deref(), Some("без разрешений"));
        assert!(!row.row.primary.contains("Android VM"));
    }
}
