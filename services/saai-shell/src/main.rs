//! `saai-shell`: the real system Wayland client for SaaiOS (ADR-005),
//! replacing drm-splash.c's hand-rolled UI. Run manually for now
//! (`WAYLAND_DISPLAY=... saai-shell`); ADR-014's auto-launch-as-
//! saai-displayd's-child wiring is Change step 7.
//!
//! S04 Change step 3 built the minimal vertical slice (normal
//! `xdg_shell` toplevel, no new protocols). Change step 4 adds
//! `ext-session-lock-v1` (ADR-015): right after the normal window comes
//! up, this test also requests a session lock, creates a lock surface
//! per output, and renders it in a visibly different color -- the point
//! is to physically confirm the *security* side of ADR-015 works, not
//! just the protocol plumbing: touch should reach only the lock surface
//! while locked, never the toplevel underneath (enforced in
//! saai-displayd's touch routing, not by this client).
//!
//! `wlr-layer-shell` (the other half of ADR-015, for status bar/
//! navigation-style system surfaces) rounds out Change step 4: this
//! test also creates a single top-anchored layer surface (namespace
//! "saai-shell-statusbar-test") and fills it a solid, empirically
//! distinct color -- proving the protocol renders end to end. No real
//! status bar content, and no touch dispatch to it yet (saai-displayd's
//! touch routing still only knows `focused_surface`/`lock_surface`,
//! not layer surfaces) -- both are follow-up work, not this step's
//! goal.
//!
//! Change step 5 ports drm-splash.c's lock/idle behavior: boots locked
//! (matching `bool locked = true` at the top of drm-splash's own main
//! loop -- this was never a Change-4-only test hack, it's the real
//! boot-to-lock-screen behavior a phone is expected to have), unlocks
//! on a touch-down-then-release over the lock surface, and re-locks
//! after 60s of no touch activity while unlocked. Two things
//! drm-splash.c also does are deliberately NOT ported here:
//! - **Display power off.** drm-splash calls `disable_display()`
//!   (blanks the CRTC) on the same idle timeout. That's a DRM
//!   operation only saai-displayd can perform (ADR-005/010: this
//!   client gets no DRM access), and no protocol/IPC to ask for it
//!   exists yet -- needs its own design (`ext-idle-notify-v1` +
//!   `wlr-output-power-management-v1`, or a private mechanism), not
//!   assumed here. The panel simply stays on and shows the lock
//!   surface indefinitely instead of the phone-realistic
//!   dim-then-blank sequence.
//! - **Haptic feedback on unlock.** drm-splash opens `/dev/input/haptic`
//!   directly. This client has no raw evdev access either (same
//!   ADR-005/010 boundary) and there's no existing path to ask
//!   saai-displayd to play a haptic effect on this client's behalf.
//!
//! Both are logged as known limitations in the S04 sprint doc, not
//! silently dropped.
//!
//! Change step 6 ports drm-splash.c's four root sections (`root_page()`,
//! `render_root_controls()`): "Сейчас"/"Входящие"/"Пространства"/"Я",
//! navigable via a bottom tab bar. No real per-section content yet
//! (placeholder-only is explicitly in scope for this step, per the S04
//! sprint doc) and no text rendering exists in this client at all
//! (drm-splash.c has its own bitmap font; porting that is out of scope
//! here) -- each section is a distinct solid color instead, same
//! "color as the physically-verifiable signal" approach already used
//! for the lock surface and layer-shell bar. The tab bar lives inside
//! the toplevel's own buffer, not a separate layer surface: touch
//! routing only knows `focused_surface`/`lock_surface` (a known
//! limitation from Change 4), so a real layer surface couldn't
//! receive the taps that switch pages.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

mod appd_client;
mod entityd_client;
mod portal_server;
mod render;

use saai_app_protocol::{
    AppSummary, LifecycleEventKind, ResponseResult as AppResponseResult,
    ServerMessage as AppServerMessage,
};
use saai_entity_protocol::{
    Entity, EntitydEvent, ResponseResult as EntityResponseResult,
    ServerMessage as EntityServerMessage, Space,
};
use saai_ui_core::{layout, Axis, LayoutNode, Length, Node, Rect};
use serde_json::{json, Map};
use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_seat, wl_shm, wl_surface, wl_touch},
    Connection, QueueHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    delegate_session_lock, delegate_shm, delegate_touch, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{touch::TouchHandler, Capability, SeatHandler, SeatState},
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};

