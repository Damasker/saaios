//! APP-04 / ADR-273: foreign OSK chrome is a bottom layer of Keyboard
//! keys. Shown only while `zwp_input_method_v2` is active. Shell Intent /
//! PIN / Wi-Fi fields keep the in-window Keyboard. Not wvkbd.

use saai_ui_core::{
    layout, Axis, EdgeInsets, Keyboard, KeyboardMode, Keystroke, LayoutNode, Length, LogicalUnit,
    Node, OskImeOp, Rect, SpacingToken, SurfaceScale, MIN_TOUCH_TARGET,
};

pub const OSK_LAYER_NAMESPACE: &str = "saai-shell-osk";

const OSK_ROWS_ID: &str = "osk-rows";
const CANCEL: &str = "intent:cancel";
const MODE_TOGGLE: &str = "intent:mode:toggle";
const SPACE: &str = "intent:space";
const BACKSPACE: &str = "intent:backspace";
const SEND: &str = "intent:send";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OskLayerGeom {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

fn physical_unit(value: LogicalUnit) -> u32 {
    SurfaceScale::PIXEL_7.logical_to_physical(value)
}

/// Same four-row dock as the Intent keyboard, so a foreign field does
/// not grow a second key map.
pub fn osk_layer_height(panel_height: u32) -> u32 {
    let row = physical_unit(MIN_TOUCH_TARGET);
    let pad = physical_unit(SpacingToken::Small.value()).saturating_mul(2);
    let wanted = row.saturating_mul(4).saturating_add(pad);
    let keep_field = row.saturating_mul(2);
    wanted.min(panel_height.saturating_sub(keep_field)).max(row)
}

pub fn osk_layer_geom(panel_width: u32, panel_height: u32) -> OskLayerGeom {
    let height = osk_layer_height(panel_height) as i32;
    let width = panel_width.max(1) as i32;
    OskLayerGeom {
        x: 0,
        y: (panel_height as i32).saturating_sub(height),
        width,
        height,
    }
}

/// Layer chrome is only for a third-party text-input field. A shell
/// Field already has its own Keyboard in the toplevel.
pub fn osk_layer_visible(ime_active: bool, shell_owns_field: bool) -> bool {
    ime_active && !shell_owns_field
}

pub fn apply_osk_action(keyboard: &mut Keyboard, action: &str) -> Option<OskImeOp> {
    let stroke = Keyboard::keystroke_from_osk_action(action)?;
    if stroke == Keystroke::ModeToggle {
        keyboard.mode = keyboard.mode.toggled();
        return None;
    }
    stroke.to_ime_op()
}

fn key_action(ch: char) -> String {
    format!("intent:key:{ch}")
}

fn keyboard_rows(mode: KeyboardMode) -> &'static [&'static str; 3] {
    Keyboard::qwerty_rows(mode)
}

fn mode_toggle_label(mode: KeyboardMode) -> String {
    match mode {
        KeyboardMode::Letters => "123".to_string(),
        KeyboardMode::Symbols => "ABC".to_string(),
    }
}

fn row_inset(letters: &str, width: u32) -> u32 {
    let count = letters.chars().count() as u32;
    if count >= 10 {
        0
    } else {
        ((10 - count).saturating_mul(width)) / 20
    }
}

fn side_key_width(width: u32) -> u32 {
    (width / 10 * 2).max(physical_unit(MIN_TOUCH_TARGET))
}

fn osk_view(width: u32, height: u32, mode: KeyboardMode) -> LayoutNode {
    let side = side_key_width(width);
    let modifier = physical_unit(MIN_TOUCH_TARGET);
    let mut focus = 1u32;
    let mut rows: Vec<Node> = Vec::new();
    for (row_index, letters) in keyboard_rows(mode).iter().enumerate() {
        let inset = row_inset(letters, width);
        let mut keys = Vec::new();
        for ch in letters.chars() {
            keys.push(
                Node::leaf(format!("osk-key-{ch}"))
                    .with_action(key_action(ch))
                    .with_focus_order(focus),
            );
            focus += 1;
        }
        rows.push(
            Node::linear(format!("osk-row-{row_index}"), Axis::Horizontal, keys).with_padding(
                EdgeInsets {
                    top: 0,
                    right: inset,
                    bottom: 0,
                    left: inset,
                },
            ),
        );
    }
    let controls = [
        ("osk-cancel", "Отмена", CANCEL, Length::Px(side)),
        ("osk-mode", "123", MODE_TOGGLE, Length::Px(modifier)),
        ("osk-space", "␣", SPACE, Length::Fill),
        ("osk-backspace", "⌫", BACKSPACE, Length::Px(modifier)),
        ("osk-send", "Отправить", SEND, Length::Px(side)),
    ];
    let mut control_nodes = Vec::new();
    for (id, _label, action, size) in controls {
        control_nodes.push(
            Node::leaf(id)
                .with_action(action)
                .with_focus_order(focus)
                .with_size(size, Length::Fill),
        );
        focus += 1;
    }
    rows.push(Node::linear(
        "osk-controls",
        Axis::Horizontal,
        control_nodes,
    ));
    let pad = physical_unit(SpacingToken::XSmall.value());
    let root = Node::linear(OSK_ROWS_ID, Axis::Vertical, rows)
        .with_size(Length::Fill, Length::Fill)
        .with_padding(EdgeInsets::all(pad));
    layout(&root, Rect::new(0, 0, width, height))
}

