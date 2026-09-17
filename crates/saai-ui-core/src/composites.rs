//! Composite contracts from `docs/os/ui/component-library-v1.md` section 7
//! -- built entirely from section 6 primitives, per that section's own
//! rule: composites never draw their own text or own a rendering path a
//! primitive does not already provide. VUI-03's three
//! (`ContextHeader`, `SystemSection`, `ObjectSummary`) plus VUI-04's two
//! (`BottomNavigation`, `OrbHost`) per section 3's inventory table;
//! `EventRow`/`IntentSummary`/`TaskSummary`/`AgentSummary` remain
//! deferred to VUI-05 and do not exist here.

use crate::{
    AccessibilityInfo, AccessibilityRole, ColorRole, DataRow, Divider, IconGlyph, Metric,
    SemanticText, StatusIndicator, StatusMark, TextRole, UniversalState,
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
/// 7.2: "any `DataRow`/`StatusIndicator`/`Metric` a caller composes into
/// it." This enum is that composition boundary, not a fourth primitive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemSectionRow {
    Data(DataRow),
    Status(StatusIndicator),
    Metric(Metric),
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
}

impl OrbHost {
    pub fn new(state: UniversalState) -> Self {
        Self {
            state,
            reduced_motion: false,
        }
    }

    pub fn with_reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = reduced;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataRowVariant, UniversalState};

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
    }
}
