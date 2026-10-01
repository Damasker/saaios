//! Local search that answers "where is it", not only "open it". Every hit
//! carries a route across the sphere so the interface can show the way
//! (a trail, a controlled rotation, zoom guidance) instead of teleporting.

use crate::camera::{depth_threshold, Camera, ZOOM_MAX, ZOOM_MIN};
use crate::geo::{smoothstep, Geo};
use crate::model::{Availability, ObjectClass};
use crate::scene::Entry;

pub const DEFAULT_LIMIT: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub target: String,
    pub from: Geo,
    pub to: Geo,
    pub zoom_from: f32,
    pub zoom_to: f32,
}

impl Route {
    pub fn distance(&self) -> f64 {
        self.from.distance(self.to)
    }

    /// Longer ways take longer, within bounds that keep search snappy.
    pub fn duration_s(&self) -> f32 {
        (0.3 + 0.8 * self.distance() as f32).clamp(0.3, 1.2)
    }

    /// The camera `t` of the way along (`0..=1`). Long routes ease out a
    /// little before easing in, which keeps the destination in context.
    pub fn at(&self, t: f32) -> Camera {
        let t = t.clamp(0.0, 1.0);
        let e = smoothstep(0.0, 1.0, t);
        let center = self.from.slerp(self.to, f64::from(e));
        let dip = (self.distance() as f32 * 0.8).clamp(0.0, 0.4);
        let bell = (std::f32::consts::PI * e).sin();
        let zoom = self.zoom_from + (self.zoom_to - self.zoom_from) * e;
        Camera {
            center,
            zoom: (zoom * (1.0 - dip * bell)).clamp(ZOOM_MIN, ZOOM_MAX),
        }
    }

