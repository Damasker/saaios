//! Typed view tree and deterministic layout for Saai UI (ADR-017).
//!
//! This is the runtime-independent intermediate representation. The future
//! `.sui` compiler will produce this tree; shells and apps do not implement a
//! second set of rectangles for touch handling.

mod components;
mod composites;
mod foundations;

pub use components::{
    AccessibilityInfo, AccessibilityRole, Button, ButtonVariant, DataRow, DataRowVariant,
    Disclosure, DisclosureState, Divider, Field, FieldKind, Icon, Metric, MetricValue, Progress,
    SemanticText, StatusIndicator, StatusIndicatorVariant, TextOverflow,
};
pub use composites::{
    BottomNavigation, ContextHeader, NavigationItem, ObjectSummary, ObjectSummaryTrailing, OrbHost,
    SystemSection, SystemSectionRow, SystemStatus,
};
pub use foundations::{
    FontFamily, FontWeight, IconGlyph, IconSize, LogicalUnit, MotionToken, RadiusToken, SafeInsets,
    SpacingToken, StrokeToken, SurfaceLevel, SurfaceScale, SurfaceStyle, TextRole, TextStyle,
    CONTROL_VISUAL_HEIGHT, MIN_TOUCH_TARGET, TWO_LINE_ROW_HEIGHT,
};

/// Backend-independent sRGB color. Renderers are responsible for converting
/// this logical value to their native pixel/scanout packing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    pub const fn from_hex(value: u32) -> Self {
        Self::new(
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        )
    }
}

/// Product meaning requested by a component. These are intentionally not
/// storage names or backend colors: one complete Theme owns the mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorRole {
    Canvas,
    Surface,
    Elevated,
    Accent,
    AccentHighlight,
    TextPrimary,
    TextSecondary,
    Success,
    Attention,
    Critical,
    Border,
    Grid,
    Pressed,
    Focus,
    DisabledSurface,
    DisabledText,
    HighContrastText,
}

/// HIA context colors live in a separate namespace from semantic status.
/// A Space color must never be interpreted as severity or action affordance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextColor {
    Default,
    Blue,
    Green,
    Orange,
    Purple,
    Pink,
}

/// States shared by tasks, agents, objects, services, and system components.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniversalState {
    Idle,
    Active,
    Running,
    Waiting,
    Blocked,
    Attention,
    Failed,
    Complete,
    Offline,
}

