//! ADR-184: layout and hit-test from compiled `.sui` v1 `ScreenSpec`.
//! ADR-194: `layout_v2()` matches those tab hits for public NOW.
//! ADR-199: nested `row` ids dock as the live NOW footer.
//! ADR-200: `ObjectSummary` docks as the live NOW object hit.
//!
//! Production chrome uses `layout_v2()` (ADR-216). Nested `tab` ids
//! under `BottomNavigation` own the v2 strip (ADR-196). Footer hits
//! come from named `row`s, not from v1 `content_actions`. Object hits
//! come from `ObjectSummary` (ADR-200).
//! ADR-202: `EventRow` docks as live Inbox stacked rows.
//! ADR-203: `SpaceRow` docks as live Spaces stacked rows.
//! ADR-204: `SettingRow` docks as live Me stacked rows.
//! ADR-205: `WifiRow` docks as live Wi-Fi list stacked rows.
//! ADR-206: `BluetoothRow` docks as live Bluetooth list stacked rows.
//! ADR-207: `TrustedClientRow` docks as live trusted-client stacked rows.
//! ADR-208: `CapabilityRow` docks as live Me app stacked rows.
//! ADR-214: nested `row refresh`/`scan`/`back` dock as live list
//! trailing controls.
//! ADR-215: Me `SystemSection`/`SettingRow` flatten clips like
//! `scrolled_row_rect` at offset 0.

use saai_ui_core::{
    layout, Axis, EdgeInsets, LayoutNode, Length, Node, Rect, SafeInsets, SpacingToken,
    SurfaceScale, TextRole, MIN_TOUCH_TARGET,
};

use crate::{ScreenSpec, SuiV2Screen};

/// Design-canvas tab height scaled to the panel, never below
/// `MIN_TOUCH_TARGET`. Landscape 2400×1080 would otherwise shrink
/// the strip under 48 logical units.
pub fn v1_tab_strip_height(panel_height: u32, tab_height_2400: u32) -> u32 {
    let scaled = ((panel_height as u64 * tab_height_2400 as u64) / 2400) as u32;
    scaled.max(SurfaceScale::PIXEL_7.logical_to_physical(MIN_TOUCH_TARGET))
}

pub fn v1_root_node(spec: &ScreenSpec, width: u32, height: u32) -> Node {
    let tab_height = v1_tab_strip_height(height, spec.tab_height);
    let tabs = Node::linear(
        spec.tabs_id.clone(),
        Axis::Horizontal,
        spec.tabs
            .iter()
            .map(|tab| Node::leaf(tab.id.clone()).with_action(tab.action.clone()))
            .collect(),
    )
    .with_size(Length::Fill, Length::Px(tab_height));
    Node::linear(
        spec.id.clone(),
        Axis::Vertical,
        vec![v1_content_node(spec, width, height), tabs],
    )
}

pub fn layout_v1_root(spec: &ScreenSpec, width: u32, height: u32) -> LayoutNode {
    layout(
        &v1_root_node(spec, width, height),
        Rect::new(0, 0, width, height),
    )
}

pub fn layout_v1_find<'a>(node: &'a LayoutNode, id: &str) -> Option<&'a LayoutNode> {
    if node.id == id {
        Some(node)
    } else {
        node.children
            .iter()
            .find_map(|child| layout_v1_find(child, id))
    }
}

fn v1_content_node(spec: &ScreenSpec, width: u32, height: u32) -> Node {
    let margin = width / 22;
    let mut children = Vec::new();
    let mut cursor = 0u32;
    let mut actions = spec.content_actions.clone();
    actions.sort_by_key(|action| action.top);
    for action in &actions {
        let top = ((action.top as u64 * height as u64) / 2400) as u32;
        let action_height = ((action.height as u64 * height as u64) / 2400) as u32;
        if top > cursor {
            children.push(
                Node::leaf(format!("{}-gap-{cursor}", spec.content_id))
                    .with_size(Length::Fill, Length::Px(top - cursor)),
            );
        }
        children.push(
            Node::leaf(action.id.clone())
                .with_action(action.action.clone())
                .with_size(Length::Fill, Length::Px(action_height)),
        );
        cursor = cursor.max(top.saturating_add(action_height));
    }
    children.push(
        Node::leaf(format!("{}-fill", spec.content_id)).with_size(Length::Fill, Length::Fill),
    );
    Node::linear(spec.content_id.clone(), Axis::Vertical, children).with_padding(EdgeInsets {
        top: 0,
        right: margin,
        bottom: 0,
        left: margin,
    })
}

