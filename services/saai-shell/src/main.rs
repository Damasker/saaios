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
//! test also creates a single top-anchored layer surface, proving the
//! protocol renders end to end. Real content (time/Wi-Fi/battery)
//! landed later, S13 Change 1. Still no touch dispatch to it
//! (saai-displayd's touch routing still only knows
//! `focused_surface`/`lock_surface`, not layer surfaces) -- follow-up
//! work if the bar ever needs to be interactive, not this step's goal.
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
use serde_json::{json, Map, Value};
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
use uuid::Uuid;

/// Matches drm-splash.c's own idle-to-lock constant.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// S13 Change 1: how often the status bar re-reads time/Wi-Fi/battery
/// and redraws. A plain poll, like `refresh_apps_if_due`'s own
/// interval just below -- none of these three sources have a push
/// mechanism worth wiring up for a once-a-second clock display.
const STATUSBAR_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
/// S11 Change 2 (ADR-041): how much *additional* idle time, on top of
/// the `IDLE_TIMEOUT` lock above, before this actually suspends the
/// device (`SAAIOS_DEEP_IDLE_SECS` overrides for testing -- a real
/// multi-minute wait isn't practical to sit through physically every
/// verification round). Five minutes is a first, deliberately
/// conservative production default -- ADR-040's spike proved the
/// suspend/resume cycle itself safe, but not any particular threshold
/// for how eager to be about it.
const DEFAULT_DEEP_IDLE_TIMEOUT: Duration = Duration::from_secs(300);
/// S16: first persisted, user-changeable setting in this project --
/// everything before this (deep idle timeout included) was either a
/// compile-time constant or an env-var override for testing, never
/// something the running shell itself could write back. One flat JSON
/// object, no `serde` derive (only `serde_json`, already a dependency,
/// is used elsewhere in this file for exactly this shape of ad hoc
/// object) under the same `/data/saaios` root S05's app data already
/// lives under.
const SETTINGS_PATH: &str = "/data/saaios/var/shell-settings.json";
/// Real sysfs path, confirmed present on this hardware by the old C
/// boot/recovery UI (`os/targets/panther/src/drm-splash.c`) -- never
/// read or written anywhere in the new Wayland stack before S16.
const BACKLIGHT_PATH: &str =
    "/sys/devices/platform/1c2c0000.drmdsim/1c2c0000.drmdsim.0/backlight/panel0-backlight/brightness";
const BACKLIGHT_MAX: u32 = 4095;
/// Discrete steps a tap cycles through, rather than a drag-to-adjust
/// slider -- this UI has no drag-gesture rendering anywhere yet (every
/// existing interaction is tap-a-card), and inventing one just for
/// this would be a much larger, separate piece of work.
const BRIGHTNESS_LEVELS_PCT: [u8; 4] = [25, 50, 75, 100];
const IDLE_TIMEOUT_LEVELS_SECS: [u64; 4] = [30, 60, 120, 300];
const DEEP_IDLE_TIMEOUT_LEVELS_SECS: [u64; 4] = [15, 60, 300, 900];
const VOLUME_LEVELS_PCT: [u8; 5] = [0, 25, 50, 75, 100];
/// S25. `100` is the base size every literal `size` argument in
/// `render.rs`'s draw calls already means -- 85/125/150 shrink or
/// grow every one of them by the same factor via `render::set_text_
/// scale`.
const TEXT_SCALE_LEVELS_PCT: [u8; 4] = [85, 100, 125, 150];
/// S25. Three honest levels rather than a bare on/off toggle, same
/// cycle-through-presets convention as every other setting in this
/// file. `0` (off) is the default -- see `ShellSettings.contrast_
/// pct`'s doc comment.
const CONTRAST_LEVELS_PCT: [u8; 3] = [0, 50, 100];

fn next_in_cycle<T: PartialEq + Copy>(levels: &[T], current: T) -> T {
    let index = levels.iter().position(|&level| level == current).unwrap_or(0);
    levels[(index + 1) % levels.len()]
}

/// Scales a 0-100 percentage onto the panel's real 0-4095 backlight
/// range and writes it. Errors are swallowed the same way every other
/// best-effort sysfs write in this file already is (`wifi_is_up`,
/// `read_battery`) -- a phone that can't dim its screen should still
/// otherwise work.
fn apply_brightness(pct: u8) {
    let value = (BACKLIGHT_MAX as u64 * pct.min(100) as u64 / 100) as u32;
    let _ = std::fs::write(BACKLIGHT_PATH, value.to_string());
}

/// S18: **honest gap, unlike `apply_brightness`.** No userspace volume
/// control existed anywhere in the project to confirm against before
/// this -- the kernel modules load (`snd-soc-cs35l41`, `aoc_alsa_dev`,
/// `native-init.c`'s own "ALSA sound devices ready" log line) but no
/// `amixer`/`alsactl` binary or any other precedent for which ALSA
/// simple-mixer control actually maps to output volume on this
/// hardware was ever established (confirmed absent by survey before
/// writing this). Tries `amixer` on the off chance a minimal build of
/// it is present on-device after all; silently no-ops (matching every
/// other best-effort hardware write in this file) if it isn't -- the
/// setting/UI/persistence side of this feature is real regardless of
/// whether this specific call has any effect, and the right follow-up
/// once the device is available again is to find the actual control
/// name (`amixer scontrols` or reading `/proc/asound/.../controls`)
/// and replace this guess.
fn apply_volume(pct: u8) {
    let _ = std::process::Command::new("amixer")
        .args(["sset", "Master", &format!("{}%", pct.min(100))])
        .status();
}

struct ShellSettings {
    brightness_pct: u8,
    idle_timeout_secs: u64,
    deep_idle_timeout_secs: u64,
    /// S17. Signed minutes, `0` = UTC -- matches `TIMEZONE_PRESETS_
    /// MINUTES`'s own units, deliberately not hours-only (some real
    /// timezones use a half-hour or 45-minute offset).
    utc_offset_minutes: i32,
    /// S18. See `apply_volume`'s doc comment -- the setting/UI side is
    /// real, the hardware effect is an unverified best-effort guess.
    volume_pct: u8,
    /// S24. `None` (default) means unlock stays "any tap" -- the
    /// original behavior this device has been tested with all along
    /// -- so turning this on is opt-in, never a silent behavior
    /// change for an existing settings file. Stored in plaintext in
    /// the same settings file as every other setting here; no worse
    /// than `wpa_supplicant.conf`'s own plaintext Wi-Fi passwords
    /// (S19) already on this same disk, and there's no other local
    /// user account on this device for a PIN to protect against.
    pin_code: Option<String>,
    /// S25. Percent of every text draw call's base size
    /// (`render::set_text_scale`) -- see that function's doc comment
    /// for why this is process-global state inside `render.rs`
    /// rather than a parameter threaded through its ~40 existing
    /// `draw_text`/`draw_text_centered` call sites.
    text_scale_pct: u8,
    /// S25. Post-process contrast stretch (`render::apply_contrast_
    /// boost`), applied once per rendered frame rather than as a
    /// second color palette threaded through every `fill_rect`/
    /// `draw_text` call. `0` is a no-op.
    contrast_pct: u8,
}

impl ShellSettings {
    /// `SAAIOS_DEEP_IDLE_SECS` (S11) predates persisted settings and
    /// stays live as the same practical-for-physical-testing escape
    /// hatch it always was -- takes priority over both the settings
    /// file and the compiled-in default whenever it's actually set.
    fn default_deep_idle_secs() -> u64 {
        std::env::var("SAAIOS_DEEP_IDLE_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_DEEP_IDLE_TIMEOUT.as_secs())
    }

    fn load() -> Self {
        let default = Self {
            brightness_pct: 100,
            idle_timeout_secs: IDLE_TIMEOUT.as_secs(),
            deep_idle_timeout_secs: Self::default_deep_idle_secs(),
            utc_offset_minutes: 0,
            volume_pct: 75,
            pin_code: None,
            text_scale_pct: 100,
            contrast_pct: 0,
        };
        let Some(value) = std::fs::read_to_string(SETTINGS_PATH)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        else {
            return default;
        };
        Self {
            brightness_pct: value
                .get("brightness_pct")
                .and_then(Value::as_u64)
                .map(|pct| pct as u8)
                .unwrap_or(default.brightness_pct),
            idle_timeout_secs: value
                .get("idle_timeout_secs")
                .and_then(Value::as_u64)
                .unwrap_or(default.idle_timeout_secs),
            deep_idle_timeout_secs: value
                .get("deep_idle_timeout_secs")
                .and_then(Value::as_u64)
                .unwrap_or(default.deep_idle_timeout_secs),
            utc_offset_minutes: value
                .get("utc_offset_minutes")
                .and_then(Value::as_i64)
                .map(|minutes| minutes as i32)
                .unwrap_or(default.utc_offset_minutes),
            volume_pct: value
                .get("volume_pct")
                .and_then(Value::as_u64)
                .map(|pct| pct as u8)
                .unwrap_or(default.volume_pct),
            pin_code: value
                .get("pin_code")
                .and_then(Value::as_str)
                .filter(|pin| !pin.is_empty())
                .map(str::to_string)
                .or(default.pin_code),
            text_scale_pct: value
                .get("text_scale_pct")
                .and_then(Value::as_u64)
                .map(|pct| pct as u8)
                .unwrap_or(default.text_scale_pct),
            contrast_pct: value
                .get("contrast_pct")
                .and_then(Value::as_u64)
                .map(|pct| pct as u8)
                .unwrap_or(default.contrast_pct),
        }
    }

    fn save(&self) {
        let value = json!({
            "brightness_pct": self.brightness_pct,
            "idle_timeout_secs": self.idle_timeout_secs,
            "deep_idle_timeout_secs": self.deep_idle_timeout_secs,
            "utc_offset_minutes": self.utc_offset_minutes,
            "volume_pct": self.volume_pct,
            "pin_code": self.pin_code,
            "text_scale_pct": self.text_scale_pct,
            "contrast_pct": self.contrast_pct,
        });
        let Ok(text) = serde_json::to_string_pretty(&value) else {
            return;
        };
        if let Some(parent) = std::path::Path::new(SETTINGS_PATH).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(SETTINGS_PATH, text);
    }
}
/// Bright red -- deliberately unmistakable against the toplevel's dark
/// slate placeholder, so a photo of the panel makes it obvious which
/// surface is actually receiving the compositor's output while locked.
///
/// Byte order here is [0x00, 0xd0, 0x00, 0x00], *not* the [B, G, R, X] a
/// standard XRGB8888 LE layout would predict for red. A three-band
/// on-device diagnostic (one solid color per byte position, read back
/// directly from this pool's memfd via /proc/<pid>/fd to confirm the
/// client-side write itself before ever trusting the photo) proved this
/// panel's pipeline reads R from byte-index 1 and G from byte-index 2 --
/// swapped from the conventional B,G,R,X -- while byte-index 0 produced
/// no visible output at all in the same test (untested whether that's a
/// true "blue" that just read as too dark to name, or genuinely unused;
/// not re-verified here since only red was needed for that milestone).
/// Root cause on the DRM/driver side not identified -- no standard
/// fourcc swaps R and G while leaving B in place, so this is applied as
/// an empirically-verified byte order, not a fourcc fix.
const LOCK_SCREEN_COLOR: [u8; 4] = [0x00, 0xd0, 0x00, 0x00];
/// Plain black -- every byte-order permutation of all-zero reads as
/// black, so this needs none of `LOCK_SCREEN_COLOR`'s empirical care.
/// Shown on the lock surface (the panel's actual visible content while
/// locked -- not the toplevel, which stays hidden underneath it, ADR-016)
/// immediately before a real `mem`-suspend and while resuming from one.
/// Real, physically confirmed UX gap otherwise (S11 Change 2/ADR-041):
/// nothing made the panel visibly go dark before suspending, so there
/// was no reliable cue for when it was actually safe -- or necessary --
/// to press power.
const SLEEP_INDICATOR_COLOR: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

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

/// `wlan0`'s own `operstate` (S13 Change 1) -- `true` only for `up`,
/// matching what the interface itself reports rather than assuming a
/// connection exists. Confirmed on-device to read `down` honestly when
/// `wpa_supplicant` hasn't associated to anything, not just when the
/// radio is off.
fn wifi_is_up() -> bool {
    std::fs::read_to_string("/sys/class/net/wlan0/operstate")
        .map(|state| state.trim() == "up")
        .unwrap_or(false)
}

/// S19: real, unlike S18's `apply_volume`. `native-init.c`'s
/// `setup_wifi` already starts a live `wpa_supplicant` against
/// `/saaios/wpa_supplicant.conf` (`update_config=1`) with its control
/// socket at `/run/wpa_supplicant`, plus a `wpa_cli -a` action-script
/// watcher (confirmed by reading `native-init.c` before writing this)
/// -- scanning/connecting here drives infrastructure that is already
/// running on every boot, not a guess. No daemon of our own is
/// needed: `wpa_cli` is a stateless client per invocation, so every
/// call here just reopens the same control socket.
const WPA_CLI_BIN: &str = "/saaios/wpa_cli";
const WPA_CLI_ARGS: [&str; 4] = ["-p", "/run/wpa_supplicant", "-i", "wlan0"];

fn wpa_cli(args: &[&str]) -> String {
    let mut full_args = WPA_CLI_ARGS.to_vec();
    full_args.extend_from_slice(args);
    std::process::Command::new(WPA_CLI_BIN)
        .args(&full_args)
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}

struct WifiNetwork {
    ssid: String,
    secured: bool,
    signal_dbm: i32,
}

/// Parses `wpa_cli scan_results`'s own tab-separated table (header
/// row, then one row per BSS: bssid / frequency / signal level /
/// flags / ssid). Doesn't itself trigger a fresh scan -- `wpa_
/// supplicant` already scans periodically on its own in the
/// background; `wifi_trigger_scan` is the separate explicit
/// "Обновить" action. Collapses multiple BSSIDs of the same SSID
/// (common with mesh/multi-AP networks) into one row, keeping the
/// strongest signal seen.
fn wifi_scan_results() -> Vec<WifiNetwork> {
    let output = wpa_cli(&["scan_results"]);
    let mut networks: Vec<WifiNetwork> = Vec::new();
    for line in output.lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 5 {
            continue;
        }
        let ssid = fields[4].trim();
        if ssid.is_empty() {
            continue;
        }
        let signal_dbm: i32 = fields[2].trim().parse().unwrap_or(0);
        let secured = fields[3].contains("WPA") || fields[3].contains("WEP");
        if let Some(existing) = networks.iter_mut().find(|network| network.ssid == ssid) {
            if signal_dbm > existing.signal_dbm {
                existing.signal_dbm = signal_dbm;
                existing.secured = secured;
            }
        } else {
            networks.push(WifiNetwork {
                ssid: ssid.to_string(),
                secured,
                signal_dbm,
            });
        }
    }
    networks.sort_by_key(|network| std::cmp::Reverse(network.signal_dbm));
    networks
}