/// Matches drm-splash.c's own idle-to-lock constant.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const DEMO_APP_ID: &str = "org.saaios.demo-surface";
const DEMO_APP_ACTION: &str = "manage_app:org.saaios.demo-surface";
const DEMO_PACKAGE_PATH: &str = "/data/saaios/packages/org.saaios.demo-surface";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TabDefinition {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    action: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentActionDefinition {
    id: &'static str,
    page: &'static str,
    top: u32,
    height: u32,
    label: &'static str,
    action: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/root_sui.rs"));

/// The four root sections (drm-splash.c's `root_page()`/`root_pages`),
/// in bottom-tab-bar order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootPage {
    Now,
    Inbox,
    Spaces,
    Me,
}

impl RootPage {
    fn index(self) -> usize {
        match self {
            RootPage::Now => 0,
            RootPage::Inbox => 1,
            RootPage::Spaces => 2,
            RootPage::Me => 3,
        }
    }

    fn id(self) -> &'static str {
        match self {
            RootPage::Now => "now",
            RootPage::Inbox => "inbox",
            RootPage::Spaces => "spaces",
            RootPage::Me => "me",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DemoAppState {
    Unavailable,
    Missing,
    Installed,
    Running,
    Stopped,
    CrashLimited,
    Pending,
    Error,
}

impl DemoAppState {
    fn from_summary(app: &AppSummary) -> Self {
        match app.state.as_str() {
            "running" => Self::Running,
            "stopped" => Self::Stopped,
            "crash_limited" => Self::CrashLimited,
            _ => Self::Installed,
        }
    }

    fn view(self, label: &'static str) -> render::ActionCardView {
        match self {
            Self::Unavailable => {
                render::ActionCardView::new(label, "Сервис недоступен", "Ожидание")
            }
            Self::Missing => render::ActionCardView::new(label, "Не установлено", "Установить"),
            Self::Installed | Self::Stopped => {
                render::ActionCardView::new(label, "Готово к запуску", "Открыть")
            }
            Self::Running => render::ActionCardView::new(label, "Работает", "Открыто"),
            Self::CrashLimited => {
                render::ActionCardView::new(label, "Остановлено после сбоя", "Повторить")
            }
            Self::Pending => render::ActionCardView::new(label, "Выполняется…", "Подождите"),
            Self::Error => render::ActionCardView::new(label, "Не удалось выполнить", "Повторить"),
        }
    }
}

fn root_view(width: u32, height: u32) -> LayoutNode {
    let tab_height = ((height as u64 * ROOT_TAB_HEIGHT as u64) / 2400) as u32;
    let tabs = Node::linear(
        ROOT_TABS_ID,
        Axis::Horizontal,
        ROOT_TABS
            .iter()
            .map(|tab| Node::leaf(tab.id).with_action(tab.action))
            .collect(),
    )
    .with_size(Length::Fill, Length::Px(tab_height));
    let root = Node::linear(
        ROOT_SCREEN_ID,
        Axis::Vertical,
        vec![Node::leaf(ROOT_CONTENT_ID), tabs],
    );
    layout(&root, Rect::new(0, 0, width, height))
}

fn page_from_id(id: &str) -> Option<RootPage> {
    match id {
        "now" => Some(RootPage::Now),
        "inbox" => Some(RootPage::Inbox),
        "spaces" => Some(RootPage::Spaces),
        "me" => Some(RootPage::Me),
        _ => None,
    }
}

fn page_from_action(action: &str) -> Option<RootPage> {
    action.strip_prefix("select_root:").and_then(page_from_id)
}

/// Touch and rendering consume the same computed Saai UI tree. There is no
/// second set of tab rectangles to drift away from what is drawn.
fn tab_at(pos: (f64, f64), width: u32, height: u32) -> Option<RootPage> {
    if width == 0 || height == 0 {
        return None;
    }
    root_view(width, height)
        .hit_test(pos.0, pos.1)
        .and_then(|node| node.action.as_deref())
        .and_then(page_from_action)
}

const CONSENT_SCREEN_ID: &str = "consent";
const CONSENT_HEADER_ID: &str = "consent-header";
const CONSENT_BUTTONS_ID: &str = "consent-buttons";
const CONSENT_ACCEPT_ID: &str = "consent-accept";
const CONSENT_DECLINE_ID: &str = "consent-decline";
const CONSENT_ACCEPT_ACTION: &str = "consent:accept";
const CONSENT_DECLINE_ACTION: &str = "consent:decline";
/// Matches the root tab bar's own height (ROOT_TAB_HEIGHT) so the two
/// screens share the same bottom-button proportions.
const CONSENT_BUTTON_HEIGHT: u32 = ROOT_TAB_HEIGHT;

/// The requested-capability screen (ADR-020 section 2, S07 Change 4): a
/// second full-screen `ui_tree` layout, built and hit-tested the same way
/// as `root_view()`, shown instead of the normal root content while an
/// app's launch is blocked on consent.
struct PendingConsent {
    app_id: String,
    app_name: &'static str,
    requested: Vec<String>,
}

const INTENT_SCREEN_ID: &str = "intent-input";
const INTENT_HEADER_ID: &str = "intent-header";
const INTENT_ROWS_ID: &str = "intent-rows";
const INTENT_CANCEL_ACTION: &str = "intent:cancel";
const INTENT_SPACE_ACTION: &str = "intent:space";
const INTENT_BACKSPACE_ACTION: &str = "intent:backspace";
const INTENT_SEND_ACTION: &str = "intent:send";
const INTENT_KEY_PREFIX: &str = "intent:key:";
/// Same reasoning as `CONSENT_BUTTON_HEIGHT`: a plain literal, not scaled
/// against `ROOT_TAB_HEIGHT`'s 2400-unit design space, matching how
/// `consent_view()`'s own header/buttons are sized.
const INTENT_HEADER_HEIGHT: u32 = 260;

/// ADR-029's proven touch-hit-test keyboard, grown from its 6-key spike
/// (`H`/`I`/`!`) to a real (if Latin-only -- Cyrillic is future polish,
/// not an architectural question) lowercase QWERTY, per that ADR's own
/// "Последствия" section. Change 2's job is proving the Intent -> Task
/// -> Action -> Result workflow end to end, not re-proving text entry
/// works -- ADR-029 already did that.
const INTENT_KEY_ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

struct IntentControlDef {
    id: &'static str,
    label: &'static str,
    action: &'static str,
}

const INTENT_CONTROLS: [IntentControlDef; 4] = [
    IntentControlDef {
        id: "intent-cancel",
        label: "Отмена",
        action: INTENT_CANCEL_ACTION,
    },
    IntentControlDef {
        id: "intent-space",
        label: "␣",
        action: INTENT_SPACE_ACTION,
    },
    IntentControlDef {
        id: "intent-backspace",
        label: "⌫",
        action: INTENT_BACKSPACE_ACTION,
    },
    IntentControlDef {
        id: "intent-send",
        label: "Отправить",
        action: INTENT_SEND_ACTION,
    },
];

/// Active while the on-screen keyboard (S09 Change 2) is composing a new
/// `saaios.intent`'s text. Modal, same as `PendingConsent` -- owns every
/// touch while it's showing (see `TouchHandler::up()`).
#[derive(Default)]
struct IntentInputState {
    buffer: String,
}

/// What `draw()` renders this frame, computed up front from `&self` before
/// `buffer`/`canvas` take a mutable borrow for the rest of the function.
enum Frame {
    Consent {
        app_name: &'static str,
        labels: Vec<String>,
        header: Rect,
        accept: Rect,
        decline: Rect,
    },
    IntentInput {
        buffer: String,
        header: Rect,
        keys: Vec<(Rect, String)>,
    },
    Root {
        content_rect: Rect,
        tabs: Vec<(Rect, &'static str)>,
        content_cards: Vec<(Rect, render::ActionCardView)>,
        context_label: String,
    },
}

fn intent_key_action(ch: char) -> String {
    format!("{INTENT_KEY_PREFIX}{ch}")
}

/// Built and hit-tested the same way as `root_view()`/`consent_view()`:
/// one `Node`/`layout()` tree, no second set of rectangles for touch.
/// Every row (letters and controls alike) is `Length::Fill` on both
/// axes, so the keyboard reflows to whatever the real panel size is
/// instead of assuming a fixed design canvas.
fn intent_view(width: u32, height: u32) -> LayoutNode {
    let mut rows: Vec<Node> = INTENT_KEY_ROWS
        .iter()
        .enumerate()
        .map(|(row_index, letters)| {
            Node::linear(
                format!("intent-row-{row_index}"),
                Axis::Horizontal,
                letters
                    .chars()
                    .map(|ch| {
                        Node::leaf(format!("intent-key-{ch}")).with_action(intent_key_action(ch))
                    })
                    .collect(),
            )
        })
        .collect();
    rows.push(Node::linear(
        "intent-controls",
        Axis::Horizontal,
        INTENT_CONTROLS
            .iter()
            .map(|control| Node::leaf(control.id).with_action(control.action))
            .collect(),
    ));
    let keyboard = Node::linear(INTENT_ROWS_ID, Axis::Vertical, rows);
    let root = Node::linear(
        INTENT_SCREEN_ID,
        Axis::Vertical,
        vec![
            Node::leaf(INTENT_HEADER_ID).with_size(Length::Fill, Length::Px(INTENT_HEADER_HEIGHT)),
            keyboard,
        ],
    );
    layout(&root, Rect::new(0, 0, width, height))
}

fn intent_action_at(pos: (f64, f64), width: u32, height: u32) -> Option<String> {
    if width == 0 || height == 0 {
        return None;
    }
    intent_view(width, height)
        .hit_test(pos.0, pos.1)
        .and_then(|node| node.action.clone())
}

fn consent_view(width: u32, height: u32) -> LayoutNode {
    let buttons = Node::linear(
        CONSENT_BUTTONS_ID,
        Axis::Horizontal,
        vec![
            Node::leaf(CONSENT_ACCEPT_ID).with_action(CONSENT_ACCEPT_ACTION),
            Node::leaf(CONSENT_DECLINE_ID).with_action(CONSENT_DECLINE_ACTION),
        ],
    )
    .with_size(Length::Fill, Length::Px(CONSENT_BUTTON_HEIGHT));
    let root = Node::linear(
        CONSENT_SCREEN_ID,
        Axis::Vertical,
        vec![Node::leaf(CONSENT_HEADER_ID), buttons],
    );
    layout(&root, Rect::new(0, 0, width, height))
}

/// `Some(true)` for accept, `Some(false)` for decline, `None` if the touch
/// missed both buttons -- same tree `draw()` renders from, per the same
/// "no second set of rectangles" rule as `tab_at()`.
fn consent_action_at(pos: (f64, f64), width: u32, height: u32) -> Option<bool> {
    if width == 0 || height == 0 {
        return None;
    }
    match consent_view(width, height)
        .hit_test(pos.0, pos.1)
        .and_then(|node| node.action.as_deref())
    {
        Some(CONSENT_ACCEPT_ACTION) => Some(true),
        Some(CONSENT_DECLINE_ACTION) => Some(false),
        _ => None,
    }
}

/// Human-readable (Russian) label for one ADR-020 vocabulary entry. Falls
/// back to the raw dotted name for anything not yet in this list, so an
/// unrecognized capability is still visible, not silently dropped from the
/// screen.
fn capability_label(name: &str) -> &str {
    match name {
        "space.entities.read" => "Чтение объектов пространства",
        "space.entities.write" => "Изменение объектов пространства",
        "net.internet" => "Доступ в интернет",
        "clipboard.read" => "Чтение буфера обмена",
        "clipboard.write" => "Запись в буфер обмена",
        "portal.open_file" => "Выбор файла",
        other => other,
    }
}

fn content_action_rect(action: &ContentActionDefinition, width: u32, height: u32) -> Rect {
    let margin = width / 22;
    let top = ((action.top as u64 * height as u64) / 2400) as u32;
    let action_height = ((action.height as u64 * height as u64) / 2400) as u32;
    Rect::new(
        margin,
        top,
        width.saturating_sub(margin.saturating_mul(2)),
        action_height,
    )
}

fn content_action_at(
    page: RootPage,
    pos: (f64, f64),
    width: u32,
    height: u32,
) -> Option<ContentActionDefinition> {
    ROOT_CONTENT_ACTIONS.iter().copied().find(|action| {
        action.page == page.id()
            && content_action_rect(action, width, height).contains(pos.0, pos.1)
    })
}

fn main() {
    let conn = Connection::connect_to_env().expect("failed to connect to Wayland display");
    let (globals, event_queue) = registry_queue_init(&conn).expect("failed to init registry");
    let qh = event_queue.handle();

    let mut event_loop: smithay_client_toolkit::reexports::calloop::EventLoop<'static, Shell> =
        smithay_client_toolkit::reexports::calloop::EventLoop::try_new()
            .expect("failed to create event loop");
    smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource::new(
        conn.clone(),
        event_queue,
    )
    .insert(event_loop.handle())
    .expect("failed to insert Wayland source");

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg_wm_base not available");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");
    let session_lock_state = SessionLockState::new(&globals, &qh);
    let layer_shell = LayerShell::bind(&globals, &qh).expect("wlr-layer-shell not available");

    let surface = compositor.create_surface(&qh);
    let window = xdg_shell.create_window(surface, WindowDecorations::ServerDefault, &qh);
    window.set_title("SaaiOS");
    window.set_app_id("org.saaios.shell");
    // Fullscreen, not a resizable desktop window -- saai-shell is the
    // system shell, not an app; matches drm-splash.c's own fixed
    // 1080x2400 panel assumption for now (real multi-output handling is
    // future work, not this vertical slice).
    //
    // set_min_size() alone does NOT request fullscreen -- it only
    // constrains resizing, so the server was free to configure whatever
    // size it wanted (observed on hardware: 800x480, the same default
    // saai-displayd hands out when it has no better information). This
    // was a real, previously-unnoticed bug: every visual test this
    // sprint ran against an 800x480 toplevel, not the real panel.
    window.set_min_size(Some((1080, 2400)));
    window.set_fullscreen(None);
    window.commit();

    // Second half of ADR-015 (Change 4): a real system-surface layer,
    // for the status bar/nav-style content the four root sections will
    // eventually need (Change 6) -- this test slice just proves the
    // protocol renders, no real content yet.
    let bar_surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(
        &qh,
        bar_surface,
        Layer::Top,
        Some("saai-shell-statusbar-test"),
        None,
    );
    layer.set_anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
    layer.set_size(0, 120);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    // Initial commit with no attached buffer -- required by the
    // protocol before the compositor will send the first configure
    // (mirrors the toolkit's own simple_layer.rs example).
    layer.commit();

    let pool = SlotPool::new(1080 * 2400 * 4, &shm).expect("failed to create SHM pool");

    let fonts = match render::Fonts::load_system() {
        Ok(fonts) => Some(fonts),
        Err(error) => {
            eprintln!("saai-shell: Inter unavailable, continuing without text: {error}");
            None
        }
    };

    let appd_socket = std::env::var_os("SAAIOS_APPD_SOCKET")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/run/saaios/appd.sock".into());
    let entityd_socket = std::env::var_os("SAAIOS_ENTITYD_SOCKET")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/run/saaios/entityd.sock".into());
    let portal_socket = std::env::var_os("SAAIOS_PORTAL_SOCKET")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/run/saaios/portal.sock".into());
    let mut shell = Shell {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        touch: None,
        compositor,
        shm,
        exit: false,
        first_configure: true,
        pool,
        width: 1080,
        height: 2400,
        buffer: None,
        window,
        session_lock_state,
        session_lock: None,
        lock_surfaces: Vec::new(),
        lock_pool: None,
        lock_buffer: None,
        locked: true,
        unlock_pending: false,
        last_activity: Instant::now(),
        current_page: RootPage::Now,
        last_touch_pos: (0.0, 0.0),
        tab_touch_pending: false,
        layer,
        layer_width: 0,
        layer_height: 120,
        layer_pool: None,
        layer_buffer: None,
        fonts,
        appd: appd_client::AppdClient::new(appd_socket),
        demo_app_state: DemoAppState::Unavailable,
        pending_consent: None,
        intent_input: None,
        entityd: entityd_client::EntitydClient::new(entityd_socket),
        spaces: Vec::new(),
        selected_space_id: "home".into(),
        entity_counts: BTreeMap::new(),
        selected_entities: Vec::new(),
        portal: portal_server::PortalServer::bind(portal_socket)
            .expect("failed to bind saai-shell portal socket"),
        apps_by_pid: BTreeMap::new(),
        apps_grants: BTreeMap::new(),
        clipboard: None,
        last_apps_refresh: Instant::now(),
    };

    println!("saai-shell: connected, toplevel created");

    // Boots locked, matching drm-splash.c's own `bool locked = true` at
    // the top of its main loop -- a phone that boots straight to an
    // unlocked launcher would be a real regression, not a simplification.
    shell.session_lock = Some(
        shell
            .session_lock_state
            .lock(&qh)
            .expect("ext-session-lock-v1 not supported by saai-displayd"),
    );
    println!("saai-shell: session lock requested");

    while !shell.exit {
        event_loop
            .dispatch(Duration::from_millis(16), &mut shell)
            .expect("event loop dispatch failed");
        shell.poll_appd(&conn, &qh);
        shell.poll_entityd(&conn, &qh);
        shell.refresh_apps_if_due();
        shell.poll_portal();
        shell.check_idle_timeout(&qh);
    }
}

struct Shell {
    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,
    touch: Option<wl_touch::WlTouch>,
    compositor: CompositorState,
    shm: Shm,

    exit: bool,
    first_configure: bool,
    pool: SlotPool,
    width: u32,
    height: u32,
    buffer: Option<Buffer>,
    window: Window,

    session_lock_state: SessionLockState,
    session_lock: Option<SessionLock>,
    lock_surfaces: Vec<SessionLockSurface>,
    /// Kept alive for as long as the lock surface's buffer is attached --
    /// dropping the pool would unmap the shared memory the compositor
    /// still needs to read after `commit()` returns.
    lock_pool: Option<SlotPool>,
    lock_buffer: Option<Buffer>,
    /// Mirrors saai-displayd's own `locked` bool -- this client is the
    /// only one that ever calls lock()/unlock(), so tracking it here
    /// (rather than round-tripping through the server) is enough to
    /// drive the idle timer and the touch-to-unlock gesture.
    locked: bool,
    /// Set on a touch-down that started on the lock surface while
    /// locked; the matching touch-up is what actually unlocks (mirrors
    /// drm-splash.c requiring touch *release* over the lock screen, not
    /// just a touch-start, so a drag-through or accidental brush
    /// doesn't unlock).
    unlock_pending: bool,
    last_activity: Instant,
    /// Currently visible root section (Change step 6).
    current_page: RootPage,
    /// Last known touch position (from `down()`/`motion()`) -- `up()`
    /// doesn't carry a position itself, so this is what tells it where
    /// the touch actually ended for tab-bar hit-testing.
    last_touch_pos: (f64, f64),
    /// Set on a touch-down over the toplevel's own tab bar while
    /// unlocked; the matching touch-up is what actually switches pages
    /// (same "release, not press" rule as `unlock_pending`, so a drag
    /// through the tab bar doesn't switch pages by accident).
    tab_touch_pending: bool,

    layer: LayerSurface,
    layer_width: u32,
    layer_height: u32,
    layer_pool: Option<SlotPool>,
    layer_buffer: Option<Buffer>,
    fonts: Option<render::Fonts>,
    appd: appd_client::AppdClient,
    demo_app_state: DemoAppState,
    /// Set while a launch is blocked on the ADR-020 consent screen -- see
    /// `apply_appd_message`'s `ConsentRequired`/`ConsentDecided` handling.
    pending_consent: Option<PendingConsent>,
    /// S09 Change 2: set while the on-screen keyboard is composing a new
    /// `saaios.intent`. Modal, same as `pending_consent`.
    intent_input: Option<IntentInputState>,
    entityd: entityd_client::EntitydClient,
    spaces: Vec<Space>,
    selected_space_id: String,
    entity_counts: BTreeMap<String, usize>,
    selected_entities: Vec<Entity>,
    /// ADR-020 section 8 / S07 Change 7: the portal socket sandboxed apps
    /// connect to for `clipboard.read`/`clipboard.write`/`portal.open_file`.
    portal: portal_server::PortalServer,
    /// Resolves a portal connection's `SO_PEERCRED` pid to an app_id --
    /// rebuilt from every `list()` response and kept current between
    /// refreshes by `Running`/`Removed` lifecycle events (see
    /// `update_app_caches`).
    apps_by_pid: BTreeMap<u32, String>,
    /// Canonical granted-capability names per app_id, from the same
    /// `AppSummary.granted_capabilities` `saai-appd` now reports -- what
    /// the portal actually authorizes requests against.
    apps_grants: BTreeMap<String, Vec<String>>,
    /// The portal's in-memory clipboard. No persistence, no history --
    /// cleared on `saai-shell` restart.
    clipboard: Option<String>,
    last_apps_refresh: Instant,
}

impl CompositorHandler for Shell {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // The shell is event-driven. Presentation feedback must not
        // redraw an unchanged full-screen scene forever.
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for Shell {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl WindowHandler for Shell {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        self.buffer = None;
        self.width = configure.new_size.0.map(|v| v.get()).unwrap_or(1080);
        self.height = configure.new_size.1.map(|v| v.get()).unwrap_or(2400);

        if self.first_configure {
            self.first_configure = false;
            println!(
                "saai-shell: first configure at {}x{}, drawing placeholder",
                self.width, self.height
            );
            self.draw(conn, qh);
        }
    }
}

impl ShmHandler for Shell {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl SessionLockHandler for Shell {
    fn locked(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, session_lock: SessionLock) {
        println!("saai-shell: session locked, creating lock surface(s)");
        self.locked = true;
        for output in self.output_state.outputs() {
            let surface = self.compositor.create_surface(qh);
            let lock_surface = session_lock.create_lock_surface(surface, &output, qh);
            self.lock_surfaces.push(lock_surface);
        }
    }

    fn finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
        // Compositor refused or dropped the lock (e.g. ext-session-lock-v1
        // not implemented). Not fatal for this test client -- keep
        // running as a plain toplevel.
        println!("saai-shell: session lock finished (refused or dropped)");
        self.session_lock = None;
        self.lock_surfaces.clear();
        self.locked = false;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        session_lock_surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;
        let (width, height) = (width.max(1), height.max(1));
        println!("saai-shell: lock surface configure at {width}x{height}");

        let stride = width as i32 * 4;
        let pool = self.lock_pool.get_or_insert_with(|| {
            SlotPool::new(width as usize * height as usize * 4, &self.shm)
                .expect("create lock surface pool")
        });
        let (buffer, canvas) = pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Xrgb8888,
            )
            .expect("create buffer");

        // Bright red -- deliberately unmistakable against the toplevel's
        // dark slate placeholder, so a photo of the panel makes it
        // obvious which surface is actually receiving the compositor's
        // output while locked.
        //
        // Byte order here is [0x00, 0xd0, 0x00, 0x00], *not* the
        // [B, G, R, X] a standard XRGB8888 LE layout would predict for
        // red. A three-band on-device diagnostic (one solid color per
        // byte position, read back directly from this pool's memfd via
        // /proc/<pid>/fd to confirm the client-side write itself before
        // ever trusting the photo) proved this panel's pipeline reads R
        // from byte-index 1 and G from byte-index 2 -- swapped from the
        // conventional B,G,R,X -- while byte-index 0 produced no visible
        // output at all in the same test (untested whether that's a true
        // "blue" that just read as too dark to name, or genuinely
        // unused; not re-verified here since only red is needed for this
        // milestone). Root cause on the DRM/driver side not identified --
        // no standard fourcc swaps R and G while leaving B in place, so
        // this is applied as an empirically-verified byte order, not a
        // fourcc fix.
        let pixel: [u8; 4] = [0x00, 0xd0, 0x00, 0x00];
        for chunk in canvas.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pixel);
        }

        let surface = session_lock_surface.wl_surface();
        surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
        self.lock_buffer = Some(buffer);
    }
}

impl LayerShellHandler for Shell {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        // Not fatal for this test client, same stance as SessionLockHandler::finished --
        // keep running as a plain toplevel if the compositor takes the layer surface away.
        println!("saai-shell: layer surface closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;
        let (width, height) = (width.max(1), height.max(1));
        println!("saai-shell: layer surface configure at {width}x{height}");
        self.layer_width = width;
        self.layer_height = height;

        let stride = width as i32 * 4;
        let pool = self.layer_pool.get_or_insert_with(|| {
            SlotPool::new(width as usize * height as usize * 4, &self.shm)
                .expect("create layer surface pool")
        });
        let (buffer, canvas) = pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Xrgb8888,
            )
            .expect("create buffer");

        // The status surface is part of the permanent phone chrome now, not
        // the old yellow protocol probe. Its panel packing comes from the
        // same calibrated palette as the toplevel renderer.
        let pixel = render::BACKGROUND;
        for chunk in canvas.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pixel);
        }

        let surface = self.layer.wl_surface();
        surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(surface).expect("buffer attach");
        self.layer.commit();
        self.layer_buffer = Some(buffer);
    }
}