fn v2_named_tabs(screen: &SuiV2Screen) -> &[crate::SuiV2Tab] {
    screen
        .components
        .iter()
        .find(|component| component.type_name == "BottomNavigation")
        .map(|component| component.tabs.as_slice())
        .unwrap_or(&[])
}

/// Live `now_footer_action_rect` row height at the design canvas.
const V2_FOOTER_ROW_HEIGHT_2400: u32 = 160;

fn v2_footer_row_height(panel_height: u32) -> u32 {
    ((panel_height as u64 * u64::from(V2_FOOTER_ROW_HEIGHT_2400)) / 2400) as u32
}

/// Live `draw_now` starts identity 150 design-canvas units below y=0
/// so the status layer does not cover the heading.
const V2_NOW_TOP_INSET_2400: u32 = 150;

fn v2_physical(value: saai_ui_core::LogicalUnit) -> u32 {
    SurfaceScale::PIXEL_7.logical_to_physical(value)
}

fn v2_line_height(role: TextRole) -> u32 {
    v2_physical(role.style().line_height)
}

fn v2_now_header_height(content_height: u32) -> u32 {
    let top_inset =
        ((u64::from(V2_NOW_TOP_INSET_2400) * u64::from(content_height.max(1))) / 2400) as u32;
    top_inset + v2_line_height(TextRole::Title) + v2_physical(SpacingToken::Medium.value())
}

fn v2_now_object_height() -> u32 {
    (v2_line_height(TextRole::Body) + v2_line_height(TextRole::Caption))
        .max(v2_physical(MIN_TOUCH_TARGET))
}

const V2_STACKED_ROW_TOP_2400: u32 = 430;
const V2_STACKED_ROW_PITCH_2400: u32 = 220;
const V2_STACKED_ROW_HEIGHT_2400: u32 = 190;

fn v2_stacked_row_top(index: usize, panel_height: u32) -> u32 {
    let top_2400 = V2_STACKED_ROW_TOP_2400 + index as u32 * V2_STACKED_ROW_PITCH_2400;
    ((u64::from(top_2400) * u64::from(panel_height)) / 2400) as u32
}

fn v2_stacked_row_height(panel_height: u32) -> u32 {
    ((u64::from(V2_STACKED_ROW_HEIGHT_2400) * u64::from(panel_height)) / 2400) as u32
}

fn v2_stacked_row_pitch(panel_height: u32) -> u32 {
    ((u64::from(V2_STACKED_ROW_PITCH_2400) * u64::from(panel_height)) / 2400) as u32
}

fn v2_is_footer_row(id: &str) -> bool {
    matches!(id, "apps" | "intent")
}

fn v2_is_trailing_row(id: &str) -> bool {
    matches!(id, "refresh" | "scan" | "back")
}

/// Live `me_action_at` uses `scrolled_row_rect` at offset 0: a stacked
/// row that does not sit wholly inside the content pane (above tabs)
/// is neither drawn nor hittable. Inbox/Spaces/lists keep unclipped
/// `stacked_row_rect`.
fn v2_me_scroll_clip(screen: &SuiV2Screen) -> bool {
    !v2_named_tabs(screen).is_empty()
        && screen.components.iter().any(|component| {
            matches!(
                component.type_name.as_str(),
                "SystemSection" | "SettingRow" | "DataRow" | "CapabilityRow"
            )
        })
}

fn v2_stacked_fits_content(index: usize, panel_height: u32, content_height: u32) -> bool {
    let top = v2_stacked_row_top(index, panel_height);
    let bottom = top.saturating_add(v2_stacked_row_height(panel_height));
    top < content_height && bottom <= content_height
}

/// Live `stacked_control_rect`: if the stacked slot would paint below
/// the panel, dock to the last on-screen row.
fn v2_control_top(index: usize, panel_height: u32) -> u32 {
    let desired = v2_stacked_row_top(index, panel_height);
    let row_height = v2_stacked_row_height(panel_height);
    if desired.saturating_add(row_height) <= panel_height {
        desired
    } else {
        panel_height.saturating_sub(row_height)
    }
}

/// Live `stacked_trailing_rect`.
fn v2_trailing_top(index: usize, last_index: usize, panel_height: u32) -> u32 {
    let back_top = v2_control_top(last_index, panel_height);
    let steps = last_index.saturating_sub(index) as u32;
    let y = back_top.saturating_sub(steps.saturating_mul(v2_stacked_row_pitch(panel_height)));
    y.max(v2_stacked_row_top(0, panel_height))
}