fn wifi_trigger_scan() {
    let _ = wpa_cli(&["scan"]);
}

/// `wpa_cli status`'s own `key=value` lines. `wpa_state=COMPLETED`
/// plus a non-empty `ssid=` is the same "actually associated" signal
/// `wpa_supplicant` itself uses internally, not a guess at one.
fn wifi_status_line() -> String {
    let output = wpa_cli(&["status"]);
    let mut state = String::new();
    let mut ssid = String::new();
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("wpa_state=") {
            state = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("ssid=") {
            ssid = value.trim().to_string();
        }
    }
    if state == "COMPLETED" && !ssid.is_empty() {
        format!("Подключено: {ssid}")
    } else {
        "Не подключено".to_string()
    }
}

fn wifi_connect_open(ssid: &str) {
    wifi_add_and_select(ssid, None);
}

fn wifi_connect_psk(ssid: &str, psk: &str) {
    wifi_add_and_select(ssid, Some(psk));
}

/// One `add_network`/`set_network`/`enable_network`/`select_network`
/// sequence per connect attempt -- `wpa_cli`'s own documented
/// non-interactive usage. `update_config=1` in `wpa_supplicant.conf`
/// (confirmed before writing this) means `save_config` also persists
/// this across reboots, not just for the current boot session.
fn wifi_add_and_select(ssid: &str, psk: Option<&str>) {
    let id = wpa_cli(&["add_network"]).trim().to_string();
    if id.parse::<u32>().is_err() {
        return;
    }
    let quoted_ssid = format!("\"{ssid}\"");
    let _ = wpa_cli(&["set_network", &id, "ssid", &quoted_ssid]);
    match psk {
        Some(psk) => {
            let quoted_psk = format!("\"{psk}\"");
            let _ = wpa_cli(&["set_network", &id, "psk", &quoted_psk]);
        }
        None => {
            let _ = wpa_cli(&["set_network", &id, "key_mgmt", "NONE"]);
        }
    }
    let _ = wpa_cli(&["enable_network", &id]);
    let _ = wpa_cli(&["select_network", &id]);
    let _ = wpa_cli(&["save_config"]);
}

/// S20: real, same shape as S19's Wi-Fi -- `/saaios/bt-scan` and
/// `/saaios/bt-pair` are complete raw-HCI (Bluetooth management
/// socket) tools already built and shipped in the image (confirmed
/// by reading their source and `build-native-c-image.sh` before
/// writing this), just never invoked automatically at boot or from
/// any UI. Unlike Wi-Fi's `wpa_supplicant`, there is no long-running
/// daemon to ask -- `bt-scan` itself blocks for ~8s doing discovery,
/// and a real pairing handshake (`bt-pair <index>`) can legitimately
/// take tens of seconds waiting on the peer. Both are `spawn()`ed
/// detached with stdout redirected to a log file instead of run
/// synchronously, so a tap here never freezes `saai-shell`'s own
/// event loop -- the UI re-reads whichever log file has accumulated
/// so far on every redraw, same "read fresh, no caching" convention
/// as `wifi_status_line`.
const BT_SCAN_BIN: &str = "/saaios/bt-scan";
const BT_PAIR_BIN: &str = "/saaios/bt-pair";
const BT_SCAN_LOG_PATH: &str = "/run/saai-shell-bt-scan.log";
const BT_PAIR_LOG_PATH: &str = "/run/saai-shell-bt-pair.log";
/// `bt-pair`'s own persisted list of previously paired devices
/// (`publish_devices()` in `bt-pair.c`) -- one "SAVED\t<name>" line
/// per remembered device, refreshed on every successful pair.
const BT_SAVED_LOG_PATH: &str = "/run/bluetooth-saved.log";

struct BluetoothDevice {
    name: String,
    transport: String,
}

fn spawn_detached(binary: &str, args: &[&str], log_path: &str) {
    let Ok(log_file) = std::fs::File::create(log_path) else {
        return;
    };
    let _ = std::process::Command::new(binary)
        .args(args)
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn bluetooth_trigger_scan() {
    spawn_detached(BT_SCAN_BIN, &[], BT_SCAN_LOG_PATH);
}

/// Parses `bt-scan`'s own stdout protocol (`DEVICE\t<name>`, then
/// `TRANSPORT\t<name>\t<CLASSIC|BLE>` for the device just printed,
/// finally `DONE\t<count>`) from whatever's accumulated in the log
/// file so far -- `done` is `false` while a scan is still running or
/// hasn't been started yet. A device's list position here is the
/// same index `bt-pair <index>` expects: both walk
/// `/run/bluetooth-devices.bin`'s records in the order `bt-scan`
/// appended them.
fn bluetooth_scan_results() -> (Vec<BluetoothDevice>, bool) {
    let Ok(text) = std::fs::read_to_string(BT_SCAN_LOG_PATH) else {
        return (Vec::new(), false);
    };
    let mut devices: Vec<BluetoothDevice> = Vec::new();
    let mut done = false;
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["DEVICE", name] => devices.push(BluetoothDevice {
                name: (*name).to_string(),
                transport: String::new(),
            }),
            ["TRANSPORT", name, transport] => {
                if let Some(device) = devices.iter_mut().rev().find(|d| d.name == *name) {
                    device.transport = (*transport).to_string();
                }
            }
            ["DONE", _] => done = true,
            _ => {}
        }
    }
    (devices, done)
}

fn bluetooth_pair(index: usize) {
    spawn_detached(BT_PAIR_BIN, &[&index.to_string()], BT_PAIR_LOG_PATH);
}

/// `bt-pair <index>`'s own result line, if the background pairing
/// attempt has reached one yet (`None` while only its initial
/// "PAIRING" line is present, or before it's been run at all).
fn bluetooth_pair_result() -> Option<String> {
    let text = std::fs::read_to_string(BT_PAIR_LOG_PATH).ok()?;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("PAIRED\t") {
            return Some(format!("Сопряжено: {name}"));
        }
        if let Some(reason) = line.strip_prefix("PAIR-ERROR\t") {
            return Some(format!("Ошибка сопряжения: {reason}"));
        }
    }
    None
}

/// Prefers a pairing result in progress/just finished over the scan
/// state, so tapping a device to pair immediately starts showing
/// that outcome instead of being silently overwritten by scan status
/// text.
fn bluetooth_status_summary() -> String {
    if let Some(result) = bluetooth_pair_result() {
        return result;
    }
    if !std::path::Path::new(BT_SCAN_LOG_PATH).exists() {
        return "Поиск ещё не запускался".to_string();
    }
    let (devices, done) = bluetooth_scan_results();
    if done {
        format!("Поиск завершён: найдено {}", devices.len())
    } else {
        "Идёт поиск... (~8 с)".to_string()
    }
}

fn bluetooth_paired_count() -> usize {
    std::fs::read_to_string(BT_SAVED_LOG_PATH)
        .map(|text| text.lines().filter(|line| line.starts_with("SAVED\t")).count())
        .unwrap_or(0)
}

/// The fuel gauge's own power_supply node is named `maxfg`, not
/// `battery` -- confirmed by listing `/sys/class/power_supply/` on
/// device (S13 Change 1 investigation). Returns `(percent, charging)`;
/// `None` if the node is missing entirely (host builds, or hardware
/// this shell has never run on).
fn read_battery() -> Option<(u8, bool)> {
    let capacity: u8 = std::fs::read_to_string("/sys/class/power_supply/maxfg/capacity")
        .ok()?
        .trim()
        .parse()
        .ok()?;
    let status = std::fs::read_to_string("/sys/class/power_supply/maxfg/status").unwrap_or_default();
    let charging = matches!(status.trim(), "Charging" | "Full");
    Some((capacity, charging))
}

/// Shells out to `date` rather than pulling in a datetime crate for one
/// `HH:MM` string a second -- `chrono` is already a dev-only dependency
/// here (tests only); promoting it to a real runtime dependency for
/// this alone isn't worth the size/build cost on the `pixel7` profile.
/// Matches this file's existing pattern of shelling out to `busybox`
/// for one-off system state elsewhere.
/// S17: no timezone concept existed anywhere in the project before
/// this (verified: no `/etc/localtime`, no `/etc/timezone`, no `TZ`
/// set anywhere in `native-init.c`) -- the system clock itself is UTC
/// (`os/targets/panther/src/sntp-sync.c` sets it via `settimeofday`,
/// standard Unix practice of keeping the kernel clock in UTC
/// regardless of what a user sees displayed). This used to shell out
/// to `date +%H:%M` (S13), which would only ever have shown UTC too,
/// with no way to apply an offset short of also managing a `TZ` env
/// var and its notoriously inverted POSIX sign convention
/// (`TZ=UTC-3` means *ahead* of UTC, not behind). Doing the arithmetic
/// directly on the raw epoch sidesteps that footgun entirely, and
/// happens to need no process spawn at all anymore either.
fn current_time_string(utc_offset_minutes: i32) -> String {
    let epoch_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let local_seconds = epoch_seconds + i64::from(utc_offset_minutes) * 60;
    let seconds_of_day = local_seconds.rem_euclid(86400);
    let hours = seconds_of_day / 3600;
    let minutes = (seconds_of_day % 3600) / 60;
    format!("{hours:02}:{minutes:02}")
}

/// Curated UTC offsets (minutes) rather than full IANA tzdata --
/// no timezone database ships on this image, and shipping/maintaining
/// one just for a handful of DST-naive fixed offsets would be a much
/// bigger undertaking than this UI actually needs. Deliberately
/// includes a couple of half-hour offsets (India, among others use
/// non-whole-hour offsets) to prove the representation isn't
/// hour-only, not to be exhaustive.
const TIMEZONE_PRESETS_MINUTES: [i32; 9] = [
    0,   // UTC
    60,  // Центральная Европа (UTC+1)
    120, // Восточная Европа (UTC+2)
    180, // Москва (UTC+3)
    270, // Иран (UTC+4:30)
    330, // Индия (UTC+5:30)
    480, // Китай (UTC+8)
    540, // Япония (UTC+9)
    -300, // США, восточное побережье (UTC-5)
];

/// S21: below this (and not charging), `check_low_battery` creates one
/// `saaios.notification`.
const LOW_BATTERY_THRESHOLD_PCT: u8 = 15;

fn format_utc_offset(minutes: i32) -> String {
    let sign = if minutes < 0 { '-' } else { '+' };
    let abs_minutes = minutes.unsigned_abs();
    format!("UTC{sign}{:02}:{:02}", abs_minutes / 60, abs_minutes % 60)
}

/// S24: a real numeric keypad, not ADR-029's letters-only keyboard
/// (`INTENT_KEY_ROWS` has no digit keys at all). Plain
/// `stacked_row_rect`-style index-to-rect math (see `now_grid_rect`),
/// not a `Node`/`layout()` tree -- this needs to render both on the
/// normal toplevel surface ("Я"'s "Изменить PIN") and on the
/// session-lock surface (`present_lock_surface`), and the lock
/// surface has no `Node` tree infrastructure of its own at all.
/// Layout: 1-9, then a blank cell, 0, and backspace -- same shape as
/// a phone dialer. Two more slots (indices 12-13, an extra row) are
/// "Отмена"/"Готово" controls, used only by the PIN-setup screen
/// (`pin_setup_action_at`), never during unlock.
const PIN_KEYPAD_COLUMNS: u32 = 3;
const PIN_KEYPAD_DIGIT_LABELS: [&str; 12] =
    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "", "0", "⌫"];

fn pin_keypad_rect(index: usize, width: u32, height: u32) -> Rect {
    let margin = width / 10;
    let columns = PIN_KEYPAD_COLUMNS;
    let gap = margin / 3;
    let usable_width = width.saturating_sub(margin * 2);
    let cell_width = usable_width.saturating_sub(gap * (columns - 1)) / columns;
    let cell_height_2400 = 240u32;
    let row = index as u32 / columns;
    let column = index as u32 % columns;
    let top_2400 = 900 + row * (cell_height_2400 + 30);
    let top = ((u64::from(top_2400) * u64::from(height)) / 2400) as u32;
    let cell_height = ((u64::from(cell_height_2400) * u64::from(height)) / 2400) as u32;
    Rect::new(
        margin + column * (cell_width + gap),
        top,
        cell_width,
        cell_height,
    )
}

