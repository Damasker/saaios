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
//! ADR-219: stacked rows overlay via `Node::stack` so `layout_v2_scrolled`
//! matches `scrolled_row_rect` after a drag.
//! ADR-220: `Button` tiles overlay the 3-column apps grid so hits match
//! `now_grid_rect`. Linear layout cannot place a column beside another.
//! ADR-221: overlay `Field` and decision `Button`s dock through
//! `layout_v2()`. ADR-222: privileged `Keyboard` is the Field-bound
//! IME object; its presence docks the on-screen reserve. Hardware
//! omits the component and USB HID replaces the panel.
//! ADR-223: privileged `OrbHost` overlays the live Orb zone so
//! closed-dot and open-menu hits match `orb_zone_rect`. Gallery page
//! taps stay a whole-surface formula.
//! ADR-224: privileged `diagnostic` DataRows plus trailing `row back`
//! dock live DevSurface hits. Rows stay read-only.
//! ADR-225: NOW chrome paint reads the same `layout_v2(now.sui)`
//! tree as hits. Live SystemSection rows stay runtime content.
//! ADR-226: Inbox/Spaces/Wi-Fi/Bluetooth/trusted paint reads the
//! generated list trees hit-test already uses.
//! ADR-227: Me paint reads the same `layout_v2_scrolled` tree as hits.
//! ADR-228: apps grid paint reads the same generated `layout_v2` tree.
//! ADR-229: overlay Field/decision paint reads the same generated tree.
//! ADR-230: OrbHost paint reads the same generated `layout_v2` tree.
//! ADR-231: diagnostic paint reads the same generated `layout_v2_scrolled`
//! tree as Назад hits, clipping DataRows to the first stacked slot.

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