pub fn osk_action_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    mode: KeyboardMode,
) -> Option<String> {
    if width == 0 || height == 0 {
        return None;
    }
    osk_view(width, height, mode)
        .hit_test(pos.0, pos.1)
        .and_then(|node| node.action.clone())
}

pub fn osk_keys(width: u32, height: u32, mode: KeyboardMode) -> Vec<(Rect, String)> {
    let view = osk_view(width, height, mode);
    let keyboard = layout_by_id(&view, OSK_ROWS_ID).unwrap_or(&view);
    let rows = keyboard_rows(mode);
    let mut keys = Vec::new();
    for (row_index, letters) in rows.iter().enumerate() {
        let Some(row_node) = keyboard.children.get(row_index) else {
            continue;
        };
        for (key_node, ch) in row_node.children.iter().zip(letters.chars()) {
            keys.push((key_node.rect, ch.to_uppercase().to_string()));
        }
    }
    if let Some(controls) = keyboard.children.get(rows.len()) {
        let labels = [
            "Отмена".to_string(),
            mode_toggle_label(mode),
            "␣".to_string(),
            "⌫".to_string(),
            "Отправить".to_string(),
        ];
        for (key_node, label) in controls.children.iter().zip(labels) {
            keys.push((key_node.rect, label));
        }
    }
    keys
}

fn layout_by_id<'a>(node: &'a LayoutNode, id: &str) -> Option<&'a LayoutNode> {
    if node.id == id {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| layout_by_id(child, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use saai_ui_core::{apply_ime_op, Keyboard};

    #[test]
    fn osk_docks_at_the_bottom_of_the_panel() {
        let geom = osk_layer_geom(1080, 2400);
        assert_eq!(geom.x, 0);
        assert_eq!(geom.width, 1080);
        assert_eq!(geom.y + geom.height, 2400);
        assert!(geom.height >= 144);
        assert!(geom.y > 0);
    }

    #[test]
    fn foreign_ime_activate_shows_the_layer() {
        assert!(osk_layer_visible(true, false));
        assert!(!osk_layer_visible(false, false));
    }

    #[test]
    fn shell_owned_field_keeps_the_layer_hidden() {
        assert!(!osk_layer_visible(true, true));
    }

    #[test]
    fn keys_live_inside_the_layer_not_the_full_panel() {
        let geom = osk_layer_geom(1080, 2400);
        let keys = osk_keys(geom.width as u32, geom.height as u32, KeyboardMode::Letters);
        assert!(keys.iter().any(|(_, label)| label == "Q"));
        for (rect, _) in &keys {
            assert!(rect.y + rect.height <= geom.height as u32);
        }
    }

    #[test]
    fn letter_tap_becomes_commit_string() {
        let geom = osk_layer_geom(1080, 2400);
        let keys = osk_keys(geom.width as u32, geom.height as u32, KeyboardMode::Letters);
        let q = keys.iter().find(|(_, label)| label == "Q").expect("Q");
        let pos = (
            (q.0.x + q.0.width / 2) as f64,
            (q.0.y + q.0.height / 2) as f64,
        );
        let action = osk_action_at(
            pos,
            geom.width as u32,
            geom.height as u32,
            KeyboardMode::Letters,
        )
        .expect("Q action");
        let mut keyboard = Keyboard::bind_foreign_ime();
        assert_eq!(
            apply_osk_action(&mut keyboard, &action),
            Some(OskImeOp::CommitString("q".into()))
        );
    }

    #[test]
    fn hi_bang_from_layer_keys_reaches_the_ime_buffer() {
        let mut keyboard = Keyboard::bind_foreign_ime();
        let mut buffer = String::new();
        for action in [
            "intent:key:h",
            "intent:key:i",
            "intent:mode:toggle",
            "intent:backspace",
            "intent:key:i",
            "intent:key:!",
        ] {
            if let Some(op) = apply_osk_action(&mut keyboard, action) {
                apply_ime_op(&mut buffer, &op);
            }
        }
        assert_eq!(buffer, "hi!");
        assert_eq!(keyboard.mode, KeyboardMode::Symbols);
    }
}
