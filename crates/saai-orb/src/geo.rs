//! Sphere geometry. Longitude/latitude in radians; `Vec3` is the same point
//! as a unit vector with +Z at (0,0), +X east, +Y north.

use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

pub const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
        }
    }

    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn scale(self, k: f64) -> Vec3 {
        Vec3 {
            x: self.x * k,
            y: self.y * k,
            z: self.z * k,
        }
    }

    pub fn plus(self, o: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + o.x,
            y: self.y + o.y,
            z: self.z + o.z,
        }
    }

    pub fn normalized(self) -> Option<Vec3> {
        let l = self.len();
        (l > 1e-12).then(|| self.scale(1.0 / l))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Geo {
    pub lon: f64,
    pub lat: f64,
}

impl Geo {
    pub const HOME: Geo = Geo { lon: 0.0, lat: 0.0 };

    pub fn new(lon: f64, lat: f64) -> Geo {
        let lat = if lat.is_finite() {
            lat.clamp(-FRAC_PI_2, FRAC_PI_2)
        } else {
            0.0
        };
        let lon = if lon.is_finite() { wrap_lon(lon) } else { 0.0 };
        Geo { lon, lat }
    }

    pub fn from_degrees(lon: f64, lat: f64) -> Geo {
        Geo::new(lon.to_radians(), lat.to_radians())
    }

    pub fn to_vec(self) -> Vec3 {
        let (sl, cl) = self.lat.sin_cos();
        let (so, co) = self.lon.sin_cos();
        Vec3 {
            x: cl * so,
            y: sl,
            z: cl * co,
        }
    }

    pub fn from_vec(v: Vec3) -> Geo {
        Geo::new(v.x.atan2(v.z), v.y.clamp(-1.0, 1.0).asin())
    }

    /// Great-circle angle to `other`, radians in `[0, pi]`.
    pub fn distance(self, other: Geo) -> f64 {
        let a = self.to_vec();
        let b = other.to_vec();
        a.cross(b).len().atan2(a.dot(b))
    }

    /// Initial bearing toward `other`, clockwise from north, `[0, tau)`.
    pub fn bearing(self, other: Geo) -> f64 {
        let dl = other.lon - self.lon;
        let y = dl.sin() * other.lat.cos();
        let x = self.lat.cos() * other.lat.sin() - self.lat.sin() * other.lat.cos() * dl.cos();
        y.atan2(x).rem_euclid(TAU)
    }

    /// Walk `dist` radians along the great circle that leaves at `bearing`.
    pub fn offset(self, bearing: f64, dist: f64) -> Geo {
        let (sd, cd) = dist.sin_cos();
        let (sp, cp) = self.lat.sin_cos();
        let lat = (sp * cd + cp * sd * bearing.cos()).clamp(-1.0, 1.0).asin();
        let lon = self.lon + (bearing.sin() * sd * cp).atan2(cd - sp * lat.sin());
        Geo::new(lon, lat)
    }

    /// Point at fraction `t` of the shortest arc to `other`.
    pub fn slerp(self, other: Geo, t: f64) -> Geo {
        let a = self.to_vec();
        let b = other.to_vec();
        let omega = a.cross(b).len().atan2(a.dot(b));
        if omega < 1e-9 {
            return self;
        }
        if (PI - omega).abs() < 1e-9 {
            return if t < 0.5 { self } else { other };
        }
        let so = omega.sin();
        let v = a
            .scale(((1.0 - t) * omega).sin() / so)
            .plus(b.scale((t * omega).sin() / so));
        Geo::from_vec(v)
    }

    pub fn clamp_lat(self, max: f64) -> Geo {
        Geo {
            lon: self.lon,
            lat: self.lat.clamp(-max, max),
        }
    }
}

fn wrap_lon(lon: f64) -> f64 {
    let w = (lon + PI).rem_euclid(TAU) - PI;
    if w <= -PI {
        PI
    } else {
        w
    }
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge1 <= edge0 {
        return if x >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Deterministic across platforms and releases; placement must never depend
/// on `std`'s hasher, which is allowed to change.
pub fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn vec_roundtrip_and_home_axis() {
        let h = Geo::HOME.to_vec();
        assert!(close(h.z, 1.0) && close(h.x, 0.0) && close(h.y, 0.0));
        let g = Geo::from_degrees(37.0, -52.0);
        let r = Geo::from_vec(g.to_vec());
        assert!(close(g.lon, r.lon) && close(g.lat, r.lat));
    }

    #[test]
    fn offset_is_inverse_of_bearing_and_distance() {
        let a = Geo::from_degrees(10.0, 20.0);
        let b = a.offset(1.1, 0.3);
        assert!(close(a.distance(b), 0.3));
        assert!(close(a.bearing(b), 1.1));
    }

    #[test]
    fn north_bearing_goes_up_and_east_goes_right() {
        let n = Geo::HOME.offset(0.0, 0.1);
        assert!(n.lat > 0.09 && n.lon.abs() < 1e-12);
        let e = Geo::HOME.offset(FRAC_PI_2, 0.1);
        assert!(e.lon > 0.09 && e.lat.abs() < 1e-12);
    }

    #[test]
    fn slerp_hits_endpoints_and_midpoint_is_equidistant() {
        let a = Geo::from_degrees(-30.0, 10.0);
        let b = Geo::from_degrees(50.0, -20.0);
        assert!(a.slerp(b, 0.0).distance(a) < 1e-9);
        assert!(a.slerp(b, 1.0).distance(b) < 1e-9);
        let m = a.slerp(b, 0.5);
        assert!((m.distance(a) - m.distance(b)).abs() < 1e-9);
        assert!((m.distance(a) * 2.0 - a.distance(b)).abs() < 1e-9);
    }

    #[test]
    fn longitude_wraps_and_non_finite_input_is_neutralised() {
        assert!(close(Geo::new(3.0 * PI, 0.0).lon.abs(), PI));
        let g = Geo::new(f64::NAN, f64::INFINITY);
        assert_eq!((g.lon, g.lat), (0.0, 0.0));
        assert!(Geo::new(0.0, 9.0).lat <= FRAC_PI_2);
    }

    #[test]
    fn fnv1a_is_pinned() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn smoothstep_is_monotone_and_clamped() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
        assert!(smoothstep(0.0, 1.0, 0.3) < smoothstep(0.0, 1.0, 0.6));
    }
}