fn v2_content_node(screen: &SuiV2Screen, width: u32, height: u32, content_height: u32) -> Node {
    let margin = width / 22;
    let mut header = None;
    let mut object = None;
    let mut stacked = Vec::new();
    let mut rest = Vec::new();
    for component in screen
        .components
        .iter()
        .filter(|component| component.type_name != "BottomNavigation")
    {
        match component.type_name.as_str() {
            "ContextHeader" if header.is_none() => header = Some(component),
            "ObjectSummary" if object.is_none() => object = Some(component),
            "EventRow" | "SpaceRow" | "SettingRow" | "DataRow" | "SystemSection" | "WifiRow"
            | "BluetoothRow" | "TrustedClientRow" | "CapabilityRow" => stacked.push(component),
            _ => rest.push(component),
        }
    }
    let mut children = Vec::new();
    let mut cursor = 0u32;
    if header.is_some() {
        let header_height = if object.is_some() {
            v2_now_header_height(content_height)
        } else if !stacked.is_empty() {
            v2_stacked_row_top(0, height)
        } else {
            0
        };
        let mut node = Node::leaf("ContextHeader");
        if header_height > 0 {
            node = node.with_size(Length::Fill, Length::Px(header_height));
            cursor = header_height;
        }
        children.push(node);
    }
    if object.is_some() {
        let object_height = v2_now_object_height();
        children.push(
            Node::leaf("ObjectSummary")
                .with_action("open_object")
                .with_size(Length::Fill, Length::Px(object_height)),
        );
        cursor = cursor.saturating_add(object_height);
    }
    let trailing: Vec<&crate::SuiV2Row> = screen
        .rows
        .iter()
        .filter(|row| v2_is_trailing_row(&row.id))
        .collect();
    let footer: Vec<&crate::SuiV2Row> = screen
        .rows
        .iter()
        .filter(|row| v2_is_footer_row(&row.id))
        .collect();
    let trailing_first_top = if trailing.is_empty() {
        None
    } else {
        let last = stacked.len() + trailing.len() - 1;
        Some(v2_trailing_top(stacked.len(), last, height))
    };
    let stacked_height = v2_stacked_row_height(height);
    let clip_to_content = v2_me_scroll_clip(screen);
    for (index, row) in stacked.iter().enumerate() {
        let top = v2_stacked_row_top(index, height);
        if trailing_first_top.is_some_and(|first| top.saturating_add(stacked_height) > first) {
            continue;
        }
        if clip_to_content && !v2_stacked_fits_content(index, height, content_height) {
            continue;
        }
        if top > cursor {
            children.push(
                Node::leaf(format!("{}-gap-{cursor}", screen.id))
                    .with_size(Length::Fill, Length::Px(top - cursor)),
            );
            cursor = top;
        }
        let id = row
            .props
            .loc
            .clone()
            .unwrap_or_else(|| format!("{}-{index}", row.type_name));
        let mut node = Node::leaf(id).with_size(Length::Fill, Length::Px(stacked_height));
        if !matches!(row.type_name.as_str(), "SystemSection" | "CapabilityRow")
            && row.props.a11y.as_deref() == Some("Button")
        {
            let loc = row.props.loc.as_deref();
            let action = match row.type_name.as_str() {
                "SpaceRow" => format!("select_space:{}", loc.unwrap_or("SpaceRow")),
                "SettingRow" | "DataRow" => loc.unwrap_or("SettingRow").to_string(),
                "WifiRow" => "connect_wifi".to_string(),
                "BluetoothRow" => "pair_bluetooth".to_string(),
                "TrustedClientRow" => "revoke_trusted_client".to_string(),
                _ => "open_object".to_string(),
            };
            node = node.with_action(action);
        }
        children.push(node);
        cursor = cursor.max(top.saturating_add(stacked_height));
    }
    if !trailing.is_empty() {
        let last = stacked.len() + trailing.len() - 1;
        for (offset, row) in trailing.iter().enumerate() {
            let index = stacked.len() + offset;
            let top = v2_trailing_top(index, last, height);
            if top > cursor {
                children.push(
                    Node::leaf(format!("{}-trail-gap-{cursor}", screen.id))
                        .with_size(Length::Fill, Length::Px(top - cursor)),
                );
                cursor = top;
            }
            children.push(
                Node::leaf(row.id.clone())
                    .with_action(row.action.clone())
                    .with_size(Length::Fill, Length::Px(stacked_height)),
            );
            cursor = cursor.max(top.saturating_add(stacked_height));
        }
    }
    for component in rest {
        children.push(Node::leaf(component.type_name.clone()));
    }
    if children.is_empty() {
        children
            .push(Node::leaf(format!("{}-fill", screen.id)).with_size(Length::Fill, Length::Fill));
    }
    let row_height = v2_footer_row_height(height);
    for row in footer {
        children.push(
            Node::leaf(row.id.clone())
                .with_action(row.action.clone())
                .with_size(Length::Fill, Length::Px(row_height)),
        );
    }
    Node::linear(format!("{}-content", screen.id), Axis::Vertical, children).with_padding(
        EdgeInsets {
            top: 0,
            right: margin,
            bottom: 0,
            left: margin,
        },
    )
}