/// Shape/icon cue paired with status color so color never carries the only
/// meaning. VUI-02 maps these semantic marks to the selected icon family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusMark {
    Outline,
    ActiveDot,
    Activity,
    Waiting,
    Blocked,
    Alert,
    Failure,
    Complete,
    Offline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionCue {
    None,
    ActivityPulse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateStyle {
    pub color: ColorRole,
    pub label_key: &'static str,
    pub mark: StatusMark,
    pub motion: MotionCue,
}

impl UniversalState {
    pub const fn style(self) -> StateStyle {
        match self {
            Self::Idle => StateStyle {
                color: ColorRole::TextSecondary,
                label_key: "state.idle",
                mark: StatusMark::Outline,
                motion: MotionCue::None,
            },
            Self::Active => StateStyle {
                color: ColorRole::Accent,
                label_key: "state.active",
                mark: StatusMark::ActiveDot,
                motion: MotionCue::None,
            },
            Self::Running => StateStyle {
                color: ColorRole::Accent,
                label_key: "state.running",
                mark: StatusMark::Activity,
                motion: MotionCue::ActivityPulse,
            },
            Self::Waiting => StateStyle {
                color: ColorRole::TextSecondary,
                label_key: "state.waiting",
                mark: StatusMark::Waiting,
                motion: MotionCue::None,
            },
            Self::Blocked => StateStyle {
                color: ColorRole::Attention,
                label_key: "state.blocked",
                mark: StatusMark::Blocked,
                motion: MotionCue::None,
            },
            Self::Attention => StateStyle {
                color: ColorRole::Attention,
                label_key: "state.attention",
                mark: StatusMark::Alert,
                motion: MotionCue::None,
            },
            Self::Failed => StateStyle {
                color: ColorRole::Critical,
                label_key: "state.failed",
                mark: StatusMark::Failure,
                motion: MotionCue::None,
            },
            Self::Complete => StateStyle {
                color: ColorRole::Success,
                label_key: "state.complete",
                mark: StatusMark::Complete,
                motion: MotionCue::None,
            },
            Self::Offline => StateStyle {
                color: ColorRole::TextSecondary,
                label_key: "state.offline",
                mark: StatusMark::Offline,
                motion: MotionCue::None,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeColors {
    pub canvas: Rgb,
    pub surface: Rgb,
    pub elevated: Rgb,
    pub accent: Rgb,
    pub accent_highlight: Rgb,
    pub text_primary: Rgb,
    pub text_secondary: Rgb,
    pub success: Rgb,
    pub attention: Rgb,
    pub critical: Rgb,
    pub border: Rgb,
    pub grid: Rgb,
    pub pressed: Rgb,
    pub focus: Rgb,
    pub disabled_surface: Rgb,
    pub disabled_text: Rgb,
    pub high_contrast_text: Rgb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextPalette {
    pub default: Rgb,
    pub blue: Rgb,
    pub green: Rgb,
    pub orange: Rgb,
    pub purple: Rgb,
    pub pink: Rgb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub colors: ThemeColors,
    pub contexts: ContextPalette,
}

impl Theme {
    /// Visual Language v1. Raw channels are intentionally centralized here;
    /// production screen code requests roles instead of constructing colors.
    pub const SAAIOS_DARK: Self = Self {
        colors: ThemeColors {
            canvas: Rgb::from_hex(0x071011),
            surface: Rgb::from_hex(0x0D181A),
            elevated: Rgb::from_hex(0x142326),
            accent: Rgb::from_hex(0x63D4D6),
            accent_highlight: Rgb::from_hex(0xA1EEF0),
            text_primary: Rgb::from_hex(0xD7E2DF),
            text_secondary: Rgb::from_hex(0x829796),
            success: Rgb::from_hex(0x6FB79A),
            attention: Rgb::from_hex(0xD4B658),
            critical: Rgb::from_hex(0xC7514B),
            border: Rgb::from_hex(0x315054),
            grid: Rgb::from_hex(0x20383A),
            pressed: Rgb::from_hex(0xA1EEF0),
            focus: Rgb::from_hex(0xA1EEF0),
            disabled_surface: Rgb::from_hex(0x0D181A),
            disabled_text: Rgb::from_hex(0x829796),
            high_contrast_text: Rgb::from_hex(0xFFFFFF),
        },
        // These six HIA context identities preserve the physically accepted
        // pre-VUI choices while moving them behind a typed, non-status API.
        contexts: ContextPalette {
            default: Rgb::from_hex(0x63D4D6),
            blue: Rgb::from_hex(0x589CE8),
            green: Rgb::from_hex(0x78C878),
            orange: Rgb::from_hex(0xE6A050),
            purple: Rgb::from_hex(0xAA82DC),
            pink: Rgb::from_hex(0xE678A0),
        },
    };

    pub const fn color(self, role: ColorRole) -> Rgb {
        match role {
            ColorRole::Canvas => self.colors.canvas,
            ColorRole::Surface => self.colors.surface,
            ColorRole::Elevated => self.colors.elevated,
            ColorRole::Accent => self.colors.accent,
            ColorRole::AccentHighlight => self.colors.accent_highlight,
            ColorRole::TextPrimary => self.colors.text_primary,
            ColorRole::TextSecondary => self.colors.text_secondary,
            ColorRole::Success => self.colors.success,
            ColorRole::Attention => self.colors.attention,
            ColorRole::Critical => self.colors.critical,
            ColorRole::Border => self.colors.border,
            ColorRole::Grid => self.colors.grid,
            ColorRole::Pressed => self.colors.pressed,
            ColorRole::Focus => self.colors.focus,
            ColorRole::DisabledSurface => self.colors.disabled_surface,
            ColorRole::DisabledText => self.colors.disabled_text,
            ColorRole::HighContrastText => self.colors.high_contrast_text,
        }
    }

    pub const fn context_color(self, color: ContextColor) -> Rgb {
        match color {
            ContextColor::Default => self.contexts.default,
            ContextColor::Blue => self.contexts.blue,
            ContextColor::Green => self.contexts.green,
            ContextColor::Orange => self.contexts.orange,
            ContextColor::Purple => self.contexts.purple,
            ContextColor::Pink => self.contexts.pink,
        }
    }

    pub const fn state_style(self, state: UniversalState) -> StateStyle {
        state.style()
    }

    pub const fn state_color(self, state: UniversalState) -> Rgb {
        self.color(self.state_style(state).color)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Length {
    Fill,
    Px(u32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x as f64
            && y >= self.y as f64
            && x < self.x.saturating_add(self.width) as f64
            && y < self.y.saturating_add(self.height) as f64
    }
}

/// Padding for one `Node`, in the same physical-pixel space `Rect`/
/// `Length::Px` already use here -- deliberately not `foundations::
/// SafeInsets` (logical units), since this layout tree has no `SurfaceScale`
/// to convert with today (`layout()` takes none). Reconciling the two unit
/// domains is future work, likely alongside VUI-09's compiled shared layout
/// output; this stays self-contained and consistent with what every other
/// field in this tree already assumes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EdgeInsets {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    pub const fn all(value: u32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub const fn symmetric(horizontal: u32, vertical: u32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Leaf,
    Linear(Axis),
    Stack,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub width: Length,
    pub height: Length,
    pub padding: EdgeInsets,
    /// Tab/switch-navigation order (VUI-02's accessibility metadata task).
    /// `None` means this node is not a focus stop; two nodes may share an
    /// order value, in which case traversal order between them is
    /// unspecified -- callers that care assign distinct values.
    pub focus_order: Option<u32>,
    pub action: Option<String>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn leaf(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: NodeKind::Leaf,
            width: Length::Fill,
            height: Length::Fill,
            padding: EdgeInsets::ZERO,
            focus_order: None,
            action: None,
            children: Vec::new(),
        }
    }

    pub fn linear(id: impl Into<String>, axis: Axis, children: Vec<Node>) -> Self {
        Self {
            id: id.into(),
            kind: NodeKind::Linear(axis),
            width: Length::Fill,
            height: Length::Fill,
            padding: EdgeInsets::ZERO,
            focus_order: None,
            action: None,
            children,
        }
    }

    /// A fixed-thickness divider (`StrokeToken::Hairline`) sized to fill the
    /// cross axis of the container it will sit inside -- `axis` is that
    /// container's own axis (a divider between vertically stacked rows
    /// takes `Axis::Vertical`, matching the stack's own axis, not the line's
    /// visual direction).
    pub fn separator(id: impl Into<String>, axis: Axis) -> Self {
        let thickness = Length::Px(StrokeToken::Hairline.value().get() as u32);
        match axis {
            Axis::Vertical => Self::leaf(id).with_size(Length::Fill, thickness),
            Axis::Horizontal => Self::leaf(id).with_size(thickness, Length::Fill),
        }
    }

    pub fn with_size(mut self, width: Length, height: Length) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_focus_order(mut self, order: u32) -> Self {
        self.focus_order = Some(order);
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutNode {
    pub id: String,
    pub rect: Rect,
    pub focus_order: Option<u32>,
    pub action: Option<String>,
    pub children: Vec<LayoutNode>,
}

impl LayoutNode {
    /// Returns the deepest actionable node at this point. Reverse traversal
    /// gives the last child foreground priority for future stack layouts.
    pub fn hit_test(&self, x: f64, y: f64) -> Option<&LayoutNode> {
        if !self.rect.contains(x, y) {
            return None;
        }
        self.children
            .iter()
            .rev()
            .find_map(|child| child.hit_test(x, y))
            .or_else(|| self.action.as_ref().map(|_| self))
    }
}

pub fn layout(root: &Node, bounds: Rect) -> LayoutNode {
    layout_node(root, bounds)
}

fn layout_node(node: &Node, bounds: Rect) -> LayoutNode {
    let inner = inset_rect(bounds, node.padding);
    let child_rects = match node.kind {
        NodeKind::Leaf => Vec::new(),
        NodeKind::Stack => vec![inner; node.children.len()],
        NodeKind::Linear(axis) => linear_rects(&node.children, inner, axis),
    };
    LayoutNode {
        id: node.id.clone(),
        rect: bounds,
        focus_order: node.focus_order,
        action: node.action.clone(),
        children: node
            .children
            .iter()
            .zip(child_rects)
            .map(|(child, rect)| layout_node(child, rect))
            .collect(),
    }
}

fn inset_rect(bounds: Rect, padding: EdgeInsets) -> Rect {
    let horizontal = padding.left.saturating_add(padding.right);
    let vertical = padding.top.saturating_add(padding.bottom);
    Rect::new(
        bounds.x.saturating_add(padding.left),
        bounds.y.saturating_add(padding.top),
        bounds.width.saturating_sub(horizontal),
        bounds.height.saturating_sub(vertical),
    )
}

fn linear_rects(children: &[Node], bounds: Rect, axis: Axis) -> Vec<Rect> {
    let available = match axis {
        Axis::Horizontal => bounds.width,
        Axis::Vertical => bounds.height,
    };
    let fixed = children
        .iter()
        .map(|child| match main_length(child, axis) {
            Length::Px(value) => value,
            Length::Fill => 0,
        })
        .sum::<u32>()
        .min(available);
    let fill_count = children
        .iter()
        .filter(|child| main_length(child, axis) == Length::Fill)
        .count() as u32;
    let fill_space = available - fixed;
    let mut fill_seen = 0;
    let mut cursor = 0;

    children
        .iter()
        .map(|child| {
            let requested = match main_length(child, axis) {
                Length::Px(value) => value.min(available.saturating_sub(cursor)),
                Length::Fill => {
                    fill_seen += 1;
                    if fill_seen == fill_count {
                        fill_space.saturating_sub((fill_space / fill_count) * (fill_count - 1))
                    } else {
                        fill_space / fill_count
                    }
                }
            };
            let rect = match axis {
                Axis::Horizontal => Rect::new(
                    bounds.x + cursor,
                    bounds.y,
                    requested,
                    cross_size(child.height, bounds.height),
                ),
                Axis::Vertical => Rect::new(
                    bounds.x,
                    bounds.y + cursor,
                    cross_size(child.width, bounds.width),
                    requested,
                ),
            };
            cursor = cursor.saturating_add(requested);
            rect
        })
        .collect()
}

fn main_length(node: &Node, axis: Axis) -> Length {
    match axis {
        Axis::Horizontal => node.width,
        Axis::Vertical => node.height,
    }
}

fn cross_size(length: Length, available: u32) -> u32 {
    match length {
        Length::Fill => available,
        Length::Px(value) => value.min(available),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        layout, Axis, ColorRole, ContextColor, EdgeInsets, Length, MotionCue, Node, Rect, Rgb,
        StatusMark, Theme, UniversalState,
    };

    fn four_tabs() -> Node {
        Node::linear(
            "tabs",
            Axis::Horizontal,
            ["now", "inbox", "spaces", "me"]
                .into_iter()
                .map(|id| Node::leaf(id).with_action(format!("select_root:{id}")))
                .collect(),
        )
    }

    #[test]
    fn four_fill_tabs_cover_both_screen_edges() {
        let tree = layout(&four_tabs(), Rect::new(0, 2100, 1080, 300));
        assert_eq!(tree.hit_test(0.0, 2399.0).unwrap().id, "now");
        assert_eq!(tree.hit_test(1079.0, 2399.0).unwrap().id, "me");
        assert!(tree.hit_test(1080.0, 2399.0).is_none());
    }

    #[test]
    fn fixed_footer_leaves_remaining_space_for_content() {
        let root = Node::linear(
            "root",
            Axis::Vertical,
            vec![
                Node::leaf("content"),
                four_tabs().with_size(Length::Fill, Length::Px(300)),
            ],
        );
        let tree = layout(&root, Rect::new(0, 0, 1080, 2400));
        assert_eq!(tree.children[0].rect, Rect::new(0, 0, 1080, 2100));
        assert_eq!(tree.children[1].rect, Rect::new(0, 2100, 1080, 300));
    }

    #[test]
    fn padding_shrinks_only_the_children_not_the_node_own_rect() {
        let root = Node::linear("card", Axis::Vertical, vec![Node::leaf("body")])
            .with_padding(EdgeInsets::all(20));
        let tree = layout(&root, Rect::new(0, 0, 300, 300));
        assert_eq!(tree.rect, Rect::new(0, 0, 300, 300));
        assert_eq!(tree.children[0].rect, Rect::new(20, 20, 260, 260));
    }

    #[test]
    fn asymmetric_padding_offsets_each_edge_independently() {
        let root = Node::linear("card", Axis::Vertical, vec![Node::leaf("body")]).with_padding(
            EdgeInsets {
                top: 10,
                right: 20,
                bottom: 30,
                left: 40,
            },
        );
        let tree = layout(&root, Rect::new(0, 0, 300, 300));
        assert_eq!(tree.children[0].rect, Rect::new(40, 10, 240, 260));
    }

    #[test]
    fn padding_wider_than_bounds_never_underflows() {
        let root = Node::leaf("card").with_padding(EdgeInsets::all(1000));
        let tree = layout(&root, Rect::new(0, 0, 100, 100));
        assert_eq!(tree.rect, Rect::new(0, 0, 100, 100));
    }

    #[test]
    fn separator_is_a_hairline_across_the_cross_axis() {
        let column = Node::linear(
            "list",
            Axis::Vertical,
            vec![
                Node::leaf("row-a"),
                Node::separator("div", Axis::Vertical),
                Node::leaf("row-b"),
            ],
        );
        let tree = layout(&column, Rect::new(0, 0, 1080, 401));
        assert_eq!(tree.children[1].rect.width, 1080);
        assert_eq!(tree.children[1].rect.height, 1);

        let row = Node::linear(
            "toolbar",
            Axis::Horizontal,
            vec![
                Node::leaf("left"),
                Node::separator("div", Axis::Horizontal),
                Node::leaf("right"),
            ],
        );
        let tree = layout(&row, Rect::new(0, 0, 401, 100));
        assert_eq!(tree.children[1].rect.width, 1);
        assert_eq!(tree.children[1].rect.height, 100);
    }

    #[test]
    fn focus_order_defaults_to_none_and_survives_layout() {
        let plain = Node::leaf("a");
        assert_eq!(plain.focus_order, None);

        let root = Node::linear(
            "form",
            Axis::Vertical,
            vec![
                Node::leaf("first").with_focus_order(0),
                Node::leaf("second").with_focus_order(1),
            ],
        );
        let tree = layout(&root, Rect::new(0, 0, 100, 100));
        assert_eq!(tree.children[0].focus_order, Some(0));
        assert_eq!(tree.children[1].focus_order, Some(1));
    }

    #[test]
    fn visual_v1_core_palette_matches_the_contract() {
        let theme = Theme::SAAIOS_DARK;
        assert_eq!(theme.color(ColorRole::Canvas), Rgb::from_hex(0x071011));
        assert_eq!(theme.color(ColorRole::Surface), Rgb::from_hex(0x0D181A));
        assert_eq!(theme.color(ColorRole::Elevated), Rgb::from_hex(0x142326));
        assert_eq!(theme.color(ColorRole::Accent), Rgb::from_hex(0x63D4D6));
        assert_eq!(theme.color(ColorRole::TextPrimary), Rgb::from_hex(0xD7E2DF));
        assert_eq!(theme.color(ColorRole::Success), Rgb::from_hex(0x6FB79A));
        assert_eq!(theme.color(ColorRole::Attention), Rgb::from_hex(0xD4B658));
        assert_eq!(theme.color(ColorRole::Critical), Rgb::from_hex(0xC7514B));
        assert_eq!(theme.color(ColorRole::Border), Rgb::from_hex(0x315054));
        assert_eq!(theme.color(ColorRole::Grid), Rgb::from_hex(0x20383A));
    }

    #[test]
    fn context_colors_are_not_semantic_status_roles() {
        let theme = Theme::SAAIOS_DARK;
        assert_eq!(
            theme.context_color(ContextColor::Default),
            theme.color(ColorRole::Accent)
        );
        assert_eq!(
            theme.context_color(ContextColor::Blue),
            Rgb::from_hex(0x589CE8)
        );
        assert_ne!(
            theme.context_color(ContextColor::Orange),
            theme.state_color(UniversalState::Attention)
        );
    }

    #[test]
    fn universal_states_have_non_color_cues_and_stable_keys() {
        let running = UniversalState::Running.style();
        assert_eq!(running.color, ColorRole::Accent);
        assert_eq!(running.label_key, "state.running");
        assert_eq!(running.mark, StatusMark::Activity);
        assert_eq!(running.motion, MotionCue::ActivityPulse);

        let failed = UniversalState::Failed.style();
        assert_eq!(failed.color, ColorRole::Critical);
        assert_eq!(failed.mark, StatusMark::Failure);
        assert_eq!(failed.motion, MotionCue::None);

        assert_eq!(UniversalState::Blocked.style().color, ColorRole::Attention);
        assert_eq!(UniversalState::Complete.style().color, ColorRole::Success);
        assert_eq!(UniversalState::Offline.style().mark, StatusMark::Offline);
    }
}
