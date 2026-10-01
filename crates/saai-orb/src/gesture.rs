//! Touch tracking that turns raw pointer events into the few things Orb
//! cares about: a tap, a pan, a pinch, and the velocity at release. Pure:
//! the caller supplies positions and timestamps in seconds.

use std::collections::VecDeque;

const VELOCITY_WINDOW_S: f32 = 0.1;
const MAX_SAMPLES: usize = 12;
/// A touch that stays within this many units and ends quickly is a tap.
pub const TAP_SLOP_UNITS: f32 = 10.0;
pub const TAP_MAX_S: f32 = 0.45;

#[derive(Debug, Clone, Default)]
pub struct VelocityTracker {
    samples: VecDeque<(f32, f32, f32)>,
}

impl VelocityTracker {
    pub fn reset(&mut self) {
        self.samples.clear();
    }

    pub fn push(&mut self, t: f32, x: f32, y: f32) {
        if !(t.is_finite() && x.is_finite() && y.is_finite()) {
            return;
        }
        self.samples.push_back((t, x, y));
        while self.samples.len() > MAX_SAMPLES {
            self.samples.pop_front();
        }
        while self
            .samples
            .front()
            .is_some_and(|s| t - s.0 > VELOCITY_WINDOW_S)
        {
            self.samples.pop_front();
        }
    }