/// Public NOW chrome: stacked header/object, Fill leftovers, named
/// footer rows, plus a tab strip from nested `tab` ids. Empty
/// `ObjectSummary`, `row`, and `BottomNavigation` invent no live hits.
pub fn v2_root_node(screen: &SuiV2Screen, width: u32, height: u32) -> Node {
    let tabs = v2_named_tabs(screen);
    let tab_height_2400 =
        EdgeInsets::from_safe(SafeInsets::PIXEL_7_PORTRAIT, SurfaceScale::PIXEL_7).bottom;
    let tab_height = if tabs.is_empty() {
        0
    } else {
        v1_tab_strip_height(height, tab_height_2400)
    };
    let content = v2_content_node(screen, width, height, height.saturating_sub(tab_height));
    if tabs.is_empty() {
        return content;
    }
    let strip = Node::linear(
        "BottomNavigation".to_string(),
        Axis::Horizontal,
        tabs.iter()
            .map(|tab| Node::leaf(tab.id.clone()).with_action(format!("select_root:{}", tab.id)))
            .collect(),
    )
    .with_size(Length::Fill, Length::Px(tab_height));
    Node::linear(screen.id.clone(), Axis::Vertical, vec![content, strip])
}

pub fn layout_v2(screen: &SuiV2Screen, width: u32, height: u32) -> LayoutNode {
    layout(
        &v2_root_node(screen, width, height),
        Rect::new(0, 0, width, height),
    )
}

#[cfg(test)]
mod tests {
    use super::{layout_v1_find, layout_v1_root, layout_v2};
    use crate::{compile_v1_rollback, compile_v2, compile_v2_public};

    #[test]
    fn v1_rollback_layout_maps_the_four_tabs() {
        let spec = compile_v1_rollback().expect("root.sui v1");
        let tree = layout_v1_root(&spec, 1080, 2400);
        assert_eq!(
            tree.hit_test(135.0, 2250.0).map(|node| node.id.as_str()),
            Some("now")
        );
        assert_eq!(
            tree.hit_test(405.0, 2250.0).map(|node| node.id.as_str()),
            Some("inbox")
        );
        assert_eq!(
            tree.hit_test(675.0, 2250.0).map(|node| node.id.as_str()),
            Some("spaces")
        );
        assert_eq!(
            tree.hit_test(945.0, 2250.0).map(|node| node.id.as_str()),
            Some("me")
        );
        assert!(tree.hit_test(540.0, 1200.0).is_none());
    }

    #[test]
    fn v1_rollback_layout_has_no_leftover_now_actions() {
        let spec = compile_v1_rollback().expect("root.sui v1");
        let tree = layout_v1_root(&spec, 1080, 2400);
        assert!(tree.hit_test(540.0, 800.0).is_none());
        assert!(tree.hit_test(540.0, 1000.0).is_none());
        assert!(layout_v1_find(&tree, "selected-entity").is_none());
        assert!(layout_v1_find(&tree, "new-intent").is_none());
        assert_eq!(tree.children[0].id, "content");
        assert_eq!(tree.children[1].id, "root-tabs");
        assert_eq!(
            tree.hit_test(135.0, 2250.0).map(|node| node.id.as_str()),
            Some("now")
        );
    }

    #[test]
    fn shell_root_view_uses_v2_layout() {
        let main = include_str!("../../../services/saai-shell/src/main.rs");
        assert!(main.contains("saai_ui_compiler::layout_v2"));
        assert!(main.contains("saai_ui_compiler::compile_v2"));
        assert!(!main.contains("layout_v1_root("));
        let build = include_str!("../../../services/saai-shell/build.rs");
        assert!(build.contains("saai_ui_compiler::compile_v2"));
        assert!(!build.contains("saai_ui_compiler::compile("));
    }

