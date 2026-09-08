//! Typed view tree and deterministic layout for Saai UI (ADR-017).
//!
//! This is the runtime-independent intermediate representation. The future
//! `.sui` compiler will produce this tree; shells and apps do not implement a
//! second set of rectangles for touch handling.

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
            action: None,
            children,
        }
    }

    pub fn with_size(mut self, width: Length, height: Length) -> Self {
        self.width = width;
        self.height = height;
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
    let child_rects = match node.kind {
        NodeKind::Leaf => Vec::new(),
        NodeKind::Stack => vec![bounds; node.children.len()],
        NodeKind::Linear(axis) => linear_rects(&node.children, bounds, axis),
    };
    LayoutNode {
        id: node.id.clone(),
        rect: bounds,
        action: node.action.clone(),
        children: node
            .children
            .iter()
            .zip(child_rects)
            .map(|(child, rect)| layout_node(child, rect))
            .collect(),
    }
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
    use super::{layout, Axis, Length, Node, Rect};

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
}
