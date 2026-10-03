//! Input is device-agnostic. A swipe, a wheel notch, a stick deflection, a
//! trackpad scroll and a physical wheel detent are all the same thing — a
//! rotation of the sphere — and are lowered to one `Motion` here. The shell
//! only translates its raw events into `NavInput`.

/// One mouse-wheel notch or physical-wheel detent, in drag-equivalent pixels
/// per unit. Chosen so a notch is a comfortable fraction of a neighbour gap.
pub const WHEEL_NOTCH_UNITS: f32 = 32.0;
/// Full stick deflection, in units per second.
pub const STICK_UNITS_PER_S: f32 = 300.0;
pub const STICK_DEADZONE: f32 = 0.15;
/// A notch of zoom wheel scales by this much.
pub const ZOOM_NOTCH: f32 = 1.12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelAxis {
    Horizontal,
    Vertical,
    Zoom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavInput {
    /// Finger, pen, mouse drag or trackpad pan, in pixels.
    Drag { dx: f32, dy: f32 },
    /// Wheel notches or detents; fractional for smooth wheels.
    Wheel { steps: f32, axis: WheelAxis },
    /// Analog stick, each axis `-1..=1`, held for `dt` seconds.
    Stick { x: f32, y: f32, dt: f32 },
    /// Two-finger pinch; `scale` is the ratio since the last event.
    Pinch { scale: f32, ax: f32, ay: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Motion {
    /// Move the surface by this many pixels, as if dragged.
    Pan {
        dx: f32,
        dy: f32,
    },
    Zoom {
        factor: f32,
        ax: Option<f32>,
        ay: Option<f32>,
    },
    None,
}

fn finite(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

fn deadzone(v: f32) -> f32 {
    let v = finite(v).clamp(-1.0, 1.0);
    let a = v.abs();
    if a <= STICK_DEADZONE {
        0.0
    } else {
        v.signum() * (a - STICK_DEADZONE) / (1.0 - STICK_DEADZONE)
    }
}

/// `unit` is pixels per design unit, so physical size of a step is the same
/// on every display.
pub fn lower(input: NavInput, unit: f32) -> Motion {
    let u = if unit.is_finite() && unit > 0.0 {
        unit
    } else {
        1.0
    };
    match input {
        NavInput::Drag { dx, dy } => Motion::Pan {
            dx: finite(dx),
            dy: finite(dy),
        },
        NavInput::Wheel { steps, axis } => {
            let s = finite(steps);
            match axis {
                WheelAxis::Horizontal => Motion::Pan {
                    dx: s * WHEEL_NOTCH_UNITS * u,
                    dy: 0.0,
                },
                WheelAxis::Vertical => Motion::Pan {
                    dx: 0.0,
                    dy: s * WHEEL_NOTCH_UNITS * u,
                },
                WheelAxis::Zoom => Motion::Zoom {
                    factor: ZOOM_NOTCH.powf(s),
                    ax: None,
                    ay: None,
                },
            }
        }
        NavInput::Stick { x, y, dt } => {
            let dt = finite(dt).clamp(0.0, 0.25);
            Motion::Pan {
                dx: deadzone(x) * STICK_UNITS_PER_S * u * dt,
                dy: deadzone(y) * STICK_UNITS_PER_S * u * dt,
            }
        }
        NavInput::Pinch { scale, ax, ay } => {
            let s = finite(scale);
            if s <= 0.0 {
                Motion::None
            } else {
                Motion::Zoom {
                    factor: s,
                    ax: Some(finite(ax)),
                    ay: Some(finite(ay)),
                }
            }
        }
    }
}

/// Flick momentum after the finger lifts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Inertia {
    vx: f32,
    vy: f32,
}

const FRICTION_PER_S: f32 = 4.0;
const STOP_SPEED: f32 = 24.0;
const MAX_FLING_SPEED: f32 = 6000.0;

impl Inertia {
    pub fn fling(vx: f32, vy: f32, unit: f32) -> Inertia {
        let cap = MAX_FLING_SPEED * unit.max(0.1);
        let (vx, vy) = (finite(vx), finite(vy));
        let speed = vx.hypot(vy);
        let k = if speed > cap { cap / speed } else { 1.0 };
        let mut i = Inertia {
            vx: vx * k,
            vy: vy * k,
        };
        if i.vx.hypot(i.vy) < STOP_SPEED * unit.max(0.1) {
            i = Inertia::default();
        }
        i
    }

    pub fn is_moving(&self) -> bool {
        self.vx != 0.0 || self.vy != 0.0
    }

    pub fn stop(&mut self) {
        *self = Inertia::default();
    }

    /// Displacement for this frame, or `None` once the glide has ended.
    pub fn step(&mut self, dt: f32, unit: f32) -> Option<(f32, f32)> {
        if !self.is_moving() {
            return None;
        }
        let dt = finite(dt).clamp(0.0, 0.1);
        let d = (self.vx * dt, self.vy * dt);
        let k = (-FRICTION_PER_S * dt).exp();
        self.vx *= k;
        self.vy *= k;
        if self.vx.hypot(self.vy) < STOP_SPEED * unit.max(0.1) {
            self.stop();
        }
        Some(d)
    }
}

/// The rise of the sphere: the one parameter that takes Orb from a small
/// exposed point to the whole navigation space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Presence {
    value: f32,
    target: f32,
    pull: Option<Pull>,
    pub reduced_motion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Pull {
    start_y: f32,
    start_value: f32,
}

const RISE_RATE: f32 = 11.0;
const COMMIT_AT: f32 = 0.35;
const COMMIT_SPEED: f32 = 700.0;

impl Presence {
    pub fn resting() -> Presence {
        Presence {
            value: 0.0,
            target: 0.0,
            pull: None,
            reduced_motion: false,
        }
    }

    pub fn rise(&self) -> f32 {
        self.value
    }

    /// Any part of the sphere above the bottom edge needs frames.
    pub fn is_visible(&self) -> bool {
        self.value > 0.0 || self.target > 0.0
    }

    pub fn is_open(&self) -> bool {
        self.target >= 1.0 && self.value >= 0.999
    }

    pub fn wants_open(&self) -> bool {
        self.target > 0.0
    }

    pub fn is_pulling(&self) -> bool {
        self.pull.is_some()
    }

    pub fn needs_frames(&self) -> bool {
        self.pull.is_none() && (self.value - self.target).abs() > f32::EPSILON
    }

    pub fn summon(&mut self) {
        self.pull = None;
        self.target = 1.0;
        if self.reduced_motion {
            self.value = 1.0;
        }
    }

    pub fn dismiss(&mut self) {
        self.pull = None;
        self.target = 0.0;
        if self.reduced_motion {
            self.value = 0.0;
        }
    }

    pub fn begin_pull(&mut self, y: f32) {
        self.pull = Some(Pull {
            start_y: finite(y),
            start_value: self.value,
        });
    }

    /// The sphere follows the finger: moving up by `travel` raises it fully.
    pub fn pull_to(&mut self, y: f32, travel: f32) {
        if let Some(p) = self.pull {
            let t = travel.max(1.0);
            let v = p.start_value + (p.start_y - finite(y)) / t;
            self.value = v.clamp(0.0, 1.0);
            self.target = self.value;
        }
    }

    /// `vy` is the vertical velocity in px/s (negative = upward).
    pub fn release(&mut self, vy: f32) {
        if self.pull.take().is_none() {
            return;
        }
        let vy = finite(vy);
        let open = vy < -COMMIT_SPEED || (vy <= COMMIT_SPEED && self.value >= COMMIT_AT);
        if open {
            self.summon();
        } else {
            self.dismiss();
        }
    }

    pub fn step(&mut self, dt: f32) {
        if self.pull.is_some() || !dt.is_finite() {
            return;
        }
        if self.reduced_motion {
            self.value = self.target;
            return;
        }
        let k = 1.0 - (-RISE_RATE * dt.clamp(0.0, 0.1)).exp();
        self.value += (self.target - self.value) * k;
        if (self.target - self.value).abs() < 0.002 {
            self.value = self.target;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pan(m: Motion) -> (f32, f32) {
        match m {
            Motion::Pan { dx, dy } => (dx, dy),
            other => panic!("not a pan: {other:?}"),
        }
    }

    #[test]
    fn every_device_lowers_to_the_same_motion() {
        let u = 2.0;
        let travel = WHEEL_NOTCH_UNITS * u * 3.0;
        let swipe = pan(lower(
            NavInput::Drag {
                dx: 0.0,
                dy: travel,
            },
            u,
        ));
        let wheel = pan(lower(
            NavInput::Wheel {
                steps: 3.0,
                axis: WheelAxis::Vertical,
            },
            u,
        ));
        assert_eq!(swipe, wheel);
        let full_deflection_for = travel / (STICK_UNITS_PER_S * u);
        let mut sum = (0.0, 0.0);
        let frames = 10;
        for _ in 0..frames {
            let (dx, dy) = pan(lower(
                NavInput::Stick {
                    x: 0.0,
                    y: 1.0,
                    dt: full_deflection_for / frames as f32,
                },
                u,
            ));
            sum = (sum.0 + dx, sum.1 + dy);
        }
        assert!((sum.1 - travel).abs() < 0.01 && sum.0 == 0.0);
        let h = pan(lower(
            NavInput::Wheel {
                steps: -2.0,
                axis: WheelAxis::Horizontal,
            },
            u,
        ));
        assert_eq!(h, (-2.0 * WHEEL_NOTCH_UNITS * u, 0.0));
    }

    #[test]
    fn stick_has_a_deadzone_and_garbage_does_not_move_anything() {
        assert_eq!(
            lower(
                NavInput::Stick {
                    x: 0.1,
                    y: -0.14,
                    dt: 0.016
                },
                1.0
            ),
            Motion::Pan { dx: 0.0, dy: 0.0 }
        );
        let (dx, _) = pan(lower(
            NavInput::Stick {
                x: 1.0,
                y: 0.0,
                dt: 0.1,
            },
            1.0,
        ));
        assert!((dx - STICK_UNITS_PER_S * 0.1).abs() < 1e-3);
        assert_eq!(
            pan(lower(
                NavInput::Drag {
                    dx: f32::NAN,
                    dy: f32::INFINITY
                },
                1.0
            )),
            (0.0, 0.0)
        );
        assert_eq!(
            lower(
                NavInput::Pinch {
                    scale: -1.0,
                    ax: 0.0,
                    ay: 0.0
                },
                1.0
            ),
            Motion::None
        );
        let (dx, _) = pan(lower(
            NavInput::Stick {
                x: 1.0,
                y: 0.0,
                dt: 99.0,
            },
            1.0,
        ));
        assert!(
            dx <= STICK_UNITS_PER_S * 0.25 + 1e-3,
            "a stalled frame cannot teleport"
        );
    }

    #[test]
    fn zoom_wheel_and_pinch_are_both_zoom() {
        let Motion::Zoom { factor, .. } = lower(
            NavInput::Wheel {
                steps: 2.0,
                axis: WheelAxis::Zoom,
            },
            1.0,
        ) else {
            panic!()
        };
        assert!((factor - ZOOM_NOTCH * ZOOM_NOTCH).abs() < 1e-5);
        assert!(matches!(
            lower(NavInput::Pinch { scale: 1.2, ax: 5.0, ay: 6.0 }, 1.0),
            Motion::Zoom { ax: Some(a), ay: Some(b), .. } if a == 5.0 && b == 6.0
        ));
    }

    #[test]
    fn inertia_glides_decays_and_ends() {
        let mut i = Inertia::fling(0.0, 1200.0, 1.0);
        assert!(i.is_moving());
        let mut total = 0.0;
        let mut frames = 0;
        let mut last = f32::MAX;
        while let Some((_, dy)) = i.step(1.0 / 60.0, 1.0) {
            assert!(dy <= last + 1e-4);
            last = dy;
            total += dy;
            frames += 1;
            assert!(frames < 600, "must terminate");
        }
        assert!(total > 100.0 && total < 400.0, "{total}");
        assert!(!i.is_moving());
        assert!(
            !Inertia::fling(3.0, 2.0, 1.0).is_moving(),
            "a slow lift is not a fling"
        );
        let capped = Inertia::fling(1e9, 0.0, 1.0);
        assert!(capped.vx <= MAX_FLING_SPEED + 1.0);
    }

    #[test]
    fn rising_follows_the_finger_and_commits_by_distance_or_speed() {
        let mut p = Presence::resting();
        assert!(!p.is_visible());
        p.begin_pull(2380.0);
        p.pull_to(2380.0 - 300.0, 600.0);
        assert!((p.rise() - 0.5).abs() < 1e-5);
        p.release(0.0);
        assert!(p.wants_open());

        let mut q = Presence::resting();
        q.begin_pull(2380.0);
        q.pull_to(2380.0 - 90.0, 600.0);
        q.release(0.0);
        assert!(!q.wants_open(), "a short pull settles back");

        let mut f = Presence::resting();
        f.begin_pull(2380.0);
        f.pull_to(2380.0 - 60.0, 600.0);
        f.release(-1500.0);
        assert!(f.wants_open(), "a quick flick opens it");

        let mut d = Presence::resting();
        d.begin_pull(2380.0);
        d.pull_to(2380.0 - 400.0, 600.0);
        d.release(1500.0);
        assert!(!d.wants_open(), "a quick flick down closes it");
    }

    #[test]
    fn rise_converges_without_overshoot_and_reduced_motion_snaps() {
        let mut p = Presence::resting();
        p.summon();
        let mut last = 0.0;
        for _ in 0..120 {
            p.step(1.0 / 60.0);
            assert!(p.rise() >= last && p.rise() <= 1.0);
            last = p.rise();
        }
        assert!(p.is_open() && !p.needs_frames());
        p.dismiss();
        for _ in 0..120 {
            p.step(1.0 / 60.0);
        }
        assert_eq!(p.rise(), 0.0);
        assert!(!p.is_visible());

        let mut r = Presence::resting();
        r.reduced_motion = true;
        r.summon();
        assert_eq!(r.rise(), 1.0);
        r.dismiss();
        assert_eq!(r.rise(), 0.0);
    }

    #[test]
    fn pulling_back_down_below_the_start_cannot_go_negative_and_release_without_pull_is_inert() {
        let mut p = Presence::resting();
        p.release(-9999.0);
        assert!(!p.wants_open());
        p.begin_pull(100.0);
        p.pull_to(5000.0, 600.0);
        assert_eq!(p.rise(), 0.0);
        p.step(0.016);
        assert_eq!(p.rise(), 0.0);
    }
}