impl SeatHandler for Shell {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        // No pointer/keyboard handling -- this is a touchscreen-only
        // device (ADR-012 already made the same call for saai-displayd).
        if capability == Capability::Touch && self.touch.is_none() {
            match self.seat_state.get_touch(qh, &seat) {
                Ok(touch) => self.touch = Some(touch),
                Err(err) => eprintln!("saai-shell: failed to get wl_touch: {err}"),
            }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Touch {
            if let Some(touch) = self.touch.take() {
                touch.release();
            }
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
    }
}

impl TouchHandler for Shell {
    fn down(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        surface: wl_surface::WlSurface,
        _id: i32,
        position: (f64, f64),
    ) {
        self.last_activity = Instant::now();
        self.last_touch_pos = position;
        self.unlock_pending = self.locked
            && self
                .lock_surfaces
                .iter()
                .any(|ls| *ls.wl_surface() == surface);
        self.tab_touch_pending = !self.locked && surface == *self.window.wl_surface();
    }

    fn up(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        _id: i32,
    ) {
        self.last_activity = Instant::now();
        // Release, not just touch-start, is what unlocks -- matches
        // drm-splash.c's own `touch_released` gate, so a drag that
        // starts on the lock surface but ends elsewhere (or a
        // multi-touch gesture) doesn't unlock by accident.
        if self.unlock_pending {
            self.unlock_pending = false;
            if let Some(session_lock) = self.session_lock.take() {
                session_lock.unlock();
            }
            self.lock_surfaces.clear();
            self.locked = false;
            println!("saai-shell: unlocked by touch");
        } else if self.tab_touch_pending {
            self.tab_touch_pending = false;
            if self.pending_consent.is_some() {
                // Modal: the consent screen owns every touch while it is
                // showing, not the tab bar or content cards underneath it.
                if let Some(accept) =
                    consent_action_at(self.last_touch_pos, self.width, self.height)
                {
                    if let Some(pending) = self.pending_consent.take() {
                        println!(
                            "saai-shell: consent {} for {}",
                            if accept { "accepted" } else { "declined" },
                            pending.app_id
                        );
                        self.appd.decide_consent(pending.app_id, accept);
                    }
                    self.draw(conn, qh);
                }
            } else if self.intent_input.is_some() {
                // Modal, same as consent: the on-screen keyboard owns
                // every touch while it's showing.
                if let Some(action) = intent_action_at(self.last_touch_pos, self.width, self.height)
                {
                    self.handle_intent_input_action(&action, conn, qh);
                }
            } else if let Some(page) = tab_at(self.last_touch_pos, self.width, self.height) {
                if page != self.current_page {
                    println!("saai-shell: switched to {page:?}");
                    self.current_page = page;
                    self.draw(conn, qh);
                }
            } else if let Some(action) = content_action_at(
                self.current_page,
                self.last_touch_pos,
                self.width,
                self.height,
            ) {
                self.invoke_content_action(action, conn, qh);
            }
        }
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _time: u32,
        _id: i32,
        position: (f64, f64),
    ) {
        self.last_touch_pos = position;
    }

