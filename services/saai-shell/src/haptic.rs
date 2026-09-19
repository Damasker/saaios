//! Haptic policy for ADR-151 / ADR-171.
//!
//! Components name a `HapticEvent`. Policy returns a `HapticIntent`.
//! Only `HapticMotor` talks to `/dev/input/haptic`. Displayd still
//! has no haptic protocol (S04). Missing device is a silent no-op
//! so host tests do not need the Pixel motor.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::time::Instant;

/// Who asked. Not a waveform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HapticEvent {
    KeyPress,
    TabPress,
    OrbActivity,
}

/// Product meaning, not a waveform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HapticIntent {
    KeyTick,
}

/// Uploaded FF replay length. A second play inside this window overlaps.
pub const KEY_TICK_REPLAY_MS: u16 = 15;

pub fn haptic_intent_for(event: HapticEvent, enabled: bool) -> Option<HapticIntent> {
    if !enabled {
        return None;
    }
    match event {
        HapticEvent::KeyPress => Some(HapticIntent::KeyTick),
        HapticEvent::TabPress | HapticEvent::OrbActivity => None,
    }
}

pub fn haptic_rate_limit_allows(elapsed_since_last_ms: u32) -> bool {
    elapsed_since_last_ms >= u32::from(KEY_TICK_REPLAY_MS)
}

pub struct HapticMotor {
    inner: Option<LinuxHaptic>,
    last_play: Option<Instant>,
}

struct LinuxHaptic {
    file: File,
    effect_id: u16,
    #[allow(dead_code)]
    _custom: [i16; 2],
}

impl HapticMotor {
    pub fn open() -> Self {
        Self {
            inner: LinuxHaptic::open(),
            last_play: None,
        }
    }

    pub fn play(&mut self, intent: HapticIntent) {
        let HapticIntent::KeyTick = intent;
        let now = Instant::now();
        if let Some(last) = self.last_play {
            if !haptic_rate_limit_allows(now.saturating_duration_since(last).as_millis() as u32) {
                return;
            }
        }
        if let Some(inner) = self.inner.as_mut() {
            inner.play();
        }
        self.last_play = Some(now);
    }
}

#[repr(C)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
struct InputEvent {
    time: TimeVal,
    type_: u16,
    code: u16,
    value: i32,
}

#[repr(C)]
struct FfReplay {
    length: u16,
    delay: u16,
}

#[repr(C)]
struct FfTrigger {
    button: u16,
    interval: u16,
}

#[repr(C)]
struct FfEnvelope {
    attack_length: u16,
    attack_level: u16,
    fade_length: u16,
    fade_level: u16,
}

#[repr(C)]
struct FfPeriodic {
    waveform: u16,
    period: u16,
    magnitude: i16,
    offset: i16,
    phase: u16,
    envelope: FfEnvelope,
    custom_len: u32,
    custom_data: *mut i16,
}

#[repr(C)]
union FfEffectBody {
    periodic: std::mem::ManuallyDrop<FfPeriodic>,
}

#[repr(C)]
struct FfEffect {
    type_: u16,
    id: i16,
    direction: u16,
    trigger: FfTrigger,
    replay: FfReplay,
    u: FfEffectBody,
}

const EV_FF: u16 = 0x15;
const FF_PERIODIC: u16 = 0x51;
const FF_CUSTOM: u16 = 0x5d;
const FF_GAIN: u16 = 0x60;
const EVIOCSFF: libc::c_ulong = {
    const WRITE: u64 = 1;
    const NR: u64 = 0x80;
    const TYP: u64 = b'E' as u64;
    const SIZE: u64 = std::mem::size_of::<FfEffect>() as u64;
    (WRITE << 30) | (TYP << 8) | NR | (SIZE << 16)
};

const _: () = assert!(std::mem::size_of::<FfEffect>() == 48);
const _: () = assert!(std::mem::size_of::<InputEvent>() == 24);

impl LinuxHaptic {
    fn open() -> Option<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/input/haptic")
            .ok()?;
        let mut custom = [0i16, 9];
        let mut effect = FfEffect {
            type_: FF_PERIODIC,
            id: -1,
            direction: 0,
            trigger: FfTrigger {
                button: 0,
                interval: 0,
            },
            replay: FfReplay {
                length: 15,
                delay: 0,
            },
            u: FfEffectBody {
                periodic: std::mem::ManuallyDrop::new(FfPeriodic {
                    waveform: FF_CUSTOM,
                    period: 0,
                    magnitude: 0,
                    offset: 0,
                    phase: 0,
                    envelope: FfEnvelope {
                        attack_length: 0,
                        attack_level: 0,
                        fade_length: 0,
                        fade_level: 0,
                    },
                    custom_len: 2,
                    custom_data: custom.as_mut_ptr(),
                }),
            },
        };
        let rc = unsafe { libc::ioctl(file.as_raw_fd(), EVIOCSFF as _, &mut effect) };
        if rc < 0 {
            eprintln!(
                "saai-shell: haptic upload failed: {}",
                std::io::Error::last_os_error()
            );
            return None;
        }
        let gain = InputEvent {
            time: TimeVal {
                tv_sec: 0,
                tv_usec: 0,
            },
            type_: EV_FF,
            code: FF_GAIN,
            value: 30,
        };
        let gain_bytes = unsafe {
            std::slice::from_raw_parts(
                (&gain as *const InputEvent) as *const u8,
                std::mem::size_of::<InputEvent>(),
            )
        };
        let mut file = file;
        if file.write_all(gain_bytes).is_err() {
            eprintln!(
                "saai-shell: haptic gain failed: {}",
                std::io::Error::last_os_error()
            );
        }
        Some(Self {
            file,
            effect_id: effect.id as u16,
            _custom: custom,
        })
    }

    fn play(&mut self) {
        let event = InputEvent {
            time: TimeVal {
                tv_sec: 0,
                tv_usec: 0,
            },
            type_: EV_FF,
            code: self.effect_id,
            value: 1,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (&event as *const InputEvent) as *const u8,
                std::mem::size_of::<InputEvent>(),
            )
        };
        if self.file.write_all(bytes).is_err() {
            eprintln!(
                "saai-shell: haptic playback failed: {}",
                std::io::Error::last_os_error()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        haptic_intent_for, haptic_rate_limit_allows, HapticEvent, HapticIntent, KEY_TICK_REPLAY_MS,
    };

    #[test]
    fn key_press_requests_a_key_tick_when_haptics_are_on() {
        assert_eq!(
            haptic_intent_for(HapticEvent::KeyPress, true),
            Some(HapticIntent::KeyTick)
        );
        assert_eq!(haptic_intent_for(HapticEvent::KeyPress, false), None);
    }

    #[test]
    fn tabs_and_orb_do_not_own_a_tick() {
        assert_eq!(haptic_intent_for(HapticEvent::TabPress, true), None);
        assert_eq!(haptic_intent_for(HapticEvent::OrbActivity, true), None);
    }

    #[test]
    fn rate_limit_matches_the_uploaded_replay() {
        assert_eq!(KEY_TICK_REPLAY_MS, 15);
        assert!(!haptic_rate_limit_allows(0));
        assert!(!haptic_rate_limit_allows(14));
        assert!(haptic_rate_limit_allows(15));
        assert!(haptic_rate_limit_allows(120));
    }
}
