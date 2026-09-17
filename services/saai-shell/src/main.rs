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
use std::time::{Duration, Instant, SystemTime};

mod appd_client;
mod dmabuf_canvas;
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

/// HIA-01 (docs/os/sprints/HIA-ROADMAP.md): the builtin system space
/// (`saai-entity-store::BUILTIN_SPACE_IDS` already reserves it,
/// `SpaceKind::System` vs `SpaceKind::User`) is where this project's
/// own cross-space bookkeeping lives, same "not user content" role
/// as `/data/saaios/var/...` config directories one layer down. Space
/// relations/lifecycle are stored here, not scattered across every
/// individual space they describe, specifically because
/// `saai-entityd`'s `ListEntities` is scoped to one space at a
/// time -- keeping them all in one well-known space is what makes
/// "list every relation" a single query instead of N.
const SYSTEM_SPACE_ID: &str = "saaios";
/// One edge of the space graph. `properties`: `from_space_id`,
/// `to_space_id`, `kind` (one of the document's own vocabulary --
/// parent_of/child_of/related_to/contains/usually_with/
/// exclusive_with/activates/deactivates/inherits_from, though nothing
/// here enforces that list, same "convention over schema" this
/// project already applies to every other `saaios.*` entity type).
/// Directional storage only -- a symmetric kind like `related_to`
/// needs one record per direction to be findable from both ends; a
/// known simplification, not a bug, for this first version.
const SPACE_RELATION_ENTITY_TYPE: &str = "saaios.space-relation";
/// One record per space that has ever had its lifecycle changed from
/// the default. `properties`: `space_id`, `lifecycle`. Absence means
/// `SpaceLifecycle::Stable` (`space_lifecycle()`'s own default) --
/// the four builtin spaces (S06) never needed this before and
/// shouldn't require a migration to keep behaving the same way.
const SPACE_LIFECYCLE_ENTITY_TYPE: &str = "saaios.space-lifecycle";
/// HIA-02: one record per (space, physical signal) association --
/// `properties`: `space_id`, `signal_type` (only `"wifi_ssid"` so
/// far -- Bluetooth is a real future source per HIA-ROADMAP.md, not
/// built here), `value` (the SSID itself). No UI creates these yet,
/// same "reads-only, authoring deferred" gap ADR-086 already left
/// for `saaios.space-relation`.
const SPACE_SIGNAL_ENTITY_TYPE: &str = "saaios.space-signal";
const SPACE_SIGNAL_TYPE_WIFI_SSID: &str = "wifi_ssid";
/// HIA-03: one record per space that has ever had its status-bar dot
/// color changed from the default -- `properties`: `space_id`,
/// `color`. Absence means `SpaceColor::Default` (the theme's default
/// teal every space already used before this existed), same "silent
/// default" shape `SPACE_LIFECYCLE_ENTITY_TYPE`'s own doc comment
/// already established.
const SPACE_COLOR_ENTITY_TYPE: &str = "saaios.space-color";

/// HIA-01's lifecycle vocabulary (document section 4.3/section 69) --
/// `Stable` is the default for every space that has never been
/// touched, matching how the four builtin spaces (S06) have always
/// behaved with no lifecycle concept at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpaceLifecycle {
    Temporary,
    Emerging,
    Stable,
    Archived,
}

impl SpaceLifecycle {
    fn as_str(self) -> &'static str {
        match self {
            SpaceLifecycle::Temporary => "temporary",
            SpaceLifecycle::Emerging => "emerging",
            SpaceLifecycle::Stable => "stable",
            SpaceLifecycle::Archived => "archived",
        }
    }

    /// Shown on the space's own card (`content_card`'s `select_space:`
    /// branch) -- `None` for `Stable` specifically, so the common,
    /// untouched case doesn't clutter every card with a label nobody
    /// asked to see (same "only show what's not the default" spirit
    /// as `capability_label`'s callers already apply elsewhere).
    fn label(self) -> Option<&'static str> {
        match self {
            SpaceLifecycle::Temporary => Some("Временное"),
            SpaceLifecycle::Emerging => Some("Складывается"),
            SpaceLifecycle::Stable => None,
            SpaceLifecycle::Archived => Some("Архивировано"),
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "temporary" => Some(SpaceLifecycle::Temporary),
            "emerging" => Some(SpaceLifecycle::Emerging),
            "stable" => Some(SpaceLifecycle::Stable),
            "archived" => Some(SpaceLifecycle::Archived),
            _ => None,
        }
    }

    /// Drives the tap-an-already-selected-space gesture
    /// (`invoke_content_action`'s `select_space:` branch) -- a fixed
    /// cycle, same shape as `next_in_cycle` used everywhere on "Я",
    /// just over four fixed states instead of a settings array.
    fn next(self) -> Self {
        match self {
            SpaceLifecycle::Temporary => SpaceLifecycle::Emerging,
            SpaceLifecycle::Emerging => SpaceLifecycle::Stable,
            SpaceLifecycle::Stable => SpaceLifecycle::Archived,
            SpaceLifecycle::Archived => SpaceLifecycle::Temporary,
        }
    }
}

/// `Stable` (the default) for a space with no matching entity in
/// `system_entities` at all -- not an `Option`, there is always a
/// well-defined answer.
fn space_lifecycle(system_entities: &[Entity], space_id: &str) -> SpaceLifecycle {
    system_entities
        .iter()
        .find(|entity| {
            entity.entity_type == SPACE_LIFECYCLE_ENTITY_TYPE
                && entity.properties.get("space_id").and_then(Value::as_str) == Some(space_id)
        })
        .and_then(|entity| entity.properties.get("lifecycle").and_then(Value::as_str))
        .and_then(SpaceLifecycle::parse)
        .unwrap_or(SpaceLifecycle::Stable)
}

/// The existing `saaios.space-lifecycle` record for `space_id`, if
/// this space has ever had one written -- `cycle_space_lifecycle`
/// needs the real `Entity` (id/revision) to `update_entity` it
/// in place instead of accumulating duplicates.
fn space_lifecycle_entity<'a>(system_entities: &'a [Entity], space_id: &str) -> Option<&'a Entity> {
    system_entities.iter().find(|entity| {
        entity.entity_type == SPACE_LIFECYCLE_ENTITY_TYPE
            && entity.properties.get("space_id").and_then(Value::as_str) == Some(space_id)
    })
}

/// `(target_space_id, kind)` for every edge whose `from_space_id` is
/// `space_id` -- directional only, see `SPACE_RELATION_ENTITY_TYPE`'s
/// own doc comment for why.
fn space_relation_targets(system_entities: &[Entity], space_id: &str) -> Vec<(String, String)> {
    system_entities
        .iter()
        .filter(|entity| entity.entity_type == SPACE_RELATION_ENTITY_TYPE)
        .filter_map(|entity| {
            let from = entity
                .properties
                .get("from_space_id")
                .and_then(Value::as_str)?;
            if from != space_id {
                return None;
            }
            let to = entity
                .properties
                .get("to_space_id")
                .and_then(Value::as_str)?;
            let kind = entity.properties.get("kind").and_then(Value::as_str)?;
            Some((to.to_string(), kind.to_string()))
        })
        .collect()
}

/// Known builtin id -> Russian name, unknown id -> the id itself --
/// the exact fallback `context_label` always had, now shared with the
/// relation summary in `content_card`'s `select_space:` branch too.
fn space_display_name(spaces: &[Space], space_id: &str) -> String {
    spaces
        .iter()
        .find(|space| space.id == space_id)
        .map(|space| space.name.clone())
        .unwrap_or_else(|| match space_id {
            "home" => "Дом".into(),
            "work" => "Работа".into(),
            "personal" => "Личное".into(),
            "saaios" => "SaaiOS".into(),
            other => other.to_owned(),
        })
}

/// HIA-03 (docs/os/sprints/HIA-ROADMAP.md): the status-bar dot's
/// color. `Default` is deliberately a real, reachable member of the
/// cycle (not `None`/absence) -- same shape as `SpaceLifecycle`'s
/// `Stable`: the silent default a space starts at and can cycle back
/// around to, not a special case the rest of this file has to know
/// about separately. Six choices, not a free-form picker -- no color
/// picker widget exists anywhere in this codebase, and every other
/// per-value setting here (brightness, contrast, volume...) is
/// already a small fixed cycle, not free text entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpaceColor {
    Default,
    Blue,
    Green,
    Orange,
    Purple,
    Pink,
}

impl SpaceColor {
    fn as_str(self) -> &'static str {
        match self {
            SpaceColor::Default => "default",
            SpaceColor::Blue => "blue",
            SpaceColor::Green => "green",
            SpaceColor::Orange => "orange",
            SpaceColor::Purple => "purple",
            SpaceColor::Pink => "pink",
        }
    }

    fn label(self) -> &'static str {
        match self {
            SpaceColor::Default => "Обычный",
            SpaceColor::Blue => "Синий",
            SpaceColor::Green => "Зелёный",
            SpaceColor::Orange => "Оранжевый",
            SpaceColor::Purple => "Фиолетовый",
            SpaceColor::Pink => "Розовый",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "default" => Some(SpaceColor::Default),
            "blue" => Some(SpaceColor::Blue),
            "green" => Some(SpaceColor::Green),
            "orange" => Some(SpaceColor::Orange),
            "purple" => Some(SpaceColor::Purple),
            "pink" => Some(SpaceColor::Pink),
            _ => None,
        }
    }

    /// Drives the "Цвет пространства" card on "Я" -- same fixed-cycle
    /// shape as `SpaceLifecycle::next`.
    fn next(self) -> Self {
        match self {
            SpaceColor::Default => SpaceColor::Blue,
            SpaceColor::Blue => SpaceColor::Green,
            SpaceColor::Green => SpaceColor::Orange,
            SpaceColor::Orange => SpaceColor::Purple,
            SpaceColor::Purple => SpaceColor::Pink,
            SpaceColor::Pink => SpaceColor::Default,
        }
    }

    /// Context identity is mapped separately from status severity by
    /// ADR-094. `Default` deliberately matches the theme accent, but
    /// callers still request it through `ContextColor` so a Space color
    /// cannot accidentally become a failure/attention state.
    fn pixel(self) -> render::Pixel {
        let context = match self {
            SpaceColor::Default => ContextColor::Default,
            SpaceColor::Blue => ContextColor::Blue,
            SpaceColor::Green => ContextColor::Green,
            SpaceColor::Orange => ContextColor::Orange,
            SpaceColor::Purple => ContextColor::Purple,
            SpaceColor::Pink => ContextColor::Pink,
        };
        render::context_color(context)
    }
}

/// `SpaceColor::Default` for a space with no matching entity at all --
/// not an `Option`, same "always a well-defined answer" shape as
/// `space_lifecycle`. This is HIA-03's negative scenario
/// (`HIA-ROADMAP.md`'s own acceptance line): a space with no color
/// ever set gets a deterministic default, never an empty/missing dot.
fn space_color(system_entities: &[Entity], space_id: &str) -> SpaceColor {
    system_entities
        .iter()
        .find(|entity| {
            entity.entity_type == SPACE_COLOR_ENTITY_TYPE
                && entity.properties.get("space_id").and_then(Value::as_str) == Some(space_id)
        })
        .and_then(|entity| entity.properties.get("color").and_then(Value::as_str))
        .and_then(SpaceColor::parse)
        .unwrap_or(SpaceColor::Default)
}

/// The existing `saaios.space-color` record for `space_id`, if one
/// exists -- `cycle_space_color` needs the real `Entity` (id/
/// revision) to `update_entity` it in place instead of accumulating
/// duplicates, same reasoning as `space_lifecycle_entity`.
fn space_color_entity<'a>(system_entities: &'a [Entity], space_id: &str) -> Option<&'a Entity> {
    system_entities.iter().find(|entity| {
        entity.entity_type == SPACE_COLOR_ENTITY_TYPE
            && entity.properties.get("space_id").and_then(Value::as_str) == Some(space_id)
    })
}

/// HIA-02 (docs/os/sprints/HIA-ROADMAP.md): which real-world signal
/// put a `ContextFrameEntry` into `Shell::context_frame`. `Manual`
/// is set only from a real tap (`upsert_manual_context`'s own doc
/// comment explains why an auto-switch must NOT also touch it) --
/// so it's always the safety net a lost physical signal falls back
/// to (see `effective_context_space`'s own doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextSource {
    Manual,
    Wifi,
}

/// Fixed, not learned -- a real physical presence signal (currently
/// connected to a known SSID) should generally win over whatever was
/// last manually/previously selected, and losing that signal should
/// fall back to the manual entry, which never expires on its own.
/// `MANUAL_CONFIDENCE < WIFI_CONFIDENCE` is the entire policy; no
/// decay/staleness timer needed on top of it (`refresh_context_
/// signals_if_due` recomputes the Wifi entry from scratch, live,
/// every `CONTEXT_SIGNAL_REFRESH_INTERVAL`).
const MANUAL_CONFIDENCE: u8 = 50;
const WIFI_CONFIDENCE: u8 = 80;

/// One space this shell currently has *some* evidence for being the
/// right context -- HIA-01's "Goal" line asks for confidence/source
/// per entry; `observed_at` isn't tracked separately because nothing
/// here needs staleness beyond "was this source's entry ever
/// refreshed to reflect the space it currently names" -- each source
/// keeps at most one entry (`upsert_context_entry` replaces, never
/// accumulates), so an entry's mere presence already means "current
/// as of the last refresh for its source".
#[derive(Debug, Clone, PartialEq)]
struct ContextFrameEntry {
    space_id: String,
    confidence: u8,
    source: ContextSource,
}

/// `selected_space_id` (the Rollback this ADR's own roadmap entry
/// names) stays the single field everything else in this file reads
/// -- this just decides what it *should* be. `fallback` covers the
/// only case with no entries at all (before the first `Selection`
/// response ever arrives at boot).
fn effective_context_space(frame: &[ContextFrameEntry], fallback: &str) -> String {
    frame
        .iter()
        .max_by_key(|entry| entry.confidence)
        .map(|entry| entry.space_id.clone())
        .unwrap_or_else(|| fallback.to_string())
}

/// At most one entry per `ContextSource` -- a fresh entry for a
/// source always replaces that source's previous one instead of
/// accumulating history nobody reads.
fn upsert_context_entry(frame: &mut Vec<ContextFrameEntry>, entry: ContextFrameEntry) {
    frame.retain(|existing| existing.source != entry.source);
    frame.push(entry);
}

fn remove_context_source(frame: &mut Vec<ContextFrameEntry>, source: ContextSource) {
    frame.retain(|existing| existing.source != source);
}

/// The `saaios.space-signal` record (system space, same reasoning as
/// `SPACE_RELATION_ENTITY_TYPE`'s own doc comment) mapping a Wi-Fi
/// SSID to the space that should activate while connected to it.
/// Like `saaios.space-relation`, creation has no UI yet -- this is
/// read-only plumbing, seeded manually for now (see ADR-087).
fn space_for_wifi_ssid(system_entities: &[Entity], ssid: &str) -> Option<String> {
    system_entities
        .iter()
        .find(|entity| {
            entity.entity_type == SPACE_SIGNAL_ENTITY_TYPE
                && entity.properties.get("signal_type").and_then(Value::as_str)
                    == Some(SPACE_SIGNAL_TYPE_WIFI_SSID)
                && entity.properties.get("value").and_then(Value::as_str) == Some(ssid)
        })
        .and_then(|entity| entity.properties.get("space_id").and_then(Value::as_str))
        .map(str::to_string)
}
use saai_ui_core::{
    layout, Axis, ContextColor, ContextHeader, DataRow, DataRowVariant, LayoutNode, Length,
    Node, ObjectSummary, Rect, StatusIndicator, StatusIndicatorVariant, SystemSection,
    SystemSectionRow, UniversalState,
};
use serde_json::{json, Map, Value};
use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init,
    protocol::{wl_buffer, wl_output, wl_seat, wl_shm, wl_surface, wl_touch},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1, zwp_linux_dmabuf_v1,
};

use dmabuf_canvas::{Busy, DmabufCanvas};
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
/// HIA-02: how often `refresh_context_signals_if_due` re-checks
/// ambient physical signals (Wi-Fi today) and, if the top-confidence
/// entry in `context_frame` disagrees with `selected_space_id`,
/// auto-switches. Coarser than `STATUSBAR_REFRESH_INTERVAL` on
/// purpose -- "did I enter a known space" doesn't need sub-second
/// reaction time, and this shells out to `wpa_cli status` each time.
const CONTEXT_SIGNAL_REFRESH_INTERVAL: Duration = Duration::from_secs(5);
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
    let index = levels
        .iter()
        .position(|&level| level == current)
        .unwrap_or(0);
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

/// The Unix socket `pair-recv.c` connects to for every SSH pairing
/// request -- see that file's own doc comment for the wire shape
/// (`poll_remote_pairing` decodes it) and `PendingPairRequest` for
/// what happens with it.
const REMOTE_PAIR_SOCKET_PATH: &str = "/run/saaios/remote-pair.sock";
/// S32: `saai-displayd` touches this file's mtime on every touch
/// down/up it routes -- to `focused_surface`, i.e. whichever app's
/// toplevel currently has it, not just this client's own surfaces.
/// `check_idle_timeout` reads it alongside its own `last_activity`
/// because this client only ever sees touches that land on ITS OWN
/// surfaces (the root UI, the lock screen); a third-party app in the
/// foreground (`org.saaios.mahjong`, say) receives touch input
/// directly from the compositor and this client is never told about
/// it at all, so it used to lock the screen out from under someone
/// actively playing. The compositor is the one process that already
/// sees every touch regardless of which client's surface it's routed
/// to, so it's the only place that can answer "is anything happening
/// anywhere" -- a marker file is the same "shell out / poll a file,
/// don't grow a new protocol for one boolean" convention this project
/// already uses for Wi-Fi status, storage, and the remote-access
/// toggle.
const GLOBAL_LAST_INPUT_PATH: &str = "/run/saaios/last-input";
/// VUI-01's volatile developer switch. `/run` is recreated on boot, so a
/// calibration session can survive a supervised shell restart but can never
/// become a persistent user setting or strand the device after reboot.
const UI_CALIBRATION_MARKER: &str = "/run/saaios/ui-calibration";

fn calibration_requested(environment: Option<&str>, runtime_marker_exists: bool) -> bool {
    environment == Some("1") || runtime_marker_exists
}

/// VUI-02: same developer-only gate as `UI_CALIBRATION_MARKER`, one
/// screen over -- `render::draw_gallery`'s device component gallery
/// (ADR-105) is reached the same way, never through normal navigation,
/// and is just as volatile (the marker lives under `/run`, never
/// survives a reboot, and setting it can never become a persistent user
/// setting).
const UI_GALLERY_MARKER: &str = "/run/saaios/ui-gallery";

/// Development-only escape hatch: skips both the boot-time session lock
/// and `check_idle_timeout`'s own re-lock, so repeated shell restarts
/// during active development do not each require unlocking by hand before
/// the next screenshot/test can proceed. Same volatile gate as
/// `UI_CALIBRATION_MARKER`/`UI_GALLERY_MARKER` -- `/run` never survives a
/// reboot, and this can never become a persistent user setting. Does not
/// touch `ShellSettings.pin_code`, `idle_timeout_secs`, or any other real
/// setting; a device with this marker present still has its PIN and its
/// idle timeout configured exactly as before, it simply is not being
/// asked to act on them right now.
const DEV_NO_LOCK_MARKER: &str = "/run/saaios/dev-no-lock";

/// Development-only preview gate for VUI-03's composed `Сейчас` (ADR-112):
/// `RootPage::Now` still renders through `draw_root`'s existing app-grid
/// scaffold by default -- this flag switches it to the new `Frame::Now`/
/// `draw_now` composition instead, for physical verification before the
/// app grid's own relocation (VUI-03's next task) makes that the only
/// path. Same volatile `/run` gate as every other dev preview this
/// project has added (`UI_CALIBRATION_MARKER`, `UI_GALLERY_MARKER`,
/// `DEV_NO_LOCK_MARKER`) -- cannot survive a reboot, cannot become a
/// persistent setting.
const NOW_COMPOSED_MARKER: &str = "/run/saaios/ui-now-composed";
/// The master "Удалённый доступ" switch's on-disk signal to `pair-
/// recv` (a separate process, native-init.c-started, that can't read
/// `ShellSettings`'s own JSON directly without duplicating its parse
/// logic) -- present means enabled. `pair-recv.c` checks this before
/// ever asking this process for a decision, so turning the toggle off
/// stops new pairings from even reaching the on-device prompt, not
/// just from being approved.
const REMOTE_ACCESS_MARKER: &str = "/data/saaios/var/remote-access-enabled";