    fn shape(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _id: i32,
        _major: f64,
        _minor: f64,
    ) {
    }

    fn orientation(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _id: i32,
        _orientation: f64,
    ) {
    }

    fn cancel(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _touch: &wl_touch::WlTouch) {
        self.unlock_pending = false;
        self.tab_touch_pending = false;
    }
}

impl Shell {
    /// Renders the active root section's placeholder content plus the
    /// bottom tab bar (Change step 6) -- proves the real
    /// client<->compositor vertical slice end to end (surface
    /// creation, configure, SHM buffer and commit), same
    /// as the single dark-slate fill this replaced, just with content
    /// that actually changes on navigation instead of a static color.
    fn draw(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;
        let stride = width as i32 * 4;

        // Every `&self` read this frame needs (content cards, context
        // label, consent labels) happens here, before `buffer`/`canvas`
        // take a mutable borrow tied to `self.buffer`/`self.pool` for the
        // rest of the function -- `self.content_card()`/`self.context_
        // label()` need the whole of `self`, not just those two fields.
        let frame = if let Some(pending) = &self.pending_consent {
            let view = consent_view(width, height);
            let header = view.children[0].rect;
            let buttons = &view.children[1].children;
            let labels = pending
                .requested
                .iter()
                .map(|name| capability_label(name).to_owned())
                .collect::<Vec<_>>();
            Frame::Consent {
                app_name: pending.app_name,
                labels,
                header,
                accept: buttons[0].rect,
                decline: buttons[1].rect,
            }
        } else if let Some(state) = &self.intent_input {
            let view = intent_view(width, height);
            let header = view.children[0].rect;
            let keyboard_rows = &view.children[1].children;
            let mut keys = Vec::new();
            for (row_index, letters) in INTENT_KEY_ROWS.iter().enumerate() {
                let row_node = &keyboard_rows[row_index];
                for (key_node, ch) in row_node.children.iter().zip(letters.chars()) {
                    keys.push((key_node.rect, ch.to_uppercase().to_string()));
                }
            }
            let controls_node = &keyboard_rows[INTENT_KEY_ROWS.len()];
            for (key_node, control) in controls_node.children.iter().zip(INTENT_CONTROLS.iter()) {
                keys.push((key_node.rect, control.label.to_string()));
            }
            Frame::IntentInput {
                buffer: state.buffer.clone(),
                header,
                keys,
            }
        } else {
            let view = root_view(width, height);
            let content_rect = view.children[0].rect;
            let tabs = view.children[1]
                .children
                .iter()
                .zip(ROOT_TABS)
                .map(|(node, tab)| (node.rect, tab.label))
                .collect::<Vec<_>>();
            let content_cards = ROOT_CONTENT_ACTIONS
                .iter()
                .filter(|action| action.page == self.current_page.id())
                .map(|action| {
                    (
                        content_action_rect(action, width, height),
                        self.content_card(action),
                    )
                })
                .collect::<Vec<_>>();
            let context_label = self.context_label();
            Frame::Root {
                content_rect,
                tabs,
                content_cards,
                context_label,
            }
        };

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Xrgb8888,
                )
                .expect("create buffer")
                .0
        });

        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (second_buffer, canvas) = self
                    .pool
                    .create_buffer(
                        width as i32,
                        height as i32,
                        stride,
                        wl_shm::Format::Xrgb8888,
                    )
                    .expect("create buffer");
                *buffer = second_buffer;
                canvas
            }
        };

        match frame {
            Frame::Consent {
                app_name,
                labels,
                header,
                accept,
                decline,
            } => {
                render::draw_consent(
                    &mut render::Canvas::new(canvas, width, height),
                    app_name,
                    &labels,
                    header,
                    accept,
                    decline,
                    self.fonts.as_ref(),
                );
            }
            Frame::IntentInput {
                buffer,
                header,
                keys,
            } => {
                render::draw_intent_input(
                    &mut render::Canvas::new(canvas, width, height),
                    &buffer,
                    header,
                    &keys,
                    self.fonts.as_ref(),
                );
            }
            Frame::Root {
                content_rect,
                tabs,
                content_cards,
                context_label,
            } => {
                render::draw_root(
                    &mut render::Canvas::new(canvas, width, height),
                    content_rect,
                    &tabs,
                    self.current_page.index(),
                    &context_label,
                    self.fonts.as_ref(),
                    &content_cards,
                );
            }
        }

        self.window
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        buffer
            .attach_to(self.window.wl_surface())
            .expect("buffer attach");
        self.window.commit();
    }

    fn invoke_content_action(
        &mut self,
        action: ContentActionDefinition,
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        println!(
            "saai-shell: invoked content action {} ({})",
            action.id, action.action
        );
        if let Some(space_id) = action.action.strip_prefix("select_space:") {
            if self.entityd.is_connected() {
                self.entityd.select_space(space_id);
            }
            return;
        }
        if action.action == "open_intent_input" {
            self.intent_input = Some(IntentInputState::default());
            self.draw(conn, qh);
            return;
        }
        if action.action != DEMO_APP_ACTION {
            return;
        }
        match self.demo_app_state {
            DemoAppState::Missing | DemoAppState::Error => {
                self.appd.install(DEMO_PACKAGE_PATH);
                self.demo_app_state = DemoAppState::Pending;
            }
            DemoAppState::Installed | DemoAppState::Stopped | DemoAppState::CrashLimited => {
                self.appd.launch(DEMO_APP_ID);
                self.demo_app_state = DemoAppState::Pending;
            }
            DemoAppState::Unavailable | DemoAppState::Running | DemoAppState::Pending => return,
        }
        self.draw(conn, qh);
    }

    /// S09 Change 2: drives the on-screen keyboard opened by
    /// `open_intent_input`. `intent:send` is the one path that talks to
    /// `saai-entityd` -- everything else only touches `self.intent_input`'s
    /// local buffer.
    fn handle_intent_input_action(
        &mut self,
        action: &str,
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let Some(state) = self.intent_input.as_mut() else {
            return;
        };
        match action {
            INTENT_CANCEL_ACTION => {
                self.intent_input = None;
            }
            INTENT_SPACE_ACTION => {
                state.buffer.push(' ');
            }
            INTENT_BACKSPACE_ACTION => {
                state.buffer.pop();
            }
            INTENT_SEND_ACTION => {
                let text = state.buffer.trim().to_string();
                if !text.is_empty() && self.entityd.is_connected() {
                    let mut properties = Map::new();
                    properties.insert("text".into(), json!(text));
                    // "saaios.intent" -- must match saai-taskd's own
                    // model::INTENT_TYPE (ADR-030). The two crates share
                    // no dependency by design, so this is a convention,
                    // not a compile-time guarantee.
                    self.entityd.create_entity(
                        self.selected_space_id.clone(),
                        "saaios.intent",
                        &text,
                        properties,
                    );
                    println!("saai-shell: submitted intent \"{text}\"");
                }
                self.intent_input = None;
            }
            other => {
                if let Some(key) = other.strip_prefix(INTENT_KEY_PREFIX) {
                    if let Some(ch) = key.chars().next() {
                        state.buffer.push(ch);
                    }
                }
            }
        }
        self.draw(conn, qh);
    }

    fn poll_appd(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        let was_available = self.appd.is_connected();
        let messages = self.appd.poll();
        let available = self.appd.is_connected();
        let mut changed = was_available != available;
        if !available {
            changed |= self.demo_app_state != DemoAppState::Unavailable;
            self.demo_app_state = DemoAppState::Unavailable;
        }
        for message in messages {
            changed |= self.apply_appd_message(message);
        }
        if changed && !self.first_configure {
            self.draw(conn, qh);
        }
    }

    /// Re-issues `list()` roughly once a second while connected, bounding
    /// how stale `apps_by_pid`/`apps_grants` can get between the events
    /// that already update them incrementally (`Running`, `Removed`,
    /// `ConsentDecided`) -- without this, the portal's authorization check
    /// would only ever see a fresh pid/grant snapshot right after a
    /// reconnect, which for a long-lived shell process could be hours ago.
    fn refresh_apps_if_due(&mut self) {
        if self.appd.is_connected() && self.last_apps_refresh.elapsed() >= Duration::from_secs(1) {
            self.appd.list();
            self.last_apps_refresh = Instant::now();
        }
    }

    fn poll_portal(&mut self) {
        self.portal
            .poll(&self.apps_by_pid, &self.apps_grants, &mut self.clipboard);
    }

    /// Keeps the portal's authorization caches (`apps_by_pid`,
    /// `apps_grants`) current -- separate from `apply_appd_message`'s own
    /// match below because that one is still filtering everything down to
    /// the single hardcoded `DEMO_APP_ID` card (a known, pre-existing
    /// limitation of this UI, not something Change 7 needs to fix), while
    /// the portal has to authorize *any* installed app, not just the demo.
    fn update_app_caches(&mut self, message: &AppServerMessage) {
        match message {
            AppServerMessage::Response {
                result: Some(AppResponseResult::List { apps }),
                ..
            } => {
                // A List response is authoritative for the whole registry --
                // rebuilding from scratch here means a removed app's stale
                // pid can never survive past the next refresh, even if some
                // lifecycle event was missed.
                self.apps_by_pid.clear();
                self.apps_grants.clear();
                for app in apps {
                    self.note_app_summary(app);
                }
            }
            AppServerMessage::Response {
                result: Some(AppResponseResult::Installed { app }),
                ..
            } => self.note_app_summary(app),
            AppServerMessage::Response {
                result: Some(AppResponseResult::ConsentDecided { app_id, granted }),
                ..
            } => {
                self.apps_grants.insert(app_id.clone(), granted.clone());
            }
            AppServerMessage::Event { event, .. } => match event.event {
                LifecycleEventKind::Running => {
                    if let Some(pid) = event.pid {
                        self.apps_by_pid.insert(pid, event.app_id.clone());
                    }
                }
                LifecycleEventKind::Stopped
                | LifecycleEventKind::Crashed
                | LifecycleEventKind::CrashLimited
                | LifecycleEventKind::Removed => {
                    if let Some(pid) = event.pid {
                        self.apps_by_pid.remove(&pid);
                    }
                    if event.event == LifecycleEventKind::Removed {
                        self.apps_by_pid.retain(|_, app_id| *app_id != event.app_id);
                        self.apps_grants.remove(&event.app_id);
                    }
                }
                LifecycleEventKind::Installed => {}
            },
            _ => {}
        }
    }

    fn note_app_summary(&mut self, app: &AppSummary) {
        self.apps_grants
            .insert(app.id.clone(), app.granted_capabilities.clone());
        for &pid in &app.pids {
            self.apps_by_pid.insert(pid, app.id.clone());
        }
    }

    fn apply_appd_message(&mut self, message: AppServerMessage) -> bool {
        self.update_app_caches(&message);
        let previous = self.demo_app_state;
        match message {
            AppServerMessage::Response { ok: false, .. } => {
                self.demo_app_state = DemoAppState::Error
            }
            AppServerMessage::Response {
                result: Some(AppResponseResult::List { apps }),
                ..
            } => {
                self.demo_app_state = apps
                    .iter()
                    .find(|app| app.id == DEMO_APP_ID)
                    .map(DemoAppState::from_summary)
                    .unwrap_or(DemoAppState::Missing);
            }
            AppServerMessage::Response {
                result: Some(AppResponseResult::Installed { app }),
                ..
            } if app.id == DEMO_APP_ID => self.demo_app_state = DemoAppState::Installed,
            AppServerMessage::Response {
                result: Some(AppResponseResult::Launched { app_id, .. }),
                ..
            } if app_id == DEMO_APP_ID => self.demo_app_state = DemoAppState::Running,
            AppServerMessage::Response {
                result: Some(AppResponseResult::Stopped { app_id, .. }),
                ..
            } if app_id == DEMO_APP_ID => self.demo_app_state = DemoAppState::Stopped,
            AppServerMessage::Response {
                result: Some(AppResponseResult::Removed { app_id, .. }),
                ..
            } if app_id == DEMO_APP_ID => self.demo_app_state = DemoAppState::Missing,
            AppServerMessage::Response {
                result: Some(AppResponseResult::ConsentRequired { app_id, requested }),
                ..
            } if app_id == DEMO_APP_ID => {
                self.pending_consent = Some(PendingConsent {
                    app_id,
                    app_name: "Saai Demo",
                    requested,
                });
                // The launch that triggered this is refused, not merely
                // delayed -- back to a launchable state, not stuck Pending.
                self.demo_app_state = DemoAppState::Installed;
            }
            AppServerMessage::Response {
                result: Some(AppResponseResult::ConsentDecided { app_id, .. }),
                ..
            } if app_id == DEMO_APP_ID => {
                self.pending_consent = None;
                // Accept or decline, the daemon now has a decision that
                // covers this request -- retry the launch the user
                // originally asked for; it can no longer come back as
                // ConsentRequired for the same capability set.
                self.appd.launch(app_id);
                self.demo_app_state = DemoAppState::Pending;
            }
            AppServerMessage::Event { event, .. } if event.app_id == DEMO_APP_ID => {
                self.demo_app_state = match event.event {
                    LifecycleEventKind::Installed => DemoAppState::Installed,
                    LifecycleEventKind::Running => DemoAppState::Running,
                    LifecycleEventKind::Stopped => DemoAppState::Stopped,
                    LifecycleEventKind::Crashed => DemoAppState::Pending,
                    LifecycleEventKind::CrashLimited => DemoAppState::CrashLimited,
                    LifecycleEventKind::Removed => DemoAppState::Missing,
                };
            }
            _ => {}
        }
        self.demo_app_state != previous
    }

    fn content_card(&self, action: &ContentActionDefinition) -> render::ActionCardView {
        if action.action == DEMO_APP_ACTION {
            return self.demo_app_state.view(action.label);
        }
        if action.action == "inspect_selected_entity" {
            return match self.selected_entities.first() {
                Some(entity) => render::ActionCardView::new(
                    entity.title.clone(),
                    format!("{} · версия {}", entity.entity_type, entity.revision),
                    "Локально",
                ),
                None if self.entityd.is_connected() => render::ActionCardView::new(
                    action.label,
                    "В этом пространстве пока пусто",
                    "Нет объектов",
                ),
                None => render::ActionCardView::new(
                    action.label,
                    "Сервис пространств недоступен",
                    "Ожидание",
                ),
            };
        }
        if action.action == "open_intent_input" {
            let status = if self.entityd.is_connected() {
                "Ввести текст намерения"
            } else {
                "Сервис пространств недоступен"
            };
            return render::ActionCardView::new(action.label, status, "Открыть");
        }
        if let Some(space_id) = action.action.strip_prefix("select_space:") {
            let selected = space_id == self.selected_space_id;
            let status = if self.entityd.is_connected() {
                match self.entity_counts.get(space_id) {
                    Some(count) => format!("Объектов: {count}"),
                    None => "Загрузка объектов…".into(),
                }
            } else {
                "Сервис пространств недоступен".into()
            };
            return render::ActionCardView::new(
                action.label,
                status,
                if selected {
                    "Выбрано"
                } else {
                    "Открыть"
                },
            )
            .selected(selected);
        }
        render::ActionCardView::new(action.label, "", "")
    }

    fn context_label(&self) -> String {
        self.spaces
            .iter()
            .find(|space| space.id == self.selected_space_id)
            .map(|space| space.name.clone())
            .unwrap_or_else(|| match self.selected_space_id.as_str() {
                "home" => "Дом".into(),
                "work" => "Работа".into(),
                "personal" => "Личное".into(),
                "saaios" => "SaaiOS".into(),
                other => other.to_owned(),
            })
    }

    fn poll_entityd(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        let was_available = self.entityd.is_connected();
        let messages = self.entityd.poll();
        let available = self.entityd.is_connected();
        let mut changed = was_available != available;
        if !available && was_available {
            self.spaces.clear();
            self.entity_counts.clear();
            self.selected_entities.clear();
        }
        for message in messages {
            changed |= self.apply_entityd_message(message);
        }
        if changed && !self.first_configure {
            self.draw(conn, qh);
        }
    }

    fn apply_entityd_message(&mut self, message: EntityServerMessage) -> bool {
        let mut changed = false;
        match message {
            EntityServerMessage::Response {
                ok: true,
                result: Some(result),
                ..
            } => match *result {
                EntityResponseResult::Spaces { spaces } => {
                    let ids = spaces
                        .iter()
                        .map(|space| space.id.clone())
                        .collect::<Vec<_>>();
                    changed = self.spaces != spaces;
                    self.spaces = spaces;
                    for id in ids {
                        self.entityd.list_entities(id);
                    }
                }
                EntityResponseResult::Selection { selection } => {
                    changed = self.selected_space_id != selection.space_id;
                    self.selected_space_id = selection.space_id.clone();
                    self.entityd.list_entities(selection.space_id);
                }
                EntityResponseResult::Entities {
                    space_id,
                    mut entities,
                } => {
                    entities.sort_by_key(|entity| std::cmp::Reverse(entity.updated_at));
                    changed |= self.entity_counts.get(&space_id) != Some(&entities.len());
                    self.entity_counts.insert(space_id.clone(), entities.len());
                    if space_id == self.selected_space_id {
                        changed |= self.selected_entities != entities;
                        self.selected_entities = entities;
                    }
                }
                EntityResponseResult::Entity { entity, .. } => {
                    self.entityd.list_entities(entity.space_id);
                }
                EntityResponseResult::Deleted { space_id, .. } => {
                    self.entityd.list_entities(space_id);
                }
                EntityResponseResult::Subscribed => {}
            },
            EntityServerMessage::Event {
                event: EntitydEvent::SelectionChanged { selection },
                ..
            } => {
                changed = self.selected_space_id != selection.space_id;
                self.selected_space_id = selection.space_id.clone();
                self.entityd.list_entities(selection.space_id);
            }
            EntityServerMessage::Event {
                event: EntitydEvent::EntityChanged { record },
                ..
            } => self.entityd.list_entities(record.space_id),
            EntityServerMessage::Response { ok: false, .. } => changed = true,
            _ => {}
        }
        changed
    }

    /// Re-locks after `IDLE_TIMEOUT` of no touch activity while
    /// unlocked -- matches drm-splash.c's own 1s-granularity idle poll,
    /// just driven by this event loop's existing 16ms tick instead of a
    /// separate timer source.
    fn check_idle_timeout(&mut self, qh: &QueueHandle<Self>) {
        if self.locked || self.last_activity.elapsed() < IDLE_TIMEOUT {
            return;
        }
        println!("saai-shell: idle timeout, locking");
        self.last_activity = Instant::now();
        match self.session_lock_state.lock(qh) {
            Ok(session_lock) => self.session_lock = Some(session_lock),
            Err(err) => eprintln!("saai-shell: failed to re-lock: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        capability_label, consent_action_at, content_action_at, intent_action_at, tab_at, RootPage,
        INTENT_CANCEL_ACTION, INTENT_SEND_ACTION, ROOT_CONTENT_ACTIONS, ROOT_TABS,
    };

    #[test]
    fn root_tabs_come_from_sui_markup() {
        assert_eq!(ROOT_TABS.len(), 4);
        assert_eq!(ROOT_TABS[0].label, "Сейчас");
        assert_eq!(ROOT_TABS[3].icon, "person");
        assert_eq!(ROOT_TABS[3].action, "select_root:me");
    }

    #[test]
    fn bottom_bar_maps_all_four_tabs() {
        assert_eq!(tab_at((135.0, 2250.0), 1080, 2400), Some(RootPage::Now));
        assert_eq!(tab_at((405.0, 2250.0), 1080, 2400), Some(RootPage::Inbox));
        assert_eq!(tab_at((675.0, 2250.0), 1080, 2400), Some(RootPage::Spaces));
        assert_eq!(tab_at((945.0, 2250.0), 1080, 2400), Some(RootPage::Me));
    }

    #[test]
    fn content_area_is_not_a_tab() {
        assert_eq!(super::tab_at((540.0, 1200.0), 1080, 2400), None);
    }

    #[test]
    fn demo_action_geometry_comes_from_sui_markup() {
        assert_eq!(ROOT_CONTENT_ACTIONS.len(), 7);
        assert_eq!(ROOT_CONTENT_ACTIONS[0].label, "Saai Demo");
        assert_eq!(
            content_action_at(RootPage::Now, (540.0, 500.0), 1080, 2400).map(|action| action.id),
            Some("demo-app")
        );
        assert!(content_action_at(RootPage::Inbox, (540.0, 500.0), 1080, 2400).is_none());
    }

    #[test]
    fn space_actions_and_selected_entity_come_from_sui_markup() {
        for (point, expected) in [
            ((540.0, 500.0), "space-home"),
            ((540.0, 720.0), "space-work"),
            ((540.0, 940.0), "space-personal"),
            ((540.0, 1160.0), "space-saaios"),
        ] {
            assert_eq!(
                content_action_at(RootPage::Spaces, point, 1080, 2400).map(|action| action.id),
                Some(expected)
            );
        }
        assert_eq!(
            content_action_at(RootPage::Now, (540.0, 800.0), 1080, 2400).map(|action| action.id),
            Some("selected-entity")
        );
        assert_eq!(
            content_action_at(RootPage::Now, (540.0, 1020.0), 1080, 2400).map(|action| action.id),
            Some("new-intent")
        );
    }

    #[test]
    fn consent_screen_left_half_of_button_row_accepts() {
        assert_eq!(consent_action_at((270.0, 2250.0), 1080, 2400), Some(true));
    }

    #[test]
    fn consent_screen_right_half_of_button_row_declines() {
        assert_eq!(consent_action_at((810.0, 2250.0), 1080, 2400), Some(false));
    }

    #[test]
    fn consent_screen_header_area_is_not_a_button() {
        assert_eq!(consent_action_at((540.0, 1000.0), 1080, 2400), None);
    }

    #[test]
    fn intent_keyboard_rows_map_to_their_own_letters() {
        assert_eq!(
            intent_action_at((50.0, 300.0), 1080, 2400).as_deref(),
            Some("intent:key:q")
        );
        assert_eq!(
            intent_action_at((50.0, 850.0), 1080, 2400).as_deref(),
            Some("intent:key:a")
        );
        assert_eq!(
            intent_action_at((50.0, 1400.0), 1080, 2400).as_deref(),
            Some("intent:key:z")
        );
    }

    #[test]
    fn intent_keyboard_controls_row_has_cancel_and_send_at_the_ends() {
        assert_eq!(
            intent_action_at((50.0, 2000.0), 1080, 2400).as_deref(),
            Some(INTENT_CANCEL_ACTION)
        );
        assert_eq!(
            intent_action_at((950.0, 2000.0), 1080, 2400).as_deref(),
            Some(INTENT_SEND_ACTION)
        );
    }

    #[test]
    fn intent_keyboard_header_is_not_a_key() {
        assert_eq!(intent_action_at((540.0, 100.0), 1080, 2400), None);
    }

    #[test]
    fn capability_label_translates_known_vocabulary_and_falls_back_for_unknown() {
        assert_eq!(capability_label("net.internet"), "Доступ в интернет");
        assert_eq!(capability_label("net.bluetooth"), "net.bluetooth");
    }
}

delegate_compositor!(Shell);
delegate_layer!(Shell);
delegate_output!(Shell);
delegate_seat!(Shell);
delegate_session_lock!(Shell);
delegate_shm!(Shell);
delegate_touch!(Shell);
delegate_xdg_shell!(Shell);
delegate_xdg_window!(Shell);
delegate_registry!(Shell);

impl ProvidesRegistryState for Shell {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