    #[test]
    fn production_root_v2_tab_hits_match_v1_rollback() {
        let spec = compile_v1_rollback().expect("frozen v1");
        let v1 = layout_v1_root(&spec, 1080, 2400);
        let source = include_str!("../../../services/saai-shell/ui/root.sui");
        let screen = compile_v2(source).expect("production root v2");
        let v2 = layout_v2(&screen, 1080, 2400);
        for x in [135.0, 405.0, 675.0, 945.0] {
            assert_eq!(
                v1.hit_test(x, 2250.0).map(|node| node.id.as_str()),
                v2.hit_test(x, 2250.0).map(|node| node.id.as_str())
            );
        }
        assert!(v1.hit_test(540.0, 1200.0).is_none());
        assert!(v2.hit_test(540.0, 1200.0).is_none());
    }

    #[test]
    fn layout_v2_public_now_matches_v1_tab_hits() {
        let spec = compile_v1_rollback().expect("root.sui v1");
        let v1 = layout_v1_root(&spec, 1080, 2400);
        let source = include_str!("../../../docs/os/ui/examples/now-public.sui");
        let screen = compile_v2_public(source).expect("public NOW");
        let v2 = layout_v2(&screen, 1080, 2400);
        for (x, id) in [
            (135.0, "now"),
            (405.0, "inbox"),
            (675.0, "spaces"),
            (945.0, "me"),
        ] {
            assert_eq!(
                v1.hit_test(x, 2250.0).map(|node| node.id.as_str()),
                Some(id)
            );
            assert_eq!(
                v2.hit_test(x, 2250.0).map(|node| node.id.as_str()),
                v1.hit_test(x, 2250.0).map(|node| node.id.as_str())
            );
        }
        assert!(v1.hit_test(540.0, 1200.0).is_none());
        assert!(v2.hit_test(540.0, 1200.0).is_none());
        assert!(layout_v1_find(&v2, "ContextHeader").is_some());
        assert!(layout_v1_find(&v2, "ObjectSummary").is_some());
        assert!(layout_v1_find(&v2, "SurfacePattern").is_some());
        let header = layout_v1_find(&v2, "ContextHeader").expect("header");
        assert_eq!(header.rect.y, 0);
        let edges = saai_ui_core::EdgeInsets::from_safe(
            saai_ui_core::SafeInsets::PIXEL_7_PORTRAIT,
            saai_ui_core::SurfaceScale::PIXEL_7,
        );
        assert_eq!(spec.tab_height, edges.bottom);
        assert_ne!(header.rect.y, edges.top);
        assert_eq!(
            layout_v1_find(&v2, "BottomNavigation").map(|node| node.children.len()),
            Some(4)
        );
        assert_eq!(
            v2.hit_test(540.0, 1860.0)
                .and_then(|node| node.action.as_deref()),
            Some("open_apps")
        );
        assert_eq!(
            v2.hit_test(540.0, 2080.0)
                .and_then(|node| node.action.as_deref()),
            Some("open_intent_input")
        );
        assert!(v1.hit_test(540.0, 1860.0).is_none());
        assert!(v1.hit_test(540.0, 2080.0).is_none());
        assert_eq!(
            v2.hit_test(540.0, 335.0)
                .and_then(|node| node.action.as_deref()),
            Some("open_object")
        );
        assert!(v2.hit_test(540.0, 250.0).is_none());
        assert!(v1.hit_test(540.0, 335.0).is_none());
        let object = layout_v1_find(&v2, "ObjectSummary").expect("object");
        assert_eq!(object.rect.y, 263);
        assert_eq!(object.rect.height, 144);
        assert!(v2.hit_test(540.0, 525.0).is_none());
    }