/// Real effect, unlike `apply_volume` -- `pair-recv` reads this file's
/// mere existence on every incoming pairing request. Already-approved
/// keys in `dropbear`'s `authorized_keys` are untouched by this
/// toggle (same as Android's "USB debugging" switch not itself
/// revoking already-authorized computers) -- see docs/os/ideas.md for
/// the honest gap that leaves.
fn apply_remote_access(enabled: bool) {
    if enabled {
        let _ = std::fs::write(REMOTE_ACCESS_MARKER, "");
    } else {
        let _ = std::fs::remove_file(REMOTE_ACCESS_MARKER);
    }
}

/// The same file `pair-recv.c`'s own `AUTHORIZED_KEYS` define writes
/// to -- duplicated as a literal here rather than shared, the usual
/// no-cross-runtime-dependency convention (ADR-030) this project
/// already applies to `saaios.task`'s status strings.
const AUTHORIZED_KEYS_PATH: &str = "/data/saaios/var/dropbear/.ssh/authorized_keys";

/// One line of `authorized_keys` -- `client_name` is that line's own
/// trailing comment field, the same "one source of truth, no separate
/// copy" reasoning docs/os/ideas.md already recorded for why this
/// screen reads the file directly instead of saai-shell keeping its
/// own list.
struct TrustedClient {
    client_name: String,
    fingerprint: String,
}

/// Read fresh every time, not cached -- same reasoning as
/// `bluetooth_list_open`'s own doc comment: this list needs to stay
/// correct across taps that mutate the very file it reads (a revoke
/// removes a line then immediately redraws), and the file is small
/// enough that re-parsing it every draw costs nothing worth avoiding.
fn trusted_clients() -> Vec<TrustedClient> {
    let Ok(content) = std::fs::read_to_string(AUTHORIZED_KEYS_PATH) else {
        return Vec::new();
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| TrustedClient {
            client_name: line
                .split_whitespace()
                .nth(2)
                .unwrap_or("(без имени)")
                .to_string(),
            fingerprint: key_fingerprint(line),
        })
        .collect()
}

/// Removes exactly one line by its position in the file as `trusted_
/// clients()` currently enumerates it -- re-reads and rewrites the
/// whole file rather than trying to patch it in place, the same
/// "small file, just redo it" choice `ShellSettings::save()` already
/// makes for its own JSON. A stale index (the file changed between
/// this screen's last draw and this tap, e.g. a fresh pairing landed
/// in between) just removes whatever now sits at that position or,
/// past the end, does nothing -- no worse than the same race any
/// index-based row tap in this codebase already accepts.
fn revoke_trusted_client(index: usize) {
    let Ok(content) = std::fs::read_to_string(AUTHORIZED_KEYS_PATH) else {
        return;
    };
    let mut lines: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if index >= lines.len() {
        return;
    }
    lines.remove(index);
    let mut new_content = lines.join("\n");
    if !new_content.is_empty() {
        new_content.push('\n');
    }
    let _ = std::fs::write(AUTHORIZED_KEYS_PATH, new_content);
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
    /// Master switch for the `pair-recv`/`dropbear` SSH pairing flow
    /// (see `apply_remote_access`'s doc comment). `false` by default
    /// -- an existing settings file with no such field parses to
    /// `false`, so upgrading never silently opens this up.
    remote_access_enabled: bool,
    /// HIA-04b: the Rollback this roadmap entry names -- `true` by
    /// default (the Orb is the point of building it), but a real
    /// escape hatch back to tab-bar-only navigation for anyone who
    /// wants it, same "opt back out, not opt in" shape as
    /// `contrast_pct`/`text_scale_pct` already use for their own
    /// defaults.
    orb_enabled: bool,
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
            remote_access_enabled: false,
            orb_enabled: true,
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
            remote_access_enabled: value
                .get("remote_access_enabled")
                .and_then(Value::as_bool)
                .unwrap_or(default.remote_access_enabled),
            orb_enabled: value
                .get("orb_enabled")
                .and_then(Value::as_bool)
                .unwrap_or(default.orb_enabled),
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
            "remote_access_enabled": self.remote_access_enabled,
            "orb_enabled": self.orb_enabled,
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
fn wifi_connected_ssid() -> Option<String> {
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
        Some(ssid)
    } else {
        None
    }
}

fn wifi_status_line() -> String {
    match wifi_connected_ssid() {
        Some(ssid) => format!("Подключено: {ssid}"),
        None => "Не подключено".to_string(),
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
        .map(|text| {
            text.lines()
                .filter(|line| line.starts_with("SAVED\t"))
                .count()
        })
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
    let status =
        std::fs::read_to_string("/sys/class/power_supply/maxfg/status").unwrap_or_default();
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
    0,    // UTC
    60,   // Центральная Европа (UTC+1)
    120,  // Восточная Европа (UTC+2)
    180,  // Москва (UTC+3)
    270,  // Иран (UTC+4:30)
    330,  // Индия (UTC+5:30)
    480,  // Китай (UTC+8)
    540,  // Япония (UTC+9)
    -300, // США, восточное побережье (UTC-5)
];

/// S21: below this (and not charging), `check_low_battery` creates one
/// `saaios.notification`.
const LOW_BATTERY_THRESHOLD_PCT: u8 = 15;

/// `true` if nobody has touched anywhere on the panel (this client's
/// own surfaces or any other client's) for at least `threshold` --
/// i.e. it's safe, from this signal alone, to lock. Missing file
/// (host tests, or a boot that hasn't seen a single touch event yet)
/// counts as idle: there's no evidence of activity to defer for.
/// Split from the real `GLOBAL_LAST_INPUT_PATH`-bound call so tests
/// can point it at a throwaway file instead of the real `/run/saaios`
/// path.
fn input_idle_for_at_least(path: &std::path::Path, threshold: Duration) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return true;
    };
    let Ok(modified) = metadata.modified() else {
        return true;
    };
    SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|elapsed| elapsed >= threshold)
}

fn global_input_idle_for_at_least(threshold: Duration) -> bool {
    input_idle_for_at_least(std::path::Path::new(GLOBAL_LAST_INPUT_PATH), threshold)
}

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

/// The other half of `pair-recv.c` (see that file's own doc comment):
/// a brand new SSH client is useless until a human taps "Allow" here,
/// same policy shape as `PendingConsent` above -- modal, owns every
/// touch while it's showing. `stream` is the live connection back to
/// `pair-recv`, held open for the (up to two minutes) it takes a
/// human to actually look at the screen; the response is written
/// directly to it on accept/decline, then it's dropped.
struct PendingPairRequest {
    client_name: String,
    public_key: String,
    stream: std::os::unix::net::UnixStream,
}

/// A real cryptographic fingerprint: SHA256 over the key blob's
/// *decoded* bytes (not its base64 text), re-encoded unpadded -- the
/// exact "SHA256:<base64>" shape `ssh-keygen -l -f <pubkey>` already
/// prints, so whoever is looking at this screen can cross-check it
/// against their own client's own tooling, not just this project's
/// own invented format. Falls back to the raw key text itself if it
/// doesn't even parse as `<type> <base64> [comment]` -- still shows
/// something rather than nothing on a screen that exists specifically
/// so a human can refuse.
fn key_fingerprint(public_key: &str) -> String {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
    use base64::Engine as _;
    use sha2::{Digest, Sha256};

    let Some(base64_part) = public_key.split_whitespace().nth(1) else {
        return public_key.to_string();
    };
    let Ok(decoded) = STANDARD.decode(base64_part) else {
        return public_key.to_string();
    };
    let digest = Sha256::digest(&decoded);
    format!("SHA256:{}", STANDARD_NO_PAD.encode(digest))
}

const INTENT_SCREEN_ID: &str = "intent-input";
const INTENT_HEADER_ID: &str = "intent-header";
const INTENT_ROWS_ID: &str = "intent-rows";
const INTENT_CANCEL_ACTION: &str = "intent:cancel";
const INTENT_MODE_TOGGLE_ACTION: &str = "intent:mode:toggle";
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

/// S29: the digits/symbols side of the same keyboard, toggled in by
/// `INTENT_MODE_TOGGLE_ACTION` -- same three-row shape as
/// `INTENT_KEY_ROWS` so `intent_view()` doesn't need to know which
/// one is active beyond picking the row source (`keyboard_rows_for_
/// mode`). Curated for what a real WPA2 password (ADR-065's own
/// motivating case) actually needs, not an exhaustive ASCII table --
/// still no uppercase, that's future polish, not this sprint's scope.
const INTENT_SYMBOL_ROWS: [&str; 3] = ["1234567890", "-_/:;()$&@\"", ".,?!'#%^*+="];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum KeyboardMode {
    #[default]
    Letters,
    Symbols,
}

impl KeyboardMode {
    fn toggled(self) -> Self {
        match self {
            KeyboardMode::Letters => KeyboardMode::Symbols,
            KeyboardMode::Symbols => KeyboardMode::Letters,
        }
    }
}

/// `INTENT_KEY_ROWS` or `INTENT_SYMBOL_ROWS`, whichever `mode` is
/// currently showing -- the one place that decision is made, shared
/// by `intent_view()` (what's tappable) and `intent_keyboard_keys()`
/// (what's drawn), so the two can never disagree about which row set
/// is live.
fn keyboard_rows_for_mode(mode: KeyboardMode) -> &'static [&'static str; 3] {
    match mode {
        KeyboardMode::Letters => &INTENT_KEY_ROWS,
        KeyboardMode::Symbols => &INTENT_SYMBOL_ROWS,
    }
}

/// The one control (`IntentControlDef::label`) whose drawn text isn't
/// a fixed string -- "123" invites switching to `Symbols`, "ABC"
/// invites switching back, same convention as a real OSK.
fn mode_toggle_label(mode: KeyboardMode) -> String {
    match mode {
        KeyboardMode::Letters => "123".to_string(),
        KeyboardMode::Symbols => "ABC".to_string(),
    }
}

struct IntentControlDef {
    id: &'static str,
    label: &'static str,
    action: &'static str,
}