/// Digit ("0".."9") or "⌫" for backspace -- `None` for a miss or the
/// deliberately blank cell at index 9. Shared by both the lock
/// surface and the PIN-setup screen: same keys mean the same thing
/// in both places.
fn pin_keypad_action_at(pos: (f64, f64), width: u32, height: u32) -> Option<&'static str> {
    PIN_KEYPAD_DIGIT_LABELS
        .iter()
        .enumerate()
        .find(|(index, label)| {
            !label.is_empty() && pin_keypad_rect(*index, width, height).contains(pos.0, pos.1)
        })
        .map(|(_, label)| *label)
}

/// S24: "Отмена"/"Готово" (plus "Убрать PIN" when a PIN is already
/// set) for the PIN-setup screen only -- laid out as one more
/// `pin_keypad_rect` row (indices 12.. ) below the digit grid, not a
/// separate geometry system.
fn pin_setup_controls(has_existing_pin: bool) -> Vec<&'static str> {
    if has_existing_pin {
        vec!["Отмена", "Готово", "Убрать PIN"]
    } else {
        vec!["Отмена", "Готово"]
    }
}

fn pin_setup_action_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    has_existing_pin: bool,
) -> Option<&'static str> {
    if let Some(digit) = pin_keypad_action_at(pos, width, height) {
        return Some(digit);
    }
    pin_setup_controls(has_existing_pin)
        .into_iter()
        .enumerate()
        .find(|(offset, _)| pin_keypad_rect(12 + offset, width, height).contains(pos.0, pos.1))
        .map(|(_, label)| label)
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
    app_name: String,
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

/// S19: reuses `intent_view()`'s keyboard layout and hit-testing
/// verbatim (same rows, same control ids) rather than growing a
/// second keyboard tree -- the two are visually and structurally
/// identical, only the header text and what "Отправить" does differ,
/// both handled by which of `intent_input`/`wifi_password` is
/// currently `Some`. Inherits ADR-029's keyboard's own limitation:
/// lowercase Latin letters and spaces only, no digits/symbols/
/// uppercase -- honestly, that covers approximately no real WPA2
/// password. Open-network connect (no password needed at all) and
/// the scan/status pipeline are what this sprint makes unconditionally
/// useful today; typing a real secured-network password is blocked on
/// the keyboard itself growing a digit/symbol row, not on anything
/// here.
struct WifiPasswordState {
    ssid: String,
    buffer: String,
}

/// S24: "Изменить PIN" on "Я" -- see `PIN_KEYPAD_DIGIT_LABELS`'s doc
/// comment for why this is a dedicated numeric keypad, not the
/// ADR-029 letters keyboard `WifiPasswordState` reuses.
#[derive(Default)]
struct PinSetupState {
    buffer: String,
}

/// One row of `wifi_scan_results()`'s output, or a fixed trailing
/// "Обновить"/"Назад" control row -- returned by `wifi_list_action_at`
/// the same way `InboxRowKind` disambiguates "Входящие"'s rows.
enum WifiListTap {
    Network(usize),
    Refresh,
    Back,
}

/// Same reasoning as `me_action_at`: `network_count` is runtime-sized
/// (like `installed_apps.len()` elsewhere), so this can't be a
/// `root.sui` entry -- two more `stacked_row_rect` slots after the
/// networks are the fixed "Обновить"/"Назад" controls.
fn wifi_list_action_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    network_count: usize,
) -> Option<WifiListTap> {
    for index in 0..network_count {
        if stacked_row_rect(index, width, height).contains(pos.0, pos.1) {
            return Some(WifiListTap::Network(index));
        }
    }
    if stacked_row_rect(network_count, width, height).contains(pos.0, pos.1) {
        return Some(WifiListTap::Refresh);
    }
    if stacked_row_rect(network_count + 1, width, height).contains(pos.0, pos.1) {
        return Some(WifiListTap::Back);
    }
    None
}

/// S20: same shape as `WifiListTap`, one more trailing control row
/// ("Искать" is separate from "Обновить список" here, since a
/// Bluetooth scan takes ~8s in the background -- unlike Wi-Fi's
/// already-continuously-scanning `wpa_supplicant`, starting a new
/// scan and re-reading the current log are genuinely different
/// actions here).
enum BluetoothListTap {
    Device(usize),
    Scan,
    Refresh,
    Back,
}

fn bluetooth_list_action_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    device_count: usize,
) -> Option<BluetoothListTap> {
    for index in 0..device_count {
        if stacked_row_rect(index, width, height).contains(pos.0, pos.1) {
            return Some(BluetoothListTap::Device(index));
        }
    }
    if stacked_row_rect(device_count, width, height).contains(pos.0, pos.1) {
        return Some(BluetoothListTap::Scan);
    }
    if stacked_row_rect(device_count + 1, width, height).contains(pos.0, pos.1) {
        return Some(BluetoothListTap::Refresh);
    }
    if stacked_row_rect(device_count + 2, width, height).contains(pos.0, pos.1) {
        return Some(BluetoothListTap::Back);
    }
    None
}

const TASK_CONFIRM_HEADER_ID: &str = "task-confirm-header";
const TASK_CONFIRM_BUTTONS_ID: &str = "task-confirm-buttons";
const TASK_CONFIRM_ACCEPT_ID: &str = "task-confirm-accept";
const TASK_CONFIRM_DECLINE_ID: &str = "task-confirm-decline";
const TASK_CONFIRM_ACCEPT_ACTION: &str = "task_confirm:accept";
const TASK_CONFIRM_DECLINE_ACTION: &str = "task_confirm:decline";
/// Same reasoning as `CONSENT_BUTTON_HEIGHT`: matches the tab bar's own
/// button proportions on this "second full-screen `ui_tree`" class of
/// modal.
const TASK_CONFIRM_BUTTON_HEIGHT: u32 = ROOT_TAB_HEIGHT;
/// `saaios.task`'s own recognized values for the property this screen
/// reads and writes -- kept local to `saai-shell` rather than imported
/// from `saai-taskd`, per ADR-030's no-cross-runtime-dependency
/// principle: the two crates agree on this string by convention, not
/// by sharing a type.
const TASK_STATUS_WAITING_CONFIRMATION: &str = "waiting_confirmation";
const TASK_STATUS_RUNNING: &str = "running";
const TASK_STATUS_CANCELLED: &str = "cancelled";

/// What `draw()` renders this frame, computed up front from `&self` before
/// `buffer`/`canvas` take a mutable borrow for the rest of the function.
enum Frame {
    Consent {
        app_name: String,
        labels: Vec<String>,
        header: Rect,
        accept: Rect,
        decline: Rect,
    },
    TaskConfirm {
        title: String,
        header: Rect,
        accept: Rect,
        decline: Rect,
    },
    IntentInput {
        buffer: String,
        header: Rect,
        keys: Vec<(Rect, String)>,
    },
    WifiPasswordInput {
        ssid: String,
        buffer: String,
        header: Rect,
        keys: Vec<(Rect, String)>,
    },
    WifiList {
        header: Rect,
        status_line: String,
        rows: Vec<(Rect, String)>,
    },
    BluetoothList {
        header: Rect,
        status_line: String,
        rows: Vec<(Rect, String)>,
    },
    PinSetup {
        buffer: String,
        header: Rect,
        keys: Vec<(Rect, &'static str)>,
    },
    Root {
        content_rect: Rect,
        tabs: Vec<(Rect, &'static str)>,
        content_cards: Vec<(Rect, render::ActionCardView)>,
        context_label: String,
    },
}

fn task_confirm_view(width: u32, height: u32) -> LayoutNode {
    let buttons = Node::linear(
        TASK_CONFIRM_BUTTONS_ID,
        Axis::Horizontal,
        vec![
            Node::leaf(TASK_CONFIRM_ACCEPT_ID).with_action(TASK_CONFIRM_ACCEPT_ACTION),
            Node::leaf(TASK_CONFIRM_DECLINE_ID).with_action(TASK_CONFIRM_DECLINE_ACTION),
        ],
    )
    .with_size(Length::Fill, Length::Px(TASK_CONFIRM_BUTTON_HEIGHT));
    let root = Node::linear(
        "task-confirm",
        Axis::Vertical,
        vec![Node::leaf(TASK_CONFIRM_HEADER_ID), buttons],
    );
    layout(&root, Rect::new(0, 0, width, height))
}

/// `Some(true)` for confirm, `Some(false)` for decline -- same shape as
/// `consent_action_at()`.
fn task_confirm_action_at(pos: (f64, f64), width: u32, height: u32) -> Option<bool> {
    if width == 0 || height == 0 {
        return None;
    }
    match task_confirm_view(width, height)
        .hit_test(pos.0, pos.1)
        .and_then(|node| node.action.as_deref())
    {
        Some(TASK_CONFIRM_ACCEPT_ACTION) => Some(true),
        Some(TASK_CONFIRM_DECLINE_ACTION) => Some(false),
        _ => None,
    }
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

/// S13 Change 3: `saai-appd`'s own state vocabulary
/// (`services/saai-appd/src/daemon.rs`), translated the same
/// fallback-to-the-raw-string way `capability_label` handles anything
/// it doesn't recognize.
fn app_state_label(state: &str) -> &str {
    match state {
        "running" => "Работает",
        "stopped" => "Остановлено",
        "crash_limited" => "Отключено после сбоев",
        "installed" => "Установлено",
        other => other,
    }
}

/// S14: "О телефоне" -- the compiled-in build id (`build.rs`, S14) plus
/// three plain `/proc` reads. No `system-tools` crate reuse here on
/// purpose: that crate pulls in `tokio`/`async_trait`/`tool_registry`
/// for the cognitive-core runtime's needs, far heavier than this
/// `panic_immediate_abort` binary (ADR-013) should carry for three
/// strings shown once on one screen.
fn kernel_release() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|_| "неизвестно".to_string())
}

fn hardware_model() -> String {
    std::fs::read_to_string("/proc/device-tree/model")
        .map(|text| text.trim_end_matches('\0').trim().to_string())
        .unwrap_or_else(|_| "неизвестно".to_string())
}

fn uptime_string() -> String {
    let seconds = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|text| text.split_whitespace().next().map(str::to_string))
        .and_then(|first| first.parse::<f64>().ok());
    let Some(seconds) = seconds else {
        return "неизвестно".to_string();
    };
    let total = seconds as u64;
    let days = total / 86400;
    let hours = (total % 86400) / 3600;
    let minutes = (total % 3600) / 60;
    if days > 0 {
        format!("{days} дн {hours} ч")
    } else if hours > 0 {
        format!("{hours} ч {minutes} мин")
    } else {
        format!("{minutes} мин")
    }
}