    /// Pixels per second over the last ~100 ms; zero when the finger rested.
    pub fn velocity(&self, now: f32) -> (f32, f32) {
        let (Some(first), Some(last)) = (self.samples.front(), self.samples.back()) else {
            return (0.0, 0.0);
        };
        if now - last.0 > VELOCITY_WINDOW_S {
            return (0.0, 0.0);
        }
        let dt = last.0 - first.0;
        if dt < 0.008 {
            return (0.0, 0.0);
        }
        ((last.1 - first.1) / dt, (last.2 - first.2) / dt)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Delta {
    Pan { dx: f32, dy: f32 },
    Pinch { scale: f32, ax: f32, ay: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lift {
    /// A short touch that barely moved.
    Tap { x: f32, y: f32 },
    /// The last finger left after a drag; velocity in px/s.
    Release { vx: f32, vy: f32 },
    /// A finger left but others remain, or the touch was unknown.
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Pointer {
    id: i32,
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Pointers {
    active: Vec<Pointer>,
    start: Option<(f32, f32, f32)>,
    travelled: f32,
    multi: bool,
    velocity: VelocityTracker,
}

impl Pointers {
    pub fn is_down(&self) -> bool {
        !self.active.is_empty()
    }

    pub fn start(&self) -> Option<(f32, f32)> {
        self.start.map(|(x, y, _)| (x, y))
    }

    pub fn travelled(&self) -> f32 {
        self.travelled
    }

    pub fn velocity_now(&self, now: f32) -> (f32, f32) {
        self.velocity.velocity(now)
    }

    pub fn down(&mut self, id: i32, x: f32, y: f32, t: f32) {
        if self.active.iter().any(|p| p.id == id) {
            return;
        }
        if self.active.is_empty() {
            self.start = Some((x, y, t));
            self.travelled = 0.0;
            self.multi = false;
            self.velocity.reset();
            self.velocity.push(t, x, y);
        } else {
            self.multi = true;
        }
        self.active.push(Pointer { id, x, y });
    }

    pub fn motion(&mut self, id: i32, x: f32, y: f32, t: f32) -> Option<Delta> {
        if !(x.is_finite() && y.is_finite()) {
            return None;
        }
        let i = self.active.iter().position(|p| p.id == id)?;
        let before = self.active[i];
        if self.active.len() >= 2 {
            let other = *self.active.iter().find(|p| p.id != id)?;
            let d0 = (before.x - other.x).hypot(before.y - other.y);
            self.active[i].x = x;
            self.active[i].y = y;
            let d1 = (x - other.x).hypot(y - other.y);
            self.travelled += (x - before.x).hypot(y - before.y);
            if d0 < 1.0 || d1 < 1.0 {
                return None;
            }
            return Some(Delta::Pinch {
                scale: d1 / d0,
                ax: (x + other.x) * 0.5,
                ay: (y + other.y) * 0.5,
            });
        }
        let (dx, dy) = (x - before.x, y - before.y);
        self.active[i].x = x;
        self.active[i].y = y;
        self.travelled += dx.hypot(dy);
        self.velocity.push(t, x, y);
        if self.multi {
            return None;
        }
        (dx != 0.0 || dy != 0.0).then_some(Delta::Pan { dx, dy })
    }

    pub fn up(&mut self, id: i32, t: f32, slop: f32) -> Lift {
        let Some(i) = self.active.iter().position(|p| p.id == id) else {
            return Lift::Ignored;
        };
        let lifted = self.active.remove(i);
        if !self.active.is_empty() {
            return Lift::Ignored;
        }
        let (sx, sy, st) = self.start.take().unwrap_or((lifted.x, lifted.y, t));
        let was_multi = self.multi;
        self.multi = false;
        let moved = (lifted.x - sx).hypot(lifted.y - sy);
        if !was_multi && moved <= slop && self.travelled <= slop * 2.0 && t - st <= TAP_MAX_S {
            return Lift::Tap {
                x: lifted.x,
                y: lifted.y,
            };
        }
        if was_multi {
            return Lift::Ignored;
        }
        let (vx, vy) = self.velocity.velocity(t);
        Lift::Release { vx, vy }
    }

    pub fn cancel(&mut self) {
        self.active.clear();
        self.start = None;
        self.multi = false;
        self.travelled = 0.0;
        self.velocity.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_still_touch_is_a_tap_at_the_lift_point() {
        let mut p = Pointers::default();
        p.down(1, 100.0, 200.0, 0.0);
        assert_eq!(p.motion(1, 102.0, 201.0, 0.05), Some(Delta::Pan { dx: 2.0, dy: 1.0 }));
        assert_eq!(p.up(1, 0.1, 30.0), Lift::Tap { x: 102.0, y: 201.0 });
        assert!(!p.is_down());
    }

    #[test]
    fn a_long_press_or_a_drag_is_not_a_tap() {
        let mut p = Pointers::default();
        p.down(1, 0.0, 0.0, 0.0);
        assert!(matches!(p.up(1, 1.0, 30.0), Lift::Release { .. }));
        p.down(1, 0.0, 0.0, 2.0);
        p.motion(1, 200.0, 0.0, 2.05);
        assert!(matches!(p.up(1, 2.1, 30.0), Lift::Release { .. }));
    }

    #[test]
    fn jitter_that_wanders_off_and_back_is_not_a_tap() {
        let mut p = Pointers::default();
        p.down(1, 0.0, 0.0, 0.0);
        p.motion(1, 60.0, 0.0, 0.05);
        p.motion(1, 0.0, 0.0, 0.1);
        assert!(matches!(p.up(1, 0.15, 30.0), Lift::Release { .. }));
    }

    #[test]
    fn release_velocity_reflects_the_last_moments() {
        let mut p = Pointers::default();
        p.down(1, 0.0, 1000.0, 0.0);
        let mut y = 1000.0;
        let mut t = 0.0;
        for _ in 0..8 {
            t += 0.016;
            y -= 16.0;
            p.motion(1, 0.0, y, t);
        }
        let Lift::Release { vx, vy } = p.up(1, t, 10.0) else {
            panic!("expected release");
        };
        assert!(vx.abs() < 1.0);
        assert!((vy + 1000.0).abs() < 60.0, "{vy}");
    }

    #[test]
    fn resting_before_lift_means_no_fling() {
        let mut p = Pointers::default();
        p.down(1, 0.0, 0.0, 0.0);
        p.motion(1, 0.0, 300.0, 0.05);
        let Lift::Release { vx, vy } = p.up(1, 0.6, 10.0) else {
            panic!()
        };
        assert_eq!((vx, vy), (0.0, 0.0));
    }

    #[test]
    fn two_fingers_pinch_with_the_ratio_of_distances_and_never_pan() {
        let mut p = Pointers::default();
        p.down(1, 100.0, 500.0, 0.0);
        p.down(2, 300.0, 500.0, 0.0);
        let d = p.motion(2, 500.0, 500.0, 0.05).unwrap();
        let Delta::Pinch { scale, ax, ay } = d else {
            panic!("{d:?}")
        };
        assert!((scale - 2.0).abs() < 1e-5);
        assert!((ax - 300.0).abs() < 1e-3 && (ay - 500.0).abs() < 1e-3);
        let d = p.motion(1, 200.0, 500.0, 0.06).unwrap();
        assert!(matches!(d, Delta::Pinch { scale, .. } if scale < 1.0));
    }

    #[test]
    fn lifting_one_of_two_fingers_does_not_turn_the_other_into_a_pan_or_a_tap() {
        let mut p = Pointers::default();
        p.down(1, 100.0, 500.0, 0.0);
        p.down(2, 300.0, 500.0, 0.0);
        assert_eq!(p.up(2, 0.1, 30.0), Lift::Ignored);
        assert_eq!(p.motion(1, 150.0, 500.0, 0.12), None);
        assert_eq!(p.up(1, 0.15, 30.0), Lift::Ignored);
        assert!(!p.is_down());
    }

    #[test]
    fn unknown_ids_garbage_and_cancel_are_harmless() {
        let mut p = Pointers::default();
        assert_eq!(p.motion(9, 1.0, 1.0, 0.0), None);
        assert_eq!(p.up(9, 0.0, 10.0), Lift::Ignored);
        p.down(1, 0.0, 0.0, 0.0);
        assert_eq!(p.motion(1, f32::NAN, 0.0, 0.1), None);
        p.cancel();
        assert!(!p.is_down());
        assert_eq!(p.up(1, 0.2, 10.0), Lift::Ignored);
    }

    #[test]
    fn pinch_degenerate_distance_is_ignored() {
        let mut p = Pointers::default();
        p.down(1, 100.0, 100.0, 0.0);
        p.down(2, 100.0, 100.0, 0.0);
        assert_eq!(p.motion(2, 150.0, 100.0, 0.05), None);
    }
}