const INTENT_CONTROLS: [IntentControlDef; 5] = [
    IntentControlDef {
        id: "intent-cancel",
        label: "Отмена",
        action: INTENT_CANCEL_ACTION,
    },
    IntentControlDef {
        id: "intent-mode",
        label: "123",
        action: INTENT_MODE_TOGGLE_ACTION,
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
    mode: KeyboardMode,
}

/// S19: reuses `intent_view()`'s keyboard layout and hit-testing
/// verbatim (same rows, same control ids) rather than growing a
/// second keyboard tree -- the two are visually and structurally
/// identical, only the header text and what "Отправить" does differ,
/// both handled by which of `intent_input`/`wifi_password` is
/// currently `Some`. S29 added a digits/symbols mode (`KeyboardMode`,
/// toggled by the same "123"/"ABC" control both screens share) --
/// still lowercase-only and no uppercase, but a real WPA2 password
/// with digits and punctuation is now actually typeable.
struct WifiPasswordState {
    ssid: String,
    buffer: String,
    mode: KeyboardMode,
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

/// Same shape as `BluetoothListTap`, minus a scan/refresh control --
/// this list has no live-scanning state to refresh, it's a plain file
/// re-read fresh on every draw (`trusted_clients()`'s own doc
/// comment).
enum TrustedClientTap {
    Revoke(usize),
    Back,
}

fn trusted_client_action_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    client_count: usize,
) -> Option<TrustedClientTap> {
    for index in 0..client_count {
        if stacked_row_rect(index, width, height).contains(pos.0, pos.1) {
            return Some(TrustedClientTap::Revoke(index));
        }
    }
    if stacked_row_rect(client_count, width, height).contains(pos.0, pos.1) {
        return Some(TrustedClientTap::Back);
    }
    None
}

/// HIA-20: every `dev_surface_rows()` row is read-only diagnostic
/// text, not a button -- the only real tap target on this screen is
/// the trailing "Назад" row right after them, same convention
/// `trusted_client_action_at`'s own `Back` variant already uses, just
/// without the per-row action this screen has no need for.
fn dev_surface_back_tapped(pos: (f64, f64), width: u32, height: u32, row_count: usize) -> bool {
    stacked_row_rect(row_count, width, height).contains(pos.0, pos.1)
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
/// `saaios.action`'s own recognized value for an Action `saai-taskd` has
/// created but not yet executed -- same no-cross-runtime-dependency
/// convention as the three constants above.
const TASK_STATUS_PENDING: &str = "pending";

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
    /// HIA-07: replaces the old task-only `TaskConfirm` -- one
    /// variant for any entity, `actions` sized to whatever
    /// `ObjectViewContent::actions` produced (0-2 today).
    ObjectView {
        title: String,
        status: String,
        related: Option<String>,
        header: Rect,
        actions: Vec<(Rect, &'static str)>,
    },
    RemotePairing {
        client_name: String,
        fingerprint: String,
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
    TrustedClients {
        header: Rect,
        status_line: String,
        rows: Vec<(Rect, String)>,
    },
    /// HIA-20: the hidden diagnostic screen -- same row-list shape as
    /// `TrustedClients` just above, reused verbatim rather than
    /// inventing new geometry for a screen that's read-only text.
    DevSurface {
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
    /// VUI-03 (ADR-112): the real composed `Сейчас`, behind
    /// `NOW_COMPOSED_MARKER` while the app grid it will eventually
    /// replace is still `RootPage::Now`'s default. See that marker's own
    /// doc comment.
    Now {
        content_rect: Rect,
        tabs: Vec<(Rect, &'static str)>,
        header: ContextHeader,
        sections: Vec<SystemSection>,
        object: Option<ObjectSummary>,
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

const OBJECT_VIEW_HEADER_ID: &str = "object-view-header";
const OBJECT_VIEW_BUTTONS_ID: &str = "object-view-buttons";
const OBJECT_VIEW_ACTION_PREFIX: &str = "object-view-action:";

/// HIA-07 (docs/os/sprints/HIA-ROADMAP.md): one reusable screen for
/// any entity -- same header-plus-buttons shape as `task_confirm_
/// view`, generalized to 0-2 buttons instead of always exactly two
/// (`action_count` comes from `ObjectViewContent::actions`' own
/// length at the call site). No separate "related" leaf -- like
/// `task_confirm_view`, this is one header leaf plus an optional
/// button row; `draw_object_view` places title/status/related at
/// fixed offsets within the header rect itself, the same pattern
/// the old task-only draw function already used for title alone.
fn object_view(width: u32, height: u32, action_count: usize) -> LayoutNode {
    let mut children = vec![Node::leaf(OBJECT_VIEW_HEADER_ID)];
    if action_count > 0 {
        let button_leaves: Vec<Node> = (0..action_count)
            .map(|index| {
                Node::leaf(format!("object-view-action-{index}"))
                    .with_action(format!("{OBJECT_VIEW_ACTION_PREFIX}{index}"))
            })
            .collect();
        children.push(
            Node::linear(OBJECT_VIEW_BUTTONS_ID, Axis::Horizontal, button_leaves)
                .with_size(Length::Fill, Length::Px(TASK_CONFIRM_BUTTON_HEIGHT)),
        );
    }
    let root = Node::linear("object-view", Axis::Vertical, children);
    layout(&root, Rect::new(0, 0, width, height))
}

/// The tapped button's position (0 = first/primary), or `None` if the
/// tap missed every button or there are none -- same "layout only
/// returns a position, the call site decides what it means" shape as
/// `task_confirm_action_at`'s own `bool`, generalized past two fixed
/// buttons since which entity is showing decides how many buttons
/// (and what they do) at any given moment.
fn object_view_action_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    action_count: usize,
) -> Option<usize> {
    if width == 0 || height == 0 || action_count == 0 {
        return None;
    }
    object_view(width, height, action_count)
        .hit_test(pos.0, pos.1)
        .and_then(|node| node.action.as_deref())
        .and_then(|action| action.strip_prefix(OBJECT_VIEW_ACTION_PREFIX))
        .and_then(|index| index.parse::<usize>().ok())
}

/// What Object View shows for one entity -- `saaios.task`/`saaios.
/// notification` are the two real, pre-existing types "Входящие"
/// already listed before this (S13 Change 2 / S21); every other
/// entity_type falls through to the `_` arm below, HIA-ROADMAP.md's
/// own negative scenario: still a real title and a non-empty status
/// line, never blank, never a crash, just no type-specific actions.
struct ObjectViewContent {
    title: String,
    status: String,
    related: Option<String>,
    actions: Vec<&'static str>,
}

fn object_view_content(entity: &Entity, selected_entities: &[Entity]) -> ObjectViewContent {
    match entity.entity_type.as_str() {
        "saaios.task" => {
            // A real related-entity lookup, not a placeholder: every
            // saaios.task saai-taskd creates carries the intent_id
            // that produced it (see `confirm_pending_task`'s own old
            // doc comment, now folded into `handle_object_view_
            // action`) -- shown here only when that intent is
            // actually present in the same already-loaded entity list,
            // never a second round-trip to entityd just for this.
            let related = entity
                .properties
                .get("intent_id")
                .and_then(Value::as_str)
                .and_then(|id| id.parse::<Uuid>().ok())
                .and_then(|id| {
                    selected_entities.iter().find(|candidate| {
                        candidate.id == id && candidate.entity_type == "saaios.intent"
                    })
                })
                .map(|intent| format!("Из намерения: {}", intent.title));
            ObjectViewContent {
                title: entity.title.clone(),
                status: "Ждёт подтверждения".to_string(),
                related,
                // Same wording the old saaios.task-only confirm
                // screen already used -- users who saw it shouldn't
                // see the button text change out from under them.
                actions: vec!["Подтвердить", "Отклонить"],
            }
        }
        NOTIFICATION_ENTITY_TYPE => ObjectViewContent {
            title: entity.title.clone(),
            status: entity
                .properties
                .get("body")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            related: None,
            actions: vec!["Скрыть"],
        },
        _ => {
            let status = if entity.properties.is_empty() {
                "Нет дополнительных данных".to_string()
            } else {
                entity
                    .properties
                    .iter()
                    .map(|(key, value)| format!("{key}: {}", object_view_property_text(value)))
                    .collect::<Vec<_>>()
                    .join(" · ")
            };
            ObjectViewContent {
                title: entity.title.clone(),
                status,
                related: None,
                actions: Vec::new(),
            }
        }
    }
}

/// A raw string's own text, not its quoted JSON form -- everything
/// else (numbers/bools/null/arrays/objects) falls back to `Value`'s
/// own `Display`, which is honest enough for the fallback path this
/// feeds (HIA-07's negative scenario is about never being empty, not
/// about being pretty).
fn object_view_property_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

const ORB_DOT_ID: &str = "orb-dot";
const ORB_TOGGLE_ACTION: &str = "orb:toggle";
const ORB_MENU_INBOX_ACTION: &str = "orb-menu:inbox";
const ORB_MENU_INTENT_ACTION: &str = "orb-menu:intent";
const ORB_MENU_BLUETOOTH_ACTION: &str = "orb-menu:bluetooth";
/// HIA-04a's spike (ADR-090) proved the mechanics; this is the
/// permanent shape. `Idle`/`Attention` are silent (no menu drawn),
/// `Menu` is the one state a tap actually opens. `Listening`/
/// `ControlLayer` (source document section 55) are deliberately not
/// built yet -- HIA-ROADMAP.md defers both to Phase 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrbState {
    Idle,
    Menu,
    Attention,
}

/// Pure decision, no `Shell` needed -- `Menu` wins outright (it's a
/// deliberate user action in progress), otherwise `Attention` reflects
/// a real, already-live signal (`inbox_notifications`, S21/S30, now
/// also fed by ADR-089's failed-task notifications) rather than a
/// synthetic flag invented for this feature alone.
fn orb_state(menu_open: bool, has_pending_notifications: bool) -> OrbState {
    if menu_open {
        OrbState::Menu
    } else if has_pending_notifications {
        OrbState::Attention
    } else {
        OrbState::Idle
    }
}

/// Square, not a circle -- same reasoning as `draw_status_bar`'s HIA-03
/// dot: no circle-drawing primitive exists in `render.rs`.
fn orb_dot_size(width: u32, height: u32) -> u32 {
    90.min(width / 10).min(height / 10)
}

/// HIA-04b's own negative scenario (HIA-ROADMAP.md): the Orb must
/// never occupy hit-test space the tab-bar/cards already use. Every
/// `Frame::Root` page's cards start at `y=430` (2400-scale --
/// `stacked_row_rect`/`now_grid_rect` both hardcode that same
/// constant) and the tab bar sits at the very bottom of the screen;
/// this zone's bottom edge is pinned to `y=410`, twenty px of margin
/// short of where a card could ever start, in EVERY state including
/// an open menu, regardless of how many rows it holds -- HIA-05 adds
/// a variable action count (1-3 today) but keeps the exact same fixed
/// zone bounds, just splitting the available fill-space among however
/// many rows there are. The menu grows upward into the header box's
/// own dead space (`draw_root`'s `SURFACE`-filled rect at
/// `y=150..340`, which has never had a hit-test target of its own),
/// never downward into card territory.
fn orb_zone_rect(width: u32, height: u32, menu_action_count: usize) -> Rect {
    let margin = width / 22;
    let dot_size = orb_dot_size(width, height);
    let bottom = ((410_u64 * height as u64) / 2400) as u32;
    if menu_action_count == 0 {
        return Rect::new(
            width.saturating_sub(margin + dot_size),
            bottom.saturating_sub(dot_size),
            dot_size,
            dot_size,
        );
    }
    let top = ((160_u64 * height as u64) / 2400) as u32;
    let zone_width = 420.min(width.saturating_sub(margin * 2));
    Rect::new(
        width.saturating_sub(margin + zone_width),
        top,
        zone_width,
        bottom.saturating_sub(top),
    )
}

/// HIA-05: which real thing a tapped Orb row does -- `Toggle` is
/// always the dot itself (open/close), never a labeled row. The other
/// three are `orb_menu_actions`' own vocabulary; adding a
/// fourth someday only needs a new variant plus its `wire()`/`parse()`/
/// `label()` arms, `orb_view`/`orb_action_at` already handle any
/// length list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrbAction {
    Toggle,
    OpenInbox,
    OpenIntent,
    OpenBluetooth,
}

impl OrbAction {
    fn wire(self) -> &'static str {
        match self {
            OrbAction::Toggle => ORB_TOGGLE_ACTION,
            OrbAction::OpenInbox => ORB_MENU_INBOX_ACTION,
            OrbAction::OpenIntent => ORB_MENU_INTENT_ACTION,
            OrbAction::OpenBluetooth => ORB_MENU_BLUETOOTH_ACTION,
        }
    }

    fn parse(action: &str) -> Option<Self> {
        match action {
            ORB_TOGGLE_ACTION => Some(OrbAction::Toggle),
            ORB_MENU_INBOX_ACTION => Some(OrbAction::OpenInbox),
            ORB_MENU_INTENT_ACTION => Some(OrbAction::OpenIntent),
            ORB_MENU_BLUETOOTH_ACTION => Some(OrbAction::OpenBluetooth),
            _ => None,
        }
    }

    /// Never called for `Toggle` -- that one's drawn as the dot
    /// itself, not a text row (`build_orb_frame` never puts it in
    /// `menu_rows`).
    fn label(self) -> &'static str {
        match self {
            OrbAction::Toggle => "",
            OrbAction::OpenInbox => "Входящие",
            OrbAction::OpenIntent => "Новое намерение",
            OrbAction::OpenBluetooth => "Bluetooth устройства",
        }
    }
}

/// HIA-05: the two real, already-live signals that vary this list --
/// which space is active (`is_system_space` -- HIA-02's own
/// ContextFrame is what decides `selected_space_id` in the first
/// place) and whether Bluetooth has any paired device at all
/// (`bluetooth_paired`, from `bluetooth_paired_count() > 0`, S20,
/// real -- "paired", not necessarily "currently connected"; no live-
/// connection-state concept exists anywhere in this codebase, an
/// honest gap noted in `docs/os/ideas.md` rather than papered over
/// here). `OpenInbox` is the one action every context keeps --
/// "Входящие" always makes sense regardless of which space or device
/// state is active. Pure, no `Shell` needed -- the two booleans are
/// all the real-world state this decision actually depends on, same
/// split `orb_state` already uses.
fn orb_menu_actions(is_system_space: bool, bluetooth_paired: bool) -> Vec<OrbAction> {
    let mut actions = vec![OrbAction::OpenInbox];
    if !is_system_space {
        actions.push(OrbAction::OpenIntent);
    }
    if bluetooth_paired {
        actions.push(OrbAction::OpenBluetooth);
    }
    actions
}

/// Closed (`menu_actions` empty): the whole zone IS the dot, one
/// leaf, nothing to stack. Open: a vertical list within the (now
/// taller) zone -- one row per `menu_actions` entry, the dot itself
/// last, doubling as the close control -- same "layout only returns a
/// position, the call site decides what it means" shape `object_view`/
/// `task_confirm_view` already use.
fn orb_view(width: u32, height: u32, menu_actions: &[OrbAction]) -> LayoutNode {
    let zone = orb_zone_rect(width, height, menu_actions.len());
    if menu_actions.is_empty() {
        return layout(&Node::leaf(ORB_DOT_ID).with_action(ORB_TOGGLE_ACTION), zone);
    }
    let mut children: Vec<Node> = menu_actions
        .iter()
        .enumerate()
        .map(|(index, action)| Node::leaf(format!("orb-menu-{index}")).with_action(action.wire()))
        .collect();
    children.push(
        Node::leaf(ORB_DOT_ID)
            .with_action(ORB_TOGGLE_ACTION)
            .with_size(Length::Fill, Length::Px(orb_dot_size(width, height))),
    );
    layout(&Node::linear("orb-menu", Axis::Vertical, children), zone)
}

fn orb_action_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    menu_actions: &[OrbAction],
) -> Option<OrbAction> {
    if width == 0 || height == 0 {
        return None;
    }
    orb_view(width, height, menu_actions)
        .hit_test(pos.0, pos.1)
        .and_then(|node| node.action.as_deref())
        .and_then(OrbAction::parse)
}

/// What `draw_orb` needs, computed once per frame in `build_orb_
/// frame` (a `Shell` method -- needs `&self` for the current space's
/// color and notification state, which `orb_view`/`orb_state` alone
/// can't see).
struct OrbFrame {
    dot: Rect,
    dot_color: render::Pixel,
    /// HIA-16: drives `draw_orb`'s non-color signal -- a hollow ring
    /// instead of a solid square, so `Attention` is distinguishable
    /// by shape alone, not only by its fixed alert color.
    is_attention: bool,
    menu_rows: Vec<(Rect, &'static str)>,
}

/// ADR-093 follow-up: what actually needs to change for `present_
/// status_bar` to redraw -- compared against the last-drawn snapshot
/// so an unchanged tick (the common case, since `current_time_string`
/// only changes once a minute) can skip the real Wayland commit
/// entirely. `saai-displayd` only treats a layer-surface commit as
/// scene-affecting while unlocked (`affects_scene` in its own commit
/// handler); while locked it already ignores the status bar's commits
/// regardless. So this dedup's only real effect is letting the
/// kernel's self-refresh idle timer actually complete during the
/// unlocked-but-idle window, up to `IDLE_TIMEOUT`, instead of being
/// reset every second by a repaint nothing asked for.
#[derive(Clone, PartialEq)]
struct StatusBarSnapshot {
    time_text: String,
    wifi_up: bool,
    battery: Option<(u8, bool)>,
    dot_color: render::Pixel,
}

fn intent_key_action(ch: char) -> String {
    format!("{INTENT_KEY_PREFIX}{ch}")
}

/// Built and hit-tested the same way as `root_view()`/`consent_view()`:
/// one `Node`/`layout()` tree, no second set of rectangles for touch.
/// Every row (letters and controls alike) is `Length::Fill` on both
/// axes, so the keyboard reflows to whatever the real panel size is
/// instead of assuming a fixed design canvas.
fn intent_view(width: u32, height: u32, mode: KeyboardMode) -> LayoutNode {
    let mut rows: Vec<Node> = keyboard_rows_for_mode(mode)
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

fn intent_action_at(
    pos: (f64, f64),
    width: u32,
    height: u32,
    mode: KeyboardMode,
) -> Option<String> {
    if width == 0 || height == 0 {
        return None;
    }
    intent_view(width, height, mode)
        .hit_test(pos.0, pos.1)
        .and_then(|node| node.action.clone())
}

/// Builds both the header rect and the drawn `(Rect, label)` pairs for
/// every key -- letters/digits uppercased for display the same way a
/// real keyboard shows capital letter-caps while typing lowercase,
/// controls labelled from `INTENT_CONTROLS` except the one whose
/// label depends on `mode` (`mode_toggle_label`). Shared by both
/// `intent_input` and `wifi_password`'s `draw()` branches -- see
/// `WifiPasswordState`'s doc comment for why they share one keyboard.
fn intent_keyboard_keys(
    width: u32,
    height: u32,
    mode: KeyboardMode,
) -> (Rect, Vec<(Rect, String)>) {
    let view = intent_view(width, height, mode);
    let header = view.children[0].rect;
    let keyboard_rows = &view.children[1].children;
    let rows = keyboard_rows_for_mode(mode);
    let mut keys = Vec::new();
    for (row_index, letters) in rows.iter().enumerate() {
        let row_node = &keyboard_rows[row_index];
        for (key_node, ch) in row_node.children.iter().zip(letters.chars()) {
            keys.push((key_node.rect, ch.to_uppercase().to_string()));
        }
    }
    let controls_node = &keyboard_rows[rows.len()];
    for (key_node, control) in controls_node.children.iter().zip(INTENT_CONTROLS.iter()) {
        let label = if control.action == INTENT_MODE_TOGGLE_ACTION {
            mode_toggle_label(mode)
        } else {
            control.label.to_string()
        };
        keys.push((key_node.rect, label));
    }
    (header, keys)
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
        "notifications.post" => "Отправка уведомлений",
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

/// HIA-08 (docs/os/sprints/HIA-ROADMAP.md), deliberately minimal: a
/// named display surface and what it can do -- a registry of exactly
/// one today, existing purely so future work (HIA-15 AOD, or a real
/// second physical Surface) has an actual type to extend instead of
/// inventing one from scratch under time pressure then. Nothing reads
/// this to change render or input behavior yet -- `known_surfaces()`
/// is logged once at startup and otherwise unused, same "scaffold,
/// not an architecture decision" spirit the roadmap's own acceptance
/// line calls for (no ADR for this item, unlike HIA-01/02/07).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Surface {
    name: &'static str,
    capabilities: SurfaceCapabilities,
}

/// Plain booleans, not a bitflags/enum set -- two fields today, no
/// reason yet to pay for more machinery than that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct SurfaceCapabilities {
    /// Real: `TouchHandler`'s `impl` below is this shell's only input
    /// path, there has never been a pointer/keyboard-only build.
    touch: bool,
    /// Not real yet -- no low-power partial-refresh path exists
    /// anywhere in this codebase. `false` here is the honest current
    /// fact, not a placeholder; HIA-15 is what would flip it.
    always_on_display: bool,
}

/// The one Surface this shell has ever run on, filled in from facts
/// already established elsewhere in this file (see each field's own
/// doc comment on `SurfaceCapabilities`) -- not new discovery, and
/// not a live query of the actual Wayland/DRM state, which this
/// binary has never needed to introspect for a single fixed panel.
fn known_surfaces() -> Vec<Surface> {
    vec![Surface {
        name: "Pixel main display",
        capabilities: SurfaceCapabilities {
            touch: true,
            always_on_display: false,
        },
    }]
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

/// VUI-03: real schedule entries for the "Сегодня" `SystemSection` --
/// enabled `saaios.schedule` triggers in the space (ADR-036), the closest
/// real analogue this project has to a calendar/reminder entry. This
/// project's schedules are recurring intervals (`every_secs`), not
/// specific times of day -- "Сегодня" shows what schedule text is
/// active, never an invented clock time no real data backs.
const SCHEDULE_ENTITY_TYPE: &str = "saaios.schedule";

fn today_schedules(entities: &[Entity]) -> Vec<&Entity> {
    entities
        .iter()
        .filter(|entity| {
            entity.entity_type == SCHEDULE_ENTITY_TYPE
                && entity
                    .properties
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .collect()
}

/// VUI-03: real in-progress system activity for the "Продолжается"
/// `SystemSection` -- a `saaios.task` genuinely `running` (the status
/// `handle_object_view_action` writes once the user confirms it), not
/// merely `waiting_confirmation`: a task still waiting on the user stays
/// in "Требует внимания" via `inbox_rows` instead, since it needs the
/// user to act, not the system.
fn in_progress_work(entities: &[Entity]) -> Vec<&Entity> {
    entities
        .iter()
        .filter(|entity| {
            entity.entity_type == "saaios.task"
                && entity.properties.get("status").and_then(Value::as_str)
                    == Some(TASK_STATUS_RUNNING)
        })
        .collect()
}

/// VUI-03: the "next action" data source the sprint's own inventory task
/// found entirely missing -- the first place in this file that reads
/// `saaios.action` at all. A `pending` Action is one `saai-taskd` created
/// from a confirmed Task but has not yet executed -- literally the next
/// thing the system is about to do. `entities` arrives already sorted
/// newest-`updated_at`-first (see the `EntityResponseResult::Entities`
/// handler), so `find` returns the most recently touched match, the same
/// "most recent" convention `selected_entities.first()` already uses for
/// the current object.
const ACTION_ENTITY_TYPE: &str = "saaios.action";

fn next_pending_action(entities: &[Entity]) -> Option<&Entity> {
    entities.iter().find(|entity| {
        entity.entity_type == ACTION_ENTITY_TYPE
            && entity.properties.get("status").and_then(Value::as_str)
                == Some(TASK_STATUS_PENDING)
    })
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

/// How far a touch has to move (in either direction, on this
/// 1080x2400 panel) before `TouchHandler::up` treats it as a real
/// drag on "Я" rather than a tap that merely twitched a few pixels --
/// small enough that an intentional scroll is never mistaken for a
/// tap, large enough that a normal tap on a real device never
/// accidentally suppresses its own action.
const ME_DRAG_TAP_SLOP_PX: f64 = 24.0;

/// Physically discovered during the S14-S25 series' first device
/// pass: 14 fixed rows plus N installed apps stopped fitting on one
/// screen a while before S25 even landed. Originally worked around
/// with Ещё/Назад pagination (this shell had no drag gesture
/// recognition at all back then); replaced with real vertical
/// drag-to-scroll once `TouchHandler` gained one (`down`/`motion`/`up`
/// below) -- `scrolled_row_rect`/`me_max_scroll_offset` are the same
/// idea `stacked_row_rect` already used, offset by how far the user
/// has dragged.
fn scrolled_row_rect(
    index: usize,
    width: u32,
    height: u32,
    scroll_offset: i32,
    content_rect: Rect,
) -> Option<Rect> {
    let base = stacked_row_rect(index, width, height);
    let top = base.y as i64 - scroll_offset as i64;
    let bottom = top + base.height as i64;
    let content_top = content_rect.y as i64;
    let content_bottom = (content_rect.y + content_rect.height) as i64;
    // A row that doesn't fit entirely inside the content area at this
    // offset is skipped outright (neither drawn nor hit-tested) rather
    // than clipped mid-row -- rows appear/disappear whole as the user
    // scrolls, which is enough for a plain stacked full-width list.
    if top < content_top || bottom > content_bottom {
        return None;
    }
    Some(Rect::new(base.x, top as u32, base.width, base.height))
}

/// How far "Я"'s content list can scroll before its last row's bottom
/// edge reaches the content area's own bottom edge -- the usual
/// "don't scroll past the end" clamp, shared by the live drag in
/// `TouchHandler::motion` and the defensive clamp `me_content_cards`/
/// `me_action_at` apply in case `installed_apps` shrank while "Я"
/// wasn't the visible tab and left a stale, now-too-large offset.
fn me_max_scroll_offset(total_rows: usize, width: u32, height: u32, content_rect: Rect) -> i32 {
    if total_rows == 0 {
        return 0;
    }
    let last_row = stacked_row_rect(total_rows - 1, width, height);
    let last_row_bottom = last_row.y + last_row.height;
    let content_bottom = content_rect.y + content_rect.height;
    (last_row_bottom as i32 - content_bottom as i32).max(0)
}
/// Count of `me_all_card_views`'s fixed (non-app) entries -- kept as
/// one literal here rather than derived from that function's return
/// length, since `me_fixed_card_action` has to agree with it and
/// there's no way to assert two functions' lengths match at compile
/// time anyway.
const ME_FIXED_CARD_COUNT: usize = 18;

/// HIA-20: how many silent taps on the build-id card
/// (`me_fixed_card_action`'s index 1) open the hidden diagnostic
/// screen -- same number as Android's own well-known "tap build
/// number" developer-options unlock.
const DEV_SURFACE_TAP_THRESHOLD: u32 = 7;

/// The `me_all_card_views`'s logical index -> tap action mapping.
/// `None` for the two purely read-only info rows (device summary,
/// storage), "Обновления" (S22, read-only by design), and every
/// installed-app row (info-only on "Я", unlike "Сейчас"'s `now_
/// action_at`). Index 1 (build info) looks read-only too -- its tap
/// action is silent by design, see `DEV_SURFACE_TAP_THRESHOLD`.
fn me_fixed_card_action(logical_index: usize) -> Option<&'static str> {
    match logical_index {
        1 => Some("tap_build_info"),
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
        14 => Some("toggle_remote_access"),
        15 => Some("open_trusted_clients"),
        16 => Some("cycle_space_color"),
        17 => Some("toggle_orb"),
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
    // ADR-024 continued: GPU-native status bar presentation. Neither
    // failure is fatal -- `present_status_bar` falls back to the
    // proven wl_shm path below when either is `None`.
    let dmabuf_global: Option<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1> =
        globals.bind(&qh, 3..=3, ()).ok();
    if dmabuf_global.is_none() {
        eprintln!("saai-shell: zwp_linux_dmabuf_v1 v3 unavailable, status bar will use wl_shm");
    }
    let dmabuf_canvas = match DmabufCanvas::new() {
        Ok(canvas) => Some(canvas),
        Err(error) => {
            eprintln!(
                "saai-shell: dma-buf canvas unavailable ({error}), status bar will use wl_shm"
            );
            None
        }
    };
    // Separate `DmabufCanvas` (own DRM fd, own double-buffered slots) for
    // the main toplevel surface -- different geometry (1080x2400 vs the
    // status bar's 1080x120), so it cannot share the status bar's
    // instance. Opening `/dev/dri/card0` a second time is cheap (no DRM
    // master claimed by either, same as `dmabuf_probe.rs`).
    let main_dmabuf_canvas = match DmabufCanvas::new() {
        Ok(canvas) => Some(canvas),
        Err(error) => {
            eprintln!(
                "saai-shell: dma-buf canvas unavailable ({error}), main surface will use wl_shm"
            );
            None
        }
    };
    // Third `DmabufCanvas` (own DRM fd, own slots) shared by both lock-
    // surface presentation functions (`present_lock_surface`,
    // `present_lock_pin_entry`) -- they already share the wl_shm
    // `lock_buffer`/`lock_pool` pair for the same reason: same surface,
    // same size, mutually exclusive content.
    let lock_dmabuf_canvas = match DmabufCanvas::new() {
        Ok(canvas) => Some(canvas),
        Err(error) => {
            eprintln!(
                "saai-shell: dma-buf canvas unavailable ({error}), lock surface will use wl_shm"
            );
            None
        }
    };
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
    // VUI-01: explicit developer-only calibration fixture. Both switches are
    // read once so normal redraws never touch environment or filesystem.
    let calibration_environment = std::env::var("SAAIOS_UI_CALIBRATION").ok();
    let calibration_mode = calibration_requested(
        calibration_environment.as_deref(),
        std::path::Path::new(UI_CALIBRATION_MARKER).exists(),
    );
    let gallery_environment = std::env::var("SAAIOS_UI_GALLERY").ok();
    let gallery_mode = calibration_requested(
        gallery_environment.as_deref(),
        std::path::Path::new(UI_GALLERY_MARKER).exists(),
    );
    let dev_no_lock_environment = std::env::var("SAAIOS_DEV_NO_LOCK").ok();
    let dev_no_lock = calibration_requested(
        dev_no_lock_environment.as_deref(),
        std::path::Path::new(DEV_NO_LOCK_MARKER).exists(),
    );
    let now_composed_environment = std::env::var("SAAIOS_UI_NOW_COMPOSED").ok();
    let now_composed = calibration_requested(
        now_composed_environment.as_deref(),
        std::path::Path::new(NOW_COMPOSED_MARKER).exists(),
    );
    let settings = ShellSettings::load();
    apply_brightness(settings.brightness_pct);
    apply_volume(settings.volume_pct);
    apply_remote_access(settings.remote_access_enabled);
    let remote_pair_listener = {
        let socket_path = REMOTE_PAIR_SOCKET_PATH;
        if let Some(parent) = std::path::Path::new(socket_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Best-effort: a stale socket file left by a previous
        // saai-shell process, same reasoning as `PortalServer::bind`'s
        // own cleanup.
        let _ = std::fs::remove_file(socket_path);
        std::os::unix::net::UnixListener::bind(socket_path)
            .and_then(|listener| {
                listener.set_nonblocking(true)?;
                Ok(listener)
            })
            .ok()
    };
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
        main_dmabuf: main_dmabuf_canvas,
        window,
        session_lock_state,
        session_lock: None,
        lock_surfaces: Vec::new(),
        lock_pool: None,
        lock_buffer: None,
        lock_dmabuf: lock_dmabuf_canvas,
        lock_width: 0,
        lock_height: 0,
        locked: !dev_no_lock,
        dev_no_lock,
        now_composed,
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
        dmabuf_global,
        dmabuf: dmabuf_canvas,
        last_statusbar_snapshot: None,
        last_statusbar_refresh: Instant::now(),
        low_battery_notified: false,
        fonts,
        appd: appd_client::AppdClient::new(appd_socket),
        pending_consent: None,
        remote_pair_listener,
        pending_pair_request: None,
        intent_input: None,
        wifi_password: None,
        wifi_list: None,
        bluetooth_list_open: false,
        trusted_clients_open: false,
        pin_setup: None,
        pin_entry_buffer: String::new(),
        me_scroll_offset: 0,
        me_drag: None,
        me_scroll_dirty: false,
        entityd: entityd_client::EntitydClient::new(entityd_socket),
        spaces: Vec::new(),
        selected_space_id: "home".into(),
        entity_counts: BTreeMap::new(),
        viewing_entity_id: None,
        orb_menu_open: false,
        dev_surface_tap_count: 0,
        dev_surface_open: false,
        calibration_mode,
        gallery_mode,
        selected_entities: Vec::new(),
        system_space_entities: Vec::new(),
        context_frame: Vec::new(),
        last_context_signal_refresh: Instant::now(),
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
    if shell.calibration_mode {
        println!("saai-shell: VUI-01 calibration fixture enabled");
    }
    if shell.gallery_mode {
        println!("saai-shell: VUI-02 component gallery enabled");
    }
    // HIA-08: logged, not read back anywhere -- see `known_surfaces`'s
    // own doc comment for why this exists at all right now.
    println!("saai-shell: known surfaces: {:?}", known_surfaces());

    // Boots locked, matching drm-splash.c's own `bool locked = true` at
    // the top of its main loop -- a phone that boots straight to an
    // unlocked launcher would be a real regression, not a simplification.
    // `dev_no_lock` is the one deliberate, volatile, developer-only
    // exception -- see `DEV_NO_LOCK_MARKER`'s own doc comment.
    if shell.dev_no_lock {
        println!("saai-shell: dev_no_lock enabled, skipping boot-time session lock");
    } else {
        shell.session_lock = Some(
            shell
                .session_lock_state
                .lock(&qh)
                .expect("ext-session-lock-v1 not supported by saai-displayd"),
        );
        println!("saai-shell: session lock requested");
    }

    while !shell.exit {
        event_loop
            .dispatch(Duration::from_millis(16), &mut shell)
            .expect("event loop dispatch failed");
        // Consume every queued touch motion first, then render only the
        // newest scroll position once. Rendering from inside motion()
        // made each ~40 ms frame prevent the event queue from catching
        // up, so the picture followed old finger positions indefinitely.
        shell.draw_pending_scroll(&conn, &qh);
        shell.poll_appd(&conn, &qh);
        shell.poll_entityd(&conn, &qh);
        shell.poll_remote_pairing(&conn, &qh);
        shell.refresh_apps_if_due();
        shell.refresh_statusbar_if_due(&qh);
        shell.refresh_context_signals_if_due();
        shell.poll_portal();
        shell.check_idle_timeout(&qh);
        shell.check_deep_idle(&conn, &qh);
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
    /// GPU-native alternative to `buffer`/`pool` for the main
    /// toplevel surface (ADR-024 continued) -- `None` whenever
    /// `zwp_linux_dmabuf_v1` or the DRM device is unavailable, or once
    /// any error on this surface's path is hit (permanent fallback to
    /// wl_shm for the rest of this process's life, same pattern the
    /// status bar's own `dmabuf` field already uses).
    main_dmabuf: Option<DmabufCanvas>,
    window: Window,

    session_lock_state: SessionLockState,
    session_lock: Option<SessionLock>,
    lock_surfaces: Vec<SessionLockSurface>,
    /// Kept alive for as long as the lock surface's buffer is attached --
    /// dropping the pool would unmap the shared memory the compositor
    /// still needs to read after `commit()` returns.
    lock_pool: Option<SlotPool>,
    lock_buffer: Option<Buffer>,
    /// GPU-native alternative to `lock_buffer`/`lock_pool`, shared by
    /// `present_lock_surface` and `present_lock_pin_entry` the same
    /// way those two already share the wl_shm pair (ADR-024
    /// continued). `None` on unavailability or any error, same
    /// permanent-fallback contract as `main_dmabuf`/`dmabuf`.
    lock_dmabuf: Option<DmabufCanvas>,
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
    /// See `DEV_NO_LOCK_MARKER`'s own doc comment.
    dev_no_lock: bool,
    /// See `NOW_COMPOSED_MARKER`'s own doc comment.
    now_composed: bool,
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
    /// ADR-024 continued: GPU-native status bar presentation.
    /// `None` for either field means the wl_shm path above
    /// (`layer_pool`/`layer_buffer`) is used instead -- absent at
    /// startup if the compositor's `zwp_linux_dmabuf_v1` global or
    /// `/dev/dri/card0` access was unavailable, and permanently
    /// cleared by `present_status_bar` on the first real failure.
    dmabuf_global: Option<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1>,
    dmabuf: Option<DmabufCanvas>,
    /// ADR-093 follow-up: last snapshot actually drawn into
    /// `layer_buffer`, so `present_status_bar` can skip redundant
    /// commits. `None` both initially and whenever the buffer itself
    /// was just invalidated (forces a real redraw either way).
    last_statusbar_snapshot: Option<StatusBarSnapshot>,
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
    /// The "adb"-style pairing policy layer: `None` if `pair-recv`
    /// isn't running or its socket couldn't be bound (remote access
    /// simply doesn't work in that case, not a fatal error for the
    /// shell itself).
    remote_pair_listener: Option<std::os::unix::net::UnixListener>,
    /// Set while a new SSH client is waiting for an on-device tap --
    /// modal, same as `pending_consent`.
    pending_pair_request: Option<PendingPairRequest>,
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
    /// Same shape as `bluetooth_list_open` -- "Доверенные клиенты"
    /// (opened from "Я") has no cached rows either, `trusted_clients()`
    /// is read fresh from `authorized_keys` on every touch/draw.
    trusted_clients_open: bool,
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
    /// Vertical drag-to-scroll position for "Я"'s content list, in
    /// pixels -- 0 is the top. Deliberately not reset when leaving "Я"
    /// for another tab -- returning to it keeps the scroll position
    /// the user was last looking at. Clamped against the current
    /// content length on every read (`me_max_scroll_offset`), not
    /// here, since `installed_apps` can change size while "Я" isn't
    /// the visible tab.
    me_scroll_offset: i32,
    /// `Some((touch_start_y, offset_at_touch_start))` from a
    /// touch-down that landed inside "Я"'s content area (and no modal
    /// was covering it) until the matching touch-up; `None` the rest
    /// of the time, including mid-drag on any other page.
    /// `TouchHandler::up` uses the stored start position to tell a
    /// real drag apart from a tap that merely twitched a few pixels
    /// (`ME_DRAG_TAP_SLOP_PX`) before deciding whether to run
    /// `me_action_at` at all.
    me_drag: Option<(f64, i32)>,
    /// Set whenever a motion event changes `me_scroll_offset`. The main
    /// loop clears it only after Wayland has dispatched the complete
    /// currently queued input batch and a free dma-buf slot is available,
    /// coalescing arbitrarily many stale finger positions into one frame.
    me_scroll_dirty: bool,
    entityd: entityd_client::EntitydClient,
    spaces: Vec<Space>,
    selected_space_id: String,
    entity_counts: BTreeMap<String, usize>,
    selected_entities: Vec<Entity>,
    /// HIA-01: `saaios.space-relation`/`saaios.space-lifecycle`
    /// entities all live in the system space (`SYSTEM_SPACE_ID`),
    /// regardless of which space the user currently has selected --
    /// same reasoning as `selected_entities` above, just keyed to a
    /// fixed id instead of the dynamic selection, since this data
    /// needs to be visible no matter what's on screen.
    system_space_entities: Vec<Entity>,
    /// HIA-02: at most one entry per `ContextSource`, kept current by
    /// `upsert_manual_context` (Manual, on a real tap) and
    /// `refresh_context_signals_if_due` (Wifi, on a timer) --
    /// `effective_context_space` picks the top-confidence entry.
    context_frame: Vec<ContextFrameEntry>,
    /// Throttles `refresh_context_signals_if_due`, same pattern as
    /// `last_statusbar_refresh`/`last_apps_refresh` just below.
    last_context_signal_refresh: Instant,
    /// HIA-07: which entity Object View is currently showing --
    /// `Some` only after the user taps a row on "Входящие" (S13
    /// Change 2 / S21), `None` again once its action is taken
    /// (`handle_object_view_action`) or it turns out stale
    /// (`viewing_entity`). Was `confirming_task_id`/`saaios.task`-only
    /// before this; same "explicit, page-scoped entry point, no
    /// auto-popup" shape S13 Change 2 already established, now
    /// covering any entity_type "Входящие" ever lists a row for.
    viewing_entity_id: Option<Uuid>,
    /// HIA-04b: `true` only while the Orb's own menu is showing --
    /// `orb_state()` reports `Menu` whenever this is set, regardless
    /// of any pending notification underneath it.
    orb_menu_open: bool,
    /// HIA-20: silent, un-hinted tap counter on the build-id card
    /// (`me_fixed_card_action`'s index 1, "tap_build_info") -- the
    /// same well-known convention Android's own "tap build number"
    /// developer-options unlock uses. Never shown anywhere; resets to
    /// 0 the moment it actually opens `dev_surface_open`, per HIA-
    /// ROADMAP.md's own "must not pollute the normal interface".
    dev_surface_tap_count: u32,
    /// HIA-20: the hidden diagnostic screen (`Frame::DevSurface`) --
    /// real, live `ContextFrame`/grants, not static placeholder text.
    dev_surface_open: bool,
    /// VUI-01: developer-only, immutable for this process. The explicit
    /// environment switch renders the semantic calibration fixture and
    /// suppresses unlocked content input; it is never persisted as a user
    /// setting and therefore cannot accidentally become normal navigation.
    calibration_mode: bool,
    gallery_mode: bool,
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

// ADR-024 continued: GPU-native status bar presentation. None of
// smithay-client-toolkit's delegate_*! macros know about these
// linux-dmabuf protocol objects or our own dmabuf_canvas::Busy
// user-data, so they are dispatched by hand instead.
impl Dispatch<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1, ()> for Shell {
    fn event(
        _state: &mut Self,
        _proxy: &zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        _event: zwp_linux_dmabuf_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, ()> for Shell {
    fn event(
        _state: &mut Self,
        _proxy: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
        _event: zwp_linux_buffer_params_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, Busy> for Shell {
    fn event(
        _state: &mut Self,
        _proxy: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        data: &Busy,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        dmabuf_canvas::handle_release(data, &event);
    }
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
        qh: &QueueHandle<Self>,
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
        self.present_lock_pin_entry(qh);
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
        qh: &QueueHandle<Self>,
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
        self.last_statusbar_snapshot = None;
        self.present_status_bar(qh);
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
        qh: &QueueHandle<Self>,
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
            self.me_drag = None;
            self.pin_entry_buffer.clear();
            self.present_lock_pin_entry(qh);
            println!("saai-shell: woke from pseudo-sleep");
            return;
        }
        self.unlock_pending = self.locked
            && self
                .lock_surfaces
                .iter()
                .any(|ls| *ls.wl_surface() == surface);
        self.tab_touch_pending =
            !self.locked && !self.calibration_mode && surface == *self.window.wl_surface();
        // Real drag-to-scroll for "Я" (replacing this shell's old
        // Ещё/Назад pagination, back when it had no drag gesture
        // recognition at all): armed only when this touch starts
        // inside the content area (not the tab bar) of "Я" itself,
        // with no modal covering it -- `TouchHandler::motion` updates
        // `me_scroll_offset` from here on while it stays `Some`, and
        // `up` uses the stored start position to tell a real drag
        // apart from a tap.
        self.me_drag = if self.tab_touch_pending
            && self.current_page == RootPage::Me
            && !self.any_modal_open()
            && root_view(self.width, self.height).children[0]
                .rect
                .contains(position.0, position.1)
        {
            Some((position.1, self.me_scroll_offset))
        } else {
            None
        };
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
        // Taken (not just read) here, once, regardless of which
        // branch below actually runs -- `down()` always sets a fresh
        // value (`Some` or `None`) on the next touch, so nothing is
        // lost by not clearing it in every other branch individually.
        let me_drag = self.me_drag.take();
        let was_me_drag = me_drag.is_some_and(|(start_y, _)| {
            (self.last_touch_pos.1 - start_y).abs() > ME_DRAG_TAP_SLOP_PX
        });
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
                        self.present_lock_pin_entry(qh);
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
            } else if self.pending_pair_request.is_some() {
                // Modal, same as consent -- reuses task_confirm_
                // action_at's hit-test verbatim (identical geometry
                // to the frame this draws, see the frame-building
                // branch above).
                if let Some(accept) =
                    task_confirm_action_at(self.last_touch_pos, self.width, self.height)
                {
                    self.respond_to_pair_request(accept, conn, qh);
                }
            } else if self.viewing_entity_id.is_some() {
                // Modal, same as consent: Object View owns every
                // touch while it's showing (S09 Change 3 / ADR-031's
                // follow-up, generalized past `saaios.task` alone by
                // HIA-07). Only reachable by first tapping a row on
                // "Входящие" (S13 Change 2 / S21) -- no auto-popup.
                // Button count varies by entity_type, so it has to be
                // recomputed here, same "read fresh" reasoning
                // `object_view_content` itself already documents.
                let action_count = self
                    .viewing_entity()
                    .map(|entity| {
                        object_view_content(entity, &self.selected_entities)
                            .actions
                            .len()
                    })
                    .unwrap_or(0);
                if let Some(index) = object_view_action_at(
                    self.last_touch_pos,
                    self.width,
                    self.height,
                    action_count,
                ) {
                    self.handle_object_view_action(index);
                    self.draw(conn, qh);
                }
            } else if self.intent_input.is_some() {
                // Modal, same as consent: the on-screen keyboard owns
                // every touch while it's showing.
                let mode = self
                    .intent_input
                    .as_ref()
                    .map_or(KeyboardMode::Letters, |state| state.mode);
                if let Some(action) =
                    intent_action_at(self.last_touch_pos, self.width, self.height, mode)
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
                let mode = self
                    .wifi_password
                    .as_ref()
                    .map_or(KeyboardMode::Letters, |state| state.mode);
                if let Some(action) =
                    intent_action_at(self.last_touch_pos, self.width, self.height, mode)
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
            } else if self.trusted_clients_open {
                // Modal, same as the others -- owns every touch while
                // "Доверенные клиенты" is open. Recomputes the client
                // count fresh, same reasoning as the Bluetooth branch
                // just above.
                let client_count = trusted_clients().len();
                if let Some(tap) = trusted_client_action_at(
                    self.last_touch_pos,
                    self.width,
                    self.height,
                    client_count,
                ) {
                    self.handle_trusted_client_tap(tap, conn, qh);
                }
            } else if self.dev_surface_open {
                // HIA-20: modal, same as the others -- every row here
                // is read-only diagnostic text, only "Назад" (the row
                // right after them) does anything.
                let row_count = self.dev_surface_rows().len();
                if dev_surface_back_tapped(self.last_touch_pos, self.width, self.height, row_count)
                {
                    self.dev_surface_open = false;
                    self.draw(conn, qh);
                }
            } else if was_me_drag {
                // A scroll may end over the bottom navigation bar. Consume
                // that release as part of the gesture instead of switching
                // tabs, and make sure its final position is presented.
                self.me_scroll_dirty = true;
            } else if let Some(action) = self
                .settings
                .orb_enabled
                .then(|| {
                    // HIA-05: only actually computes the (possibly
                    // context-dependent) action list when the menu is
                    // showing -- closed, an empty slice is enough to
                    // hit-test the dot alone.
                    let menu_actions = if self.orb_menu_open {
                        orb_menu_actions(
                            self.selected_space_id == SYSTEM_SPACE_ID,
                            bluetooth_paired_count() > 0,
                        )
                    } else {
                        Vec::new()
                    };
                    orb_action_at(self.last_touch_pos, self.width, self.height, &menu_actions)
                })
                .flatten()
            {
                // HIA-04b: only reachable once every modal above has
                // said no -- the exact condition under which
                // `Frame::Root` (the only frame the Orb is ever drawn
                // on) is what's actually showing. Checked before
                // `tab_at`/page-content below on principle, though
                // `orb_zone_rect`'s own doc comment already guarantees
                // their hit-test rects never overlap in practice.
                self.handle_orb_action(action, conn, qh);
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
                if let Some((_kind, id)) = inbox_row_at(
                    self.last_touch_pos,
                    self.width,
                    self.height,
                    &self.selected_entities,
                ) {
                    // HIA-07: every row, task or notification alike,
                    // opens the same Object View now -- a task's own
                    // Подтвердить/Отклонить and a notification's own
                    // Скрыть live in that screen's action row instead
                    // of one kind opening a dedicated modal and the
                    // other dismissing on the bare row tap.
                    self.viewing_entity_id = Some(id);
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
                if let Some(action) =
                    self.me_action_at(self.last_touch_pos, self.width, self.height)
                {
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
        if let Some((start_y, start_offset)) = self.me_drag {
            let content_rect = root_view(self.width, self.height).children[0].rect;
            let total = ME_FIXED_CARD_COUNT + self.installed_apps.len();
            let max_offset = me_max_scroll_offset(total, self.width, self.height, content_rect);
            // Finger moving up (position.1 decreasing) scrolls the
            // content down (offset increases) -- the usual touch-
            // scroll convention (content follows the finger).
            let delta = start_y - position.1;
            let new_offset = (start_offset as f64 + delta).round() as i32;
            let clamped = new_offset.clamp(0, max_offset);
            if clamped != self.me_scroll_offset {
                self.me_scroll_offset = clamped;
                self.me_scroll_dirty = true;
            }
        }
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
        self.me_drag = None;
    }
}

impl Shell {
    /// Present at most one coalesced "Я" scroll frame after a complete
    /// Wayland dispatch batch. If both dma-buf slots are still owned by
    /// the compositor, keep the latest position dirty and retry after the
    /// next dispatch (which is also how the release event reaches us).
    fn draw_pending_scroll(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        if !self.me_scroll_dirty {
            return;
        }
        if self.current_page != RootPage::Me || self.locked || self.sleeping {
            self.me_scroll_dirty = false;
            return;
        }
        if self
            .main_dmabuf
            .as_ref()
            .is_some_and(|canvas| !canvas.has_free_slot())
        {
            return;
        }
        self.me_scroll_dirty = false;
        self.draw(conn, qh);
    }

    /// Renders the active root section's placeholder content plus the
    /// bottom tab bar (Change step 6) -- proves the real
    /// client<->compositor vertical slice end to end (surface
    /// creation, configure, SHM buffer and commit), same
    /// as the single dark-slate fill this replaced, just with content
    /// that actually changes on navigation instead of a static color.
    fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
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
        } else if let Some(entity) = self.viewing_entity() {
            let content = object_view_content(entity, &self.selected_entities);
            let view = object_view(width, height, content.actions.len());
            let header = view.children[0].rect;
            let actions: Vec<(Rect, &'static str)> = if content.actions.is_empty() {
                Vec::new()
            } else {
                let button_rects = &view.children[1].children;
                content
                    .actions
                    .iter()
                    .zip(button_rects.iter())
                    .map(|(label, node)| (node.rect, *label))
                    .collect()
            };
            Frame::ObjectView {
                title: content.title,
                status: content.status,
                related: content.related,
                header,
                actions,
            }
        } else if let Some(pending) = &self.pending_pair_request {
            // Reuses task_confirm_view's geometry verbatim (same
            // header-plus-two-buttons shape) -- only the drawn text
            // and the touch handler's meaning differ.
            let view = task_confirm_view(width, height);
            let header = view.children[0].rect;
            let buttons = &view.children[1].children;
            Frame::RemotePairing {
                client_name: pending.client_name.clone(),
                fingerprint: key_fingerprint(&pending.public_key),
                header,
                accept: buttons[0].rect,
                decline: buttons[1].rect,
            }
        } else if let Some(state) = &self.intent_input {
            let (header, keys) = intent_keyboard_keys(width, height, state.mode);
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
            let (header, keys) = intent_keyboard_keys(width, height, state.mode);
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
                        if network.secured {
                            "защищена"
                        } else {
                            "открыта"
                        },
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
        } else if self.trusted_clients_open {
            // Same runtime-sized-list shape as the Bluetooth branch
            // above -- `trusted_clients()`'s own doc comment explains
            // why this reads straight from disk instead of a cached
            // snapshot.
            let header = Rect::new(0, 0, width, INTENT_HEADER_HEIGHT);
            let clients = trusted_clients();
            let mut rows: Vec<(Rect, String)> = clients
                .iter()
                .enumerate()
                .map(|(index, client)| {
                    // Truncated the same way `key_fingerprint` itself
                    // used to be before ADR-083 -- full 44-char
                    // SHA256 fingerprints don't fit a row alongside a
                    // name, but enough of the prefix still lets two
                    // same-named clients be told apart.
                    let short_fingerprint =
                        client.fingerprint.get(..24).unwrap_or(&client.fingerprint);
                    (
                        stacked_row_rect(index, width, height),
                        format!(
                            "{}   ·   {short_fingerprint}…   ·   Отозвать",
                            client.client_name
                        ),
                    )
                })
                .collect();
            rows.push((
                stacked_row_rect(clients.len(), width, height),
                "Назад".to_string(),
            ));
            Frame::TrustedClients {
                header,
                status_line: format!("{} доверенных ключей", clients.len()),
                rows,
            }
        } else if self.dev_surface_open {
            // HIA-20: same runtime-sized-list shape as the two
            // branches above, read-only text rows plus one trailing
            // "Назад" -- `dev_surface_rows()`'s own doc comment
            // explains why this is always read fresh, never cached.
            let header = Rect::new(0, 0, width, INTENT_HEADER_HEIGHT);
            let data_rows = self.dev_surface_rows();
            let mut rows: Vec<(Rect, String)> = data_rows
                .iter()
                .enumerate()
                .map(|(index, text)| (stacked_row_rect(index, width, height), text.clone()))
                .collect();
            rows.push((
                stacked_row_rect(data_rows.len(), width, height),
                "Назад".to_string(),
            ));
            Frame::DevSurface {
                header,
                status_line: "Диагностика".to_string(),
                rows,
            }
        } else if self.current_page == RootPage::Now && self.now_composed {
            let view = root_view(width, height);
            Frame::Now {
                content_rect: view.children[0].rect,
                tabs: view.children[1]
                    .children
                    .iter()
                    .zip(ROOT_TABS)
                    .map(|(node, tab)| (node.rect, tab.label))
                    .collect::<Vec<_>>(),
                header: self.now_context_header(),
                sections: self.now_sections(),
                object: self.now_object_summary(),
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

        // HIA-04b: the Orb is drawn once, unconditionally, after the
        // whole `Frame` match below -- computed here, before that
        // match, because every `&self` read it needs (`orb_state`'s
        // own notification check, `space_color`) has to happen before
        // `canvas`/`buffer` take their mutable borrow, same reasoning
        // this function's own top comment already gives for `frame`.
        // Only ever `Some` on `Frame::Root` (the only frame the Orb
        // ever draws on) and only when the Rollback setting allows it.
        let orb_frame = (!self.calibration_mode
            && self.settings.orb_enabled
            && matches!(frame, Frame::Root { .. } | Frame::Now { .. }))
        .then(|| self.build_orb_frame(width, height));

        let fonts = self.fonts.as_ref();
        let contrast_pct = self.settings.contrast_pct;
        let current_page_index = self.current_page.index();
        let current_page_is_now = self.current_page == RootPage::Now;
        let calibration_mode = self.calibration_mode;
        let gallery_mode = self.gallery_mode;

        // GPU-native path (ADR-024 continued): paint directly into a
        // dma-buf backed buffer, skipping the wl_shm host-visible
        // staging copy this surface otherwise needs every frame -- the
        // most frequently redrawn surface in this process (every
        // animation/page switch), unlike the status bar's once-a-
        // minute case `present_status_bar` already proved this on.
        // Falls back to wl_shm below on any error and disables itself
        // for the session; the lock/PIN surfaces deliberately stay on
        // wl_shm for now (higher cost of a display bug there: it can
        // lock the user out).
        let dmabuf_ready = if let (Some(dmabuf_global), Some(main_dmabuf)) =
            (self.dmabuf_global.clone(), self.main_dmabuf.as_mut())
        {
            match main_dmabuf.ensure_size(width, height, &dmabuf_global, qh) {
                // Both slots still busy (compositor hasn't released
                // either yet -- e.g. a burst of redraws faster than it
                // can flip, physically observed right after startup:
                // first-configure placeholder + session-lock + appd-
                // connected redraws land within one dispatch cycle) is
                // normal and recoverable: fall back to wl_shm for just
                // this one frame, keep the dma-buf path alive for the
                // next `draw()` call instead of disabling it for good.
                Ok(()) => main_dmabuf.has_free_slot(),
                Err(error) => {
                    eprintln!(
                        "saai-shell: dma-buf main surface path failed ({error}), disabling it for this session"
                    );
                    self.main_dmabuf = None;
                    false
                }
            }
        } else {
            false
        };

        // `FnOnce` by construction (the `match frame` inside moves out
        // of `frame`) -- matches this closure's own single-call
        // invariant: it is invoked exactly once below, either by
        // `main_dmabuf.paint()` or directly against the wl_shm canvas,
        // never both (the dma-buf branch always returns).
        let paint_frame = move |canvas: &mut [u8]| {
            if calibration_mode {
                render::draw_calibration(
                    &mut render::Canvas::new(canvas, width, height),
                    width,
                    height,
                    fonts,
                );
                // The fixture must show the source tokens exactly. The user's
                // optional accessibility post-process is validated on normal
                // screens, not baked into physical palette calibration.
                return;
            }
            if gallery_mode {
                render::draw_gallery(
                    &mut render::Canvas::new(canvas, width, height),
                    width,
                    height,
                    fonts,
                );
                return;
            }
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
                        fonts,
                    );
                }
                Frame::ObjectView {
                    title,
                    status,
                    related,
                    header,
                    actions,
                } => {
                    render::draw_object_view(
                        &mut render::Canvas::new(canvas, width, height),
                        &title,
                        &status,
                        related.as_deref(),
                        header,
                        &actions,
                        fonts,
                    );
                }
                Frame::RemotePairing {
                    client_name,
                    fingerprint,
                    header,
                    accept,
                    decline,
                } => {
                    render::draw_remote_pair(
                        &mut render::Canvas::new(canvas, width, height),
                        &client_name,
                        &fingerprint,
                        header,
                        accept,
                        decline,
                        fonts,
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
                        fonts,
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
                        fonts,
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
                        fonts,
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
                        fonts,
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
                        fonts,
                    );
                }
                Frame::TrustedClients {
                    header,
                    status_line,
                    rows,
                } => {
                    render::draw_row_list(
                        &mut render::Canvas::new(canvas, width, height),
                        "Доверенные клиенты",
                        &status_line,
                        header,
                        &rows,
                        fonts,
                    );
                }
                Frame::DevSurface {
                    header,
                    status_line,
                    rows,
                } => {
                    render::draw_row_list(
                        &mut render::Canvas::new(canvas, width, height),
                        "Диагностика",
                        &status_line,
                        header,
                        &rows,
                        fonts,
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
                        current_page_index,
                        &context_label,
                        fonts,
                        &content_cards,
                        current_page_is_now,
                    );
                }
                Frame::Now {
                    content_rect,
                    tabs,
                    header,
                    sections,
                    object,
                } => {
                    render::draw_now(
                        &mut render::Canvas::new(canvas, width, height),
                        content_rect,
                        &tabs,
                        current_page_index,
                        &header,
                        &sections,
                        object.as_ref(),
                        fonts,
                    );
                }
            }
            // HIA-04b: unconditional -- `orb_frame` is already `None`
            // whenever it shouldn't draw (not `Frame::Root`, or the
            // Rollback setting turned it off), computed once above before
            // this function's own mutable canvas borrow began.
            if let Some(orb) = &orb_frame {
                render::draw_orb(
                    &mut render::Canvas::new(canvas, width, height),
                    orb.dot,
                    orb.dot_color,
                    orb.is_attention,
                    &orb.menu_rows,
                    fonts,
                );
            }
            render::apply_contrast_boost(canvas, contrast_pct);
        };

        if dmabuf_ready {
            let main_dmabuf = self
                .main_dmabuf
                .as_mut()
                .expect("just confirmed ready above");
            match main_dmabuf.paint(paint_frame) {
                Ok(wl_buffer) => {
                    let surface = self.window.wl_surface();
                    surface.attach(Some(wl_buffer), 0, 0);
                    surface.damage_buffer(0, 0, width as i32, height as i32);
                    self.window.commit();
                }
                Err(error) => {
                    eprintln!(
                        "saai-shell: dma-buf main surface paint failed ({error}), disabling it for this session"
                    );
                    self.main_dmabuf = None;
                }
            }
            return;
        }

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

        // Physically found by "Я"'s drag-to-scroll (ADR/S-follow-up):
        // creating a second buffer here every time the compositor
        // hasn't released the existing one yet -- this function's own
        // previous behavior -- grows the underlying wl_shm_pool
        // without bound under sustained rapid redraws (scrolling can
        // call `draw()` far more often than any tap-driven redraw
        // ever did) and eventually corrupts it badly enough to kill
        // the whole Wayland connection (`Protocol error 2 ... invalid
        // wl_shm_pool size`). Skipping this one frame instead is
        // always safe -- the next content change retries with whatever
        // is current by then. Drag scrolling avoids reaching this path
        // while its dma-buf slots are busy (`draw_pending_scroll`).
        let Some(canvas) = self.pool.canvas(buffer) else {
            return;
        };

        paint_frame(canvas);

        self.window
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        buffer
            .attach_to(self.window.wl_surface())
            .expect("buffer attach");
        self.window.commit();
    }

    /// The lifecycle-cycle gesture's actual write path -- full-replace
    /// `update_entity` on the space's existing `saaios.space-lifecycle`
    /// record if one exists (same pattern `dismiss_notification`/
    /// `handle_object_view_action` already use), or `create_entity`
    /// the first time this space's lifecycle is ever touched.
    fn cycle_space_lifecycle(&mut self, space_id: &str) {
        let next = space_lifecycle(&self.system_space_entities, space_id).next();
        let mut properties = Map::new();
        properties.insert("space_id".into(), Value::String(space_id.to_string()));
        properties.insert("lifecycle".into(), Value::String(next.as_str().to_string()));
        match space_lifecycle_entity(&self.system_space_entities, space_id) {
            Some(entity) => self.entityd.update_entity(entity, properties),
            None => self.entityd.create_entity(
                SYSTEM_SPACE_ID,
                SPACE_LIFECYCLE_ENTITY_TYPE,
                format!("Жизненный цикл: {space_id}"),
                properties,
            ),
        }
    }

    /// HIA-03's "Цвет пространства" card on "Я" -- same write shape as
    /// `cycle_space_lifecycle` just above, one entity type over.
    fn cycle_space_color(&mut self, space_id: &str) {
        let next = space_color(&self.system_space_entities, space_id).next();
        let mut properties = Map::new();
        properties.insert("space_id".into(), Value::String(space_id.to_string()));
        properties.insert("color".into(), Value::String(next.as_str().to_string()));
        match space_color_entity(&self.system_space_entities, space_id) {
            Some(entity) => self.entityd.update_entity(entity, properties),
            None => self.entityd.create_entity(
                SYSTEM_SPACE_ID,
                SPACE_COLOR_ENTITY_TYPE,
                format!("Цвет: {space_id}"),
                properties,
            ),
        }
    }

    /// HIA-02: called from the one place a real, deliberate user
    /// choice happens -- `invoke_content_action`'s tap-driven
    /// `select_space:` branch -- and nowhere else. Deliberately NOT
    /// called from `apply_entityd_message`'s `Selection`/
    /// `SelectionChanged` arms, even though those also update
    /// `selected_space_id`: if a Wi-Fi-triggered auto-switch also
    /// overwrote the Manual entry, losing that Wi-Fi signal
    /// afterwards would have nothing distinct left to fall back to.
    /// Keeping this optimistic (set before entityd confirms the
    /// switch, not after) is fine -- it only feeds
    /// `effective_context_space`'s decision, never drawn directly.
    fn upsert_manual_context(&mut self, space_id: &str) {
        upsert_context_entry(
            &mut self.context_frame,
            ContextFrameEntry {
                space_id: space_id.to_string(),
                confidence: MANUAL_CONFIDENCE,
                source: ContextSource::Manual,
            },
        );
    }

    /// HIA-02's physical-signal side: throttled the same way
    /// `refresh_apps_if_due`/`refresh_statusbar_if_due` are, called
    /// once per main-loop tick. Re-derives the Wi-Fi context-frame
    /// entry from scratch every time (connected SSID, if any, matched
    /// against `saaios.space-signal` records) rather than tracking
    /// deltas -- same "read fresh" convention `wifi_status_line`
    /// already followed before this. If the resulting top-confidence
    /// space differs from what's currently selected, actually
    /// switches to it through the same `select_space` path a manual
    /// tap uses -- no separate "auto-selected" state to keep in sync.
    fn refresh_context_signals_if_due(&mut self) {
        if self.last_context_signal_refresh.elapsed() < CONTEXT_SIGNAL_REFRESH_INTERVAL {
            return;
        }
        self.last_context_signal_refresh = Instant::now();
        match wifi_connected_ssid()
            .and_then(|ssid| space_for_wifi_ssid(&self.system_space_entities, &ssid))
        {
            Some(space_id) => upsert_context_entry(
                &mut self.context_frame,
                ContextFrameEntry {
                    space_id,
                    confidence: WIFI_CONFIDENCE,
                    source: ContextSource::Wifi,
                },
            ),
            None => remove_context_source(&mut self.context_frame, ContextSource::Wifi),
        }
        let effective = effective_context_space(&self.context_frame, &self.selected_space_id);
        if effective != self.selected_space_id && self.entityd.is_connected() {
            self.entityd.select_space(effective);
        }
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
            if !self.entityd.is_connected() {
                return;
            }
            // HIA-01: tapping the space that's already selected has
            // no other effect today (re-selecting a no-op selection),
            // so it's repurposed into the lifecycle-cycle gesture --
            // no new hit-test geometry needed on a page whose four
            // cards are still the static `root.sui` layout (S04).
            // Known rough edge: nothing on the card hints at this
            // gesture at all before the first tap -- the button
            // itself still just says "Выбрано" (its status line has
            // no room to explain a gesture, confirmed live: an
            // earlier version tried and overflowed the card).
            if space_id == self.selected_space_id {
                self.cycle_space_lifecycle(space_id);
            } else {
                self.upsert_manual_context(space_id);
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
    /// Whether some full-screen modal currently owns every touch --
    /// mirrors, in the same order, the condition `TouchHandler::up`'s
    /// own dispatch cascade already checks before it would ever reach
    /// "Я"'s own branch. Used by `TouchHandler::down` to decide
    /// whether to arm "Я"'s drag-to-scroll -- a touch that starts on
    /// top of an open modal must not scroll the content underneath it.
    fn any_modal_open(&self) -> bool {
        self.pending_consent.is_some()
            || self.pending_pair_request.is_some()
            || self.viewing_entity_id.is_some()
            || self.intent_input.is_some()
            || self.pin_setup.is_some()
            || self.wifi_password.is_some()
            || self.wifi_list.is_some()
            || self.bluetooth_list_open
            || self.trusted_clients_open
            || self.dev_surface_open
    }

    /// A method now (not a free function) since it needs
    /// `self.installed_apps.len()` (total row count) and
    /// `self.me_scroll_offset` (current drag position) -- the same
    /// two pieces `me_content_cards` below needs to build the
    /// matching rects, so the two can never disagree about where a
    /// row actually is.
    fn me_action_at(&self, pos: (f64, f64), width: u32, height: u32) -> Option<&'static str> {
        let total = ME_FIXED_CARD_COUNT + self.installed_apps.len();
        let content_rect = root_view(width, height).children[0].rect;
        let offset = self
            .me_scroll_offset
            .clamp(0, me_max_scroll_offset(total, width, height, content_rect));
        (0..total).find_map(|index| {
            scrolled_row_rect(index, width, height, offset, content_rect)
                .filter(|rect| rect.contains(pos.0, pos.1))
                .and_then(|_| me_fixed_card_action(index))
        })
    }

    fn invoke_me_action(&mut self, action: &str, conn: &Connection, qh: &QueueHandle<Self>) {
        match action {
            "tap_build_info" => {
                // HIA-20: no toast, no counter shown anywhere -- see
                // `dev_surface_tap_count`'s own doc comment for why
                // this stays completely silent until it actually
                // opens.
                self.dev_surface_tap_count += 1;
                if self.dev_surface_tap_count >= DEV_SURFACE_TAP_THRESHOLD {
                    self.dev_surface_tap_count = 0;
                    self.dev_surface_open = true;
                }
                self.draw(conn, qh);
                return;
            }
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
            "cycle_space_color" => {
                if self.entityd.is_connected() {
                    self.cycle_space_color(&self.selected_space_id.clone());
                }
            }
            "toggle_orb" => {
                self.settings.orb_enabled = !self.settings.orb_enabled;
                if !self.settings.orb_enabled {
                    // Turning it off mid-menu shouldn't leave a stale
                    // open menu waiting for whenever it's turned back
                    // on.
                    self.orb_menu_open = false;
                }
            }
            "toggle_remote_access" => {
                self.settings.remote_access_enabled = !self.settings.remote_access_enabled;
                apply_remote_access(self.settings.remote_access_enabled);
            }
            "open_trusted_clients" => {
                // Same reasoning as "open_wifi_list" above.
                self.trusted_clients_open = true;
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
            INTENT_MODE_TOGGLE_ACTION => {
                state.mode = state.mode.toggled();
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
            INTENT_MODE_TOGGLE_ACTION => {
                state.mode = state.mode.toggled();
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
    fn handle_wifi_list_tap(
        &mut self,
        tap: WifiListTap,
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match tap {
            WifiListTap::Network(index) => {
                let Some(network) = self.wifi_list.as_ref().and_then(|list| list.get(index)) else {
                    return;
                };
                if network.secured {
                    self.wifi_password = Some(WifiPasswordState {
                        ssid: network.ssid.clone(),
                        buffer: String::new(),
                        mode: KeyboardMode::Letters,
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

    fn handle_trusted_client_tap(
        &mut self,
        tap: TrustedClientTap,
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match tap {
            TrustedClientTap::Revoke(index) => {
                revoke_trusted_client(index);
            }
            TrustedClientTap::Back => {
                self.trusted_clients_open = false;
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
        self.portal.poll(
            &self.apps_by_pid,
            &self.apps_grants,
            &mut self.clipboard,
            &mut self.entityd,
            &self.selected_space_id,
        );
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
        // ADR-093 follow-up: `refresh_apps_if_due` polls `appd.list()`
        // roughly once a second purely to bound how stale the caches can
        // get (see that method's own doc comment) -- not because the
        // registry usually changed between polls. Reporting every List
        // response as "changed" (this used to always return `true` below,
        // for every message kind) meant that 1Hz poll alone forced a real
        // `draw()`/commit via `poll_appd` even while genuinely idle,
        // defeating self-refresh the exact same way the status bar's own
        // unconditional redraw did before `StatusBarSnapshot`. The other
        // message kinds below are real, infrequent state transitions
        // (consent decisions, lifecycle events), not a periodic poll, so
        // only List gets this treatment.
        if let AppServerMessage::Response {
            result: Some(AppResponseResult::List { .. }),
            ..
        } = &message
        {
            let before = (
                self.apps_by_pid.clone(),
                self.apps_grants.clone(),
                self.installed_apps.clone(),
            );
            self.update_app_caches(&message);
            let after = (
                self.apps_by_pid.clone(),
                self.apps_grants.clone(),
                self.installed_apps.clone(),
            );
            return before != after;
        }
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
                // HIA-07: both kinds now open the same Object View
                // on tap (see the touch-dispatch site), so both rows
                // say "Открыть" -- a notification's "Скрыть" moved
                // into that screen's own action row, it no longer
                // happens directly from this list.
                let status = match kind {
                    InboxRowKind::Task => "Ждёт подтверждения",
                    // S21: the notification's own `body` property is
                    // its message; the title is used as the card
                    // label, same split as a task's title/status.
                    InboxRowKind::Notification => entity
                        .properties
                        .get("body")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                };
                let action = "Открыть";
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
    /// Real drag-to-scroll (`TouchHandler::down`/`motion`/`up`) --
    /// every row of `me_all_card_views` that currently fits inside
    /// the content area at `self.me_scroll_offset`, positioned by
    /// `scrolled_row_rect`. No more "Ещё"/"Назад" nav rows to append:
    /// the scroll gesture itself is the navigation now.
    fn me_content_cards(&self, width: u32, height: u32) -> Vec<(Rect, render::ActionCardView)> {
        let all = self.me_all_card_views();
        let content_rect = root_view(width, height).children[0].rect;
        let offset = self.me_scroll_offset.clamp(
            0,
            me_max_scroll_offset(all.len(), width, height, content_rect),
        );
        all.into_iter()
            .enumerate()
            .filter_map(|(index, card)| {
                scrolled_row_rect(index, width, height, offset, content_rect)
                    .map(|rect| (rect, card))
            })
            .collect()
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
            // Policy layer for SSH pairing (`pair-recv.c`/`dropbear`)
            // -- see `apply_remote_access`'s doc comment. Off by
            // default, same "opt-in, never silent" convention as
            // `pin_code`.
            render::ActionCardView::new(
                "Удалённый доступ (SSH)",
                if self.settings.remote_access_enabled {
                    "Включён -- новые устройства могут запросить доступ"
                } else {
                    "Выключен"
                },
                "Изменить",
            ),
            render::ActionCardView::new(
                "Доверенные клиенты",
                format!("{} ключей", trusted_clients().len()),
                "Открыть",
            ),
            // HIA-03: cycles the CURRENTLY SELECTED space's color --
            // same "acts on selected_space_id" scoping the status bar
            // dot itself already has, nothing app-global about this
            // one card despite living among device-wide settings.
            render::ActionCardView::new(
                "Цвет пространства",
                format!(
                    "{} · {}",
                    space_color(&self.system_space_entities, &self.selected_space_id).label(),
                    space_display_name(&self.spaces, &self.selected_space_id)
                ),
                "Изменить",
            ),
            // HIA-04b's own Rollback: a real escape hatch back to
            // tab-bar-only navigation.
            render::ActionCardView::new(
                "Orb",
                if self.settings.orb_enabled {
                    "Включён"
                } else {
                    "Выключен -- только таб-бар"
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

    /// HIA-20: the hidden diagnostic screen's own content -- real,
    /// live values read fresh off `self` every time this is called
    /// (same "no cached snapshot" convention `trusted_clients()`/
    /// `me_all_card_views`'s own app-grants loop already follow),
    /// never a placeholder string. Doc sections 37/47 named
    /// "active ContextFrame, capabilities, policy decisions" --
    /// `context_frame` is the first verbatim, and in this project's
    /// actual, already-shipped vocabulary (ADR-020), a granted
    /// capability set IS the policy decision for that app; there is
    /// no separate PolicyDecision log to show, so this doesn't invent
    /// one just to look more like the aspirational document.
    fn dev_surface_rows(&self) -> Vec<String> {
        let mut rows = vec![format!(
            "Пространство: {} ({})",
            space_display_name(&self.spaces, &self.selected_space_id),
            self.selected_space_id
        )];
        if self.context_frame.is_empty() {
            rows.push("ContextFrame: пусто".to_string());
        } else {
            for entry in &self.context_frame {
                let source = match entry.source {
                    ContextSource::Manual => "manual",
                    ContextSource::Wifi => "wifi",
                };
                rows.push(format!(
                    "ContextFrame: {} · увер. {} · {source}",
                    entry.space_id, entry.confidence
                ));
            }
        }
        if self.installed_apps.is_empty() {
            rows.push("Возможности: нет установленных приложений".to_string());
        } else {
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
                rows.push(format!("{}: {grants}", app.name));
            }
        }
        rows
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

    /// VUI-03 (ADR-112): replaces `context_label()`'s own
    /// name-plus-"(архив)"-suffix string with the real `ContextHeader`
    /// composite -- a non-default lifecycle becomes a nested
    /// `StatusIndicator`, not text baked into the name itself.
    fn now_context_header(&self) -> ContextHeader {
        let header = ContextHeader::new(space_display_name(&self.spaces, &self.selected_space_id))
            .with_section_title("Сейчас");
        if space_lifecycle(&self.system_space_entities, &self.selected_space_id)
            == SpaceLifecycle::Archived
        {
            header.with_lifecycle(StatusIndicator::new(UniversalState::Blocked, "Архив"))
        } else {
            header
        }
    }

    /// VUI-03 (ADR-112): the same "most recently updated entity in the
    /// space" the ad hoc `"inspect_selected_entity"` card already used
    /// (`content_card`'s own doc comment), now returning a real
    /// `ObjectSummary` instead of a `render::ActionCardView` built by
    /// string formatting.
    fn now_object_summary(&self) -> Option<ObjectSummary> {
        self.selected_entities.first().map(|entity| {
            ObjectSummary::new(
                entity.title.clone(),
                format!("{} · версия {}", entity.entity_type, entity.revision),
            )
        })
    }

    /// VUI-03 (ADR-112): the three `SystemSection`s
    /// `human-interface-architecture-v2.md` section 13 asks for, each
    /// built from a real, already-fetched data source -- never a section
    /// with an invented row. A section that ends up with zero real rows
    /// is left out of the returned list entirely (never handed to
    /// `draw_now` empty) -- "Продолжается" and "Далее" (VUI-03's "next
    /// action") are exactly the two data sources the sprint's own
    /// inventory task found the shell reading for the first time here.
    fn now_sections(&self) -> Vec<SystemSection> {
        let mut sections = Vec::new();

        let mut today = SystemSection::new("Сегодня");
        for schedule in today_schedules(&self.selected_entities) {
            let text = schedule
                .properties
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or(&schedule.title);
            today = today.with_row(SystemSectionRow::Data(DataRow::new(
                text.to_string(),
                DataRowVariant::Static,
            )));
        }
        if !today.is_empty() {
            sections.push(today);
        }

        let mut in_progress = SystemSection::new("Продолжается");
        for entity in in_progress_work(&self.selected_entities) {
            in_progress = in_progress.with_row(SystemSectionRow::Status(StatusIndicator::new(
                UniversalState::Running,
                entity.title.clone(),
            )));
        }
        if !in_progress.is_empty() {
            sections.push(in_progress);
        }

        let mut attention = SystemSection::new("Требует внимания");
        for (kind, entity) in inbox_rows(&self.selected_entities) {
            let reason = match kind {
                InboxRowKind::Task => "Ждёт подтверждения",
                InboxRowKind::Notification => entity
                    .properties
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            };
            attention = attention.with_row(SystemSectionRow::Status(
                StatusIndicator::new(UniversalState::Attention, entity.title.clone())
                    .with_reason(reason)
                    .with_variant(StatusIndicatorVariant::Normal),
            ));
        }
        if !attention.is_empty() {
            sections.push(attention);
        }

        let mut next = SystemSection::new("Далее");
        if let Some(action) = next_pending_action(&self.selected_entities) {
            next = next.with_row(SystemSectionRow::Data(DataRow::new(
                action.title.clone(),
                DataRowVariant::Static,
            )));
        }
        if !next.is_empty() {
            sections.push(next);
        }

        sections
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
            let mut status = if self.entityd.is_connected() {
                match self.entity_counts.get(space_id) {
                    Some(count) => format!("Объектов: {count}"),
                    None => "Загрузка объектов…".into(),
                }
            } else {
                "Сервис пространств недоступен".into()
            };
            // HIA-01: lifecycle only shown when it's not the silent
            // default (`SpaceLifecycle::label`'s own doc comment),
            // relation target only when at least one real edge
            // exists -- an untouched space's card looks exactly like
            // it always has. At most one of the two, not both at
            // once: this line is drawn unwrapped in a fixed-width
            // strip (confirmed live -- the first version tried to
            // show both and overflowed the card).
            if let Some(label) = space_lifecycle(&self.system_space_entities, space_id).label() {
                status.push_str(" · ");
                status.push_str(label);
            } else if let Some((target, _kind)) =
                space_relation_targets(&self.system_space_entities, space_id).first()
            {
                status.push_str(" · → ");
                status.push_str(&space_display_name(&self.spaces, target));
            }
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
        let name = space_display_name(&self.spaces, &self.selected_space_id);
        // HIA-01: the one real, always-visible behavioral difference
        // `SpaceLifecycle::Archived` makes today -- everything else
        // about an archived space (its card, its entities) keeps
        // working exactly as before; only the status-bar label
        // admits it's archived, so nobody mistakes it for an active
        // context by accident.
        if space_lifecycle(&self.system_space_entities, &self.selected_space_id)
            == SpaceLifecycle::Archived
        {
            format!("{name} (архив)")
        } else {
            name
        }
    }

    /// S09 Change 3 (ADR-031's follow-up): the first `saaios.task` in
    /// the selected space still waiting on a live confirmation, if any
    /// -- read straight from `selected_entities` (already kept current
    /// by `poll_entityd`), not from any separate state this client
    /// tracks itself. That's the whole point: after a cold reboot this
    /// client has no memory of its own, and a Task the store still
    /// marks `waiting_confirmation` shows up here exactly the same way
    /// it did before the reboot -- never auto-confirmed, never hidden.
    /// HIA-07: the entity named by `viewing_entity_id`, if it's
    /// still present and still actually relevant -- `None` collapses
    /// Object View back to whatever page is underneath (harmless if
    /// the id is stale, e.g. the task was resolved from another
    /// client, or the notification already dismissed elsewhere).
    /// "Relevant" is type-specific (same status/dismissed checks S13
    /// Change 2 and S21 already used before this existed); an
    /// unrecognized entity_type has no staleness concept of its own,
    /// so it's always still relevant as long as the id exists at all.
    fn viewing_entity(&self) -> Option<&Entity> {
        let id = self.viewing_entity_id?;
        self.selected_entities.iter().find(|entity| {
            if entity.id != id {
                return false;
            }
            match entity.entity_type.as_str() {
                "saaios.task" => {
                    entity.properties.get("status").and_then(Value::as_str)
                        == Some(TASK_STATUS_WAITING_CONFIRMATION)
                }
                NOTIFICATION_ENTITY_TYPE => !entity
                    .properties
                    .get("dismissed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                _ => true,
            }
        })
    }

    /// Writes the one property this screen ever changes (`status`),
    /// keeping every other property -- `intent_id` in particular --
    /// exactly as `saai-taskd` wrote it. `saai-taskd`'s own `Subscribe`
    /// reaction to this update is what actually executes (or discards)
    /// the paused Action; this method only ever flips the switch.
    /// Accepts pending connections on `remote_pair_listener` and, if
    /// none is already showing, turns the first complete request
    /// into `pending_pair_request` -- the modal that
    /// `handle_client`/`respond_to_pair_request` above and below
    /// this drive. A blocking read of one JSON line is safe here:
    /// the only thing that ever connects to this socket is `pair-
    /// recv`, a process on this same device that writes its whole
    /// request immediately after connecting, never a real network
    /// client (see `pair-recv.c`'s own doc comment for why the
    /// network-facing side is a separate process at all).
    fn poll_remote_pairing(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        if self.pending_pair_request.is_some() {
            return;
        }
        let Some(listener) = self.remote_pair_listener.as_ref() else {
            return;
        };
        let Ok((stream, _address)) = listener.accept() else {
            return;
        };
        let _ = stream.set_nonblocking(false);
        let mut reader = std::io::BufReader::new(match stream.try_clone() {
            Ok(clone) => clone,
            Err(_) => return,
        });
        let mut line = String::new();
        if std::io::BufRead::read_line(&mut reader, &mut line).unwrap_or(0) == 0 {
            return;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            return;
        };
        let client_name = value
            .get("client_name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let public_key = value
            .get("public_key")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if client_name.is_empty() || public_key.is_empty() {
            return;
        }
        println!("saai-shell: pairing request from \"{client_name}\"");
        self.pending_pair_request = Some(PendingPairRequest {
            client_name,
            public_key,
            stream,
        });
        self.draw(conn, qh);
    }

    /// The other end of `poll_remote_pairing` -- writes the user's
    /// decision back over the same connection `pair-recv` is still
    /// blocked reading from, then drops it. `pair-recv` itself (not
    /// this process) is what actually appends the key to `dropbear`'s
    /// `authorized_keys` on approval -- see that file's own doc
    /// comment for why that boundary is drawn there.
    fn respond_to_pair_request(
        &mut self,
        approved: bool,
        conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let Some(mut pending) = self.pending_pair_request.take() {
            println!(
                "saai-shell: SSH pairing {} for \"{}\"",
                if approved { "approved" } else { "declined" },
                pending.client_name
            );
            let response = if approved {
                b"{\"approved\":true}\n".as_slice()
            } else {
                b"{\"approved\":false}\n".as_slice()
            };
            let _ = std::io::Write::write_all(&mut pending.stream, response);
        }
        self.draw(conn, qh);
    }

    /// HIA-07: interprets a tapped Object View button according to
    /// whichever entity is currently showing -- same "the layout only
    /// returns a position, the call site decides what it means" shape
    /// `task_confirm_action_at`'s own `bool` already used (this
    /// replaces its one other former call site, the old `confirm_
    /// pending_task`), generalized past two fixed buttons/one fixed
    /// entity_type. `index` is only ever consulted for the types that
    /// define an action at that position at all -- an entity_type
    /// with an empty `actions` list never reaches here in the first
    /// place (`object_view_action_at` returns `None` when
    /// `action_count == 0`).
    fn handle_object_view_action(&mut self, index: usize) {
        let Some(entity) = self.viewing_entity().cloned() else {
            return;
        };
        // Closes Object View regardless of outcome -- same "no
        // auto-popup, the user comes back and taps the next one
        // themselves" reasoning S13 Change 2 already established for
        // the task-only modal this replaces.
        self.viewing_entity_id = None;
        match entity.entity_type.as_str() {
            "saaios.task" => {
                let confirm = index == 0;
                let mut properties = entity.properties.clone();
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
                    entity.id,
                    if confirm { "confirmed" } else { "cancelled" }
                );
                self.entityd.update_entity(&entity, properties);
            }
            NOTIFICATION_ENTITY_TYPE => self.dismiss_notification(entity.id),
            _ => {}
        }
    }

    /// HIA-04b: the Orb's own action handler, same "layout returns a
    /// position/identity, the call site decides what it means" split
    /// as `handle_object_view_action`'s own `index`. Both menu actions
    /// reuse already-real navigation -- `RootPage::Inbox`/
    /// `IntentInputState` are the exact paths the tab-bar and "Сейчас"
    /// card already drive, not new placeholder behavior invented for
    /// this menu.
    fn handle_orb_action(&mut self, action: OrbAction, conn: &Connection, qh: &QueueHandle<Self>) {
        match action {
            OrbAction::Toggle => self.orb_menu_open = !self.orb_menu_open,
            OrbAction::OpenInbox => {
                self.orb_menu_open = false;
                self.current_page = RootPage::Inbox;
            }
            OrbAction::OpenIntent => {
                self.orb_menu_open = false;
                self.intent_input = Some(IntentInputState::default());
            }
            OrbAction::OpenBluetooth => {
                self.orb_menu_open = false;
                // Same "open_bluetooth_list" behavior the fixed card
                // on "Я" already triggers -- a fresh scan, not a
                // stale one.
                bluetooth_trigger_scan();
                self.bluetooth_list_open = true;
            }
        }
        self.draw(conn, qh);
    }

    /// `Attention`'s own fixed alert color, deliberately not one of
    /// `SpaceColor`'s six values -- a space's own accent should never
    /// be mistaken for "something needs you".
    fn build_orb_frame(&self, width: u32, height: u32) -> OrbFrame {
        let menu_actions = if self.orb_menu_open {
            orb_menu_actions(
                self.selected_space_id == SYSTEM_SPACE_ID,
                bluetooth_paired_count() > 0,
            )
        } else {
            Vec::new()
        };
        let view = orb_view(width, height, &menu_actions);
        let has_pending = !inbox_notifications(&self.selected_entities).is_empty();
        let state = orb_state(self.orb_menu_open, has_pending);
        let is_attention = state == OrbState::Attention;
        let dot_color = match state {
            OrbState::Attention => render::state_color(UniversalState::Attention),
            OrbState::Idle | OrbState::Menu => {
                space_color(&self.system_space_entities, &self.selected_space_id).pixel()
            }
        };
        if menu_actions.is_empty() {
            return OrbFrame {
                dot: view.rect,
                dot_color,
                is_attention,
                menu_rows: Vec::new(),
            };
        }
        let dot_rect = view.children[menu_actions.len()].rect;
        let menu_rows = menu_actions
            .iter()
            .enumerate()
            .map(|(index, action)| (view.children[index].rect, action.label()))
            .collect();
        OrbFrame {
            dot: dot_rect,
            dot_color,
            is_attention,
            menu_rows,
        }
    }

    /// S21: marks a notification `dismissed` rather than deleting it --
    /// same full-replace-properties convention `handle_object_view_
    /// action` already uses for tasks, so `saai-entityd`'s protocol
    /// needed no changes for either entity type.
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
                    // HIA-01: the system space's own entity list is
                    // cached independently of whatever's currently
                    // selected -- see `system_space_entities`'s doc
                    // comment on the struct field.
                    if space_id == SYSTEM_SPACE_ID {
                        changed |= self.system_space_entities != entities;
                        self.system_space_entities = entities.clone();
                    }
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
            EntityServerMessage::Response {
                ok: false, error, ..
            } => {
                // Previously silent -- cost real debugging time once
                // already (HIA-01's lifecycle-cycle feature shipped
                // with an entity_type saai-entityd rejected, and nothing
                // anywhere said why nothing happened on screen).
                println!("saai-shell: entityd request failed: {error:?}");
                changed = true;
            }
            _ => {}
        }
        changed
    }

    /// Re-locks after `IDLE_TIMEOUT` of no touch activity while
    /// unlocked -- matches drm-splash.c's own 1s-granularity idle poll,
    /// just driven by this event loop's existing 16ms tick instead of a
    /// separate timer source.
    fn check_idle_timeout(&mut self, qh: &QueueHandle<Self>) {
        if self.dev_no_lock {
            return;
        }
        let idle_timeout = Duration::from_secs(self.settings.idle_timeout_secs);
        if self.locked || self.last_activity.elapsed() < idle_timeout {
            return;
        }
        if !global_input_idle_for_at_least(idle_timeout) {
            // S32: this client's own touches went quiet, but the
            // compositor saw more recent activity somewhere else --
            // a foreground third-party app, most likely. Don't lock
            // out from under it; just keep deferring while that
            // activity continues, the same way a fresh own-surface
            // touch would.
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
    fn present_status_bar(&mut self, qh: &QueueHandle<Self>) {
        let width = self.layer_width;
        let height = self.layer_height;
        if width == 0 || height == 0 {
            return;
        }

        let snapshot = StatusBarSnapshot {
            time_text: current_time_string(self.settings.utc_offset_minutes),
            wifi_up: wifi_is_up(),
            battery: read_battery(),
            dot_color: space_color(&self.system_space_entities, &self.selected_space_id).pixel(),
        };
        // ADR-093 follow-up: skip the redraw+commit entirely when nothing
        // visible has changed since the last real paint. `current_time_
        // string` only ticks once a minute, so this is the common case --
        // and every skipped commit is one less forced DRM flip blocking
        // the kernel's self-refresh idle timer during unlocked-idle time.
        // `last_statusbar_snapshot` alone is enough to guard this on
        // either presentation path below: both reset it to `None` on
        // every real resize (the layer's own `configure` handler, before
        // either `layer_buffer` or the dma-buf canvas is touched), so a
        // stale snapshot from before a resize never compares equal to a
        // fresh one taken at the new size.
        if self.last_statusbar_snapshot.as_ref() == Some(&snapshot) {
            return;
        }

        // GPU-native path (ADR-024's "linux-dmabuf for GPU-native
        // clients"): skip saai-displayd's wl_shm host-visible staging
        // copy entirely when both the dma-buf canvas and the
        // compositor's zwp_linux_dmabuf_v1 global are available. Falls
        // back to the proven wl_shm SlotPool path below on ANY failure
        // (and disables itself for the rest of the session, so a real
        // problem degrades once, not on every redraw) -- a DRM/dma-buf
        // problem must never cost the status bar entirely.
        if let (Some(dmabuf_global), Some(dmabuf)) =
            (self.dmabuf_global.clone(), self.dmabuf.as_mut())
        {
            let fonts = self.fonts.as_ref();
            let contrast_pct = self.settings.contrast_pct;
            let painted = dmabuf
                .ensure_size(width, height, &dmabuf_global, qh)
                .and_then(|()| {
                    // Canvas::new below assumes tight width*4 rows, same
                    // as the wl_shm path uses via its own stride =
                    // width*4 -- the kernel is free to pad a dumb
                    // buffer's real pitch for alignment, so verify
                    // rather than silently writing skewed rows into a
                    // wider allocation.
                    if dmabuf.pitch() != width as usize * 4 {
                        return Err(format!(
                            "buffer pitch {} != tightly-packed {} (kernel padded this allocation, Canvas assumes it never does)",
                            dmabuf.pitch(),
                            width * 4
                        ));
                    }
                    dmabuf.paint(|canvas| {
                        render::draw_status_bar(
                            &mut render::Canvas::new(canvas, width, height),
                            width,
                            height,
                            &snapshot.time_text,
                            snapshot.wifi_up,
                            snapshot.battery,
                            snapshot.dot_color,
                            fonts,
                        );
                        render::apply_contrast_boost(canvas, contrast_pct);
                    })
                });
            match painted {
                Ok(wl_buffer) => {
                    let surface = self.layer.wl_surface();
                    surface.attach(Some(wl_buffer), 0, 0);
                    surface.damage_buffer(0, 0, width as i32, height as i32);
                    self.layer.commit();
                    self.last_statusbar_snapshot = Some(snapshot);
                    return;
                }
                Err(error) => {
                    eprintln!(
                        "saai-shell: dma-buf status bar path failed ({error}), disabling it for this session"
                    );
                    self.dmabuf = None;
                }
            }
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
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Xrgb8888,
                )
                .expect("create layer buffer");
            self.layer_buffer = Some(buffer);
        }
        let buffer = self.layer_buffer.as_mut().expect("just ensured above");

        let canvas = match pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (second_buffer, canvas) = pool
                    .create_buffer(
                        width as i32,
                        height as i32,
                        stride,
                        wl_shm::Format::Xrgb8888,
                    )
                    .expect("create layer buffer");
                *buffer = second_buffer;
                canvas
            }
        };
        render::draw_status_bar(
            &mut render::Canvas::new(canvas, width, height),
            width,
            height,
            &snapshot.time_text,
            snapshot.wifi_up,
            snapshot.battery,
            snapshot.dot_color,
            self.fonts.as_ref(),
        );
        render::apply_contrast_boost(canvas, self.settings.contrast_pct);

        let surface = self.layer.wl_surface();
        surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(surface).expect("buffer attach");
        self.layer.commit();
        self.last_statusbar_snapshot = Some(snapshot);
    }

    /// S13 Change 1: throttled the same way `refresh_apps_if_due` is --
    /// called once per main-loop tick, only actually redraws once
    /// `STATUSBAR_REFRESH_INTERVAL` has elapsed.
    fn refresh_statusbar_if_due(&mut self, qh: &QueueHandle<Self>) {
        if self.last_statusbar_refresh.elapsed() >= STATUSBAR_REFRESH_INTERVAL {
            self.present_status_bar(qh);
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

    fn present_lock_surface(&mut self, qh: &QueueHandle<Self>, color: [u8; 4]) {
        let width = self.lock_width;
        let height = self.lock_height;
        if width == 0 || height == 0 {
            return;
        }
        let Some(lock_surface) = self.lock_surfaces.last() else {
            return;
        };
        let stride = width as i32 * 4;

        // GPU-native path (ADR-024 continued), same contract as
        // `draw()`/`present_status_bar`: a transient busy state (both
        // slots still held by the compositor) falls back to wl_shm for
        // just this call; any real error disables the path for the
        // rest of the session.
        if let (Some(dmabuf_global), Some(lock_dmabuf)) =
            (self.dmabuf_global.clone(), self.lock_dmabuf.as_mut())
        {
            let ready = match lock_dmabuf.ensure_size(width, height, &dmabuf_global, qh) {
                Ok(()) => lock_dmabuf.has_free_slot(),
                Err(error) => {
                    eprintln!(
                        "saai-shell: dma-buf lock surface path failed ({error}), disabling it for this session"
                    );
                    self.lock_dmabuf = None;
                    false
                }
            };
            if ready {
                let lock_dmabuf = self
                    .lock_dmabuf
                    .as_mut()
                    .expect("just confirmed ready above");
                match lock_dmabuf.paint(|canvas| {
                    for chunk in canvas.chunks_exact_mut(4) {
                        chunk.copy_from_slice(&color);
                    }
                }) {
                    Ok(wl_buffer) => {
                        let surface = lock_surface.wl_surface();
                        surface.attach(Some(wl_buffer), 0, 0);
                        surface.damage_buffer(0, 0, width as i32, height as i32);
                        surface.commit();
                        return;
                    }
                    Err(error) => {
                        eprintln!(
                            "saai-shell: dma-buf lock surface paint failed ({error}), disabling it for this session"
                        );
                        self.lock_dmabuf = None;
                    }
                }
            }
        }

        if self.lock_pool.is_none() {
            self.lock_pool = Some(
                SlotPool::new(width as usize * height as usize * 4, &self.shm)
                    .expect("create lock surface pool"),
            );
        }
        let pool = self.lock_pool.as_mut().expect("just ensured above");

        if self.lock_buffer.is_none() {
            let (buffer, _canvas) = pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Xrgb8888,
                )
                .expect("create lock buffer");
            self.lock_buffer = Some(buffer);
        }
        let buffer = self.lock_buffer.as_mut().expect("just ensured above");

        let canvas = match pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (second_buffer, canvas) = pool
                    .create_buffer(
                        width as i32,
                        height as i32,
                        stride,
                        wl_shm::Format::Xrgb8888,
                    )
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
    fn present_lock_pin_entry(&mut self, qh: &QueueHandle<Self>) {
        let Some(pin_code) = self.settings.pin_code.clone() else {
            self.present_lock_surface(qh, LOCK_SCREEN_COLOR);
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

        let keys: Vec<(Rect, &'static str)> = PIN_KEYPAD_DIGIT_LABELS
            .iter()
            .enumerate()
            .filter(|(_, label)| !label.is_empty())
            .map(|(index, label)| (pin_keypad_rect(index, width, height), *label))
            .collect();
        let entered_len = self.pin_entry_buffer.len();
        let pin_len = pin_code.len();
        let fonts = self.fonts.as_ref();
        let contrast_pct = self.settings.contrast_pct;

        if let (Some(dmabuf_global), Some(lock_dmabuf)) =
            (self.dmabuf_global.clone(), self.lock_dmabuf.as_mut())
        {
            let ready = match lock_dmabuf.ensure_size(width, height, &dmabuf_global, qh) {
                Ok(()) => lock_dmabuf.has_free_slot(),
                Err(error) => {
                    eprintln!(
                        "saai-shell: dma-buf lock surface path failed ({error}), disabling it for this session"
                    );
                    self.lock_dmabuf = None;
                    false
                }
            };
            if ready {
                let lock_dmabuf = self
                    .lock_dmabuf
                    .as_mut()
                    .expect("just confirmed ready above");
                match lock_dmabuf.paint(|canvas| {
                    render::draw_lock_pin_entry(
                        &mut render::Canvas::new(canvas, width, height),
                        width,
                        entered_len,
                        pin_len,
                        &keys,
                        fonts,
                    );
                    render::apply_contrast_boost(canvas, contrast_pct);
                }) {
                    Ok(wl_buffer) => {
                        let surface = lock_surface.wl_surface();
                        surface.attach(Some(wl_buffer), 0, 0);
                        surface.damage_buffer(0, 0, width as i32, height as i32);
                        surface.commit();
                        return;
                    }
                    Err(error) => {
                        eprintln!(
                            "saai-shell: dma-buf lock surface paint failed ({error}), disabling it for this session"
                        );
                        self.lock_dmabuf = None;
                    }
                }
            }
        }

        if self.lock_pool.is_none() {
            self.lock_pool = Some(
                SlotPool::new(width as usize * height as usize * 4, &self.shm)
                    .expect("create lock surface pool"),
            );
        }
        let pool = self.lock_pool.as_mut().expect("just ensured above");

        if self.lock_buffer.is_none() {
            let (buffer, _canvas) = pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Xrgb8888,
                )
                .expect("create lock buffer");
            self.lock_buffer = Some(buffer);
        }
        let buffer = self.lock_buffer.as_mut().expect("just ensured above");

        let canvas = match pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (second_buffer, canvas) = pool
                    .create_buffer(
                        width as i32,
                        height as i32,
                        stride,
                        wl_shm::Format::Xrgb8888,
                    )
                    .expect("create lock buffer");
                *buffer = second_buffer;
                canvas
            }
        };

        render::draw_lock_pin_entry(
            &mut render::Canvas::new(canvas, width, height),
            width,
            entered_len,
            pin_len,
            &keys,
            fonts,
        );
        render::apply_contrast_boost(canvas, contrast_pct);

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
    fn check_deep_idle(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        let deep_idle_timeout = Duration::from_secs(self.settings.deep_idle_timeout_secs);
        if !self.locked || self.sleeping || self.last_activity.elapsed() < deep_idle_timeout {
            return;
        }
        println!("saai-shell: deep idle timeout, screen off");
        self.sleeping = true;
        self.present_lock_surface(qh, SLEEP_INDICATOR_COLOR);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bluetooth_list_action_at, calibration_requested, capability_label, consent_action_at,
        content_action_at, dev_surface_back_tapped, effective_context_space, format_utc_offset,
        in_progress_work, input_idle_for_at_least, intent_action_at, known_surfaces,
        me_fixed_card_action, next_in_cycle, next_pending_action, object_view_action_at,
        object_view_content, orb_action_at, orb_menu_actions, orb_state, orb_zone_rect,
        remove_context_source, space_color, space_color_entity, space_display_name,
        space_for_wifi_ssid, space_lifecycle, space_lifecycle_entity, space_relation_targets,
        stacked_row_rect, tab_at, task_confirm_action_at, today_schedules,
        trusted_client_action_at, upsert_context_entry, wifi_list_action_at, BluetoothListTap,
        ContextFrameEntry, ContextSource, Entity, KeyboardMode, OrbAction, OrbState, Rect,
        RootPage, Space, SpaceColor, SpaceLifecycle, TrustedClientTap, WifiListTap,
        ACTION_ENTITY_TYPE, INTENT_CANCEL_ACTION, INTENT_MODE_TOGGLE_ACTION, INTENT_SEND_ACTION,
        MANUAL_CONFIDENCE, NOTIFICATION_ENTITY_TYPE, ROOT_CONTENT_ACTIONS, ROOT_TABS,
        SCHEDULE_ENTITY_TYPE, SPACE_COLOR_ENTITY_TYPE, SPACE_LIFECYCLE_ENTITY_TYPE,
        SPACE_RELATION_ENTITY_TYPE, SPACE_SIGNAL_ENTITY_TYPE, SPACE_SIGNAL_TYPE_WIFI_SSID,
        WIFI_CONFIDENCE,
    };
    use saai_entity_store::SpaceKind;
    use std::time::Duration;

    fn test_entity(
        entity_type: &str,
        properties: serde_json::Map<String, serde_json::Value>,
    ) -> Entity {
        Entity {
            schema: 1,
            id: uuid::Uuid::new_v4(),
            space_id: "saaios".to_string(),
            entity_type: entity_type.to_string(),
            title: "test".to_string(),
            properties,
            revision: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn today_schedules_only_returns_enabled_schedule_entities() {
        let mut enabled = serde_json::Map::new();
        enabled.insert("enabled".into(), serde_json::Value::Bool(true));
        let mut disabled = serde_json::Map::new();
        disabled.insert("enabled".into(), serde_json::Value::Bool(false));
        let entities = vec![
            test_entity(SCHEDULE_ENTITY_TYPE, enabled),
            test_entity(SCHEDULE_ENTITY_TYPE, disabled),
            test_entity(SCHEDULE_ENTITY_TYPE, serde_json::Map::new()),
            test_entity("saaios.task", serde_json::Map::new()),
        ];
        assert_eq!(today_schedules(&entities).len(), 1);
    }

    #[test]
    fn in_progress_work_returns_running_tasks_but_not_ones_still_waiting_on_the_user() {
        let mut running = serde_json::Map::new();
        running.insert("status".into(), serde_json::Value::String("running".into()));
        let mut waiting = serde_json::Map::new();
        waiting.insert(
            "status".into(),
            serde_json::Value::String("waiting_confirmation".into()),
        );
        let entities = vec![
            test_entity("saaios.task", running),
            test_entity("saaios.task", waiting),
        ];
        let in_progress = in_progress_work(&entities);
        assert_eq!(in_progress.len(), 1);
        assert_eq!(
            in_progress[0]
                .properties
                .get("status")
                .and_then(serde_json::Value::as_str),
            Some("running")
        );
    }

    #[test]
    fn next_pending_action_finds_a_pending_action_and_ignores_finished_ones() {
        let mut done = serde_json::Map::new();
        done.insert("status".into(), serde_json::Value::String("done".into()));
        let mut pending = serde_json::Map::new();
        pending.insert("status".into(), serde_json::Value::String("pending".into()));
        let entities = vec![
            test_entity(ACTION_ENTITY_TYPE, done),
            test_entity(ACTION_ENTITY_TYPE, pending.clone()),
        ];
        assert!(next_pending_action(&entities).is_some());

        let none_pending = vec![test_entity(ACTION_ENTITY_TYPE, serde_json::Map::new())];
        assert!(next_pending_action(&none_pending).is_none());

        let wrong_type = vec![test_entity("saaios.task", pending)];
        assert!(next_pending_action(&wrong_type).is_none());
    }

    #[test]
    fn calibration_requires_an_explicit_environment_or_runtime_marker() {
        assert!(!calibration_requested(None, false));
        assert!(!calibration_requested(Some("0"), false));
        assert!(calibration_requested(Some("1"), false));
        assert!(calibration_requested(None, true));
    }

    fn lifecycle_entity(space_id: &str, lifecycle: &str) -> Entity {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "space_id".into(),
            serde_json::Value::String(space_id.into()),
        );
        properties.insert(
            "lifecycle".into(),
            serde_json::Value::String(lifecycle.into()),
        );
        test_entity(SPACE_LIFECYCLE_ENTITY_TYPE, properties)
    }

    fn relation_entity(from: &str, to: &str, kind: &str) -> Entity {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "from_space_id".into(),
            serde_json::Value::String(from.into()),
        );
        properties.insert("to_space_id".into(), serde_json::Value::String(to.into()));
        properties.insert("kind".into(), serde_json::Value::String(kind.into()));
        test_entity(SPACE_RELATION_ENTITY_TYPE, properties)
    }

    fn wifi_signal_entity(space_id: &str, ssid: &str) -> Entity {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "space_id".into(),
            serde_json::Value::String(space_id.into()),
        );
        properties.insert(
            "signal_type".into(),
            serde_json::Value::String(SPACE_SIGNAL_TYPE_WIFI_SSID.into()),
        );
        properties.insert("value".into(), serde_json::Value::String(ssid.into()));
        test_entity(SPACE_SIGNAL_ENTITY_TYPE, properties)
    }

    fn color_entity(space_id: &str, color: &str) -> Entity {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "space_id".into(),
            serde_json::Value::String(space_id.into()),
        );
        properties.insert("color".into(), serde_json::Value::String(color.into()));
        test_entity(SPACE_COLOR_ENTITY_TYPE, properties)
    }

    fn task_entity(title: &str, intent_id: Option<uuid::Uuid>) -> Entity {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "status".into(),
            serde_json::Value::String("waiting_confirmation".into()),
        );
        if let Some(id) = intent_id {
            properties.insert(
                "intent_id".into(),
                serde_json::Value::String(id.to_string()),
            );
        }
        let mut entity = test_entity("saaios.task", properties);
        entity.title = title.to_string();
        entity
    }

    fn notification_entity(title: &str, body: &str) -> Entity {
        let mut properties = serde_json::Map::new();
        properties.insert("body".into(), serde_json::Value::String(body.into()));
        let mut entity = test_entity(NOTIFICATION_ENTITY_TYPE, properties);
        entity.title = title.to_string();
        entity
    }

    fn intent_entity(title: &str) -> Entity {
        let mut entity = test_entity("saaios.intent", serde_json::Map::new());
        entity.title = title.to_string();
        entity
    }

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
            super::pin_keypad_action_at(
                center(super::pin_keypad_rect(0, width, height)),
                width,
                height
            ),
            Some("1")
        );
        assert_eq!(
            super::pin_keypad_action_at(
                center(super::pin_keypad_rect(10, width, height)),
                width,
                height
            ),
            Some("0")
        );
        assert_eq!(
            super::pin_keypad_action_at(
                center(super::pin_keypad_rect(11, width, height)),
                width,
                height
            ),
            Some("⌫")
        );
        // Index 9 is the deliberately blank cell between 9 and 0.
        assert_eq!(
            super::pin_keypad_action_at(
                center(super::pin_keypad_rect(9, width, height)),
                width,
                height
            ),
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
            super::pin_setup_action_at(
                center(super::pin_keypad_rect(14, width, height)),
                width,
                height,
                true
            ),
            Some("Убрать PIN")
        );
        assert_eq!(
            super::pin_setup_action_at(
                center(super::pin_keypad_rect(14, width, height)),
                width,
                height,
                false
            ),
            None
        );
        assert_eq!(
            super::pin_setup_action_at(
                center(super::pin_keypad_rect(12, width, height)),
                width,
                height,
                false
            ),
            Some("Отмена")
        );
    }

    #[test]
    fn key_fingerprint_matches_ssh_keygen_l_for_a_known_key() {
        // Golden value cross-checked independently via Python's
        // hashlib against the same key blob (base64.b64decode ->
        // hashlib.sha256 -> base64.b64encode, padding stripped) --
        // not just re-deriving the same computation this function
        // itself performs.
        assert_eq!(
            super::key_fingerprint(
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIL7cwj3z6LbyDARZYUd1Y7CuuM6xKbl2/YpSd3adSNz4 test-client"
            ),
            "SHA256:OuaL+poCXsAtdU50sBMBwOUNL+faDqJqWTmiE3baoOI"
        );
    }

    #[test]
    fn key_fingerprint_falls_back_to_the_raw_text_for_unparseable_input() {
        assert_eq!(
            super::key_fingerprint("not-a-key-at-all"),
            "not-a-key-at-all"
        );
        assert_eq!(
            super::key_fingerprint("ssh-ed25519 not-valid-base64!!"),
            "ssh-ed25519 not-valid-base64!!"
        );
    }

    #[test]
    fn scrolled_row_rect_shifts_up_by_the_scroll_offset() {
        let width = 1080;
        let height = 2400;
        let content_rect = super::Rect::new(0, 0, width, height);
        let base = super::stacked_row_rect(2, width, height);
        let scrolled = super::scrolled_row_rect(2, width, height, 100, content_rect)
            .expect("row still fits inside a full-height content area");
        assert_eq!(scrolled.y, base.y - 100);
        assert_eq!(scrolled.x, base.x);
        assert_eq!(scrolled.height, base.height);
    }

    #[test]
    fn scrolled_row_rect_hides_rows_the_offset_pushes_out_of_the_content_area() {
        let width = 1080;
        let height = 2400;
        let content_rect = super::Rect::new(0, 200, width, 1700);
        let content_bottom = content_rect.y + content_rect.height;
        // A row far enough down the list to sit entirely below the
        // content area at zero scroll (real state: nothing has been
        // dragged yet, so only the first few rows are visible).
        let row_index = 10;
        let base = super::stacked_row_rect(row_index, width, height);
        assert!(
            base.y + base.height > content_bottom,
            "row 10 is expected to overflow the bottom of this content area unscrolled"
        );
        assert!(
            super::scrolled_row_rect(row_index, width, height, 0, content_rect).is_none(),
            "row 10 should not fit at zero scroll"
        );
        // Scrolling down (positive offset) moves it up into view --
        // exactly enough to align its bottom edge with the content
        // area's own bottom edge.
        let needed_offset = (base.y + base.height - content_bottom) as i32;
        let scrolled =
            super::scrolled_row_rect(row_index, width, height, needed_offset, content_rect)
                .expect("row 10 should fit once scrolled down far enough");
        assert_eq!(scrolled.y + scrolled.height, content_bottom);
    }

    #[test]
    fn me_max_scroll_offset_is_zero_with_no_rows_and_positive_once_content_overflows() {
        let width = 1080;
        let height = 2400;
        let content_rect = super::Rect::new(0, 200, width, 1700);
        assert_eq!(
            super::me_max_scroll_offset(0, width, height, content_rect),
            0
        );
        // ME_FIXED_CARD_COUNT (18) real rows at this row height
        // comfortably overflows a 1700px-tall content area -- this
        // asserts the clamp actually engages, not a specific number.
        assert!(
            super::me_max_scroll_offset(super::ME_FIXED_CARD_COUNT, width, height, content_rect)
                > 0
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
    fn trusted_client_action_at_finds_clients_then_back() {
        let width = 1080;
        let height = 2400;
        let client_count = 1;
        let client_0 = stacked_row_rect(0, width, height);
        let back = stacked_row_rect(1, width, height);
        let center = |rect: Rect| {
            (
                (rect.x + rect.width / 2) as f64,
                (rect.y + rect.height / 2) as f64,
            )
        };
        assert!(matches!(
            trusted_client_action_at(center(client_0), width, height, client_count),
            Some(TrustedClientTap::Revoke(0))
        ));
        assert!(matches!(
            trusted_client_action_at(center(back), width, height, client_count),
            Some(TrustedClientTap::Back)
        ));
        assert!(trusted_client_action_at((10.0, 10.0), width, height, client_count).is_none());
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
            intent_action_at((50.0, 300.0), 1080, 2400, KeyboardMode::Letters).as_deref(),
            Some("intent:key:q")
        );
        assert_eq!(
            intent_action_at((50.0, 850.0), 1080, 2400, KeyboardMode::Letters).as_deref(),
            Some("intent:key:a")
        );
        assert_eq!(
            intent_action_at((50.0, 1400.0), 1080, 2400, KeyboardMode::Letters).as_deref(),
            Some("intent:key:z")
        );
    }

    #[test]
    fn intent_keyboard_controls_row_has_cancel_and_send_at_the_ends() {
        assert_eq!(
            intent_action_at((50.0, 2000.0), 1080, 2400, KeyboardMode::Letters).as_deref(),
            Some(INTENT_CANCEL_ACTION)
        );
        assert_eq!(
            intent_action_at((950.0, 2000.0), 1080, 2400, KeyboardMode::Letters).as_deref(),
            Some(INTENT_SEND_ACTION)
        );
    }

    #[test]
    fn intent_keyboard_header_is_not_a_key() {
        assert_eq!(
            intent_action_at((540.0, 100.0), 1080, 2400, KeyboardMode::Letters),
            None
        );
    }

    #[test]
    fn intent_keyboard_symbol_mode_maps_digits_and_symbols() {
        assert_eq!(
            intent_action_at((50.0, 300.0), 1080, 2400, KeyboardMode::Symbols).as_deref(),
            Some("intent:key:1")
        );
        assert_eq!(
            intent_action_at((50.0, 850.0), 1080, 2400, KeyboardMode::Symbols).as_deref(),
            Some("intent:key:-")
        );
    }

    #[test]
    fn intent_keyboard_mode_toggle_is_the_second_control() {
        assert_eq!(
            intent_action_at((300.0, 2000.0), 1080, 2400, KeyboardMode::Letters).as_deref(),
            Some(INTENT_MODE_TOGGLE_ACTION)
        );
    }

    #[test]
    fn keyboard_mode_toggles_both_ways() {
        assert_eq!(KeyboardMode::Letters.toggled(), KeyboardMode::Symbols);
        assert_eq!(KeyboardMode::Symbols.toggled(), KeyboardMode::Letters);
    }

    #[test]
    fn capability_label_translates_known_vocabulary_and_falls_back_for_unknown() {
        assert_eq!(capability_label("net.internet"), "Доступ в интернет");
        assert_eq!(capability_label("net.bluetooth"), "net.bluetooth");
    }

    #[test]
    fn input_idle_reports_idle_when_the_marker_file_is_missing() {
        let missing = std::env::temp_dir().join("saai-shell-test-missing-marker-does-not-exist");
        assert!(input_idle_for_at_least(&missing, Duration::from_secs(60)));
    }

    #[test]
    fn input_idle_reports_not_idle_right_after_a_touch() {
        let marker = tempfile::NamedTempFile::new().unwrap();
        assert!(!input_idle_for_at_least(
            marker.path(),
            Duration::from_secs(60)
        ));
    }

    #[test]
    fn input_idle_reports_idle_once_the_marker_is_old_enough() {
        let marker = tempfile::NamedTempFile::new().unwrap();
        let ancient = std::time::SystemTime::now() - Duration::from_secs(120);
        marker.as_file().set_modified(ancient).unwrap();
        assert!(input_idle_for_at_least(
            marker.path(),
            Duration::from_secs(60)
        ));
    }

    #[test]
    fn space_lifecycle_defaults_to_stable_with_no_matching_record() {
        assert_eq!(space_lifecycle(&[], "car"), SpaceLifecycle::Stable);
        let unrelated = vec![lifecycle_entity("work", "archived")];
        assert_eq!(space_lifecycle(&unrelated, "car"), SpaceLifecycle::Stable);
    }

    #[test]
    fn space_lifecycle_reads_the_matching_record() {
        let entities = vec![
            lifecycle_entity("work", "archived"),
            lifecycle_entity("car", "temporary"),
        ];
        assert_eq!(space_lifecycle(&entities, "car"), SpaceLifecycle::Temporary);
        assert_eq!(space_lifecycle(&entities, "work"), SpaceLifecycle::Archived);
    }

    #[test]
    fn space_lifecycle_entity_finds_the_real_record_to_update_in_place() {
        let entities = vec![lifecycle_entity("car", "emerging")];
        let found = space_lifecycle_entity(&entities, "car").expect("record should be found");
        assert_eq!(
            found.properties.get("lifecycle").and_then(|v| v.as_str()),
            Some("emerging")
        );
        assert!(space_lifecycle_entity(&entities, "work").is_none());
    }

    #[test]
    fn space_lifecycle_cycles_through_all_four_states_and_wraps() {
        assert_eq!(SpaceLifecycle::Temporary.next(), SpaceLifecycle::Emerging);
        assert_eq!(SpaceLifecycle::Emerging.next(), SpaceLifecycle::Stable);
        assert_eq!(SpaceLifecycle::Stable.next(), SpaceLifecycle::Archived);
        assert_eq!(SpaceLifecycle::Archived.next(), SpaceLifecycle::Temporary);
    }

    #[test]
    fn space_relation_targets_only_matches_the_from_direction() {
        let entities = vec![
            relation_entity("car", "commute", "usually_with"),
            relation_entity("commute", "car", "usually_with"),
            relation_entity("car", "dacha", "related_to"),
        ];
        let mut targets = space_relation_targets(&entities, "car");
        targets.sort();
        assert_eq!(
            targets,
            vec![
                ("commute".to_string(), "usually_with".to_string()),
                ("dacha".to_string(), "related_to".to_string()),
            ]
        );
        assert_eq!(
            space_relation_targets(&entities, "commute"),
            vec![("car".to_string(), "usually_with".to_string())]
        );
        assert!(space_relation_targets(&entities, "work").is_empty());
    }

    #[test]
    fn space_display_name_prefers_a_real_space_then_falls_back_to_known_ids() {
        let spaces = vec![Space {
            schema: 1,
            id: "car".to_string(),
            name: "Машина".to_string(),
            kind: SpaceKind::User,
            created_at: chrono::Utc::now(),
        }];
        assert_eq!(space_display_name(&spaces, "car"), "Машина");
        assert_eq!(space_display_name(&spaces, "work"), "Работа");
        assert_eq!(space_display_name(&spaces, "unknown-id"), "unknown-id");
    }

    #[test]
    fn effective_context_space_falls_back_with_an_empty_frame() {
        assert_eq!(effective_context_space(&[], "home"), "home");
    }

    #[test]
    fn effective_context_space_prefers_the_highest_confidence_entry() {
        let frame = vec![
            ContextFrameEntry {
                space_id: "work".into(),
                confidence: 50,
                source: ContextSource::Manual,
            },
            ContextFrameEntry {
                space_id: "home".into(),
                confidence: 80,
                source: ContextSource::Wifi,
            },
        ];
        assert_eq!(effective_context_space(&frame, "personal"), "home");
    }

    #[test]
    fn upsert_context_entry_replaces_the_same_source_instead_of_accumulating() {
        let mut frame = Vec::new();
        upsert_context_entry(
            &mut frame,
            ContextFrameEntry {
                space_id: "home".into(),
                confidence: 80,
                source: ContextSource::Wifi,
            },
        );
        upsert_context_entry(
            &mut frame,
            ContextFrameEntry {
                space_id: "work".into(),
                confidence: 80,
                source: ContextSource::Wifi,
            },
        );
        assert_eq!(frame.len(), 1);
        assert_eq!(frame[0].space_id, "work");
    }

    #[test]
    fn losing_the_wifi_signal_falls_back_to_the_manual_entry() {
        // The exact negative scenario HIA-ROADMAP.md's HIA-02 entry
        // names: losing the physical signal must not leave the
        // context frame empty -- it falls back to the last manual/
        // known selection, which never expires on its own.
        let mut frame = vec![ContextFrameEntry {
            space_id: "work".into(),
            confidence: MANUAL_CONFIDENCE,
            source: ContextSource::Manual,
        }];
        upsert_context_entry(
            &mut frame,
            ContextFrameEntry {
                space_id: "home".into(),
                confidence: WIFI_CONFIDENCE,
                source: ContextSource::Wifi,
            },
        );
        assert_eq!(effective_context_space(&frame, "personal"), "home");
        remove_context_source(&mut frame, ContextSource::Wifi);
        assert_eq!(effective_context_space(&frame, "personal"), "work");
    }

    #[test]
    fn space_for_wifi_ssid_matches_only_the_right_type_and_value() {
        let entities = vec![
            wifi_signal_entity("home", "HomeNet"),
            wifi_signal_entity("work", "OfficeNet"),
            relation_entity("home", "car", "related_to"),
        ];
        assert_eq!(
            space_for_wifi_ssid(&entities, "HomeNet"),
            Some("home".to_string())
        );
        assert_eq!(
            space_for_wifi_ssid(&entities, "OfficeNet"),
            Some("work".to_string())
        );
        assert_eq!(space_for_wifi_ssid(&entities, "SomeOtherNet"), None);
    }

    #[test]
    fn space_color_defaults_with_no_matching_record() {
        // HIA-03's own negative scenario (HIA-ROADMAP.md): a space
        // with no color ever set gets a deterministic default, not
        // an empty/missing dot.
        assert_eq!(space_color(&[], "car"), SpaceColor::Default);
        let unrelated = vec![color_entity("work", "blue")];
        assert_eq!(space_color(&unrelated, "car"), SpaceColor::Default);
    }

    #[test]
    fn space_color_reads_the_matching_record() {
        let entities = vec![color_entity("work", "blue"), color_entity("home", "pink")];
        assert_eq!(space_color(&entities, "work"), SpaceColor::Blue);
        assert_eq!(space_color(&entities, "home"), SpaceColor::Pink);
    }

    #[test]
    fn space_color_entity_finds_the_real_record_to_update_in_place() {
        let entities = vec![color_entity("work", "blue")];
        assert!(space_color_entity(&entities, "work").is_some());
        assert!(space_color_entity(&entities, "home").is_none());
    }

    #[test]
    fn space_color_cycles_through_all_six_states_and_wraps() {
        let mut color = SpaceColor::Default;
        let mut seen = vec![color];
        for _ in 0..5 {
            color = color.next();
            seen.push(color);
        }
        assert_eq!(
            seen,
            vec![
                SpaceColor::Default,
                SpaceColor::Blue,
                SpaceColor::Green,
                SpaceColor::Orange,
                SpaceColor::Purple,
                SpaceColor::Pink,
            ]
        );
        assert_eq!(color.next(), SpaceColor::Default);
    }

    #[test]
    fn space_color_as_str_and_parse_round_trip_for_every_variant() {
        for color in [
            SpaceColor::Default,
            SpaceColor::Blue,
            SpaceColor::Green,
            SpaceColor::Orange,
            SpaceColor::Purple,
            SpaceColor::Pink,
        ] {
            assert_eq!(SpaceColor::parse(color.as_str()), Some(color));
        }
        assert_eq!(SpaceColor::parse("not-a-real-color"), None);
    }

    #[test]
    fn object_view_action_at_finds_two_buttons_by_position() {
        // Same geometry as task_confirm_view's own accept/decline
        // split (task_confirm_screen_left/right_half_of_button_row_*
        // above) -- object_view(_, _, 2) uses the identical layout
        // shape.
        assert_eq!(
            object_view_action_at((270.0, 2250.0), 1080, 2400, 2),
            Some(0)
        );
        assert_eq!(
            object_view_action_at((810.0, 2250.0), 1080, 2400, 2),
            Some(1)
        );
        assert_eq!(object_view_action_at((540.0, 1000.0), 1080, 2400, 2), None);
    }

    #[test]
    fn object_view_action_at_finds_a_single_button_spanning_the_full_row() {
        assert_eq!(
            object_view_action_at((270.0, 2250.0), 1080, 2400, 1),
            Some(0)
        );
        assert_eq!(
            object_view_action_at((810.0, 2250.0), 1080, 2400, 1),
            Some(0)
        );
    }

    #[test]
    fn object_view_action_at_finds_nothing_with_zero_actions() {
        assert_eq!(object_view_action_at((270.0, 2250.0), 1080, 2400, 0), None);
        assert_eq!(object_view_action_at((540.0, 1000.0), 1080, 2400, 0), None);
    }

    #[test]
    fn object_view_content_for_a_task_has_no_related_line_without_a_matching_intent() {
        let task = task_entity("Подтвердите: удалить объект", None);
        let content = object_view_content(&task, &[]);
        assert_eq!(content.title, "Подтвердите: удалить объект");
        assert_eq!(content.status, "Ждёт подтверждения");
        assert_eq!(content.related, None);
        assert_eq!(content.actions, vec!["Подтвердить", "Отклонить"]);
    }

    #[test]
    fn object_view_content_for_a_task_shows_the_originating_intent_when_present() {
        let intent = intent_entity("Напомни поливать цветы");
        let task = task_entity("Подтвердите: полить цветы", Some(intent.id));
        let selected_entities = vec![intent.clone(), task.clone()];
        let content = object_view_content(&task, &selected_entities);
        assert_eq!(
            content.related,
            Some("Из намерения: Напомни поливать цветы".to_string())
        );
    }

    #[test]
    fn object_view_content_for_a_notification_shows_its_body_and_a_dismiss_action() {
        let notification = notification_entity("Маджонг: победа!", "Хорошая игра");
        let content = object_view_content(&notification, &[]);
        assert_eq!(content.title, "Маджонг: победа!");
        assert_eq!(content.status, "Хорошая игра");
        assert_eq!(content.related, None);
        assert_eq!(content.actions, vec!["Скрыть"]);
    }

    #[test]
    fn object_view_content_for_an_unknown_entity_type_is_never_empty_and_has_no_actions() {
        // HIA-07's own negative scenario (HIA-ROADMAP.md): an
        // entity_type this file has no special case for still gets a
        // real title and a non-empty status, not a blank/crashing
        // View -- just no type-specific actions.
        let mut properties = serde_json::Map::new();
        properties.insert("some_number".into(), serde_json::Value::from(42));
        properties.insert(
            "some_text".into(),
            serde_json::Value::String("hello".into()),
        );
        let mut entity = test_entity("some.unknown.type", properties);
        entity.title = "Загадочный объект".to_string();
        let content = object_view_content(&entity, &[]);
        assert_eq!(content.title, "Загадочный объект");
        assert!(!content.status.is_empty());
        assert!(content.status.contains("some_number"));
        assert!(content.status.contains("hello"));
        assert!(content.actions.is_empty());
    }

    #[test]
    fn object_view_content_for_an_unknown_entity_type_with_no_properties_still_has_a_status_line() {
        let mut entity = test_entity("some.other.unknown", serde_json::Map::new());
        entity.title = "Пустой объект".to_string();
        let content = object_view_content(&entity, &[]);
        assert_eq!(content.status, "Нет дополнительных данных");
    }

    #[test]
    fn known_surfaces_is_a_registry_of_exactly_one_real_surface() {
        // HIA-08's own acceptance line: a registry of one today, not
        // an empty/absent concept -- and its capabilities match what
        // this file already establishes elsewhere (a real touch path,
        // no always-on-display path yet).
        let surfaces = known_surfaces();
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].name, "Pixel main display");
        assert!(surfaces[0].capabilities.touch);
        assert!(!surfaces[0].capabilities.always_on_display);
    }

    #[test]
    fn orb_state_prefers_menu_over_attention() {
        assert_eq!(orb_state(true, true), OrbState::Menu);
        assert_eq!(orb_state(true, false), OrbState::Menu);
    }

    #[test]
    fn orb_state_falls_back_from_attention_to_idle() {
        assert_eq!(orb_state(false, true), OrbState::Attention);
        assert_eq!(orb_state(false, false), OrbState::Idle);
    }

    #[test]
    fn orb_zone_never_reaches_where_cards_start() {
        // HIA-04b's own negative scenario (HIA-ROADMAP.md): the Orb
        // must never occupy hit-test space the tab-bar/cards already
        // use. Every Root page's cards start at y=430 (2400-scale) --
        // confirm the zone's bottom edge always stays short of that,
        // for every real action count HIA-05 can now produce (0-3),
        // on a range of real panel sizes.
        for (width, height) in [(1080, 2400), (800, 480), (1440, 3120)] {
            let cards_start = ((430_u64 * height as u64) / 2400) as u32;
            for menu_action_count in 0..=3 {
                let zone = orb_zone_rect(width, height, menu_action_count);
                assert!(
                    zone.y + zone.height <= cards_start,
                    "zone bottom {} exceeds cards_start {} at {width}x{height}, menu_action_count={menu_action_count}",
                    zone.y + zone.height,
                    cards_start
                );
            }
        }
    }

    #[test]
    fn orb_action_at_toggles_the_closed_dot() {
        let dot = orb_zone_rect(1080, 2400, 0);
        let point = (
            (dot.x + dot.width / 2) as f64,
            (dot.y + dot.height / 2) as f64,
        );
        assert_eq!(
            orb_action_at(point, 1080, 2400, &[]),
            Some(OrbAction::Toggle)
        );
    }

    #[test]
    fn orb_action_at_misses_a_normal_card_row_when_closed() {
        // The same point a real "Пространства"/"Входящие" card would
        // occupy (stacked_row_rect(0, ..)'s own territory) must never
        // register as an Orb tap.
        let first_card = stacked_row_rect(0, 1080, 2400);
        let point = (
            (first_card.x + first_card.width / 2) as f64,
            (first_card.y + first_card.height / 2) as f64,
        );
        assert_eq!(orb_action_at(point, 1080, 2400, &[]), None);
    }

    #[test]
    fn orb_action_at_finds_a_menu_row_and_the_close_dot_when_open() {
        let actions = [OrbAction::OpenInbox, OrbAction::OpenIntent];
        let view_zone = orb_zone_rect(1080, 2400, actions.len());
        let inbox_point = (
            (view_zone.x + view_zone.width / 2) as f64,
            (view_zone.y + 10) as f64,
        );
        assert_eq!(
            orb_action_at(inbox_point, 1080, 2400, &actions),
            Some(OrbAction::OpenInbox)
        );
        let dot_point = (
            (view_zone.x + view_zone.width / 2) as f64,
            (view_zone.y + view_zone.height - 10) as f64,
        );
        assert_eq!(
            orb_action_at(dot_point, 1080, 2400, &actions),
            Some(OrbAction::Toggle)
        );
    }

    #[test]
    fn orb_action_at_finds_a_third_row_when_the_menu_has_three_actions() {
        // HIA-05's own shape: the menu isn't fixed at two rows --
        // confirm a real three-action list (as `Shell::orb_menu_
        // actions` produces with a paired Bluetooth device) is fully
        // reachable, not just the first two.
        let actions = [
            OrbAction::OpenInbox,
            OrbAction::OpenIntent,
            OrbAction::OpenBluetooth,
        ];
        let view_zone = orb_zone_rect(1080, 2400, actions.len());
        let third_row_point = (
            (view_zone.x + view_zone.width / 2) as f64,
            (view_zone.y + view_zone.height * 2 / 3 - 10) as f64,
        );
        assert_eq!(
            orb_action_at(third_row_point, 1080, 2400, &actions),
            Some(OrbAction::OpenBluetooth)
        );
    }

    #[test]
    fn orb_action_wire_and_parse_round_trip_for_every_variant() {
        for action in [
            OrbAction::Toggle,
            OrbAction::OpenInbox,
            OrbAction::OpenIntent,
            OrbAction::OpenBluetooth,
        ] {
            assert_eq!(OrbAction::parse(action.wire()), Some(action));
        }
        assert_eq!(OrbAction::parse("not-a-real-orb-action"), None);
    }

    #[test]
    fn orb_menu_actions_differs_between_two_real_context_frames() {
        // HIA-05's own acceptance line (HIA-ROADMAP.md): at least two
        // different action sets for two different real ContextFrames.
        // A user space with no paired Bluetooth device is the
        // baseline (Входящие + Новое намерение, the same fixed pair
        // HIA-04b shipped); the system space drops "Новое намерение"
        // (intents don't belong there); a paired Bluetooth device
        // adds a third action regardless of which space. All three
        // are genuinely different lists, not the same one relabeled.
        let home = orb_menu_actions(false, false);
        let system_space = orb_menu_actions(true, false);
        let home_with_bluetooth = orb_menu_actions(false, true);

        assert_eq!(home, vec![OrbAction::OpenInbox, OrbAction::OpenIntent]);
        assert_eq!(system_space, vec![OrbAction::OpenInbox]);
        assert_eq!(
            home_with_bluetooth,
            vec![
                OrbAction::OpenInbox,
                OrbAction::OpenIntent,
                OrbAction::OpenBluetooth
            ]
        );

        assert_ne!(home, system_space);
        assert_ne!(home, home_with_bluetooth);
    }

    #[test]
    fn orb_menu_actions_always_keeps_open_inbox() {
        for is_system_space in [false, true] {
            for bluetooth_paired in [false, true] {
                assert!(orb_menu_actions(is_system_space, bluetooth_paired)
                    .contains(&OrbAction::OpenInbox));
            }
        }
    }

    #[test]
    fn me_fixed_card_action_index_1_is_the_silent_build_info_tap() {
        // HIA-20's own hidden entry point -- confirm it sits at
        // exactly the index the build-id card occupies in `me_all_
        // card_views`, not silently lost to some future edit there.
        assert_eq!(me_fixed_card_action(1), Some("tap_build_info"));
    }

    #[test]
    fn dev_surface_back_tapped_finds_only_the_row_right_after_the_data() {
        let width = 1080;
        let height = 2400;
        let row_count = 3;
        let center = |rect: Rect| {
            (
                (rect.x + rect.width / 2) as f64,
                (rect.y + rect.height / 2) as f64,
            )
        };
        let data_row = center(stacked_row_rect(1, width, height));
        let back_row = center(stacked_row_rect(row_count, width, height));
        assert!(!dev_surface_back_tapped(data_row, width, height, row_count));
        assert!(dev_surface_back_tapped(back_row, width, height, row_count));
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