/// Live DevSurface scrolls DataRows between the first stacked slot
/// and docked Назад. Tabs are absent, so this is not `v2_me_scroll_clip`.
fn v2_diagnostic_scroll_clip(screen: &SuiV2Screen) -> bool {
    screen.id == "diagnostic"
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

fn v2_placed_slot(
    screen_id: &str,
    index: usize,
    id: String,
    action: Option<String>,
    top: u32,
    height: u32,
) -> Node {
    let spacer =
        Node::leaf(format!("{screen_id}-slot-{index}")).with_size(Length::Fill, Length::Px(top));
    let mut row = Node::leaf(id).with_size(Length::Fill, Length::Px(height));
    if let Some(action) = action {
        row = row.with_action(action);
    }
    Node::linear(
        format!("{screen_id}-place-{index}"),
        Axis::Vertical,
        vec![spacer, row],
    )
}

const V2_GRID_COLUMNS: u32 = 3;
const V2_GRID_CELL_HEIGHT_2400: u32 = 300;
const V2_GRID_ROW_GAP_2400: u32 = 40;
const V2_GRID_TOP_2400: u32 = 430;

/// Live `now_grid_rect`: 3 columns, design-canvas tops scaled by panel
/// height. Column gap is `margin / 2`, same as the shell paint formula.
fn v2_grid_rect(index: usize, width: u32, height: u32) -> Rect {
    let margin = width / 22;
    let columns = V2_GRID_COLUMNS;
    let gap = margin / 2;
    let usable_width = width.saturating_sub(margin * 2);
    let cell_width = usable_width.saturating_sub(gap * (columns - 1)) / columns;
    let row = index as u32 / columns;
    let column = index as u32 % columns;
    let top_2400 = V2_GRID_TOP_2400 + row * (V2_GRID_CELL_HEIGHT_2400 + V2_GRID_ROW_GAP_2400);
    let top = ((u64::from(top_2400) * u64::from(height)) / 2400) as u32;
    let cell_height = ((u64::from(V2_GRID_CELL_HEIGHT_2400) * u64::from(height)) / 2400) as u32;
    Rect::new(
        margin + column * (cell_width + gap),
        top,
        cell_width,
        cell_height,
    )
}

fn v2_placed_grid_slot(
    screen_id: &str,
    index: usize,
    id: String,
    action: Option<String>,
    panel_width: u32,
    panel_height: u32,
) -> Node {
    let margin = panel_width / 22;
    let rect = v2_grid_rect(index, panel_width, panel_height);
    let inner_x = rect.x.saturating_sub(margin);
    let y_spacer = Node::leaf(format!("{screen_id}-grid-y-{index}"))
        .with_size(Length::Fill, Length::Px(rect.y));
    let x_spacer = Node::leaf(format!("{screen_id}-grid-x-{index}"))
        .with_size(Length::Px(inner_x), Length::Px(rect.height));
    let mut cell = Node::leaf(id).with_size(Length::Px(rect.width), Length::Px(rect.height));
    if let Some(action) = action {
        cell = cell.with_action(action);
    }
    let row = Node::linear(
        format!("{screen_id}-grid-row-{index}"),
        Axis::Horizontal,
        vec![x_spacer, cell],
    )
    .with_size(Length::Fill, Length::Px(rect.height));
    Node::linear(
        format!("{screen_id}-grid-place-{index}"),
        Axis::Vertical,
        vec![y_spacer, row],
    )
}

fn v2_grid_action(tile: &crate::SuiV2Component) -> Option<String> {
    if tile.props.a11y.as_deref() != Some("Button") {
        return None;
    }
    tile.props.loc.clone()
}

fn v2_is_grid_button(component: &crate::SuiV2Component) -> bool {
    component.type_name == "Button"
        && component
            .props
            .loc
            .as_deref()
            .is_some_and(|loc| loc.starts_with("manage_app:"))
}

fn v2_is_overlay_button(component: &crate::SuiV2Component) -> bool {
    component.type_name == "Button"
        && component.props.loc.is_some()
        && !v2_is_grid_button(component)
        && !v2_is_orb_menu_button(component)
}

fn v2_is_orb_menu_button(component: &crate::SuiV2Component) -> bool {
    component.type_name == "Button"
        && component
            .props
            .loc
            .as_deref()
            .is_some_and(|loc| loc.starts_with("orb-menu:"))
}

/// Live HIA-04b Orb square: 90 px, never more than a tenth of the panel.
pub fn v2_orb_dot_size(width: u32, height: u32) -> u32 {
    90.min(width / 10).min(height / 10)
}

/// Live `orb_zone_rect`: closed is the dot; open grows upward into the
/// header dead space and stops short of stacked cards at y=430/2400.
pub fn v2_orb_zone_rect(width: u32, height: u32, menu_action_count: usize) -> Rect {
    let margin = width / 22;
    let dot_size = v2_orb_dot_size(width, height);
    let bottom = ((410_u64 * u64::from(height)) / 2400) as u32;
    if menu_action_count == 0 {
        return Rect::new(
            width.saturating_sub(margin + dot_size),
            bottom.saturating_sub(dot_size),
            dot_size,
            dot_size,
        );
    }
    let top = ((160_u64 * u64::from(height)) / 2400) as u32;
    let zone_width = 420.min(width.saturating_sub(margin * 2));
    Rect::new(
        width.saturating_sub(margin + zone_width),
        top,
        zone_width,
        bottom.saturating_sub(top),
    )
}

fn v2_place_rect(screen_id: &str, index: usize, inner: Node, rect: Rect) -> Node {
    let y_spacer = Node::leaf(format!("{screen_id}-orb-y-{index}"))
        .with_size(Length::Fill, Length::Px(rect.y));
    let x_spacer = Node::leaf(format!("{screen_id}-orb-x-{index}"))
        .with_size(Length::Px(rect.x), Length::Px(rect.height));
    let cell = inner.with_size(Length::Px(rect.width), Length::Px(rect.height));
    let row = Node::linear(
        format!("{screen_id}-orb-row-{index}"),
        Axis::Horizontal,
        vec![x_spacer, cell],
    )
    .with_size(Length::Fill, Length::Px(rect.height));
    Node::linear(
        format!("{screen_id}-orb-place-{index}"),
        Axis::Vertical,
        vec![y_spacer, row],
    )
}

fn v2_orb_toggle_action(orb: &crate::SuiV2Component) -> String {
    orb.props
        .loc
        .clone()
        .unwrap_or_else(|| "orb:toggle".to_string())
}

fn v2_orb_node(
    screen: &crate::SuiV2Screen,
    width: u32,
    height: u32,
    orb: &crate::SuiV2Component,
    menus: &[&crate::SuiV2Component],
) -> Node {
    let zone = v2_orb_zone_rect(width, height, menus.len());
    let toggle = v2_orb_toggle_action(orb);
    if menus.is_empty() {
        return v2_place_rect(
            &screen.id,
            0,
            Node::leaf(toggle.clone()).with_action(toggle),
            zone,
        );
    }
    let mut children: Vec<Node> = menus
        .iter()
        .map(|menu| {
            let loc = menu
                .props
                .loc
                .clone()
                .unwrap_or_else(|| "orb-menu".to_string());
            Node::leaf(loc.clone()).with_action(loc)
        })
        .collect();
    children.push(
        Node::leaf(toggle.clone())
            .with_action(toggle)
            .with_size(Length::Fill, Length::Px(v2_orb_dot_size(width, height))),
    );
    v2_place_rect(
        &screen.id,
        0,
        Node::linear(format!("{}-orb-menu", screen.id), Axis::Vertical, children),
        zone,
    )
}

/// Live consent / task-confirm / object-view button row uses
/// `ROOT_TAB_HEIGHT` (300) as physical `Px`.
const V2_OVERLAY_BUTTON_HEIGHT: u32 = 300;
const V2_LOCK_HEADER_HEIGHT: u32 = 260;
const V2_LOCK_FIELD_TOP: u32 = 24;

fn v2_compose_keyboard_height(screen: &crate::SuiV2Screen, panel_height: u32) -> u32 {
    let row = v2_physical(MIN_TOUCH_TARGET);
    let (rows, pad_token) = if screen.id == "pin-setup" {
        (5u32, SpacingToken::XSmall)
    } else {
        (4, SpacingToken::Small)
    };
    let pad = v2_physical(pad_token.value()).saturating_mul(2);
    let wanted = row.saturating_mul(rows).saturating_add(pad);
    let keep_field = row.saturating_mul(2);
    wanted.min(panel_height.saturating_sub(keep_field)).max(row)
}

fn v2_overlay_button_action(button: &crate::SuiV2Component) -> Option<String> {
    if button.props.a11y.as_deref() != Some("Button") {
        return None;
    }
    button.props.loc.clone()
}

fn v2_decision_node(screen: &crate::SuiV2Screen, buttons: &[&crate::SuiV2Component]) -> Node {
    let header = Node::leaf("ContextHeader".to_string());
    let row = Node::linear(
        format!("{}-buttons", screen.id),
        Axis::Horizontal,
        buttons
            .iter()
            .enumerate()
            .map(|(index, button)| {
                let id = button
                    .props
                    .loc
                    .clone()
                    .unwrap_or_else(|| format!("Button-{index}"));
                let mut leaf = Node::leaf(id);
                if let Some(action) = v2_overlay_button_action(button) {
                    leaf = leaf.with_action(action);
                }
                leaf
            })
            .collect(),
    )
    .with_size(Length::Fill, Length::Px(V2_OVERLAY_BUTTON_HEIGHT));
    Node::linear(
        format!("{}-content", screen.id),
        Axis::Vertical,
        vec![header, row],
    )
}

fn v2_compose_node(
    screen: &crate::SuiV2Screen,
    width: u32,
    height: u32,
    field: &crate::SuiV2Component,
    keyboard: Option<&crate::SuiV2Component>,
) -> Node {
    let margin = width / 22;
    let field_height = v2_stacked_row_height(height);
    let id = field
        .props
        .loc
        .clone()
        .unwrap_or_else(|| "Field".to_string());
    let field_row = Node::linear(
        format!("{}-field-row", screen.id),
        Axis::Horizontal,
        vec![Node::leaf(id)],
    )
    .with_size(Length::Fill, Length::Px(field_height))
    .with_padding(EdgeInsets {
        top: 0,
        right: margin,
        bottom: 0,
        left: margin,
    });
    let mut children = vec![Node::leaf("ContextHeader".to_string()), field_row];
    if let Some(keyboard) = keyboard {
        let keyboard_id = keyboard
            .props
            .loc
            .clone()
            .unwrap_or_else(|| format!("{}-keyboard", screen.id));
        children.push(Node::leaf(keyboard_id).with_size(
            Length::Fill,
            Length::Px(v2_compose_keyboard_height(screen, height)),
        ));
    }
    Node::linear(format!("{}-content", screen.id), Axis::Vertical, children)
}

fn v2_lock_field_node(
    screen: &crate::SuiV2Screen,
    width: u32,
    field: &crate::SuiV2Component,
) -> Node {
    let margin = width / 22;
    let id = field
        .props
        .loc
        .clone()
        .unwrap_or_else(|| "Field".to_string());
    let slot = v2_placed_slot(
        &screen.id,
        0,
        id,
        None,
        V2_LOCK_FIELD_TOP,
        V2_LOCK_HEADER_HEIGHT.saturating_sub(48),
    );
    Node::stack(format!("{}-layers", screen.id), vec![slot]).with_padding(EdgeInsets {
        top: 0,
        right: margin,
        bottom: 0,
        left: margin,
    })
}

fn v2_stacked_action(row: &crate::SuiV2Component) -> Option<String> {
    if matches!(row.type_name.as_str(), "SystemSection" | "CapabilityRow")
        || row.props.a11y.as_deref() != Some("Button")
    {
        return None;
    }
    let loc = row.props.loc.as_deref();
    Some(match row.type_name.as_str() {
        "SpaceRow" => format!("select_space:{}", loc.unwrap_or("SpaceRow")),
        "SettingRow" | "DataRow" => loc.unwrap_or("SettingRow").to_string(),
        "WifiRow" => "connect_wifi".to_string(),
        "BluetoothRow" => "pair_bluetooth".to_string(),
        "TrustedClientRow" => "revoke_trusted_client".to_string(),
        _ => "open_object".to_string(),
    })
}

fn v2_content_node(
    screen: &SuiV2Screen,
    width: u32,
    height: u32,
    content_height: u32,
    scroll_offset: i32,
) -> Node {
    let margin = width / 22;
    let mut header = None;
    let mut object = None;
    let mut stacked = Vec::new();
    let mut grid = Vec::new();
    let mut overlay_buttons = Vec::new();
    let mut fields = Vec::new();
    let mut keyboards = Vec::new();
    let mut orbs = Vec::new();
    let mut orb_menus = Vec::new();
    let mut rest = Vec::new();
    for component in screen
        .components
        .iter()
        .filter(|component| component.type_name != "BottomNavigation")
    {
        match component.type_name.as_str() {
            "ContextHeader" if header.is_none() => header = Some(component),
            "ObjectSummary" if object.is_none() => object = Some(component),
            "Field" if component.props.loc.is_some() => fields.push(component),
            "Keyboard" => keyboards.push(component),
            "OrbHost" => orbs.push(component),
            "EventRow" | "SpaceRow" | "SettingRow" | "DataRow" | "SystemSection" | "WifiRow"
            | "BluetoothRow" | "TrustedClientRow" | "CapabilityRow" => stacked.push(component),
            _ if v2_is_grid_button(component) => grid.push(component),
            _ if v2_is_orb_menu_button(component) => orb_menus.push(component),
            _ if v2_is_overlay_button(component) => overlay_buttons.push(component),
            _ => rest.push(component),
        }
    }
    if let Some(orb) = orbs.first().copied() {
        return v2_orb_node(screen, width, height, orb, &orb_menus);
    }
    if v2_named_tabs(screen).is_empty() && !overlay_buttons.is_empty() {
        return v2_decision_node(screen, &overlay_buttons);
    }
    if v2_named_tabs(screen).is_empty() && screen.id == "lock" {
        if let Some(field) = fields.first() {
            return v2_lock_field_node(screen, width, field);
        }
    }
    if v2_named_tabs(screen).is_empty() {
        if let Some(field) = fields.first() {
            return v2_compose_node(screen, width, height, field, keyboards.first().copied());
        }
    }
    let header_height = if header.is_some() {
        if object.is_some() && !v2_named_tabs(screen).is_empty() {
            v2_now_header_height(content_height)
        } else if !stacked.is_empty() || !grid.is_empty() {
            v2_stacked_row_top(0, height)
        } else {
            0
        }
    } else {
        0
    };
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
    let clip_to_first_slot = v2_diagnostic_scroll_clip(screen);
    let scroll = if clip_to_content || clip_to_first_slot {
        scroll_offset.max(0)
    } else {
        0
    };
    let min_top = if clip_to_first_slot {
        i64::from(v2_stacked_row_top(0, height))
    } else {
        0
    };
    let mut layers = Vec::new();
    if header.is_some() && header_height > 0 {
        layers.push(v2_placed_slot(
            &screen.id,
            0,
            "ContextHeader".to_string(),
            None,
            0,
            header_height,
        ));
    }
    if object.is_some() && !v2_named_tabs(screen).is_empty() {
        layers.push(v2_placed_slot(
            &screen.id,
            1,
            "ObjectSummary".to_string(),
            Some("open_object".to_string()),
            header_height,
            v2_now_object_height(),
        ));
    }
    for (index, row) in stacked.iter().enumerate() {
        let base = v2_stacked_row_top(index, height) as i64 - i64::from(scroll);
        if base < min_top {
            continue;
        }
        let top = base as u32;
        if trailing_first_top.is_some_and(|first| top.saturating_add(stacked_height) > first) {
            continue;
        }
        if clip_to_content
            && (top >= content_height || top.saturating_add(stacked_height) > content_height)
        {
            continue;
        }
        let id = row
            .props
            .loc
            .clone()
            .unwrap_or_else(|| format!("{}-{index}", row.type_name));
        layers.push(v2_placed_slot(
            &screen.id,
            index + 2,
            id,
            v2_stacked_action(row),
            top,
            stacked_height,
        ));
    }
    if !trailing.is_empty() {
        let last = stacked.len() + trailing.len() - 1;
        for (offset, row) in trailing.iter().enumerate() {
            let index = stacked.len() + offset;
            let top = v2_trailing_top(index, last, height);
            layers.push(v2_placed_slot(
                &screen.id,
                index + 100,
                row.id.clone(),
                Some(row.action.clone()),
                top,
                stacked_height,
            ));
        }
    }
    for (index, tile) in grid.iter().enumerate() {
        let id = tile
            .props
            .loc
            .clone()
            .unwrap_or_else(|| format!("Button-{index}"));
        layers.push(v2_placed_grid_slot(
            &screen.id,
            index,
            id,
            v2_grid_action(tile),
            width,
            height,
        ));
    }
    for component in rest {
        layers.push(Node::leaf(component.type_name.clone()));
    }
    if layers.is_empty() {
        layers
            .push(Node::leaf(format!("{}-fill", screen.id)).with_size(Length::Fill, Length::Fill));
    }
    let mut children = vec![Node::stack(format!("{}-layers", screen.id), layers)];
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
    v2_root_node_scrolled(screen, width, height, 0)
}

fn v2_root_node_scrolled(
    screen: &SuiV2Screen,
    width: u32,
    height: u32,
    scroll_offset: i32,
) -> Node {
    let tabs = v2_named_tabs(screen);
    let tab_height_2400 =
        EdgeInsets::from_safe(SafeInsets::PIXEL_7_PORTRAIT, SurfaceScale::PIXEL_7).bottom;
    let tab_height = if tabs.is_empty() {
        0
    } else {
        v1_tab_strip_height(height, tab_height_2400)
    };
    let content = v2_content_node(
        screen,
        width,
        height,
        height.saturating_sub(tab_height),
        scroll_offset,
    );
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
    layout_v2_scrolled(screen, width, height, 0)
}

/// ADR-219: same tree as `layout_v2`, with Me stacked rows shifted by
/// `scroll_offset` like live `scrolled_row_rect`.
pub fn layout_v2_scrolled(
    screen: &SuiV2Screen,
    width: u32,
    height: u32,
    scroll_offset: i32,
) -> LayoutNode {
    layout(
        &v2_root_node_scrolled(screen, width, height, scroll_offset),
        Rect::new(0, 0, width, height),
    )
}

#[cfg(test)]
mod tests {
    use super::{layout_v1_find, layout_v1_root, layout_v2, layout_v2_scrolled, v2_grid_rect};
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
        assert!(main.contains("now_paint_chrome_from"));
        assert!(main.contains("list_paint_cards"));
        assert!(main.contains("orb_v2_source"));
        assert!(main.contains("diagnostic_v2_source"));
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
        let scrolled = layout_v2_scrolled(&screen, 1080, 2400, 220);
        assert_eq!(
            scrolled
                .hit_test(540.0, 305.0)
                .and_then(|node| node.action.as_deref()),
            Some("row0")
        );
        assert_eq!(
            scrolled
                .hit_test(540.0, 1845.0)
                .and_then(|node| node.action.as_deref()),
            Some("row7")
        );
    }

    #[test]
    fn layout_v2_apps_grid_matches_now_grid_rect() {
        let screen = compile_v2(
            r#"
            sui 2
            screen apps {
              component ContextHeader {}
              component Button {
                a11y = Button
                loc = "manage_app:demo"
              }
              component Button {
                a11y = Status
                loc = "manage_app:quiet"
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
        .expect("apps grid");
        let v2 = layout_v2(&screen, 1080, 2400);
        let cell_0 = v2_grid_rect(0, 1080, 2400);
        let cell_1 = v2_grid_rect(1, 1080, 2400);
        assert_eq!(
            v2.hit_test(f64::from(cell_0.x + 10), f64::from(cell_0.y + 10))
                .and_then(|node| node.action.as_deref()),
            Some("manage_app:demo")
        );
        assert!(v2
            .hit_test(f64::from(cell_1.x + 10), f64::from(cell_1.y + 10))
            .is_none());
        assert!(v2.hit_test(540.0, 250.0).is_none());
        assert_eq!(
            v2.hit_test(135.0, 2250.0).map(|node| node.id.as_str()),
            Some("now")
        );
        let empty = compile_v2(
            r#"
            sui 2
            screen apps {
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
        .expect("empty apps");
        let tree = layout_v2(&empty, 1080, 2400);
        assert!(tree
            .hit_test(f64::from(cell_0.x + 10), f64::from(cell_0.y + 10))
            .is_none());
    }

    #[test]
    fn layout_v2_overlay_buttons_match_consent_row() {
        let screen = compile_v2(
            r#"
            sui 2
            screen consent {
              component ContextHeader {}
              component Button {
                a11y = Button
                loc = "consent:accept"
              }
              component Button {
                a11y = Button
                loc = "consent:decline"
              }
            }
            "#,
        )
        .expect("consent overlay");
        let v2 = layout_v2(&screen, 1080, 2400);
        assert_eq!(
            v2.hit_test(270.0, 2250.0)
                .and_then(|node| node.action.as_deref()),
            Some("consent:accept")
        );
        assert_eq!(
            v2.hit_test(810.0, 2250.0)
                .and_then(|node| node.action.as_deref()),
            Some("consent:decline")
        );
        assert!(v2.hit_test(540.0, 1000.0).is_none());
        let one = compile_v2(
            r#"
            sui 2
            screen object {
              component ObjectSummary {}
              component Button {
                a11y = Button
                loc = "object-view-action:0"
              }
            }
            "#,
        )
        .expect("object overlay");
        let tree = layout_v2(&one, 1080, 2400);
        assert_eq!(
            tree.hit_test(270.0, 2250.0)
                .and_then(|node| node.action.as_deref()),
            Some("object-view-action:0")
        );
        assert_eq!(
            tree.hit_test(810.0, 2250.0)
                .and_then(|node| node.action.as_deref()),
            Some("object-view-action:0")
        );
        assert!(tree
            .hit_test(540.0, 800.0)
            .and_then(|node| node.action.as_deref())
            .is_none());
    }

    #[test]
    fn layout_v2_compose_field_docks_above_keyboard_reserve() {
        let screen = compile_v2(
            r#"
            sui 2
            screen intent {
              component ContextHeader {}
              component Field {
                a11y = Status
                loc = "intent-field"
              }
              component Keyboard {
                a11y = Status
                loc = "intent-keyboard"
              }
            }
            "#,
        )
        .expect("intent field");
        let v2 = layout_v2(&screen, 1080, 2400);
        let field = layout_v1_find(&v2, "intent-field").expect("field");
        let keyboard = layout_v1_find(&v2, "intent-keyboard").expect("keyboard reserve");
        assert_eq!(field.rect.y + field.rect.height, keyboard.rect.y);
        assert!(v2.hit_test(540.0, f64::from(field.rect.y + 10)).is_none());
        let pin = compile_v2(
            r#"
            sui 2
            screen pin-setup {
              component ContextHeader {}
              component Field {
                a11y = Status
                loc = "pin-setup-field"
              }
              component Keyboard {
                a11y = Status
                loc = "pin-setup-keyboard"
              }
            }
            "#,
        )
        .expect("pin field");
        let pin_tree = layout_v2(&pin, 1080, 2400);
        let pin_field = layout_v1_find(&pin_tree, "pin-setup-field").expect("pin field");
        let pin_keys = layout_v1_find(&pin_tree, "pin-setup-keyboard").expect("pin reserve");
        assert_eq!(pin_field.rect.y + pin_field.rect.height, pin_keys.rect.y);
        assert!(pin_keys.rect.height > keyboard.rect.height);
        let lock = compile_v2(
            r#"
            sui 2
            screen lock {
              component Field {
                a11y = Status
                loc = "lock-pin-field"
              }
            }
            "#,
        )
        .expect("lock field");
        let lock_tree = layout_v2(&lock, 1080, 2400);
        let lock_field = layout_v1_find(&lock_tree, "lock-pin-field").expect("lock field");
        assert_eq!(lock_field.rect.y, 24);
        assert!(lock_field.rect.y + lock_field.rect.height <= 260);
    }

    #[test]
    fn layout_v2_compose_field_without_keyboard_omits_the_reserve() {
        let screen = compile_v2(
            r#"
            sui 2
            screen intent {
              component ContextHeader {}
              component Field {
                a11y = Status
                loc = "intent-field"
              }
            }
            "#,
        )
        .expect("hardware field");
        let v2 = layout_v2(&screen, 1080, 2400);
        let field = layout_v1_find(&v2, "intent-field").expect("field");
        assert!(layout_v1_find(&v2, "intent-keyboard").is_none());
        assert_eq!(field.rect.y + field.rect.height, 2400);
    }

    #[test]
    fn layout_v2_orbhost_matches_the_closed_dot_and_open_menu() {
        let closed = compile_v2(
            r#"
            sui 2
            screen now {
              component OrbHost {
                a11y = Status
                loc = "orb:toggle"
              }
            }
            "#,
        )
        .expect("closed orb");
        let closed_tree = layout_v2(&closed, 1080, 2400);
        let zone = super::v2_orb_zone_rect(1080, 2400, 0);
        assert_eq!(
            closed_tree
                .hit_test(
                    f64::from(zone.x + zone.width / 2),
                    f64::from(zone.y + zone.height / 2)
                )
                .and_then(|node| node.action.as_deref()),
            Some("orb:toggle")
        );
        assert!(closed_tree.hit_test(540.0, 525.0).is_none());
        assert!(closed_tree.hit_test(540.0, 2250.0).is_none());
        let open = compile_v2(
            r#"
            sui 2
            screen now {
              component OrbHost {
                a11y = Status
                loc = "orb:toggle"
              }
              component Button {
                a11y = Button
                loc = "orb-menu:inbox"
              }
              component Button {
                a11y = Button
                loc = "orb-menu:intent"
              }
            }
            "#,
        )
        .expect("open orb");
        let open_tree = layout_v2(&open, 1080, 2400);
        let open_zone = super::v2_orb_zone_rect(1080, 2400, 2);
        assert_eq!(
            open_tree
                .hit_test(
                    f64::from(open_zone.x + open_zone.width / 2),
                    f64::from(open_zone.y + 10)
                )
                .and_then(|node| node.action.as_deref()),
            Some("orb-menu:inbox")
        );
        assert_eq!(
            open_tree
                .hit_test(
                    f64::from(open_zone.x + open_zone.width / 2),
                    f64::from(open_zone.y + open_zone.height - 10)
                )
                .and_then(|node| node.action.as_deref()),
            Some("orb:toggle")
        );
        assert!(open_tree.hit_test(540.0, 2250.0).is_none());
    }

    #[test]
    fn layout_v2_diagnostic_back_matches_stacked_control() {
        let mut source = String::from(
            r#"
            sui 2
            screen diagnostic {
              component ContextHeader {}
            "#,
        );
        for index in 0..3 {
            source.push_str(&format!(
                "  component DataRow {{ a11y = Status loc = \"diagnostic.{index}\" }}\n"
            ));
        }
        source.push_str("  row back {}\n}\n");
        let screen = compile_v2(&source).expect("diagnostic");
        let v2 = layout_v2(&screen, 1080, 2400);
        assert!(v2.hit_test(540.0, 525.0).is_none());
        assert_eq!(
            v2.hit_test(540.0, 1185.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_back")
        );
        let mut overflow =
            String::from("sui 2\nscreen diagnostic {\n  component ContextHeader {}\n");
        for index in 0..9 {
            overflow.push_str(&format!(
                "  component DataRow {{ a11y = Status loc = \"diagnostic.{index}\" }}\n"
            ));
        }
        overflow.push_str("  row back {}\n}\n");
        let docked = layout_v2(
            &compile_v2(&overflow).expect("overflow diagnostic"),
            1080,
            2400,
        );
        assert_eq!(
            docked
                .hit_test(540.0, 2305.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_back")
        );
        assert!(compile_v2_public(&source)
            .unwrap_err()
            .to_string()
            .contains("diagnostic"));
    }

    #[test]
    fn layout_v2_diagnostic_rows_clip_above_first_slot_when_scrolled() {
        let mut overflow =
            String::from("sui 2\nscreen diagnostic {\n  component ContextHeader {}\n");
        for index in 0..9 {
            overflow.push_str(&format!(
                "  component DataRow {{ a11y = Status loc = \"diagnostic.{index}\" }}\n"
            ));
        }
        overflow.push_str("  row back {}\n}\n");
        let screen = compile_v2(&overflow).expect("overflow diagnostic");
        let rest = layout_v2(&screen, 1080, 2400);
        let first = layout_v1_find(&rest, "diagnostic.0").expect("row0").rect;
        assert!(layout_v1_find(&rest, "diagnostic.8").is_none());
        let scrolled = layout_v2_scrolled(&screen, 1080, 2400, 220);
        assert!(layout_v1_find(&scrolled, "diagnostic.0").is_none());
        assert_eq!(
            layout_v1_find(&scrolled, "diagnostic.1")
                .expect("row1")
                .rect,
            first
        );
        assert_eq!(
            scrolled
                .hit_test(540.0, 2305.0)
                .and_then(|node| node.action.as_deref()),
            Some("list_back")
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