    #[test]
    fn layout_v2_public_inbox_event_row_matches_stacked_row() {
        let source = include_str!("../../../docs/os/ui/examples/inbox-public.sui");
        let screen = compile_v2_public(source).expect("public Inbox");
        let v2 = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            v2.hit_test(540.0, 525.0)
                .and_then(|node| node.action.as_deref()),
            Some("open_object")
        );
        assert_eq!(
            v2.hit_test(540.0, 525.0).map(|node| node.id.as_str()),
            Some("inbox.item")
        );
        assert!(v2.hit_test(540.0, 250.0).is_none());
        assert_eq!(
            v2.hit_test(405.0, 2250.0).map(|node| node.id.as_str()),
            Some("inbox")
        );
        let quiet = compile_v2(
            r#"
            sui 2
            screen inbox {
              component ContextHeader {}
              component EventRow {
                a11y = Status
              }
              component BottomNavigation {
                tab now {}
                tab inbox {}
                tab spaces {}
                tab me {}
              }
            }
            "#,
        )
        .expect("quiet row");
        let tree = layout_v2(&quiet, 1080, 2400);
        assert!(tree.hit_test(540.0, 525.0).is_none());
        assert_eq!(
            tree.hit_test(405.0, 2250.0).map(|node| node.id.as_str()),
            Some("inbox")
        );
    }

    #[test]
    fn layout_v2_public_spaces_row_matches_stacked_row() {
        let source = include_str!("../../../docs/os/ui/examples/spaces-public.sui");
        let screen = compile_v2_public(source).expect("public Spaces");
        let v2 = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            v2.hit_test(540.0, 525.0)
                .and_then(|node| node.action.as_deref()),
            Some("select_space:spaces.item")
        );
        assert_eq!(
            v2.hit_test(540.0, 525.0).map(|node| node.id.as_str()),
            Some("spaces.item")
        );
        assert!(v2.hit_test(540.0, 250.0).is_none());
        assert_eq!(
            v2.hit_test(675.0, 2250.0).map(|node| node.id.as_str()),
            Some("spaces")
        );
        let quiet = compile_v2(
            r#"
            sui 2
            screen spaces {
              component ContextHeader {}
              component SpaceRow {
                a11y = Status
              }
              component BottomNavigation {
                tab now {}
                tab inbox {}
                tab spaces {}
                tab me {}
              }
            }
            "#,
        )
        .expect("quiet row");
        let tree = layout_v2(&quiet, 1080, 2400);
        assert!(tree.hit_test(540.0, 525.0).is_none());
        assert_eq!(
            tree.hit_test(675.0, 2250.0).map(|node| node.id.as_str()),
            Some("spaces")
        );
    }

    #[test]
    fn layout_v2_public_me_row_matches_stacked_row() {
        let source = include_str!("../../../docs/os/ui/examples/me-public.sui");
        let screen = compile_v2_public(source).expect("public Me");
        let v2 = layout_v2(&screen, 1080, 2400);
        assert!(v2.hit_test(540.0, 525.0).is_none());
        assert!(v2.hit_test(540.0, 745.0).is_none());
        assert_eq!(
            v2.hit_test(540.0, 965.0)
                .and_then(|node| node.action.as_deref()),
            Some("tap_build_info")
        );
        assert!(v2.hit_test(540.0, 1185.0).is_none());
        assert_eq!(
            v2.hit_test(540.0, 1405.0)
                .and_then(|node| node.action.as_deref()),
            Some("cycle_timezone")
        );
        assert_eq!(
            v2.hit_test(540.0, 1405.0).map(|node| node.id.as_str()),
            Some("cycle_timezone")
        );
        assert!(v2.hit_test(540.0, 250.0).is_none());
        assert_eq!(
            v2.hit_test(945.0, 2250.0).map(|node| node.id.as_str()),
            Some("me")
        );
        let quiet = compile_v2(
            r#"
            sui 2
            screen me {
              component ContextHeader {}
              component SystemSection {
                a11y = Heading
              }
              component SettingRow {
                a11y = Status
              }
              component BottomNavigation {
                tab now {}
                tab inbox {}
                tab spaces {}
                tab me {}
              }
            }
            "#,
        )
        .expect("quiet row");
        let tree = layout_v2(&quiet, 1080, 2400);
        assert!(tree.hit_test(540.0, 525.0).is_none());
        assert!(tree.hit_test(540.0, 745.0).is_none());
        assert_eq!(
            tree.hit_test(945.0, 2250.0).map(|node| node.id.as_str()),
            Some("me")
        );
    }

    #[test]
    fn layout_v2_me_rows_clip_to_content_at_zero_scroll() {
        let mut source = String::from("sui 2\nscreen me {\n  component ContextHeader {}\n");
        for index in 0..8 {
            source.push_str(&format!(
                "  component SettingRow {{ a11y = Button loc = row{index} }}\n"
            ));
        }
        source.push_str(
            "  component BottomNavigation {\n    tab now {}\n    tab inbox {}\n    tab spaces {}\n    tab me {}\n  }\n}\n",
        );
        let screen = compile_v2(&source).expect("clipped Me");
        let v2 = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            v2.hit_test(540.0, 525.0)
                .and_then(|node| node.action.as_deref()),
            Some("row0")
        );
        assert_eq!(
            v2.hit_test(540.0, 1845.0)
                .and_then(|node| node.action.as_deref()),
            Some("row6")
        );
        assert!(v2.hit_test(540.0, 2065.0).is_none());
        assert_eq!(
            v2.hit_test(945.0, 2250.0).map(|node| node.id.as_str()),
            Some("me")
        );
    }

    #[test]
    fn layout_v2_public_wifi_row_matches_stacked_row() {
        let source = include_str!("../../../docs/os/ui/examples/wifi-public.sui");
        let screen = compile_v2_public(source).expect("public Wi-Fi");
        let v2 = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            v2.hit_test(540.0, 525.0)
                .and_then(|node| node.action.as_deref()),
            Some("connect_wifi")
        );
        assert_eq!(
            v2.hit_test(540.0, 525.0).map(|node| node.id.as_str()),
            Some("wifi.item")
        );
        assert!(v2.hit_test(540.0, 250.0).is_none());
        assert!(v2.hit_test(135.0, 2250.0).is_none());
        assert_eq!(
            v2.hit_test(540.0, 745.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_refresh")
        );
        assert_eq!(
            v2.hit_test(540.0, 965.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_back")
        );
        let quiet = compile_v2(
            r#"
            sui 2
            screen wifi {
              component ContextHeader {}
              component WifiRow {
                a11y = Status
              }
            }
            "#,
        )
        .expect("quiet row");
        let tree = layout_v2(&quiet, 1080, 2400);
        assert!(tree.hit_test(540.0, 525.0).is_none());
        assert!(tree.hit_test(540.0, 745.0).is_none());
        assert!(tree.hit_test(135.0, 2250.0).is_none());
    }

    #[test]
    fn layout_v2_trailing_back_docks_when_rows_overflow() {
        let mut source = String::from("sui 2\nscreen wifi {\n  component ContextHeader {}\n");
        for _ in 0..9 {
            source.push_str("  component WifiRow { a11y = Button }\n");
        }
        source.push_str("  row refresh {}\n  row back {}\n}\n");
        let screen = compile_v2(&source).expect("overflow Wi-Fi");
        let v2 = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            v2.hit_test(540.0, 2305.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_back")
        );
        assert_eq!(
            v2.hit_test(540.0, 2085.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_refresh")
        );
        assert_eq!(
            v2.hit_test(540.0, 525.0)
                .and_then(|node| node.action.as_deref()),
            Some("connect_wifi")
        );
    }

    #[test]
    fn layout_v2_public_bluetooth_row_matches_stacked_row() {
        let source = include_str!("../../../docs/os/ui/examples/bluetooth-public.sui");
        let screen = compile_v2_public(source).expect("public Bluetooth");
        let v2 = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            v2.hit_test(540.0, 525.0)
                .and_then(|node| node.action.as_deref()),
            Some("pair_bluetooth")
        );
        assert_eq!(
            v2.hit_test(540.0, 525.0).map(|node| node.id.as_str()),
            Some("bluetooth.item")
        );
        assert!(v2.hit_test(540.0, 250.0).is_none());
        assert!(v2.hit_test(135.0, 2250.0).is_none());
        assert_eq!(
            v2.hit_test(540.0, 745.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_scan")
        );
        assert_eq!(
            v2.hit_test(540.0, 965.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_refresh")
        );
        assert_eq!(
            v2.hit_test(540.0, 1185.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_back")
        );
        let quiet = compile_v2(
            r#"
            sui 2
            screen bluetooth {
              component ContextHeader {}
              component BluetoothRow {
                a11y = Status
              }
            }
            "#,
        )
        .expect("quiet row");
        let tree = layout_v2(&quiet, 1080, 2400);
        assert!(tree.hit_test(540.0, 525.0).is_none());
        assert!(tree.hit_test(135.0, 2250.0).is_none());
    }

    #[test]
    fn layout_v2_privileged_trusted_row_matches_stacked_row() {
        let source = include_str!("../../../docs/os/ui/examples/trusted-privileged.sui");
        assert!(compile_v2_public(source)
            .unwrap_err()
            .to_string()
            .contains("privileged SUI v2 component `TrustedClientRow`"));
        let screen = compile_v2(source).expect("privileged trusted");
        assert!(screen.is_privileged());
        let v2 = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            v2.hit_test(540.0, 525.0)
                .and_then(|node| node.action.as_deref()),
            Some("revoke_trusted_client")
        );
        assert_eq!(
            v2.hit_test(540.0, 525.0).map(|node| node.id.as_str()),
            Some("trusted.item")
        );
        assert!(v2.hit_test(540.0, 250.0).is_none());
        assert!(v2.hit_test(135.0, 2250.0).is_none());
        assert_eq!(
            v2.hit_test(540.0, 745.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_back")
        );
        let quiet = compile_v2(
            r#"
            sui 2
            screen trusted {
              component ContextHeader {}
              component TrustedClientRow {
                a11y = Status
              }
            }
            "#,
        )
        .expect("quiet row");
        let tree = layout_v2(&quiet, 1080, 2400);
        assert!(tree.hit_test(540.0, 525.0).is_none());
        assert!(tree.hit_test(135.0, 2250.0).is_none());
    }

    #[test]
    fn layout_v2_privileged_capability_row_matches_stacked_row() {
        let source = include_str!("../../../docs/os/ui/examples/capability-privileged.sui");
        assert!(compile_v2_public(source)
            .unwrap_err()
            .to_string()
            .contains("privileged SUI v2 component `CapabilityRow`"));
        let screen = compile_v2(source).expect("privileged capability");
        assert!(screen.is_privileged());
        let v2 = layout_v2(&screen, 1080, 2400);
        let row = layout_v1_find(&v2, "me.app").expect("capability row");
        assert_eq!(row.rect.y, 430);
        assert_eq!(row.rect.height, 190);
        assert!(row.action.is_none());
        assert!(v2.hit_test(540.0, 525.0).is_none());
        assert!(v2.hit_test(540.0, 250.0).is_none());
        assert!(v2.hit_test(135.0, 2250.0).is_none());
        let forced = compile_v2(
            r#"
            sui 2
            screen me {
              component ContextHeader {}
              component CapabilityRow {
                a11y = Button
                loc = me.app
              }
            }
            "#,
        )
        .expect("button still inert");
        let tree = layout_v2(&forced, 1080, 2400);
        assert!(tree.hit_test(540.0, 525.0).is_none());
        assert!(layout_v1_find(&tree, "me.app")
            .expect("forced row")
            .action
            .is_none());
    }

    #[test]
    fn layout_v2_without_object_does_not_invent_object_hits() {
        let screen = compile_v2(
            r#"
            sui 2
            screen now {
              component ContextHeader {}
              component SurfacePattern {}
              component BottomNavigation {
                tab now {}
                tab inbox {}
                tab spaces {}
                tab me {}
              }
            }
            "#,
        )
        .expect("no object");
        let tree = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            tree.hit_test(135.0, 2250.0).map(|node| node.id.as_str()),
            Some("now")
        );
        assert!(tree.hit_test(540.0, 335.0).is_none());
        assert!(tree.hit_test(540.0, 525.0).is_none());
        assert!(layout_v1_find(&tree, "ObjectSummary").is_none());
    }

    #[test]
    fn layout_v2_without_rows_does_not_invent_footer_hits() {
        let screen = compile_v2(
            r#"
            sui 2
            screen now {
              component ContextHeader {}
              component BottomNavigation {
                tab now {}
                tab inbox {}
                tab spaces {}
                tab me {}
              }
            }
            "#,
        )
        .expect("tabs only");
        let tree = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            tree.hit_test(135.0, 2250.0).map(|node| node.id.as_str()),
            Some("now")
        );
        assert!(tree.hit_test(540.0, 1860.0).is_none());
        assert!(tree.hit_test(540.0, 2080.0).is_none());
    }

    #[test]
    fn layout_v2_empty_navigation_does_not_borrow_v1_tabs() {
        let screen = compile_v2(
            r#"
            sui 2
            screen now {
              component BottomNavigation {}
            }
            "#,
        )
        .expect("empty nav");
        let tree = layout_v2(&screen, 1080, 2400);
        assert!(tree.hit_test(135.0, 2250.0).is_none());
        assert!(layout_v1_find(&tree, "now").is_none());
        assert!(layout_v1_find(&tree, "me").is_none());
    }

    #[test]
    fn layout_v2_without_tabs_does_not_invent_v1_hits() {
        let screen = compile_v2(
            r#"
            sui 2
            screen now {
              component ContextHeader {}
            }
            "#,
        )
        .expect("header only");
        let tree = layout_v2(&screen, 1080, 2400);
        assert!(tree.hit_test(135.0, 2250.0).is_none());
        assert!(tree.hit_test(945.0, 2250.0).is_none());
    }
}
