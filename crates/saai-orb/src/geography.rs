//! The persistent geography: where things are. A place is assigned once, the
//! first time an object is seen, and never changes afterwards. Context,
//! availability and removal of the source object do not move it.

use crate::geo::{fnv1a, Geo, GOLDEN_ANGLE};
use crate::model::{ObjectClass, OrbObject, Tier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Angular scale of the first echelon (radians). Neighbours end up about
/// `1.8 * E1_SCALE` apart: roughly half a phone width at default zoom, so a
/// glance shows a few capabilities and one drag reaches the next.
pub const E1_SCALE: f64 = 8.0 * std::f64::consts::PI / 180.0;
/// Each deeper echelon is this much finer: `R_d = R_1 * F^(d-1)`.
pub const DEPTH_FACTOR: f64 = 0.3;
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub geo: Geo,
    pub echelon: u8,
    pub parent: Option<String>,
    pub slot: u32,
    /// Global first-seen order; the reading order for accessibility.
    pub seq: u32,
    pub tier: Tier,
    /// Remembered so a vanished object can still be drawn as dimmed.
    pub class: String,
    pub label: String,
}

impl Place {
    pub fn class(&self) -> ObjectClass {
        ObjectClass::from_wire(&self.class)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Geography {
    version: u32,
    places: BTreeMap<String, Place>,
    next_slot: BTreeMap<String, u32>,
    next_seq: u32,
}

impl Default for Geography {
    fn default() -> Self {
        Geography {
            version: FORMAT_VERSION,
            places: BTreeMap::new(),
            next_slot: BTreeMap::new(),
            next_seq: 0,
        }
    }
}

fn cluster_scale(echelon: u8) -> f64 {
    E1_SCALE * DEPTH_FACTOR.powi(i32::from(echelon.max(1)) - 1)
}

/// Places are stored as short decimals so a save/load cycle returns exactly
/// the same bits; a nanoradian is far below anything a pixel can show.
fn quantize(g: Geo) -> Geo {
    let q = |v: f64| (v * 1e9).round() / 1e9;
    Geo {
        lon: q(g.lon),
        lat: q(g.lat),
    }
}

fn spiral(center: Geo, start: f64, scale: f64, slot: u32) -> Geo {
    let bearing = start + f64::from(slot) * GOLDEN_ANGLE;
    let dist = scale * (f64::from(slot) + 0.5).sqrt();
    quantize(center.offset(bearing, dist))
}

fn cluster_start(parent: &str) -> f64 {
    (fnv1a(parent) % 360) as f64 * std::f64::consts::PI / 180.0
}

impl Geography {
    pub fn place(&self, id: &str) -> Option<&Place> {
        self.places.get(id)
    }

    pub fn len(&self) -> usize {
        self.places.len()
    }

    pub fn is_empty(&self) -> bool {
        self.places.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Place)> {
        self.places.iter()
    }

    /// Remember `o`. Returns `None` when it hangs off a parent that has no
    /// place yet (register the parent first), `Some(true)` when something
    /// persistent changed, `Some(false)` when nothing did. Suggestions never
    /// take a slot.
    pub fn register(&mut self, o: &OrbObject) -> Option<bool> {
        if o.tier == Tier::Suggestion {
            return Some(false);
        }
        if let Some(p) = self.places.get_mut(&o.id) {
            let class = o.class.wire();
            let mut changed = false;
            if p.label != o.label {
                p.label = o.label.clone();
                changed = true;
            }
            if p.class != class {
                p.class = class.to_string();
                changed = true;
            }
            return Some(changed);
        }
        let geo = self.next_geo(o)?;
        let key = o.parent.clone().unwrap_or_default();
        let slot = *self.next_slot.get(&key).unwrap_or(&0);
        self.next_slot.insert(key, slot + 1);
        let seq = self.next_seq;
        self.next_seq += 1;
        self.places.insert(
            o.id.clone(),
            Place {
                geo,
                echelon: o.echelon,
                parent: o.parent.clone(),
                slot,
                seq,
                tier: o.tier,
                class: o.class.wire().to_string(),
                label: o.label.clone(),
            },
        );
        Some(true)
    }

    fn next_geo(&self, o: &OrbObject) -> Option<Geo> {
        let key = o.parent.clone().unwrap_or_default();
        let slot = *self.next_slot.get(&key).unwrap_or(&0);
        match &o.parent {
            None => Some(spiral(Geo::HOME, 0.0, cluster_scale(1), slot)),
            Some(p) => {
                let parent = self.places.get(p)?;
                Some(spiral(
                    parent.geo,
                    cluster_start(p),
                    cluster_scale(o.echelon.max(2)),
                    slot,
                ))
            }
        }
    }

    /// Where a temporary suggestion is drawn: beside its parent (or home),
    /// derived from the id alone, and never stored.
    pub fn ephemeral(&self, o: &OrbObject) -> Option<Geo> {
        let anchor = match &o.parent {
            Some(p) => self.places.get(p)?.geo,
            None => Geo::HOME,
        };
        let h = fnv1a(&o.id);
        let bearing = (h % 3600) as f64 / 3600.0 * std::f64::consts::TAU;
        Some(quantize(
            anchor.offset(bearing, cluster_scale(o.echelon.max(2)) * 1.6),
        ))
    }

    /// Explicit user action only; the sole way a place is ever released.
    pub fn forget(&mut self, id: &str) -> bool {
        self.places.remove(id).is_some()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Unreadable or foreign-version data yields an empty geography rather
    /// than a half-trusted one; the learned layout is rebuilt in first-seen
    /// order, which is deterministic for the built-in capabilities.
    pub fn from_json(s: &str) -> Geography {
        match serde_json::from_str::<Geography>(s) {
            Ok(g) if g.version == FORMAT_VERSION => g,
            _ => Geography::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ObjectClass, Unavailable};

    fn cap(id: &str) -> OrbObject {
        OrbObject::new(id, ObjectClass::Capability, id, 1)
    }

    #[test]
    fn places_are_assigned_once_and_survive_everything() {
        let mut g = Geography::default();
        assert_eq!(g.register(&cap("search")), Some(true));
        let first = g.place("search").unwrap().geo;
        assert_eq!(g.register(&cap("sai")), Some(true));
        let again = cap("search").unavailable(Unavailable::Offline);
        assert_eq!(g.register(&again), Some(false));
        assert_eq!(g.place("search").unwrap().geo, first);
        let mut moved = cap("search");
        moved.echelon = 3;
        moved.parent = Some("sai".into());
        g.register(&moved);
        let p = g.place("search").unwrap();
        assert_eq!((p.geo, p.echelon, p.parent.clone()), (first, 1, None));
    }

    #[test]
    fn first_seen_order_is_the_layout_and_later_arrivals_do_not_disturb() {
        let ids = [
            "search", "sai", "tasks", "apps", "alerts", "system", "recent",
        ];
        let mut a = Geography::default();
        for id in ids {
            a.register(&cap(id));
        }
        let mut b = Geography::default();
        for id in ids {
            b.register(&cap(id));
        }
        for id in ids {
            assert_eq!(a.place(id), b.place(id));
        }
        let before: Vec<Geo> = ids.iter().map(|i| a.place(i).unwrap().geo).collect();
        a.register(&cap("newcomer"));
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(a.place(id).unwrap().geo, before[i]);
        }
    }

    #[test]
    fn first_echelon_neighbours_keep_a_readable_distance() {
        let mut g = Geography::default();
        let mut pts = Vec::new();
        for i in 0..12 {
            let id = format!("c{i}");
            g.register(&cap(&id));
            pts.push(g.place(&id).unwrap().geo);
        }
        let min = pts
            .iter()
            .enumerate()
            .flat_map(|(i, a)| pts[i + 1..].iter().map(move |b| a.distance(*b)))
            .fold(f64::MAX, f64::min);
        assert!(min > 1.2 * E1_SCALE, "min {:.2} deg", min.to_degrees());
    }

    #[test]
    fn children_gather_near_their_parent_and_far_from_other_parents() {
        let mut g = Geography::default();
        for id in ["apps", "tasks", "devices"] {
            g.register(&cap(id));
        }
        for i in 0..8 {
            let id = format!("app{i}");
            let o = OrbObject::new(&id, ObjectClass::Application, &id, 2).child_of("apps");
            assert_eq!(g.register(&o), Some(true));
        }
        let apps = g.place("apps").unwrap().geo;
        let tasks = g.place("tasks").unwrap().geo;
        let devices = g.place("devices").unwrap().geo;
        for i in 0..8 {
            let c = g.place(&format!("app{i}")).unwrap().geo;
            let to_parent = c.distance(apps);
            assert!(to_parent < 0.55 * apps.distance(tasks).min(apps.distance(devices)));
            assert!(to_parent < c.distance(tasks) && to_parent < c.distance(devices));
        }
        let pts: Vec<Geo> = (0..8)
            .map(|i| g.place(&format!("app{i}")).unwrap().geo)
            .collect();
        let min = pts
            .iter()
            .enumerate()
            .flat_map(|(i, a)| pts[i + 1..].iter().map(move |b| a.distance(*b)))
            .fold(f64::MAX, f64::min);
        assert!(min > 0.5 * cluster_scale(2), "{:.3} deg", min.to_degrees());
    }

    #[test]
    fn orphans_wait_for_their_parent_instead_of_landing_at_the_wrong_place() {
        let mut g = Geography::default();
        let o = OrbObject::new("mail", ObjectClass::Application, "Mail", 2).child_of("apps");
        assert_eq!(g.register(&o), None);
        assert!(g.place("mail").is_none());
        g.register(&cap("apps"));
        assert_eq!(g.register(&o), Some(true));
    }

    #[test]
    fn suggestions_take_no_slot_and_are_drawn_beside_the_parent() {
        let mut g = Geography::default();
        g.register(&cap("sai"));
        let s = OrbObject::new("tip", ObjectClass::Action, "Tip", 2)
            .child_of("sai")
            .tier(Tier::Suggestion);
        assert_eq!(g.register(&s), Some(false));
        assert!(g.place("tip").is_none());
        let at = g.ephemeral(&s).unwrap();
        assert_eq!(g.ephemeral(&s), Some(at));
        assert!(at.distance(g.place("sai").unwrap().geo) < E1_SCALE);
        let real = OrbObject::new("m", ObjectClass::Action, "M", 2).child_of("sai");
        g.register(&real);
        assert_eq!(g.place("m").unwrap().slot, 0);
    }

    #[test]
    fn labels_follow_the_source_but_positions_do_not_and_forget_is_explicit() {
        let mut g = Geography::default();
        g.register(&cap("a"));
        let geo = g.place("a").unwrap().geo;
        let renamed = OrbObject::new("a", ObjectClass::Capability, "Renamed", 1);
        assert_eq!(g.register(&renamed), Some(true));
        assert_eq!(g.place("a").unwrap().label, "Renamed");
        assert_eq!(g.place("a").unwrap().geo, geo);
        assert!(g.forget("a"));
        assert!(!g.forget("a"));
    }

    #[test]
    fn json_roundtrip_is_exact_and_garbage_is_discarded() {
        let mut g = Geography::default();
        for id in ["a", "b", "c"] {
            g.register(&cap(id));
        }
        let back = Geography::from_json(&g.to_json());
        assert_eq!(back, g);
        assert!(Geography::from_json("{nope").is_empty());
        let foreign = g.to_json().replace("\"version\":1", "\"version\":99");
        assert!(Geography::from_json(&foreign).is_empty());
    }
}
