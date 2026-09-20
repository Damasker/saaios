//! ADR-184: layout and hit-test from compiled `.sui` v1 `ScreenSpec`.
//! ADR-194: `layout_v2()` matches those tab hits for public NOW.
//!
//! Production chrome still uses `layout_v1_root()`. `compile_v2()`
//! stays off `build.rs`. Nested `tab` ids under `BottomNavigation`
//! own the v2 strip (ADR-196).

use saai_ui_core::{
    layout, Axis, EdgeInsets, LayoutNode, Length, Node, Rect, SafeInsets, SurfaceScale,
    MIN_TOUCH_TARGET,
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

fn v2_content_node(screen: &SuiV2Screen, width: u32) -> Node {
    let margin = width / 22;
    let mut children: Vec<Node> = screen
        .components
        .iter()
        .filter(|component| component.type_name != "BottomNavigation")
        .map(|component| Node::leaf(component.type_name.clone()))
        .collect();
    if children.is_empty() {
        children.push(Node::leaf(format!("{}-fill", screen.id)));
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

/// Public NOW chrome: non-actionable v2 content leaves plus a tab
/// strip from nested `tab` ids. Empty `BottomNavigation` does not
/// borrow `compile_v1_rollback()`.
pub fn v2_root_node(screen: &SuiV2Screen, width: u32, height: u32) -> Node {
    let content = v2_content_node(screen, width);
    let tabs = v2_named_tabs(screen);
    if tabs.is_empty() {
        return content;
    }
    let tab_height_2400 =
        EdgeInsets::from_safe(SafeInsets::PIXEL_7_PORTRAIT, SurfaceScale::PIXEL_7).bottom;
    let tab_height = v1_tab_strip_height(height, tab_height_2400);
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
    fn shell_root_view_uses_v1_layout_not_v2() {
        let main = include_str!("../../../services/saai-shell/src/main.rs");
        assert!(main.contains("layout_v1_root"));
        assert!(main.contains("compile_v1_rollback"));
        assert!(!main.contains("saai_ui_compiler::compile_v2"));
        assert!(!main.contains("layout_v2"));
        let build = include_str!("../../../services/saai-shell/build.rs");
        assert!(build.contains("saai_ui_compiler::compile("));
        assert!(!build.contains("saai_ui_compiler::compile_v2"));
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
