//! Camera, stage and orthographic projection. The sphere is far larger than
//! the screen; the screen is a window onto it. `rise` moves the sphere from
//! "mostly below the surface" to "filling the window" with one continuous
//! parameter, so there is no separate Orb screen.

use crate::geo::{smoothstep, Geo};
use std::f64::consts::PI;

pub const ZOOM_MIN: f32 = 0.55;
pub const ZOOM_MAX: f32 = 3.2;
pub const MAX_CENTER_LAT: f64 = 75.0 * PI / 180.0;
/// Sphere radius at zoom 1, as a fraction of the larger screen side.
pub const RADIUS_FRACTION: f32 = 0.9;
/// How much of the sphere shows above the bottom edge at rest, in units.
pub const REST_EXPOSURE_UNITS: f32 = 22.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub center: Geo,
    pub zoom: f32,
}

impl Camera {
    pub fn home() -> Camera {
        Camera {
            center: Geo::HOME,
            zoom: 1.0,
        }
    }

    /// Drag by `(dx, dy)` screen pixels: the surface follows the finger.
    pub fn pan_px(&mut self, dx: f32, dy: f32, stage: &Stage) {
        let r = f64::from(stage.radius(self.zoom));
        let dist = f64::from(dx.hypot(dy)) / r;
        if !dist.is_finite() || dist <= 0.0 {
            return;
        }
        let bearing = f64::from(-dx).atan2(f64::from(dy));
        self.center = self
            .center
            .offset(bearing, dist)
            .clamp_lat(MAX_CENTER_LAT);
    }