    pub fn trail(&self, points: usize) -> Vec<Geo> {
        let n = points.max(2);
        (0..n)
            .map(|i| self.from.slerp(self.to, i as f64 / (n - 1) as f64))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: String,
    pub label: String,
    pub class: ObjectClass,
    pub availability: Availability,
    pub ghost: bool,
    pub score: u32,
    pub route: Route,
}

fn norm(s: &str) -> String {
    s.to_lowercase()
}

fn token_score(t: &str, e: &Entry) -> Option<u32> {
    let label = norm(&e.label);
    let mut best = None::<u32>;
    let mut take = |v: u32| best = Some(best.map_or(v, |b| b.max(v)));
    if label == t {
        take(100);
    } else if label.starts_with(t) {
        take(80);
    } else if label.split_whitespace().any(|w| w.starts_with(t)) {
        take(60);
    } else if label.contains(t) {
        take(40);
    }
    for k in &e.keywords {
        let k = norm(k);
        if k == t {
            take(50);
        } else if k.starts_with(t) {
            take(35);
        } else if k.contains(t) {
            take(20);
        }
    }
    best
}

fn zoom_for(e: &Entry, camera: Camera) -> f32 {
    camera
        .zoom
        .max(0.8)
        .max(depth_threshold(e.echelon) * 1.05)
        .clamp(ZOOM_MIN, ZOOM_MAX)
}

/// Every token must match; unavailable and remembered-but-absent objects are
/// found too, because they still exist.
pub fn search(entries: &[Entry], query: &str, camera: Camera, limit: usize) -> Vec<Hit> {
    let q = norm(query);
    let tokens: Vec<&str> = q.split_whitespace().collect();
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(u32, &Entry)> = entries
        .iter()
        .filter_map(|e| {
            let mut total = 0;
            for t in &tokens {
                total += token_score(t, e)?;
            }
            Some((total, e))
        })
        .collect();
    scored.sort_by(|(sa, a), (sb, b)| {
        sb.cmp(sa)
            .then_with(|| {
                b.availability
                    .is_available()
                    .cmp(&a.availability.is_available())
            })
            .then_with(|| a.echelon.cmp(&b.echelon))
            .then_with(|| {
                camera
                    .center
                    .distance(a.geo)
                    .partial_cmp(&camera.center.distance(b.geo))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.id.cmp(&b.id))
    });
    scored
        .into_iter()
        .take(limit)
        .map(|(score, e)| Hit {
            id: e.id.clone(),
            label: e.label.clone(),
            class: e.class.clone(),
            availability: e.availability,
            ghost: e.ghost,
            score,
            route: Route {
                target: e.id.clone(),
                from: camera.center,
                to: e.geo,
                zoom_from: camera.zoom,
                zoom_to: zoom_for(e, camera),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geography::Geography;
    use crate::model::{Context, ObjectClass as C, OrbObject, Unavailable};
    use crate::scene::compose;

    fn world() -> Vec<Entry> {
        let objs = vec![
            OrbObject::new("search", C::Capability, "Поиск", 1),
            OrbObject::new("apps", C::Capability, "Приложения", 1),
            OrbObject::new("devices", C::Capability, "Устройства", 1),
            OrbObject::new("mail", C::Application, "Почта", 2)
                .child_of("apps")
                .keywords(&["email", "письма"]),
            OrbObject::new("music", C::Application, "Музыка", 2).child_of("apps"),
            OrbObject::new("nas", C::RemoteComputer, "Домашний сервер", 2)
                .child_of("devices")
                .unavailable(Unavailable::Offline)
                .keywords(&["nas", "server"]),
        ];
        let mut g = Geography::default();
        compose(&mut g, &objs, &Context::default()).entries
    }

    #[test]
    fn empty_and_unmatched_queries_find_nothing() {
        let e = world();
        assert!(search(&e, "", Camera::home(), 8).is_empty());
        assert!(search(&e, "   ", Camera::home(), 8).is_empty());
        assert!(search(&e, "zzzz", Camera::home(), 8).is_empty());
    }

    #[test]
    fn matching_is_case_insensitive_unicode_prefix_ranked_and_uses_keywords() {
        let e = world();
        let hits = search(&e, "ПОЧ", Camera::home(), 8);
        assert_eq!(hits[0].id, "mail");
        let by_kw = search(&e, "email", Camera::home(), 8);
        assert_eq!(by_kw[0].id, "mail");
        let exact = search(&e, "музыка", Camera::home(), 8);
        assert_eq!(exact[0].score, 100);
        let word = search(&e, "сервер", Camera::home(), 8);
        assert_eq!(word[0].id, "nas");
    }

    #[test]
    fn all_tokens_must_match() {
        let e = world();
        assert_eq!(search(&e, "домашний сервер", Camera::home(), 8)[0].id, "nas");
        assert!(search(&e, "домашний почта", Camera::home(), 8).is_empty());
    }

    #[test]
    fn offline_things_are_found_and_flagged_not_hidden() {
        let e = world();
        let hit = &search(&e, "nas", Camera::home(), 8)[0];
        assert_eq!(hit.availability, Availability::Unavailable(Unavailable::Offline));
    }

    #[test]
    fn available_beats_unavailable_on_equal_score() {
        let mut e = world();
        for x in e.iter_mut().filter(|x| x.id == "mail" || x.id == "music") {
            x.keywords = vec!["tool".into()];
            x.label = format!("{} tool", x.id);
        }
        e.iter_mut().find(|x| x.id == "mail").unwrap().availability =
            Availability::Unavailable(Unavailable::Disabled);
        let hits = search(&e, "tool", Camera::home(), 8);
        assert_eq!(hits[0].id, "music");
        assert_eq!(hits[1].id, "mail");
    }

    #[test]
    fn limit_and_determinism() {
        let e = world();
        assert_eq!(search(&e, "а", Camera::home(), 2).len(), 2);
        assert_eq!(
            search(&e, "а", Camera::home(), 8),
            search(&e, "а", Camera::home(), 8)
        );
    }

    #[test]
    fn route_starts_where_we_are_ends_at_the_target_and_guides_zoom() {
        let e = world();
        let cam = Camera {
            center: Geo::from_degrees(-35.0, 10.0),
            zoom: 0.7,
        };
        let hit = &search(&e, "почта", cam, 1)[0];
        let r = &hit.route;
        assert_eq!(r.at(0.0), cam);
        let end = r.at(1.0);
        assert!(end.center.distance(r.to) < 1e-9);
        assert!(end.zoom >= depth_threshold(2), "target echelon must be legible on arrival");
        let mid = r.at(0.5);
        assert!(mid.zoom < r.zoom_to, "long routes ease out before closing in");
        assert!(mid.center.distance(r.from) < r.distance());
    }

    #[test]
    fn route_never_leaves_the_zoom_range_and_trail_is_on_the_arc() {
        let e = world();
        let hit = &search(&e, "почта", Camera::home(), 1)[0];
        for i in 0..=20 {
            let z = hit.route.at(i as f32 / 20.0).zoom;
            assert!((ZOOM_MIN..=ZOOM_MAX).contains(&z));
        }
        let trail = hit.route.trail(9);
        assert_eq!(trail.len(), 9);
        let total = hit.route.distance();
        for (i, p) in trail.iter().enumerate() {
            let want = total * i as f64 / 8.0;
            assert!((hit.route.from.distance(*p) - want).abs() < 1e-9);
        }
        assert_eq!(hit.route.trail(0).len(), 2);
        assert!(hit.route.duration_s() >= 0.3 && hit.route.duration_s() <= 1.2);
    }
}
