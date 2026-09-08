//! Touch input for the Pixel 7 panel, read directly from
//! `/dev/input/touchscreen` as raw evdev events -- no libinput, no libudev.
//!
//! libinput's path-based interface (the obvious first choice, matching this
//! module's original design) turns out to still require the kernel device
//! to be "initialized" in udev's sense: `libinput_path_add_device()` wraps
//! the path in a `udev_device` and checks `udev_device_get_is_initialized()`,
//! which only ever becomes true once a running `udevd` has processed the
//! device's uevent and written its properties to `/run/udev/data/`. This
//! system runs no udev daemon at all (no build in this sysroot even
//! produces a `udevd` binary, only the library) -- adding one back, even
//! transiently, would be new daemon infrastructure contradicting the
//! single-owner, no-session-manager design already settled in ADR-009 and
//! ADR-010. See ADR-011.
//!
//! `drm-splash.c` already reads this exact touchscreen the same way this
//! module does: raw `struct input_event` records, tracking
//! `ABS_MT_TRACKING_ID`/`ABS_MT_POSITION_X`/`ABS_MT_POSITION_Y` per
//! `SYN_REPORT` frame. Notably it does no coordinate scaling at all (its
//! `design_x = x * 1080 / width` is an identity given `width` is already
//! 1080) -- this touchscreen reports coordinates natively in the panel's
//! own 1080x2400 pixel space, so this module doesn't scale either.

use std::fs::File;
use std::os::unix::fs::OpenOptionsExt;

pub const TOUCHSCREEN_PATH: &str = "/dev/input/touchscreen";

// O_NONBLOCK's value on Linux (stable across architectures) -- avoids
// pulling in libc for one constant, matching this crate's existing
// preference for minimal dependencies (ADR-011).
const O_NONBLOCK: i32 = 0o4000;

const EV_SYN: u16 = 0x00;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;

/// Matches the kernel's `struct input_event` layout on a 64-bit target:
/// `struct timeval time` (2x 64-bit) + `__u16 type` + `__u16 code` +
/// `__s32 value` -- 24 bytes, no padding (already 8-byte aligned).
#[repr(C)]
pub struct RawEvent {
    _tv_sec: i64,
    _tv_usec: i64,
    ev_type: u16,
    code: u16,
    value: i32,
}

pub const RAW_EVENT_SIZE: usize = std::mem::size_of::<RawEvent>();

pub fn parse(bytes: [u8; RAW_EVENT_SIZE]) -> RawEvent {
    // RawEvent is a plain repr(C) bag of integers -- any 24-byte pattern is
    // a valid value, so this transmute can't produce UB regardless of what
    // the kernel actually wrote.
    unsafe { std::mem::transmute(bytes) }
}

pub enum TouchUpdate {
    Down { x: i32, y: i32 },
    Motion { x: i32, y: i32 },
    Up,
}

/// One touch point's frame-to-frame state, matching drm-splash.c's own
/// single-tracking-id simplification (this panel's protocol-B stream never
/// carries more than one real slot in practice).
pub struct TouchState {
    tracking_id: i32,
    x: i32,
    y: i32,
    moved: bool,
    /// Whether a touch was active as of the previous `SYN_REPORT`.
    was_active: bool,
}

impl TouchState {
    pub fn new() -> Self {
        Self {
            tracking_id: -1,
            x: 0,
            y: 0,
            moved: false,
            was_active: false,
        }
    }

    /// Feeds one raw evdev event in; returns a synthesized update only on
    /// the `SYN_REPORT` that actually crosses a down/motion/up transition.
    pub fn feed(&mut self, event: &RawEvent) -> Option<TouchUpdate> {
        match event.ev_type {
            EV_ABS => {
                match event.code {
                    ABS_MT_TRACKING_ID => self.tracking_id = event.value,
                    ABS_MT_POSITION_X => {
                        self.x = event.value;
                        self.moved = true;
                    }
                    ABS_MT_POSITION_Y => {
                        self.y = event.value;
                        self.moved = true;
                    }
                    _ => {}
                }
                None
            }
            EV_SYN if event.code == SYN_REPORT => {
                let was_active = self.was_active;
                let now_active = self.tracking_id >= 0;
                self.was_active = now_active;
                let moved = std::mem::take(&mut self.moved);
                if now_active && !was_active {
                    Some(TouchUpdate::Down {
                        x: self.x,
                        y: self.y,
                    })
                } else if now_active && moved {
                    Some(TouchUpdate::Motion {
                        x: self.x,
                        y: self.y,
                    })
                } else if !now_active && was_active {
                    Some(TouchUpdate::Up)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

pub fn open() -> Result<File, String> {
    // Must be non-blocking: main.rs registers this fd with calloop as
    // `Interest::READ, Mode::Level` and then drains it in a loop until
    // `read()` returns anything other than a full `RawEvent` -- on a
    // *blocking* fd, once the currently-buffered events are drained,
    // that next `read()` call doesn't return "no more data right now",
    // it just blocks the single-threaded event loop until the next
    // physical touch arrives. That starves every other event source
    // (including the flush that actually delivers queued protocol
    // messages to clients) for the rest of the process's life, the
    // first time any touch ever happens -- this was a real bug, not a
    // hypothetical one: confirmed on hardware as the root cause of
    // wl_touch events never reaching any client (S04 sprint doc, Change
    // step 5 known limitations, has the full diagnostic trail).
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open(TOUCHSCREEN_PATH)
        .map_err(|e| format!("failed to open {TOUCHSCREEN_PATH}: {e}"))
}