    pub fn zoom_by(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            self.zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        }
    }

    /// Zoom while the point under `(ax, ay)` stays under it.
    pub fn zoom_about(&mut self, factor: f32, ax: f32, ay: f32, stage: &Stage) {
        let anchor = Projector::new(*self, *stage).unproject(ax, ay);
        self.zoom_by(factor);
        let Some(g) = anchor else { return };
        // Off-centre the surface moves less than at the centre (foreshortening),
        // so a single pan undershoots; a few corrections converge.
        for _ in 0..6 {
            let Some(p) = Projector::new(*self, *stage).project(g) else {
                return;
            };
            let (ex, ey) = (ax - p.x, ay - p.y);
            if ex.hypot(ey) < 0.2 {
                return;
            }
            self.pan_px(ex, ey, stage);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stage {
    pub width: f32,
    pub height: f32,
    /// 0 = resting below the surface, 1 = fully risen.
    pub rise: f32,
    /// Pixels per design unit.
    pub unit: f32,
}

impl Stage {
    pub fn new(width: f32, height: f32, rise: f32, unit: f32) -> Stage {
        Stage {
            width: width.max(1.0),
            height: height.max(1.0),
            rise: rise.clamp(0.0, 1.0),
            unit: if unit.is_finite() && unit > 0.0 { unit } else { 1.0 },
        }
    }

    pub fn radius(&self, zoom: f32) -> f32 {
        RADIUS_FRACTION * self.width.max(self.height) * zoom
    }

    /// Centre of the projected disc: below the screen at rest, in the middle
    /// of it when risen.
    pub fn disc_center(&self, zoom: f32) -> (f32, f32) {
        let r = self.radius(zoom);
        let rest_y = self.height + r - REST_EXPOSURE_UNITS * self.unit;
        let mid_y = self.height * 0.5;
        let k = smoothstep(0.0, 1.0, self.rise);
        (self.width * 0.5, rest_y + (mid_y - rest_y) * k)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projected {
    pub x: f32,
    pub y: f32,
    /// Cosine of the angle from the view centre: 1 facing us, 0 on the limb.
    pub facing: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Projector {
    camera: Camera,
    r: f64,
    cx: f64,
    cy: f64,
    sin0: f64,
    cos0: f64,
}

impl Projector {
    pub fn new(camera: Camera, stage: Stage) -> Projector {
        let (cx, cy) = stage.disc_center(camera.zoom);
        Projector {
            camera,
            r: f64::from(stage.radius(camera.zoom)),
            cx: f64::from(cx),
            cy: f64::from(cy),
            sin0: camera.center.lat.sin(),
            cos0: camera.center.lat.cos(),
        }
    }

    pub fn disc(&self) -> (f32, f32, f32) {
        (self.cx as f32, self.cy as f32, self.r as f32)
    }

    pub fn project(&self, g: Geo) -> Option<Projected> {
        let dl = g.lon - self.camera.center.lon;
        let (sp, cp) = g.lat.sin_cos();
        let facing = self.sin0 * sp + self.cos0 * cp * dl.cos();
        if facing <= 0.0 {
            return None;
        }
        let x = cp * dl.sin();
        let y = self.cos0 * sp - self.sin0 * cp * dl.cos();
        Some(Projected {
            x: (self.cx + self.r * x) as f32,
            y: (self.cy - self.r * y) as f32,
            facing: facing as f32,
        })
    }

    pub fn unproject(&self, px: f32, py: f32) -> Option<Geo> {
        let x = (f64::from(px) - self.cx) / self.r;
        let y = (self.cy - f64::from(py)) / self.r;
        let rho = x.hypot(y);
        if rho > 1.0 {
            return None;
        }
        if rho < 1e-12 {
            return Some(self.camera.center);
        }
        let c = rho.asin();
        let (sc, cc) = c.sin_cos();
        let lat = (cc * self.sin0 + y * sc * self.cos0 / rho)
            .clamp(-1.0, 1.0)
            .asin();
        let lon = self.camera.center.lon
            + (x * sc).atan2(rho * self.cos0 * cc - y * self.sin0 * sc);
        Some(Geo::new(lon, lat))
    }
}

/// Zoom at which echelon `depth` is fully legible. E1 and E2 sit in the
/// ordinary range; each further echelon needs a stronger zoom.
pub fn depth_threshold(depth: u8) -> f32 {
    match depth {
        0..=1 => 0.0,
        d => 1.15 * 1.9f32.powi(i32::from(d) - 2),
    }
}

/// Semantic zoom: how present echelon `depth` is at `zoom`, 0..1. Deeper
/// echelons fade in as the user closes in and fade out as they open up, so
/// low-level controls vanish when zoomed out.
pub fn detail(depth: u8, zoom: f32) -> f32 {
    let t = depth_threshold(depth);
    if t <= 0.0 {
        return 1.0;
    }
    smoothstep(t * 0.78, t, zoom)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(rise: f32) -> Stage {
        Stage::new(1080.0, 2400.0, rise, 3.0)
    }

    #[test]
    fn the_sphere_is_much_larger_than_the_screen() {
        let s = stage(1.0);
        assert!(s.radius(1.0) * 2.0 > s.height);
        assert!(s.radius(1.0) > s.width);
    }

    #[test]
    fn project_unproject_roundtrip_across_the_cap() {
        let cam = Camera {
            center: Geo::from_degrees(40.0, 25.0),
            zoom: 1.3,
        };
        let p = Projector::new(cam, stage(1.0));
        for (x, y) in [(540.0, 1200.0), (100.0, 300.0), (900.0, 2100.0), (540.0, 50.0)] {
            if let Some(g) = p.unproject(x, y) {
                let q = p.project(g).unwrap();
                assert!((q.x - x).abs() < 0.05 && (q.y - y).abs() < 0.05, "{x},{y}");
            }
        }
    }

    #[test]
    fn view_centre_lands_on_the_disc_centre_and_the_back_is_hidden() {
        let cam = Camera {
            center: Geo::from_degrees(-70.0, 33.0),
            zoom: 1.0,
        };
        let p = Projector::new(cam, stage(1.0));
        let c = p.project(cam.center).unwrap();
        let (cx, cy, _) = p.disc();
        assert!((c.x - cx).abs() < 1e-3 && (c.y - cy).abs() < 1e-3 && c.facing > 0.999);
        let back = Geo::new(cam.center.lon + PI, -cam.center.lat);
        assert!(p.project(back).is_none());
    }

    #[test]
    fn east_is_right_and_north_is_up() {
        let p = Projector::new(Camera::home(), stage(1.0));
        let (cx, cy, _) = p.disc();
        let e = p.project(Geo::from_degrees(5.0, 0.0)).unwrap();
        assert!(e.x > cx && (e.y - cy).abs() < 1.0);
        let n = p.project(Geo::from_degrees(0.0, 5.0)).unwrap();
        assert!(n.y < cy && (n.x - cx).abs() < 1.0);
    }

    #[test]
    fn at_rest_only_a_thin_cap_shows_and_rising_is_continuous() {
        let s0 = stage(0.0);
        let (_, cy0) = s0.disc_center(1.0);
        let r0 = s0.radius(1.0);
        let top = cy0 - r0;
        let exposed = s0.height - top;
        assert!((exposed - REST_EXPOSURE_UNITS * s0.unit).abs() < 0.5);
        assert!(exposed < s0.height * 0.05);
        let mut last = s0.disc_center(1.0).1;
        for i in 1..=20 {
            let y = stage(i as f32 / 20.0).disc_center(1.0).1;
            assert!(y <= last + 1e-3, "monotone rise");
            last = y;
        }
        assert!((stage(1.0).disc_center(1.0).1 - 1200.0).abs() < 1e-3);
    }

    #[test]
    fn dragging_moves_the_surface_with_the_finger() {
        let st = stage(1.0);
        let mut cam = Camera::home();
        let grabbed = Projector::new(cam, st).unproject(540.0, 1200.0).unwrap();
        cam.pan_px(120.0, 0.0, &st);
        let now = Projector::new(cam, st).project(grabbed).unwrap();
        assert!(now.x > 540.0 + 100.0, "surface went right: {}", now.x);
        let mut up = Camera::home();
        up.pan_px(0.0, 100.0, &st);
        assert!(up.center.lat > 0.0, "dragging down reveals what is north");
    }

    #[test]
    fn pan_of_zero_or_garbage_changes_nothing_and_latitude_is_bounded() {
        let st = stage(1.0);
        let mut cam = Camera::home();
        cam.pan_px(0.0, 0.0, &st);
        cam.pan_px(f32::NAN, 5.0, &st);
        assert_eq!(cam, Camera::home());
        for _ in 0..200 {
            cam.pan_px(0.0, 400.0, &st);
        }
        assert!(cam.center.lat <= MAX_CENTER_LAT + 1e-9);
    }

    #[test]
    fn zoom_is_clamped_and_anchored_zoom_keeps_the_point_under_the_finger() {
        let st = stage(1.0);
        let mut cam = Camera::home();
        cam.zoom_by(100.0);
        assert_eq!(cam.zoom, ZOOM_MAX);
        cam.zoom_by(0.0);
        cam.zoom_by(f32::NAN);
        assert_eq!(cam.zoom, ZOOM_MAX);
        let mut c2 = Camera::home();
        let g = Projector::new(c2, st).unproject(800.0, 900.0).unwrap();
        c2.zoom_about(1.8, 800.0, 900.0, &st);
        let after = Projector::new(c2, st).project(g).unwrap();
        assert!((after.x - 800.0).abs() < 1.5 && (after.y - 900.0).abs() < 1.5);
        assert!(c2.zoom > 1.7);
    }

    #[test]
    fn semantic_zoom_hides_depth_when_zoomed_out_and_reveals_it_closing_in() {
        assert_eq!(detail(1, ZOOM_MIN), 1.0);
        assert_eq!(detail(2, ZOOM_MIN), 0.0);
        assert_eq!(detail(2, 1.15), 1.0);
        assert!(detail(3, 1.15) == 0.0 && detail(3, ZOOM_MAX) == 1.0);
        let mid = detail(2, 1.0);
        assert!(mid > 0.0 && mid < 1.0);
        assert!(depth_threshold(4) > depth_threshold(3));
    }
}