/// S15: no disk-usage reading existed anywhere in the project. `std`
/// has no cross-platform statvfs API, and this file already has a
/// precedent (`current_time_string`, S13) for shelling out to a small
/// system utility rather than adding a new dependency for one
/// occasional read -- `df`'s own `1K-blocks`/`Used` columns are exactly
/// what's needed, no parsing library required.
fn storage_usage_kb() -> Option<(u64, u64)> {
    let output = std::process::Command::new("df")
        .args(["-k", "/data"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let data_line = text.lines().nth(1)?;
    let mut fields = data_line.split_whitespace();
    fields.next()?; // filesystem name, unused
    let total_kb: u64 = fields.next()?.parse().ok()?;
    let used_kb: u64 = fields.next()?.parse().ok()?;
    Some((used_kb, total_kb))
}

fn storage_string() -> String {
    match storage_usage_kb() {
        Some((used_kb, total_kb)) => {
            let used_gb = used_kb as f64 / 1_048_576.0;
            let total_gb = total_kb as f64 / 1_048_576.0;
            format!("{used_gb:.1} ГБ из {total_gb:.1} ГБ занято")
        }
        None => "неизвестно".to_string(),
    }
}

/// S22: surfaces facts S12's OTA tools (`saai-ota-write`/`-health`,
/// CLI-only, never wired into `native-init.c` or given a daemon/
/// protocol of their own) already persist -- deliberately read-only.
/// An actual "check for updates now" action needs an update-server URL
/// setting that doesn't exist yet and would call into binaries whose
/// on-device path was never fixed by S12's manual-CLI scope; both are
/// honestly out of scope here rather than built on an unverified
/// assumption about either.
fn boot_slot() -> String {
    let bootconfig = std::fs::read_to_string("/proc/bootconfig").unwrap_or_default();
    let cmdline = std::fs::read_to_string("/proc/cmdline").unwrap_or_default();
    for source in [&bootconfig, &cmdline] {
        for token in source.split_whitespace() {
            if let Some(suffix) = token.strip_prefix("androidboot.slot_suffix=") {
                return suffix.trim_start_matches('_').to_uppercase();
            }
        }
    }
    "неизвестно".to_string()
}

fn boot_attempts() -> u32 {
    std::fs::read_to_string("/data/saaios/system/boot-attempts")
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
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

/// S13 Change 2: "Входящие" has no `root.sui` entries at all -- unlike
/// every other page's content, the task list's length is runtime data
/// (however many tasks `saai-taskd` currently has waiting), not
/// something `build.rs` can bake in from markup. These three functions
/// are this page's own equivalent of `ROOT_CONTENT_ACTIONS`/
/// `content_action_rect`/`content_action_at`.
fn inbox_pending_tasks(entities: &[Entity]) -> Vec<&Entity> {
    entities
        .iter()
        .filter(|entity| {
            entity.entity_type == "saaios.task"
                && entity.properties.get("status").and_then(Value::as_str)
                    == Some(TASK_STATUS_WAITING_CONFIRMATION)
        })
        .collect()
}

/// Same 190-tall/220-apart stacking `root.sui`'s "Пространства" cards
/// use (`space-home` top=430, `space-work` top=650, ...), just computed
/// per-index instead of read from compiled-in constants. Shared by
/// every page whose card count is runtime data -- "Входящие" (S13
/// Change 2) and "Я" (S13 Change 3) so far.
/// S23: "Сейчас"'s icon grid -- 3 columns, phone-style, replacing the
/// single-column list every other page still uses (`stacked_row_
/// rect`). Same 1080x2400 reference-canvas convention as that
/// function: literal units scaled by the real panel size, not a
/// design-time assumption about actual resolution.
const NOW_GRID_COLUMNS: u32 = 3;

fn now_grid_rect(index: usize, width: u32, height: u32) -> Rect {
    let margin = width / 22;
    let columns = NOW_GRID_COLUMNS;
    let gap = margin / 2;
    let usable_width = width.saturating_sub(margin * 2);
    let cell_width = usable_width.saturating_sub(gap * (columns - 1)) / columns;
    let cell_height_2400 = 300u32;
    let row = index as u32 / columns;
    let column = index as u32 % columns;
    let top_2400 = 430 + row * (cell_height_2400 + 40);
    let top = ((u64::from(top_2400) * u64::from(height)) / 2400) as u32;
    let cell_height = ((u64::from(cell_height_2400) * u64::from(height)) / 2400) as u32;
    Rect::new(
        margin + column * (cell_width + gap),
        top,
        cell_width,
        cell_height,
    )
}

fn stacked_row_rect(index: usize, width: u32, height: u32) -> Rect {
    let margin = width / 22;
    let top_2400 = 430 + index as u32 * 220;
    let top = ((top_2400 as u64 * height as u64) / 2400) as u32;
    let row_height = ((190_u64 * height as u64) / 2400) as u32;
    Rect::new(
        margin,
        top,
        width.saturating_sub(margin.saturating_mul(2)),
        row_height,
    )
}

/// S21: generic system notifications (low battery so far -- see
/// `refresh_statusbar_if_due`) -- the first entity type in the
/// project besides `saaios.task` that "Входящие" shows. `dismissed`
/// defaults to absent/false, same "missing means no" convention
/// `saaios.task`'s own `status` lookup already uses.
const NOTIFICATION_ENTITY_TYPE: &str = "saaios.notification";

fn inbox_notifications(entities: &[Entity]) -> Vec<&Entity> {
    entities
        .iter()
        .filter(|entity| {
            entity.entity_type == NOTIFICATION_ENTITY_TYPE
                && !entity
                    .properties
                    .get("dismissed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InboxRowKind {
    Task,
    Notification,
}

/// Tasks first, notifications after -- a task needing confirmation is
/// more actionable than an informational notice, not sorted by
/// recency. Shared by rendering (`inbox_content_cards`) and hit-testing
/// (`inbox_row_at`) so the two can never disagree about row order.
fn inbox_rows(entities: &[Entity]) -> Vec<(InboxRowKind, &Entity)> {
    let mut rows: Vec<(InboxRowKind, &Entity)> = inbox_pending_tasks(entities)
        .into_iter()
        .map(|entity| (InboxRowKind::Task, entity))
        .collect();
    rows.extend(
        inbox_notifications(entities)
            .into_iter()
            .map(|entity| (InboxRowKind::Notification, entity)),
    );
    rows
}

fn inbox_row_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    entities: &[Entity],
) -> Option<(InboxRowKind, Uuid)> {
    inbox_rows(entities)
        .into_iter()
        .enumerate()
        .find(|(index, _)| stacked_row_rect(*index, width, height).contains(pos.0, pos.1))
        .map(|(_, (kind, entity))| (kind, entity.id))
}

/// S13 Change 4: "Сейчас"'s hit-test, mirroring `content_action_at`
/// but for a page that mixes a runtime-sized app list (cells
/// 0..apps.len()) with the two remaining static `root.sui` cards
/// (cells apps.len()..). S23 moved this from `stacked_row_rect`'s
/// single column to `now_grid_rect`'s 3-column grid -- the index
/// math is unchanged, only which rect function turns an index into a
/// screen position. Returns the same `action` string either
/// kind of card would carry, so the caller dispatches identically to
/// how `invoke_content_action` used to.
fn now_action_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    installed_apps: &BTreeMap<String, AppSummary>,
) -> Option<String> {
    for (index, app) in installed_apps.values().enumerate() {
        if now_grid_rect(index, width, height).contains(pos.0, pos.1) {
            return Some(format!("manage_app:{}", app.id));
        }
    }
    let base = installed_apps.len();
    ROOT_CONTENT_ACTIONS
        .iter()
        .filter(|action| action.page == "now")
        .enumerate()
        .find(|(offset, _)| now_grid_rect(base + offset, width, height).contains(pos.0, pos.1))
        .map(|(_, action)| action.action.to_string())
}

/// Physically discovered during the S14-S25 series' first device
/// pass: 14 fixed rows plus N installed apps stopped fitting on one
/// screen a while before S25 even landed, and this shell has never
/// had any scroll/drag gesture recognition (`TouchHandler::motion`
/// only ever tracks a position, never a delta) -- several settings
/// (Wi-Fi onward) were silently untappable. Paginated the same way
/// "Wi-Fi сети"/"Bluetooth устройства" already page past their own
/// fixed trailing rows, rather than inventing gesture recognition:
/// `ME_PAGE_SIZE` real rows per page plus one or two navigation rows
/// ("Ещё"/"Назад"), `Shell.me_page` tracking which page is open.
const ME_PAGE_SIZE: usize = 6;
/// Count of `me_all_card_views`'s fixed (non-app) entries -- kept as
/// one literal here rather than derived from that function's return
/// length, since `me_fixed_card_action` has to agree with it and
/// there's no way to assert two functions' lengths match at compile
/// time anyway.
const ME_FIXED_CARD_COUNT: usize = 14;

/// The `me_all_card_views`'s logical index -> tap action mapping.
/// `None` for the three read-only info rows (device summary, build
/// info, storage), "Обновления" (S22, read-only by design), and
/// every installed-app row (info-only on "Я", unlike "Сейчас"'s
/// `now_action_at`).
fn me_fixed_card_action(logical_index: usize) -> Option<&'static str> {
    match logical_index {
        3 => Some("cycle_brightness"),
        4 => Some("cycle_idle_timeout"),
        5 => Some("cycle_deep_idle_timeout"),
        6 => Some("cycle_timezone"),
        8 => Some("cycle_volume"),
        9 => Some("open_wifi_list"),
        10 => Some("open_bluetooth_list"),
        11 => Some("open_pin_setup"),
        12 => Some("cycle_text_scale"),
        13 => Some("cycle_contrast"),
        _ => None,
    }
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
    // eventually need (Change 6) -- real content (time/Wi-Fi/battery)
    // added S13 Change 1, see `present_status_bar`.
    let bar_surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(
        &qh,
        bar_surface,
        Layer::Top,
        Some("saai-shell-statusbar"),
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
            eprintln!("saai-shell: system font unavailable, continuing without text: {error}");
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
    let settings = ShellSettings::load();
    apply_brightness(settings.brightness_pct);
    apply_volume(settings.volume_pct);
    render::set_text_scale(settings.text_scale_pct);
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
        lock_width: 0,
        lock_height: 0,
        locked: true,
        unlock_pending: false,
        sleeping: false,
        last_activity: Instant::now(),
        current_page: RootPage::Now,
        last_touch_pos: (0.0, 0.0),
        tab_touch_pending: false,
        layer,
        layer_width: 0,
        layer_height: 120,
        layer_pool: None,
        layer_buffer: None,
        last_statusbar_refresh: Instant::now(),
        low_battery_notified: false,
        fonts,
        appd: appd_client::AppdClient::new(appd_socket),
        pending_consent: None,
        intent_input: None,
        wifi_password: None,
        wifi_list: None,
        bluetooth_list_open: false,
        pin_setup: None,
        pin_entry_buffer: String::new(),
        me_page: 0,
        entityd: entityd_client::EntitydClient::new(entityd_socket),
        spaces: Vec::new(),
        selected_space_id: "home".into(),
        entity_counts: BTreeMap::new(),
        confirming_task_id: None,
        selected_entities: Vec::new(),
        portal: portal_server::PortalServer::bind(portal_socket)
            .expect("failed to bind saai-shell portal socket"),
        apps_by_pid: BTreeMap::new(),
        installed_apps: BTreeMap::new(),
        apps_grants: BTreeMap::new(),
        clipboard: None,
        last_apps_refresh: Instant::now(),
        settings,
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
        shell.refresh_statusbar_if_due();
        shell.poll_portal();
        shell.check_idle_timeout(&qh);
        shell.check_deep_idle(&conn);
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
    /// Size the lock surface was last `configure()`d at -- needed by
    /// `present_lock_surface()` to redraw it later (e.g. the sleep
    /// indicator around a real suspend, S11 Change 2) outside of a fresh
    /// configure event, where `configure`'s own `new_size` parameter
    /// isn't available. Matches `self.width`/`self.height` in practice
    /// (both ultimately come from the same single physical panel), but
    /// tracked separately since nothing guarantees the two configure
    /// events always agree, and this is the value the lock surface's
    /// own buffer was actually sized for.
    lock_width: u32,
    lock_height: u32,
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
    /// Set once `check_deep_idle` blanks the lock surface to
    /// `SLEEP_INDICATOR_COLOR`; cleared by the next touch, which wakes
    /// the screen back to `LOCK_SCREEN_COLOR` instead of unlocking --
    /// unlocking still needs its own separate tap-and-release on the
    /// now-visible lock screen, same two-step as pressing a real
    /// phone's power button before swiping to unlock.
    sleeping: bool,
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
    /// S13 Change 1: throttles `refresh_statusbar_if_due` the same way
    /// `last_apps_refresh` throttles `refresh_apps_if_due`.
    last_statusbar_refresh: Instant,
    /// S21: guards `check_low_battery` against creating a fresh
    /// notification every second while the battery stays low.
    low_battery_notified: bool,
    fonts: Option<render::Fonts>,
    appd: appd_client::AppdClient,
    /// Set while a launch is blocked on the ADR-020 consent screen -- see
    /// `apply_appd_message`'s `ConsentRequired`/`ConsentDecided` handling.
    pending_consent: Option<PendingConsent>,
    /// S09 Change 2: set while the on-screen keyboard is composing a new
    /// `saaios.intent`. Modal, same as `pending_consent`.
    intent_input: Option<IntentInputState>,
    /// S19: set while the on-screen keyboard is composing a
    /// secured Wi-Fi network's password. Modal, same as
    /// `intent_input` -- and reuses the exact same keyboard layout
    /// (see `WifiPasswordState`'s doc comment).
    wifi_password: Option<WifiPasswordState>,
    /// S19: set while "Wi-Fi сети" is open, populated by
    /// `wifi_scan_results()` when opened and on each "Обновить" tap.
    /// `None` (not merely empty) means the screen itself is closed.
    wifi_list: Option<Vec<WifiNetwork>>,
    /// S20: unlike `wifi_list`, no snapshot to carry -- "Bluetooth
    /// устройства"'s rows are always recomputed straight from
    /// `BT_SCAN_LOG_PATH` on every draw (see `bluetooth_scan_
    /// results`'s doc comment), so this is just whether the screen is
    /// open at all.
    bluetooth_list_open: bool,
    /// S24: set while "Изменить PIN" (opened from "Я") is composing a
    /// new PIN. Modal, same as the others.
    pin_setup: Option<PinSetupState>,
    /// S24: digits entered so far against the lock surface's own
    /// keypad -- cleared on a wrong guess, a successful unlock, or
    /// waking from `sleeping`. Unrelated to `pin_setup`'s buffer
    /// (setting a new PIN vs. entering the existing one to unlock are
    /// different screens, on different surfaces, that happen never to
    /// be open at the same time -- `locked` is always `true` while
    /// this one matters and always `false` while `pin_setup` does).
    pin_entry_buffer: String,
    /// See `ME_PAGE_SIZE`'s doc comment -- which page of "Я"'s
    /// settings/app list is currently showing. Deliberately not
    /// reset when leaving "Я" for another tab -- returning to it
    /// keeps the page the user was last looking at.
    me_page: usize,
    entityd: entityd_client::EntitydClient,
    spaces: Vec<Space>,
    selected_space_id: String,
    entity_counts: BTreeMap<String, usize>,
    selected_entities: Vec<Entity>,
    /// S13 Change 2: which task the modal on top of "Входящие" is
    /// currently showing -- `Some` only after the user taps a specific
    /// row in that page's list, `None` again once accepted/declined.
    /// Replaces the old always-auto-popup-the-first-one behavior (see
    /// `confirming_task`) with an explicit, page-scoped entry point.
    confirming_task_id: Option<Uuid>,
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
    /// S13 Change 3/4: the full `AppSummary` per installed app, keyed
    /// by id -- `apps_by_pid`/`apps_grants` above only ever kept the
    /// two derived slices the portal needed, not enough to show a real
    /// app list ("Я"'s own grants view, "Сейчас"'s launcher) with
    /// names, versions and state.
    installed_apps: BTreeMap<String, AppSummary>,
    /// The portal's in-memory clipboard. No persistence, no history --
    /// cleared on `saai-shell` restart.
    clipboard: Option<String>,
    last_apps_refresh: Instant,
    /// S16: persisted, user-changeable (brightness, both idle
    /// timeouts) -- replaces the old fixed `IDLE_TIMEOUT` constant and
    /// per-instance-but-still-fixed-at-startup `deep_idle_timeout`
    /// field from S11.
    settings: ShellSettings,
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
        self.sleeping = false;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock_surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;
        let (width, height) = (width.max(1), height.max(1));
        println!("saai-shell: lock surface configure at {width}x{height}");
        self.lock_width = width;
        self.lock_height = height;
        // A fresh configure means a new size -- the old buffer (if any)
        // was sized for the previous one and `present_lock_surface()`
        // never resizes an existing buffer itself.
        self.lock_buffer = None;
        self.present_lock_pin_entry();
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
        // A fresh configure means a new size -- the old buffer (if any)
        // was sized for the previous one, same reasoning as the lock
        // surface's own `configure`.
        self.layer_buffer = None;
        self.present_status_bar();
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
        if self.sleeping {
            // First touch after the screen went dark just wakes it back
            // to the lock screen -- it does not also unlock, matching a
            // real phone's press-then-swipe two-step. Consumes this
            // touch entirely so a stray drag can't fall through into
            // unlock_pending below.
            self.sleeping = false;
            self.unlock_pending = false;
            self.tab_touch_pending = false;
            self.pin_entry_buffer.clear();
            self.present_lock_pin_entry();
            println!("saai-shell: woke from pseudo-sleep");
            return;
        }
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
            // S24: opt-in -- with no PIN set, every release on the lock
            // surface still unlocks immediately, unchanged from before
            // this sprint. With a PIN set, a release is a keypad tap,
            // not an unlock by itself.
            match self.settings.pin_code.clone() {
                None => {
                    if let Some(session_lock) = self.session_lock.take() {
                        session_lock.unlock();
                    }
                    self.lock_surfaces.clear();
                    self.locked = false;
                    println!("saai-shell: unlocked by touch");
                }
                Some(pin_code) => {
                    if let Some(key) =
                        pin_keypad_action_at(self.last_touch_pos, self.lock_width, self.lock_height)
                    {
                        if key == "⌫" {
                            self.pin_entry_buffer.pop();
                        } else {
                            self.pin_entry_buffer.push_str(key);
                        }
                        if self.pin_entry_buffer.len() >= pin_code.len() {
                            if self.pin_entry_buffer == pin_code {
                                self.pin_entry_buffer.clear();
                                if let Some(session_lock) = self.session_lock.take() {
                                    session_lock.unlock();
                                }
                                self.lock_surfaces.clear();
                                self.locked = false;
                                println!("saai-shell: unlocked by PIN");
                                return;
                            }
                            println!("saai-shell: PIN mismatch, retry");
                            self.pin_entry_buffer.clear();
                        }
                        self.present_lock_pin_entry();
                    }
                }
            }
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
            } else if self.confirming_task_id.is_some() {
                // Modal, same as consent: the dangerous-Task
                // confirmation screen owns every touch while it's
                // showing (S09 Change 3 / ADR-031's follow-up). Only
                // reachable by first tapping a row on "Входящие" (S13
                // Change 2) -- no longer auto-opened.
                if let Some(confirm) =
                    task_confirm_action_at(self.last_touch_pos, self.width, self.height)
                {
                    self.confirm_pending_task(confirm);
                    self.draw(conn, qh);
                }
            } else if self.intent_input.is_some() {
                // Modal, same as consent: the on-screen keyboard owns
                // every touch while it's showing.
                if let Some(action) = intent_action_at(self.last_touch_pos, self.width, self.height)
                {
                    self.handle_intent_input_action(&action, conn, qh);
                }
            } else if self.pin_setup.is_some() {
                // S24: modal, same as the others.
                let has_existing_pin = self.settings.pin_code.is_some();
                if let Some(key) = pin_setup_action_at(
                    self.last_touch_pos,
                    self.width,
                    self.height,
                    has_existing_pin,
                ) {
                    self.handle_pin_setup_action(key, conn, qh);
                }
            } else if self.wifi_password.is_some() {
                // S19: same keyboard tree as `intent_input` above
                // (see `WifiPasswordState`'s doc comment) -- checked
                // first since it visually sits on top of "Wi-Fi
                // сети" while it's open.
                if let Some(action) = intent_action_at(self.last_touch_pos, self.width, self.height)
                {
                    self.handle_wifi_password_action(&action, conn, qh);
                }
            } else if self.wifi_list.is_some() {
                // S19: modal, same as the others -- owns every touch
                // while "Wi-Fi сети" is open.
                let network_count = self.wifi_list.as_ref().map_or(0, Vec::len);
                if let Some(tap) =
                    wifi_list_action_at(self.last_touch_pos, self.width, self.height, network_count)
                {
                    self.handle_wifi_list_tap(tap, conn, qh);
                }
            } else if self.bluetooth_list_open {
                // S20: modal, same as the others -- owns every touch
                // while "Bluetooth устройства" is open. Recomputes the
                // device count fresh (see `bluetooth_list_open`'s doc
                // comment) rather than reading a stored snapshot.
                let device_count = bluetooth_scan_results().0.len();
                if let Some(tap) = bluetooth_list_action_at(
                    self.last_touch_pos,
                    self.width,
                    self.height,
                    device_count,
                ) {
                    self.handle_bluetooth_list_tap(tap, conn, qh);
                }
            } else if let Some(page) = tab_at(self.last_touch_pos, self.width, self.height) {
                if page != self.current_page {
                    println!("saai-shell: switched to {page:?}");
                    self.current_page = page;
                    self.draw(conn, qh);
                }
            } else if self.current_page == RootPage::Inbox {
                // S13 Change 2 (tasks), extended S21 (notifications):
                // "Входящие" has no `root.sui` entries, so its rows
                // aren't reachable through `content_action_at` below --
                // this page's tap target is entirely runtime data.
                if let Some((kind, id)) =
                    inbox_row_at(self.last_touch_pos, self.width, self.height, &self.selected_entities)
                {
                    match kind {
                        InboxRowKind::Task => self.confirming_task_id = Some(id),
                        InboxRowKind::Notification => self.dismiss_notification(id),
                    }
                    self.draw(conn, qh);
                }
            } else if self.current_page == RootPage::Now {
                // S13 Change 4: same reasoning as "Входящие" above --
                // the app-list rows have no `root.sui` entries, so
                // `content_action_at` alone can't find them (or, now,
                // even correctly locate the two cards that remain
                // static, since they're repositioned below the
                // runtime-sized app list).
                if let Some(action_str) = now_action_at(
                    self.last_touch_pos,
                    self.width,
                    self.height,
                    &self.installed_apps,
                ) {
                    if let Some(app_id) = action_str.strip_prefix("manage_app:") {
                        let app_id = app_id.to_string();
                        self.invoke_app_launch(&app_id, conn, qh);
                    } else if action_str == "open_intent_input" {
                        self.intent_input = Some(IntentInputState::default());
                        self.draw(conn, qh);
                    }
                    // "inspect_selected_entity" has no tap behavior --
                    // matches this card's pre-existing no-op (it was
                    // never wired into `invoke_content_action` either).
                }
            } else if self.current_page == RootPage::Me {
                // S16: same reasoning again -- the three settings rows
                // sit below "Я"'s two-then-three fixed info rows, at
                // indices `content_action_at`'s `root.sui`-driven table
                // (which has no "me" entries at all) can't reach.
                if let Some(action) = self.me_action_at(self.last_touch_pos, self.width, self.height) {
                    self.invoke_me_action(action, conn, qh);
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
                app_name: pending.app_name.clone(),
                labels,
                header,
                accept: buttons[0].rect,
                decline: buttons[1].rect,
            }
        } else if let Some(task) = self.confirming_task() {
            let view = task_confirm_view(width, height);
            let header = view.children[0].rect;
            let buttons = &view.children[1].children;
            Frame::TaskConfirm {
                title: task.title.clone(),
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
        } else if let Some(state) = &self.pin_setup {
            let header = Rect::new(0, 0, width, INTENT_HEADER_HEIGHT);
            let has_existing_pin = self.settings.pin_code.is_some();
            let mut keys: Vec<(Rect, &'static str)> = PIN_KEYPAD_DIGIT_LABELS
                .iter()
                .enumerate()
                .filter(|(_, label)| !label.is_empty())
                .map(|(index, label)| (pin_keypad_rect(index, width, height), *label))
                .collect();
            keys.extend(
                pin_setup_controls(has_existing_pin)
                    .into_iter()
                    .enumerate()
                    .map(|(offset, label)| (pin_keypad_rect(12 + offset, width, height), label)),
            );
            Frame::PinSetup {
                buffer: state.buffer.clone(),
                header,
                keys,
            }
        } else if let Some(state) = &self.wifi_password {
            // Same tree as `intent_input` above, reused verbatim --
            // see `WifiPasswordState`'s doc comment.
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
            Frame::WifiPasswordInput {
                ssid: state.ssid.clone(),
                buffer: state.buffer.clone(),
                header,
                keys,
            }
        } else if let Some(networks) = &self.wifi_list {
            // S19: runtime-sized, like "Входящие"/"Сейчас" -- see
            // `wifi_list_action_at`'s own doc comment for why this
            // uses `stacked_row_rect` instead of a `root.sui` entry.
            let header = Rect::new(0, 0, width, INTENT_HEADER_HEIGHT);
            let mut rows: Vec<(Rect, String)> = networks
                .iter()
                .enumerate()
                .map(|(index, network)| {
                    let label = format!(
                        "{}   ·   {}   ·   {} dBm",
                        network.ssid,
                        if network.secured { "защищена" } else { "открыта" },
                        network.signal_dbm
                    );
                    (stacked_row_rect(index, width, height), label)
                })
                .collect();
            rows.push((
                stacked_row_rect(networks.len(), width, height),
                "Обновить".to_string(),
            ));
            rows.push((
                stacked_row_rect(networks.len() + 1, width, height),
                "Назад".to_string(),
            ));
            Frame::WifiList {
                header,
                status_line: wifi_status_line(),
                rows,
            }
        } else if self.bluetooth_list_open {
            // S20: same runtime-sized-list shape as the Wi-Fi branch
            // above, but rows/status are recomputed straight from
            // disk each time (see `bluetooth_list_open`'s doc
            // comment) instead of reading a stored snapshot.
            let header = Rect::new(0, 0, width, INTENT_HEADER_HEIGHT);
            let (devices, _done) = bluetooth_scan_results();
            let mut rows: Vec<(Rect, String)> = devices
                .iter()
                .enumerate()
                .map(|(index, device)| {
                    let label = if device.transport.is_empty() {
                        device.name.clone()
                    } else {
                        format!("{}   ·   {}", device.name, device.transport)
                    };
                    (stacked_row_rect(index, width, height), label)
                })
                .collect();
            rows.push((
                stacked_row_rect(devices.len(), width, height),
                "Искать устройства (~8 с)".to_string(),
            ));
            rows.push((
                stacked_row_rect(devices.len() + 1, width, height),
                "Обновить список".to_string(),
            ));
            rows.push((
                stacked_row_rect(devices.len() + 2, width, height),
                "Назад".to_string(),
            ));
            Frame::BluetoothList {
                header,
                status_line: bluetooth_status_summary(),
                rows,
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
            let content_cards = if self.current_page == RootPage::Inbox {
                self.inbox_content_cards(width, height)
            } else if self.current_page == RootPage::Me {
                self.me_content_cards(width, height)
            } else if self.current_page == RootPage::Now {
                self.now_content_cards(width, height)
            } else {
                ROOT_CONTENT_ACTIONS
                    .iter()
                    .filter(|action| action.page == self.current_page.id())
                    .map(|action| {
                        (
                            content_action_rect(action, width, height),
                            self.content_card(action),
                        )
                    })
                    .collect::<Vec<_>>()
            };
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
                    &app_name,
                    &labels,
                    header,
                    accept,
                    decline,
                    self.fonts.as_ref(),
                );
            }
            Frame::TaskConfirm {
                title,
                header,
                accept,
                decline,
            } => {
                render::draw_task_confirm(
                    &mut render::Canvas::new(canvas, width, height),
                    &title,
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
                    "Новое намерение",
                    &buffer,
                    header,
                    &keys,
                    self.fonts.as_ref(),
                );
            }
            Frame::PinSetup {
                buffer,
                header,
                keys,
            } => {
                render::draw_pin_setup(
                    &mut render::Canvas::new(canvas, width, height),
                    &buffer,
                    header,
                    &keys,
                    self.fonts.as_ref(),
                );
            }
            Frame::WifiPasswordInput {
                ssid,
                buffer,
                header,
                keys,
            } => {
                // Password preview is masked (unlike the intent
                // keyboard's plaintext echo) -- what's actually typed
                // stays in `buffer`/`state.buffer`, only the on-screen
                // preview substitutes a dot per character.
                let masked: String = buffer.chars().map(|_| '•').collect();
                render::draw_intent_input(
                    &mut render::Canvas::new(canvas, width, height),
                    &format!("Пароль для «{ssid}»"),
                    &masked,
                    header,
                    &keys,
                    self.fonts.as_ref(),
                );
            }
            Frame::WifiList {
                header,
                status_line,
                rows,
            } => {
                render::draw_row_list(
                    &mut render::Canvas::new(canvas, width, height),
                    "Wi-Fi сети",
                    &status_line,
                    header,
                    &rows,
                    self.fonts.as_ref(),
                );
            }
            Frame::BluetoothList {
                header,
                status_line,
                rows,
            } => {
                render::draw_row_list(
                    &mut render::Canvas::new(canvas, width, height),
                    "Bluetooth устройства",
                    &status_line,
                    header,
                    &rows,
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
                    self.current_page == RootPage::Now,
                );
            }
        }
        render::apply_contrast_boost(canvas, self.settings.contrast_pct);

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
        }
    }

    /// S13 Change 4: launches an already-installed app by id -- the
    /// generic replacement for the old demo-only branch here (removed
    /// along with `DemoAppState`; that state machine's `Missing`/
    /// `Error` cases existed to bootstrap-install the demo specifically,
    /// a capability this generic list intentionally doesn't have, see
    /// ADR-056). A no-op if already running, matching the old demo
    /// behavior for `Running`/`Pending`.
    fn invoke_app_launch(&mut self, app_id: &str, conn: &Connection, qh: &QueueHandle<Self>) {
        let already_running = self
            .installed_apps
            .get(app_id)
            .is_some_and(|app| app.state == "running");
        if !already_running {
            self.appd.launch(app_id);
        }
        self.draw(conn, qh);
    }

    /// S16: advances whichever setting `action` names to the next
    /// value in its own fixed cycle, applies the side effect that
    /// setting actually controls (only brightness has one -- the two
    /// timeouts are read fresh by `check_idle_timeout`/`check_deep_idle`
    /// every tick, nothing to push), persists, and redraws so the
    /// card's own status text reflects the new value immediately.
    /// See `ME_PAGE_SIZE`'s doc comment -- a method now (not a free
    /// function) since it needs `self.me_page` and `self.installed_
    /// apps.len()` to know which logical indices the current page
    /// covers, the same two pieces `me_content_cards` below needs to
    /// build the matching rects.
    fn me_action_at(&self, pos: (f64, f64), width: u32, height: u32) -> Option<&'static str> {
        let total = ME_FIXED_CARD_COUNT + self.installed_apps.len();
        let start = self.me_page * ME_PAGE_SIZE;
        let visible_count = total.saturating_sub(start).min(ME_PAGE_SIZE);
        for local_index in 0..visible_count {
            if stacked_row_rect(local_index, width, height).contains(pos.0, pos.1) {
                return me_fixed_card_action(start + local_index);
            }
        }
        let mut nav_index = visible_count;
        let has_more = start + ME_PAGE_SIZE < total;
        if has_more {
            if stacked_row_rect(nav_index, width, height).contains(pos.0, pos.1) {
                return Some("me_page_next");
            }
            nav_index += 1;
        }
        if self.me_page > 0 && stacked_row_rect(nav_index, width, height).contains(pos.0, pos.1) {
            return Some("me_page_prev");
        }
        None
    }

    fn invoke_me_action(&mut self, action: &str, conn: &Connection, qh: &QueueHandle<Self>) {
        match action {
            "cycle_brightness" => {
                self.settings.brightness_pct =
                    next_in_cycle(&BRIGHTNESS_LEVELS_PCT, self.settings.brightness_pct);
                apply_brightness(self.settings.brightness_pct);
            }
            "cycle_idle_timeout" => {
                self.settings.idle_timeout_secs =
                    next_in_cycle(&IDLE_TIMEOUT_LEVELS_SECS, self.settings.idle_timeout_secs);
            }
            "cycle_deep_idle_timeout" => {
                self.settings.deep_idle_timeout_secs = next_in_cycle(
                    &DEEP_IDLE_TIMEOUT_LEVELS_SECS,
                    self.settings.deep_idle_timeout_secs,
                );
            }
            "cycle_timezone" => {
                self.settings.utc_offset_minutes =
                    next_in_cycle(&TIMEZONE_PRESETS_MINUTES, self.settings.utc_offset_minutes);
            }
            "cycle_volume" => {
                self.settings.volume_pct =
                    next_in_cycle(&VOLUME_LEVELS_PCT, self.settings.volume_pct);
                apply_volume(self.settings.volume_pct);
            }
            "open_wifi_list" => {
                // No setting changes here (unlike every other arm
                // above) -- opens a screen rather than cycling a
                // persisted value, so this returns before the
                // `settings.save()` common tail.
                wifi_trigger_scan();
                self.wifi_list = Some(wifi_scan_results());
                self.draw(conn, qh);
                return;
            }
            "open_bluetooth_list" => {
                // Same reasoning as "open_wifi_list" above.
                bluetooth_trigger_scan();
                self.bluetooth_list_open = true;
                self.draw(conn, qh);
                return;
            }
            "open_pin_setup" => {
                // Same reasoning as "open_wifi_list" above.
                self.pin_setup = Some(PinSetupState::default());
                self.draw(conn, qh);
                return;
            }
            "cycle_text_scale" => {
                self.settings.text_scale_pct =
                    next_in_cycle(&TEXT_SCALE_LEVELS_PCT, self.settings.text_scale_pct);
                render::set_text_scale(self.settings.text_scale_pct);
            }
            "cycle_contrast" => {
                self.settings.contrast_pct =
                    next_in_cycle(&CONTRAST_LEVELS_PCT, self.settings.contrast_pct);
            }
            "me_page_next" => {
                self.me_page += 1;
                self.draw(conn, qh);
                return;
            }
            "me_page_prev" => {
                self.me_page = self.me_page.saturating_sub(1);
                self.draw(conn, qh);
                return;
            }
            _ => return,
        }
        self.settings.save();
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

    /// S24: "Изменить PIN"'s own keypad handling -- digits accumulate
    /// in `state.buffer`; "Готово" saves it (a minimum length guards
    /// against an accidental one-digit PIN, no maximum), "Отмена"
    /// discards, "Убрать PIN" clears `settings.pin_code` outright
    /// (only reachable when one is already set -- see
    /// `pin_setup_controls`).
    fn handle_pin_setup_action(&mut self, key: &str, conn: &Connection, qh: &QueueHandle<Self>) {
        let Some(state) = self.pin_setup.as_mut() else {
            return;
        };
        match key {
            "Отмена" => {
                self.pin_setup = None;
            }
            "⌫" => {
                state.buffer.pop();
            }
            "Готово" => {
                let pin = state.buffer.clone();
                if pin.len() >= 4 {
                    self.settings.pin_code = Some(pin);
                    self.settings.save();
                    self.pin_setup = None;
                }
                // Too short: stays open, same as before the tap --
                // no error UI, but also no silent partial save.
            }
            "Убрать PIN" => {
                self.settings.pin_code = None;
                self.settings.save();
                self.pin_setup = None;
            }
            digit => {
                state.buffer.push_str(digit);
            }
        }
        self.draw(conn, qh);
    }

    /// S19: drives the same keyboard tree as `handle_intent_input_
    /// action`, opened instead by tapping a secured network on "Wi-Fi
    /// сети" (`handle_wifi_list_tap`). `intent:send` here means
    /// "connect", not "create an entity".
    fn handle_wifi_password_action(
        &mut self,
        action: &str,
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let Some(state) = self.wifi_password.as_mut() else {
            return;
        };
        match action {
            INTENT_CANCEL_ACTION => {
                self.wifi_password = None;
            }
            INTENT_SPACE_ACTION => {
                state.buffer.push(' ');
            }
            INTENT_BACKSPACE_ACTION => {
                state.buffer.pop();
            }
            INTENT_SEND_ACTION => {
                let ssid = state.ssid.clone();
                let psk = state.buffer.clone();
                if !psk.is_empty() {
                    wifi_connect_psk(&ssid, &psk);
                    println!("saai-shell: connecting to \"{ssid}\" (WPA)");
                }
                self.wifi_password = None;
                self.wifi_list = None;
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

    /// S19: "Wi-Fi сети"'s own tap handling -- see `WifiListTap`'s doc
    /// comment for why the row layout is runtime-computed rather than
    /// a `root.sui` entry.
    fn handle_wifi_list_tap(&mut self, tap: WifiListTap, conn: &Connection, qh: &QueueHandle<Self>) {
        match tap {
            WifiListTap::Network(index) => {
                let Some(network) = self.wifi_list.as_ref().and_then(|list| list.get(index))
                else {
                    return;
                };
                if network.secured {
                    self.wifi_password = Some(WifiPasswordState {
                        ssid: network.ssid.clone(),
                        buffer: String::new(),
                    });
                } else {
                    let ssid = network.ssid.clone();
                    wifi_connect_open(&ssid);
                    println!("saai-shell: connecting to \"{ssid}\" (open)");
                    self.wifi_list = None;
                }
            }
            WifiListTap::Refresh => {
                wifi_trigger_scan();
                self.wifi_list = Some(wifi_scan_results());
            }
            WifiListTap::Back => {
                self.wifi_list = None;
            }
        }
        self.draw(conn, qh);
    }

    /// S20: "Bluetooth устройства"'s own tap handling. `Refresh` looks
    /// like a no-op (it changes no state) but isn't really one --
    /// `draw()` re-reads both log files fresh every call, so tapping
    /// it is what actually makes a scan/pair result in progress show
    /// up, the same way tapping anything else that redraws would.
    fn handle_bluetooth_list_tap(
        &mut self,
        tap: BluetoothListTap,
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match tap {
            BluetoothListTap::Device(index) => {
                bluetooth_pair(index);
            }
            BluetoothListTap::Scan => {
                bluetooth_trigger_scan();
            }
            BluetoothListTap::Refresh => {}
            BluetoothListTap::Back => {
                self.bluetooth_list_open = false;
            }
        }
        self.draw(conn, qh);
    }

    fn poll_appd(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        let was_available = self.appd.is_connected();
        let messages = self.appd.poll();
        let available = self.appd.is_connected();
        let mut changed = was_available != available;
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
    /// `apps_grants`) current, plus `installed_apps` (S13 Change 3/4) --
    /// separate from `apply_appd_message`'s own match because that one
    /// only cares about `ConsentRequired`/`ConsentDecided`, while this
    /// one has to fold in every kind of app-list-affecting message.
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
                self.installed_apps.clear();
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
                        self.installed_apps.remove(&event.app_id);
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
        self.installed_apps.insert(app.id.clone(), app.clone());
    }

    /// S13 Change 4: generic over every installed app, not just the
    /// demo -- `update_app_caches` already keeps `installed_apps`/
    /// `apps_by_pid`/`apps_grants` current for any app id, so the only
    /// thing left here is `pending_consent`, which was already
    /// app-agnostic on the *receiving* end (keyed off whatever `app_id`
    /// the server names) even when this method only ever special-cased
    /// the demo's own launch/install flow. Always reports "changed" --
    /// this only ever runs once per message that actually arrived, so
    /// unconditionally redrawing costs nothing an empty poll wouldn't
    /// already skip.
    fn apply_appd_message(&mut self, message: AppServerMessage) -> bool {
        self.update_app_caches(&message);
        match message {
            AppServerMessage::Response {
                result: Some(AppResponseResult::ConsentRequired { app_id, requested }),
                ..
            } => {
                let app_name = self
                    .installed_apps
                    .get(&app_id)
                    .map(|app| app.name.clone())
                    .unwrap_or_else(|| app_id.clone());
                self.pending_consent = Some(PendingConsent {
                    app_id,
                    app_name,
                    requested,
                });
            }
            AppServerMessage::Response {
                result: Some(AppResponseResult::ConsentDecided { app_id, .. }),
                ..
            } => {
                self.pending_consent = None;
                // Accept or decline, the daemon now has a decision that
                // covers this request -- retry the launch the user
                // originally asked for; it can no longer come back as
                // ConsentRequired for the same capability set.
                self.appd.launch(app_id);
            }
            _ => {}
        }
        true
    }

    /// S13 Change 2: "Входящие"'s own content, built from live task
    /// data instead of `root.sui`. An honest empty state ("нет задач")
    /// rather than the placeholder gray rows `draw_root` would
    /// otherwise draw for a page with zero real cards -- Acceptance
    /// criteria explicitly called this out during the DoR.
    fn inbox_content_cards(&self, width: u32, height: u32) -> Vec<(Rect, render::ActionCardView)> {
        let rows = inbox_rows(&self.selected_entities);
        if rows.is_empty() {
            return vec![(
                stacked_row_rect(0, width, height),
                render::ActionCardView::new("Нет новых задач и уведомлений", "", ""),
            )];
        }
        rows.into_iter()
            .enumerate()
            .map(|(index, (kind, entity))| {
                let (status, action) = match kind {
                    InboxRowKind::Task => ("Ждёт подтверждения", "Открыть"),
                    // S21: the notification's own `body` property is
                    // its message; the title is used as the card
                    // label, same split as a task's title/status.
                    InboxRowKind::Notification => (
                        entity
                            .properties
                            .get("body")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        "Скрыть",
                    ),
                };
                (
                    stacked_row_rect(index, width, height),
                    render::ActionCardView::new(entity.title.clone(), status, action),
                )
            })
            .collect()
    }

    /// S13 Change 3: "Я" -- a device/apps summary built entirely from
    /// state this client already tracks (`spaces`, `entity_counts`,
    /// `installed_apps`) plus each app's currently granted
    /// capabilities (`capability_label`, same vocabulary the consent
    /// screen already uses). Read-only -- no protocol supports
    /// revoking one capability from an already-decided app (see
    /// ADR-054's notes on `saai-app-protocol`), so there is nothing
    /// for a tap here to do yet.
    /// See `ME_PAGE_SIZE`'s doc comment -- slices `me_all_card_views`
    /// into the current page's window and appends "Ещё"/"Назад" nav
    /// rows, the same pattern `Frame::WifiList`/`Frame::BluetoothList`
    /// already use for their own trailing controls.
    fn me_content_cards(&self, width: u32, height: u32) -> Vec<(Rect, render::ActionCardView)> {
        let all = self.me_all_card_views();
        let total = all.len();
        let start = self.me_page * ME_PAGE_SIZE;
        let visible: Vec<render::ActionCardView> = all.into_iter().skip(start).take(ME_PAGE_SIZE).collect();
        let visible_count = visible.len();
        let mut cards: Vec<(Rect, render::ActionCardView)> = visible
            .into_iter()
            .enumerate()
            .map(|(local_index, card)| (stacked_row_rect(local_index, width, height), card))
            .collect();
        let mut nav_index = visible_count;
        if start + ME_PAGE_SIZE < total {
            cards.push((
                stacked_row_rect(nav_index, width, height),
                render::ActionCardView::new(
                    "Ещё",
                    format!("Показаны {}-{} из {total}", start + 1, start + visible_count),
                    "Вниз",
                ),
            ));
            nav_index += 1;
        }
        if self.me_page > 0 {
            cards.push((
                stacked_row_rect(nav_index, width, height),
                render::ActionCardView::new("Назад", "", "Вверх"),
            ));
        }
        cards
    }

    /// The full, unpaginated logical list "Я" shows -- same order and
    /// content as before pagination existed, just without rects
    /// (`me_content_cards` assigns those per-page).
    fn me_all_card_views(&self) -> Vec<render::ActionCardView> {
        let total_entities: usize = self.entity_counts.values().sum();
        let mut cards = vec![
            render::ActionCardView::new(
                "Это устройство",
                format!(
                    "Пространств: {} · Объектов: {total_entities}",
                    self.spaces.len()
                ),
                "",
            ),
            // S14: "О телефоне" -- build id is compiled in (`build.rs`);
            // model/kernel/uptime are read fresh every draw since uptime
            // obviously changes and the other two are cheap enough not
            // to bother caching.
            render::ActionCardView::new(
                format!("SaaiOS · сборка {}", env!("SAAIOS_BUILD_ID")),
                format!(
                    "{} · ядро {} · работает {}",
                    hardware_model(),
                    kernel_release(),
                    uptime_string()
                ),
                "",
            ),
            // S15: `/data` usage -- the one partition apps/user state
            // actually lives on (S05's `--data-root`), a more useful
            // number here than the read-only system image's own size.
            render::ActionCardView::new("Хранилище", storage_string(), ""),
            // S16: first three tap-to-cycle settings cards -- see
            // `me_fixed_card_action` for the matching hit-test.
            render::ActionCardView::new(
                "Яркость экрана",
                format!("{}%", self.settings.brightness_pct),
                "Изменить",
            ),
            render::ActionCardView::new(
                "Блокировка экрана",
                format!("через {} с бездействия", self.settings.idle_timeout_secs),
                "Изменить",
            ),
            render::ActionCardView::new(
                "Гашение экрана",
                format!(
                    "через {} с после блокировки",
                    self.settings.deep_idle_timeout_secs
                ),
                "Изменить",
            ),
            // S17: no timezone concept existed before this ADR-061 --
            // see `current_time_string`'s own doc comment for why this
            // is a curated UTC-offset cycle, not full IANA tzdata.
            render::ActionCardView::new(
                "Часовой пояс",
                format_utc_offset(self.settings.utc_offset_minutes),
                "Изменить",
            ),
            // S22: read-only -- see the doc comment on `boot_slot` for
            // why there's no "проверить обновления" action here yet.
            render::ActionCardView::new(
                "Обновления",
                format!(
                    "Слот {} · попыток загрузки: {}",
                    boot_slot(),
                    boot_attempts()
                ),
                "",
            ),
            // S18: see `apply_volume`'s doc comment -- persistence/UI
            // real, hardware effect unverified.
            render::ActionCardView::new(
                "Громкость",
                format!("{}%", self.settings.volume_pct),
                "Изменить",
            ),
            // S19: real, unlike S18's volume -- see `wifi_status_line`/
            // `wifi_scan_results`' doc comments. Opens "Wi-Fi сети".
            render::ActionCardView::new("Wi-Fi", wifi_status_line(), "Сети"),
            // S20: real, same shape as S19 -- see `bluetooth_scan_
            // results`/`bluetooth_pair`'s doc comments. Opens
            // "Bluetooth устройства".
            render::ActionCardView::new(
                "Bluetooth",
                format!("Сопряжено устройств: {}", bluetooth_paired_count()),
                "Устройства",
            ),
            // S24: opt-in -- see `ShellSettings.pin_code`'s doc
            // comment. Default (`None`) keeps unlock as "any tap",
            // unchanged from before this sprint.
            render::ActionCardView::new(
                "PIN-код",
                if self.settings.pin_code.is_some() {
                    "Установлен"
                } else {
                    "Не установлен -- разблокировка тапом"
                },
                "Изменить",
            ),
            // S25: process-global, see `render::set_text_scale`'s doc
            // comment for why -- not a per-card local effect, this
            // changes every screen's text at once.
            render::ActionCardView::new(
                "Размер текста",
                format!("{}%", self.settings.text_scale_pct),
                "Изменить",
            ),
            // S25: `render::apply_contrast_boost`'s doc comment
            // explains why this is a post-process stretch, not a
            // second color palette.
            render::ActionCardView::new(
                "Контраст",
                if self.settings.contrast_pct == 0 {
                    "Обычный".to_string()
                } else {
                    format!("Повышенный ({}%)", self.settings.contrast_pct)
                },
                "Изменить",
            ),
        ];
        debug_assert_eq!(cards.len(), ME_FIXED_CARD_COUNT);
        for app in self.installed_apps.values() {
            let grants = self
                .apps_grants
                .get(&app.id)
                .map(|granted| {
                    if granted.is_empty() {
                        "без разрешений".to_string()
                    } else {
                        granted
                            .iter()
                            .map(|name| capability_label(name))
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                })
                .unwrap_or_else(|| "без разрешений".to_string());
            cards.push(render::ActionCardView::new(
                app.name.clone(),
                format!("{} · {grants}", app_state_label(&app.state)),
                "",
            ));
        }
        cards
    }

    /// S13 Change 4: "Сейчас"'s content -- one card per installed app
    /// (replacing the old single hardcoded demo card), followed by the
    /// two still-static `root.sui` cards for this page
    /// ("Объект пространства", "Новое намерение"), positioned right
    /// after the app list instead of at their old fixed `root.sui`
    /// coordinates -- `now_action_at` computes hit rects the same way.
    fn now_content_cards(&self, width: u32, height: u32) -> Vec<(Rect, render::ActionCardView)> {
        let mut cards: Vec<(Rect, render::ActionCardView)> = self
            .installed_apps
            .values()
            .enumerate()
            .map(|(index, app)| {
                (
                    now_grid_rect(index, width, height),
                    render::ActionCardView::new(
                        app.name.clone(),
                        app_state_label(&app.state),
                        if app.state == "running" {
                            "Работает"
                        } else {
                            "Запустить"
                        },
                    ),
                )
            })
            .collect();
        let base = self.installed_apps.len();
        for (offset, action) in ROOT_CONTENT_ACTIONS
            .iter()
            .filter(|action| action.page == "now")
            .enumerate()
        {
            cards.push((
                now_grid_rect(base + offset, width, height),
                self.content_card(action),
            ));
        }
        cards
    }

    fn content_card(&self, action: &ContentActionDefinition) -> render::ActionCardView {
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

    /// S09 Change 3 (ADR-031's follow-up): the first `saaios.task` in
    /// the selected space still waiting on a live confirmation, if any
    /// -- read straight from `selected_entities` (already kept current
    /// by `poll_entityd`), not from any separate state this client
    /// tracks itself. That's the whole point: after a cold reboot this
    /// client has no memory of its own, and a Task the store still
    /// marks `waiting_confirmation` shows up here exactly the same way
    /// it did before the reboot -- never auto-confirmed, never hidden.
    /// S13 Change 2: the task named by `confirming_task_id`, if it's
    /// still present and still actually waiting -- `None` collapses
    /// the modal back to whatever page is underneath (harmless if the
    /// id is stale, e.g. the task was resolved from another client).
    fn confirming_task(&self) -> Option<&Entity> {
        let id = self.confirming_task_id?;
        self.selected_entities.iter().find(|entity| {
            entity.id == id
                && entity.entity_type == "saaios.task"
                && entity.properties.get("status").and_then(Value::as_str)
                    == Some(TASK_STATUS_WAITING_CONFIRMATION)
        })
    }

    /// Writes the one property this screen ever changes (`status`),
    /// keeping every other property -- `intent_id` in particular --
    /// exactly as `saai-taskd` wrote it. `saai-taskd`'s own `Subscribe`
    /// reaction to this update is what actually executes (or discards)
    /// the paused Action; this method only ever flips the switch.
    fn confirm_pending_task(&mut self, confirm: bool) {
        // Cloned to an owned `Entity` up front, ending the borrow of
        // `self` before `self.entityd.update_entity()` needs its own
        // (disjoint but, from the borrow checker's point of view
        // through a `&self`-taking helper, not provably disjoint)
        // mutable borrow of `self.entityd`.
        let Some(task) = self.confirming_task().cloned() else {
            return;
        };
        // Closes the modal regardless of outcome -- the old behavior
        // (auto-popup the next waiting task, if any) is gone with S13
        // Change 2; the user comes back to "Входящие" and taps the
        // next one themselves if there is one.
        self.confirming_task_id = None;
        let mut properties = task.properties.clone();
        properties.insert(
            "status".into(),
            Value::String(
                if confirm {
                    TASK_STATUS_RUNNING
                } else {
                    TASK_STATUS_CANCELLED
                }
                .to_string(),
            ),
        );
        println!(
            "saai-shell: task {} {}",
            task.id,
            if confirm { "confirmed" } else { "cancelled" }
        );
        self.entityd.update_entity(&task, properties);
    }

    /// S21: marks a notification `dismissed` rather than deleting it --
    /// same full-replace-properties convention `confirm_pending_task`
    /// already uses for tasks, so `saai-entityd`'s protocol needed no
    /// changes for either entity type.
    fn dismiss_notification(&mut self, id: Uuid) {
        let Some(notification) = self
            .selected_entities
            .iter()
            .find(|entity| entity.id == id)
            .cloned()
        else {
            return;
        };
        let mut properties = notification.properties.clone();
        properties.insert("dismissed".into(), Value::Bool(true));
        self.entityd.update_entity(&notification, properties);
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
        let idle_timeout = Duration::from_secs(self.settings.idle_timeout_secs);
        if self.locked || self.last_activity.elapsed() < idle_timeout {
            return;
        }
        println!("saai-shell: idle timeout, locking");
        self.last_activity = Instant::now();
        match self.session_lock_state.lock(qh) {
            Ok(session_lock) => self.session_lock = Some(session_lock),
            Err(err) => eprintln!("saai-shell: failed to re-lock: {err}"),
        }
    }

    /// Fills the lock surface -- the panel's actual visible content
    /// while locked; the toplevel stays hidden underneath it (ADR-016) --
    /// with one solid color and commits it. Shared by the initial
    /// `LOCK_SCREEN_COLOR` placeholder (`SessionLockSurfaceHandler::
    /// configure`, above) and the `SLEEP_INDICATOR_COLOR` frame
    /// `check_deep_idle()` shows around a real suspend. Reuses the
    /// already-created pool/buffer the same way `draw()` reuses its own
    /// for the toplevel; a no-op before the first configure
    /// (`lock_width`/`lock_height` still 0) or if the lock surface
    /// object itself isn't present for some other reason.
    /// S13 Change 1: draws the real status bar content (time, Wi-Fi,
    /// battery) into the permanent system layer. Reuses `layer_pool`/
    /// `layer_buffer` the same way `present_lock_surface` reuses its
    /// own pool/buffer -- called both from the layer's own `configure`
    /// (first paint) and periodically from `refresh_statusbar_if_due`.
    fn present_status_bar(&mut self) {
        let width = self.layer_width;
        let height = self.layer_height;
        if width == 0 || height == 0 {
            return;
        }
        let stride = width as i32 * 4;

        if self.layer_pool.is_none() {
            self.layer_pool = Some(
                SlotPool::new(width as usize * height as usize * 4, &self.shm)
                    .expect("create layer surface pool"),
            );
        }
        let pool = self.layer_pool.as_mut().expect("just ensured above");

        if self.layer_buffer.is_none() {
            let (buffer, _canvas) = pool
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Xrgb8888)
                .expect("create layer buffer");
            self.layer_buffer = Some(buffer);
        }
        let buffer = self.layer_buffer.as_mut().expect("just ensured above");

        let canvas = match pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (second_buffer, canvas) = pool
                    .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Xrgb8888)
                    .expect("create layer buffer");
                *buffer = second_buffer;
                canvas
            }
        };
        render::draw_status_bar(
            &mut render::Canvas::new(canvas, width, height),
            width,
            height,
            &current_time_string(self.settings.utc_offset_minutes),
            wifi_is_up(),
            read_battery(),
            self.fonts.as_ref(),
        );
        render::apply_contrast_boost(canvas, self.settings.contrast_pct);

        let surface = self.layer.wl_surface();
        surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(surface).expect("buffer attach");
        self.layer.commit();
    }

    /// S13 Change 1: throttled the same way `refresh_apps_if_due` is --
    /// called once per main-loop tick, only actually redraws once
    /// `STATUSBAR_REFRESH_INTERVAL` has elapsed.
    fn refresh_statusbar_if_due(&mut self) {
        if self.last_statusbar_refresh.elapsed() >= STATUSBAR_REFRESH_INTERVAL {
            self.present_status_bar();
            self.check_low_battery();
            self.last_statusbar_refresh = Instant::now();
        }
    }

    /// S21: first real notification producer, entirely client-side --
    /// no new service needed, `read_battery()` (S13) already has what
    /// this needs. `low_battery_notified` is the guard against creating
    /// a fresh one every second while the battery stays low; it resets
    /// once the battery recovers above the threshold, so a second real
    /// discharge cycle notifies again instead of staying silently
    /// suppressed forever.
    fn check_low_battery(&mut self) {
        let Some((percent, charging)) = read_battery() else {
            return;
        };
        if percent > LOW_BATTERY_THRESHOLD_PCT || charging {
            self.low_battery_notified = false;
            return;
        }
        if self.low_battery_notified || !self.entityd.is_connected() {
            return;
        }
        self.low_battery_notified = true;
        let mut properties = Map::new();
        properties.insert("body".into(), Value::String(format!("Осталось {percent}%")));
        properties.insert("kind".into(), Value::String("low_battery".into()));
        self.entityd.create_entity(
            self.selected_space_id.clone(),
            NOTIFICATION_ENTITY_TYPE,
            "Разряжается батарея",
            properties,
        );
    }

    fn present_lock_surface(&mut self, color: [u8; 4]) {
        let width = self.lock_width;
        let height = self.lock_height;
        if width == 0 || height == 0 {
            return;
        }
        let Some(lock_surface) = self.lock_surfaces.last() else {
            return;
        };
        let stride = width as i32 * 4;

        if self.lock_pool.is_none() {
            self.lock_pool = Some(
                SlotPool::new(width as usize * height as usize * 4, &self.shm)
                    .expect("create lock surface pool"),
            );
        }
        let pool = self.lock_pool.as_mut().expect("just ensured above");

        if self.lock_buffer.is_none() {
            let (buffer, _canvas) = pool
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Xrgb8888)
                .expect("create lock buffer");
            self.lock_buffer = Some(buffer);
        }
        let buffer = self.lock_buffer.as_mut().expect("just ensured above");

        let canvas = match pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (second_buffer, canvas) = pool
                    .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Xrgb8888)
                    .expect("create lock buffer");
                *buffer = second_buffer;
                canvas
            }
        };
        for chunk in canvas.chunks_exact_mut(4) {
            chunk.copy_from_slice(&color);
        }

        let surface = lock_surface.wl_surface();
        surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
    }

    /// S24: what actually gets shown on the lock surface each time it
    /// needs repainting -- the flat `LOCK_SCREEN_COLOR` this always
    /// showed before this sprint when no PIN is set (delegates
    /// straight to `present_lock_surface`, no change in that case),
    /// or a numeric keypad plus filled-dot progress indicators when
    /// `settings.pin_code` is set. Never called for the
    /// `SLEEP_INDICATOR_COLOR` deep-idle blank (`check_deep_idle`
    /// keeps calling `present_lock_surface` directly for that) --
    /// screen-off should stay screen-off regardless of PIN.
    fn present_lock_pin_entry(&mut self) {
        let Some(pin_code) = self.settings.pin_code.clone() else {
            self.present_lock_surface(LOCK_SCREEN_COLOR);
            return;
        };
        let width = self.lock_width;
        let height = self.lock_height;
        if width == 0 || height == 0 {
            return;
        }
        let Some(lock_surface) = self.lock_surfaces.last() else {
            return;
        };
        let stride = width as i32 * 4;

        if self.lock_pool.is_none() {
            self.lock_pool = Some(
                SlotPool::new(width as usize * height as usize * 4, &self.shm)
                    .expect("create lock surface pool"),
            );
        }
        let pool = self.lock_pool.as_mut().expect("just ensured above");

        if self.lock_buffer.is_none() {
            let (buffer, _canvas) = pool
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Xrgb8888)
                .expect("create lock buffer");
            self.lock_buffer = Some(buffer);
        }
        let buffer = self.lock_buffer.as_mut().expect("just ensured above");

        let canvas = match pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (second_buffer, canvas) = pool
                    .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Xrgb8888)
                    .expect("create lock buffer");
                *buffer = second_buffer;
                canvas
            }
        };

        let keys: Vec<(Rect, &'static str)> = PIN_KEYPAD_DIGIT_LABELS
            .iter()
            .enumerate()
            .filter(|(_, label)| !label.is_empty())
            .map(|(index, label)| (pin_keypad_rect(index, width, height), *label))
            .collect();
        render::draw_lock_pin_entry(
            &mut render::Canvas::new(canvas, width, height),
            width,
            self.pin_entry_buffer.len(),
            pin_code.len(),
            &keys,
            self.fonts.as_ref(),
        );
        render::apply_contrast_boost(canvas, self.settings.contrast_pct);

        let surface = lock_surface.wl_surface();
        surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
    }

    /// S11 Change 2 (ADR-041), revised in ADR-051: once the screen has
    /// already been locked (`check_idle_timeout`) and stays untouched
    /// for a further `deep_idle_timeout`, blanks the lock surface to
    /// `SLEEP_INDICATOR_COLOR` instead of leaving the lock screen lit
    /// forever. An earlier version of this also wrote `mem` to
    /// `/sys/power/state` to actually suspend the kernel; ADR-051 found
    /// that write reliably fails with EBUSY on this hardware whenever a
    /// USB cable is attached, because the `dwc3-otg` wakeup source
    /// stays held for as long as the USB link is configured (our own
    /// serial console and USB-NCM being exactly such a link) -- not a
    /// bug in this file, but it meant every suspend attempt was
    /// immediately undone before the screen could visibly stay dark,
    /// which is what this function is actually responsible for. This
    /// version only ever changes what's on screen; `TouchHandler::down`
    /// clears `sleeping` again on the next touch.
    fn check_deep_idle(&mut self, _conn: &Connection) {
        let deep_idle_timeout = Duration::from_secs(self.settings.deep_idle_timeout_secs);
        if !self.locked || self.sleeping || self.last_activity.elapsed() < deep_idle_timeout {
            return;
        }
        println!("saai-shell: deep idle timeout, screen off");
        self.sleeping = true;
        self.present_lock_surface(SLEEP_INDICATOR_COLOR);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bluetooth_list_action_at, capability_label, consent_action_at, content_action_at,
        format_utc_offset, intent_action_at, next_in_cycle, stacked_row_rect, tab_at,
        task_confirm_action_at, wifi_list_action_at, BluetoothListTap, Rect, RootPage,
        WifiListTap, INTENT_CANCEL_ACTION, INTENT_SEND_ACTION, ROOT_CONTENT_ACTIONS, ROOT_TABS,
    };

    #[test]
    fn format_utc_offset_handles_zero_positive_negative_and_half_hours() {
        assert_eq!(format_utc_offset(0), "UTC+00:00");
        assert_eq!(format_utc_offset(180), "UTC+03:00");
        assert_eq!(format_utc_offset(330), "UTC+05:30");
        assert_eq!(format_utc_offset(-300), "UTC-05:00");
    }

    #[test]
    fn next_in_cycle_wraps_and_treats_an_unknown_value_as_index_zero() {
        let levels = [10, 20, 30];
        assert_eq!(next_in_cycle(&levels, 10), 20);
        assert_eq!(next_in_cycle(&levels, 30), 10);
        // Not found -> treated as if it were `levels[0]`, so this
        // advances to `levels[1]`, same as `next_in_cycle(&levels, 10)`
        // above -- not a special "reset to first" case.
        assert_eq!(next_in_cycle(&levels, 999), 20);
    }

    #[test]
    fn wifi_list_action_at_finds_networks_then_refresh_then_back() {
        let width = 1080;
        let height = 2400;
        let network_count = 2;
        let network_0 = stacked_row_rect(0, width, height);
        let network_1 = stacked_row_rect(1, width, height);
        let refresh = stacked_row_rect(2, width, height);
        let back = stacked_row_rect(3, width, height);
        let center = |rect: Rect| {
            (
                (rect.x + rect.width / 2) as f64,
                (rect.y + rect.height / 2) as f64,
            )
        };
        assert!(matches!(
            wifi_list_action_at(center(network_0), width, height, network_count),
            Some(WifiListTap::Network(0))
        ));
        assert!(matches!(
            wifi_list_action_at(center(network_1), width, height, network_count),
            Some(WifiListTap::Network(1))
        ));
        assert!(matches!(
            wifi_list_action_at(center(refresh), width, height, network_count),
            Some(WifiListTap::Refresh)
        ));
        assert!(matches!(
            wifi_list_action_at(center(back), width, height, network_count),
            Some(WifiListTap::Back)
        ));
    }

    #[test]
    fn wifi_list_action_at_misses_above_the_first_row() {
        let width = 1080;
        let height = 2400;
        assert!(wifi_list_action_at((10.0, 10.0), width, height, 0).is_none());
    }

    #[test]
    fn pin_keypad_action_at_finds_digits_and_backspace_but_not_the_blank_cell() {
        let width = 1080;
        let height = 2400;
        let center = |rect: Rect| {
            (
                (rect.x + rect.width / 2) as f64,
                (rect.y + rect.height / 2) as f64,
            )
        };
        assert_eq!(
            super::pin_keypad_action_at(center(super::pin_keypad_rect(0, width, height)), width, height),
            Some("1")
        );
        assert_eq!(
            super::pin_keypad_action_at(center(super::pin_keypad_rect(10, width, height)), width, height),
            Some("0")
        );
        assert_eq!(
            super::pin_keypad_action_at(center(super::pin_keypad_rect(11, width, height)), width, height),
            Some("⌫")
        );
        // Index 9 is the deliberately blank cell between 9 and 0.
        assert_eq!(
            super::pin_keypad_action_at(center(super::pin_keypad_rect(9, width, height)), width, height),
            None
        );
    }

    #[test]
    fn pin_setup_action_at_exposes_forget_only_when_a_pin_already_exists() {
        let width = 1080;
        let height = 2400;
        let center = |rect: Rect| {
            (
                (rect.x + rect.width / 2) as f64,
                (rect.y + rect.height / 2) as f64,
            )
        };
        // Index 14 (the third control slot) is only "Убрать PIN" when
        // a PIN already exists.
        assert_eq!(
            super::pin_setup_action_at(center(super::pin_keypad_rect(14, width, height)), width, height, true),
            Some("Убрать PIN")
        );
        assert_eq!(
            super::pin_setup_action_at(center(super::pin_keypad_rect(14, width, height)), width, height, false),
            None
        );
        assert_eq!(
            super::pin_setup_action_at(center(super::pin_keypad_rect(12, width, height)), width, height, false),
            Some("Отмена")
        );
    }

    #[test]
    fn now_grid_rect_lays_out_three_columns_per_row() {
        let width = 1080;
        let height = 2400;
        let cell_0 = super::now_grid_rect(0, width, height);
        let cell_1 = super::now_grid_rect(1, width, height);
        let cell_2 = super::now_grid_rect(2, width, height);
        let cell_3 = super::now_grid_rect(3, width, height);
        // Same row (0..3): equal y, strictly increasing x.
        assert_eq!(cell_0.y, cell_1.y);
        assert_eq!(cell_1.y, cell_2.y);
        assert!(cell_0.x < cell_1.x);
        assert!(cell_1.x < cell_2.x);
        // Next row (index 3, the 4th cell) starts back at the left
        // margin, below row 0.
        assert_eq!(cell_3.x, cell_0.x);
        assert!(cell_3.y > cell_0.y);
    }

    #[test]
    fn bluetooth_list_action_at_finds_devices_then_scan_then_refresh_then_back() {
        let width = 1080;
        let height = 2400;
        let device_count = 1;
        let device_0 = stacked_row_rect(0, width, height);
        let scan = stacked_row_rect(1, width, height);
        let refresh = stacked_row_rect(2, width, height);
        let back = stacked_row_rect(3, width, height);
        let center = |rect: Rect| {
            (
                (rect.x + rect.width / 2) as f64,
                (rect.y + rect.height / 2) as f64,
            )
        };
        assert!(matches!(
            bluetooth_list_action_at(center(device_0), width, height, device_count),
            Some(BluetoothListTap::Device(0))
        ));
        assert!(matches!(
            bluetooth_list_action_at(center(scan), width, height, device_count),
            Some(BluetoothListTap::Scan)
        ));
        assert!(matches!(
            bluetooth_list_action_at(center(refresh), width, height, device_count),
            Some(BluetoothListTap::Refresh)
        ));
        assert!(matches!(
            bluetooth_list_action_at(center(back), width, height, device_count),
            Some(BluetoothListTap::Back)
        ));
    }

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
    fn now_page_static_actions_come_from_sui_markup() {
        // S13 Change 4 removed the compiled-in demo-app card -- "Сейчас"
        // now has exactly the two entries that were always meant to
        // stay static (the app list itself is runtime data, handled by
        // `now_action_at`/`now_content_cards`, not this table).
        assert_eq!(ROOT_CONTENT_ACTIONS.len(), 6);
        assert_eq!(
            content_action_at(RootPage::Now, (540.0, 800.0), 1080, 2400).map(|action| action.id),
            Some("selected-entity")
        );
        assert_eq!(
            content_action_at(RootPage::Now, (540.0, 1000.0), 1080, 2400).map(|action| action.id),
            Some("new-intent")
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
    fn task_confirm_screen_left_half_of_button_row_confirms() {
        assert_eq!(
            task_confirm_action_at((270.0, 2250.0), 1080, 2400),
            Some(true)
        );
    }

    #[test]
    fn task_confirm_screen_right_half_of_button_row_declines() {
        assert_eq!(
            task_confirm_action_at((810.0, 2250.0), 1080, 2400),
            Some(false)
        );
    }

    #[test]
    fn task_confirm_screen_header_area_is_not_a_button() {
        assert_eq!(task_confirm_action_at((540.0, 1000.0), 1080, 2400), None);
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
