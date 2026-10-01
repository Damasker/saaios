//! From objects and context to pixels-ready geometry. `compose` fixes where
//! things are (via the geography); `layout` only projects them.

use crate::camera::{detail, Camera, Projector, Stage};
use crate::geo::{smoothstep, Geo};
use crate::geography::Geography;
use crate::model::{
    prominence_of, Availability, Context, ObjectClass, OrbObject, Primitive, Prominence, Tier,
    Unavailable,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: String,
    pub class: ObjectClass,
    pub label: String,
    pub echelon: u8,
    pub parent: Option<String>,
    pub tier: Tier,
    pub availability: Availability,
    pub keywords: Vec<String>,
    pub geo: Geo,
    pub prominence: Prominence,
    /// Remembered in the geography but not reported right now.
    pub ghost: bool,
    pub seq: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Composition {
    pub entries: Vec<Entry>,
    /// The geography gained or changed something worth saving.
    pub changed: bool,
}

/// Register what is reported, keep what was seen before, and read prominence
/// from the context. Objects that are no longer reported stay where they were
/// as dimmed ghosts: existence is not availability.
pub fn compose(geography: &mut Geography, objects: &[OrbObject], ctx: &Context) -> Composition {
    let mut changed = false;
    let mut pending: Vec<&OrbObject> = objects.iter().collect();
    loop {
        let before = pending.len();
        pending.retain(|o| match geography.register(o) {
            Some(c) => {
                changed |= c;
                false
            }
            None => true,
        });
        if pending.is_empty() || pending.len() == before {
            break;
        }
    }

    let mut entries = Vec::new();
    for o in objects {
        let geo = if o.tier == Tier::Suggestion {
            geography.ephemeral(o)
        } else {
            geography.place(&o.id).map(|p| p.geo)
        };
        let Some(geo) = geo else { continue };
        let seq = geography.place(&o.id).map_or(u32::MAX, |p| p.seq);
        entries.push(Entry {
            id: o.id.clone(),
            class: o.class.clone(),
            label: o.label.clone(),
            echelon: geography.place(&o.id).map_or(o.echelon, |p| p.echelon),
            parent: o.parent.clone(),
            tier: o.tier,
            availability: o.availability,
            keywords: o.keywords.clone(),
            geo,
            prominence: prominence_of(o, ctx),
            ghost: false,
            seq,
        });
    }

    let reported: std::collections::BTreeSet<&str> =
        objects.iter().map(|o| o.id.as_str()).collect();
    for (id, p) in geography.iter() {
        if reported.contains(id.as_str()) {
            continue;
        }
        let mut stand_in = OrbObject::new(id, p.class(), &p.label, p.echelon);
        stand_in.parent = p.parent.clone();
        stand_in.tier = p.tier;
        let mut prominence = prominence_of(&stand_in, ctx);
        if p.tier != Tier::Persistent {
            prominence = prominence.min(Prominence::Deeper);
        }
        entries.push(Entry {
            id: id.clone(),
            class: p.class(),
            label: p.label.clone(),
            echelon: p.echelon,
            parent: p.parent.clone(),
            tier: p.tier,
            availability: Availability::Unavailable(Unavailable::Missing),
            keywords: Vec::new(),
            geo: p.geo,
            prominence,
            ghost: true,
            seq: p.seq,
        });
    }
    entries.sort_by(|a, b| a.seq.cmp(&b.seq).then_with(|| a.id.cmp(&b.id)));
    Composition { entries, changed }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: String,
    pub class: ObjectClass,
    pub primitive: Primitive,
    pub label: String,
    pub echelon: u8,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub facing: f32,
    pub alpha: f32,
    pub prominence: Prominence,
    pub availability: Availability,
    pub tier: Tier,
    pub ghost: bool,
    pub show_label: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Centre x, centre y, radius of the sphere's disc on screen.
    pub disc: (f32, f32, f32),
    /// Far to near: draw in order, hit-test in reverse.
    pub items: Vec<Item>,
    /// Objects can be touched only when the sphere has fully risen.
    pub interactive: bool,
    pub bounds: (f32, f32),
}

const INTERACTIVE_RISE: f32 = 0.85;
const MIN_ALPHA: f32 = 0.04;

fn base_radius(echelon: u8) -> f32 {
    match echelon {
        0..=1 => 30.0,
        2 => 23.0,
        3 => 17.0,
        _ => 13.0,
    }
}

pub fn layout(entries: &[Entry], camera: Camera, stage: Stage) -> Layout {
    let proj = Projector::new(camera, stage);
    let rise_fade = smoothstep(0.4, 0.95, stage.rise);
    let zoom_scale = camera.zoom.powf(0.35).clamp(0.8, 1.5);
    let mut items = Vec::new();
    if rise_fade > 0.0 {
        for e in entries {
            let Some(p) = proj.project(e.geo) else { continue };
            let limb = smoothstep(0.03, 0.3, p.facing);
            let alpha = detail(e.echelon, camera.zoom) * e.prominence.alpha() * limb * rise_fade;
            if alpha < MIN_ALPHA {
                continue;
            }
            let mut radius = base_radius(e.echelon)
                * stage.unit
                * zoom_scale
                * (0.55 + 0.45 * p.facing);
            if e.prominence == Prominence::Now {
                radius *= 1.12;
            }
            let on_screen = p.x > -radius
                && p.x < stage.width + radius
                && p.y > -radius
                && p.y < stage.height + radius;
            if !on_screen {
                continue;
            }
            items.push(Item {
                id: e.id.clone(),
                class: e.class.clone(),
                primitive: e.class.primitive(),
                label: e.label.clone(),
                echelon: e.echelon,
                x: p.x,
                y: p.y,
                radius,
                facing: p.facing,
                alpha,
                prominence: e.prominence,
                availability: e.availability,
                tier: e.tier,
                ghost: e.ghost,
                show_label: detail(e.echelon, camera.zoom) > 0.6
                    && p.facing > 0.45
                    && e.prominence >= Prominence::Deeper,
            });
        }
    }
    items.sort_by(|a, b| {
        a.facing
            .partial_cmp(&b.facing)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    Layout {
        disc: proj.disc(),
        items,
        interactive: stage.rise >= INTERACTIVE_RISE,
        bounds: (stage.width, stage.height),
    }
}

impl Layout {
    /// Nearest touchable object under `(x, y)`; ties go to the nearer
    /// surface. Targets never shrink below `min_target` pixels in radius.
    pub fn hit(&self, x: f32, y: f32, min_target: f32) -> Option<&Item> {
        if !self.interactive {
            return None;
        }
        self.items
            .iter()
            .rev()
            .filter(|i| i.alpha >= 0.25)
            .filter_map(|i| {
                let d = (i.x - x).hypot(i.y - y);
                (d <= i.radius.max(min_target)).then_some((d, i))
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, i)| i)
    }

    /// True when the risen sphere shows nothing worth touching: the user has
    /// drifted into empty space and the view should settle back.
    pub fn is_lost(&self) -> bool {
        self.interactive
            && !self.items.iter().any(|i| {
                i.alpha >= 0.4
                    && i.x >= 0.0
                    && i.x <= self.bounds.0
                    && i.y >= 0.0
                    && i.y <= self.bounds.1
            })
    }
}

/// Where to settle when lost: the nearest object that is legible at `zoom`.
pub fn nearest_anchor(entries: &[Entry], camera: Camera) -> Option<Geo> {
    entries
        .iter()
        .filter(|e| detail(e.echelon, camera.zoom) >= 0.5 && e.prominence >= Prominence::Deeper)
        .min_by(|a, b| {
            camera
                .center
                .distance(a.geo)
                .partial_cmp(&camera.center.distance(b.geo))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        })
        .map(|e| e.geo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ObjectClass as C;

    fn caps() -> Vec<OrbObject> {
        ["search", "sai", "tasks", "apps", "alerts", "system", "recent"]
            .iter()
            .map(|id| OrbObject::new(id, C::Capability, id, 1))
            .collect()
    }

    fn stage(rise: f32) -> Stage {
        Stage::new(1080.0, 2400.0, rise, 3.0)
    }

    fn compose_default(objs: &[OrbObject]) -> (Geography, Composition) {
        let mut g = Geography::default();
        let c = compose(&mut g, objs, &Context::default());
        (g, c)
    }

    #[test]
    fn compose_places_everything_and_reports_a_change_once() {
        let mut g = Geography::default();
        let first = compose(&mut g, &caps(), &Context::default());
        assert_eq!(first.entries.len(), 7);
        assert!(first.changed);
        let second = compose(&mut g, &caps(), &Context::default());
        assert!(!second.changed);
        assert_eq!(first.entries, second.entries);
    }

    #[test]
    fn context_changes_prominence_but_never_position() {
        let mut g = Geography::default();
        let objs = caps();
        let calm = compose(&mut g, &objs, &Context::default());
        let busy = compose(
            &mut g,
            &objs,
            &Context {
                space: Some("work".into()),
                now: ["alerts".to_string()].into_iter().collect(),
                focus: Some("apps".into()),
            },
        );
        for (a, b) in calm.entries.iter().zip(&busy.entries) {
            assert_eq!(a.geo, b.geo, "{}", a.id);
        }
        let alerts = busy.entries.iter().find(|e| e.id == "alerts").unwrap();
        assert_eq!(alerts.prominence, Prominence::Now);
    }

    #[test]
    fn a_vanished_object_stays_as_a_dimmed_ghost_in_the_same_place() {
        let mut g = Geography::default();
        let mut objs = caps();
        objs.push(OrbObject::new("phone", C::Device, "Phone", 1).tier(Tier::Contextual));
        let with = compose(&mut g, &objs, &Context::default());
        let at = with.entries.iter().find(|e| e.id == "phone").unwrap().geo;
        objs.pop();
        let without = compose(&mut g, &objs, &Context::default());
        let ghost = without.entries.iter().find(|e| e.id == "phone").unwrap();
        assert!(ghost.ghost);
        assert_eq!(ghost.geo, at);
        assert_eq!(
            ghost.availability,
            Availability::Unavailable(Unavailable::Missing)
        );
        assert!(ghost.prominence <= Prominence::Deeper);
    }

    #[test]
    fn children_registered_before_their_parent_still_land_correctly() {
        let objs = vec![
            OrbObject::new("mail", C::Application, "Mail", 2).child_of("apps"),
            OrbObject::new("apps", C::Capability, "Apps", 1),
        ];
        let (g, c) = compose_default(&objs);
        assert_eq!(c.entries.len(), 2);
        assert!(g.place("mail").is_some());
    }

    #[test]
    fn rest_shows_no_touchable_objects_and_rise_fades_them_in() {
        let (_, c) = compose_default(&caps());
        let rest = layout(&c.entries, Camera::home(), stage(0.0));
        assert!(rest.items.is_empty() && !rest.interactive);
        let half = layout(&c.entries, Camera::home(), stage(0.6));
        assert!(!half.interactive);
        assert!(half.items.iter().all(|i| i.alpha < 1.0));
        let full = layout(&c.entries, Camera::home(), stage(1.0));
        assert!(full.interactive && !full.items.is_empty());
    }

    #[test]
    fn zoom_out_hides_the_fine_echelon_and_zoom_in_shows_it() {
        let mut objs = caps();
        for i in 0..4 {
            objs.push(
                OrbObject::new(&format!("a{i}"), C::Application, "App", 2).child_of("apps"),
            );
        }
        let (g, c) = compose_default(&objs);
        let apps = g.place("apps").unwrap().geo;
        let near = |zoom| Camera { center: apps, zoom };
        let out = layout(&c.entries, near(0.7), stage(1.0));
        assert!(out.items.iter().all(|i| i.echelon == 1));
        let inn = layout(&c.entries, near(1.6), stage(1.0));
        assert!(inn.items.iter().filter(|i| i.echelon == 2).count() >= 3);
    }

    #[test]
    fn draw_order_is_far_to_near_and_horizon_shrinks_things() {
        let (_, c) = compose_default(&caps());
        let l = layout(
            &c.entries,
            Camera {
                center: Geo::HOME,
                zoom: 0.7,
            },
            stage(1.0),
        );
        for w in l.items.windows(2) {
            assert!(w[0].facing <= w[1].facing);
        }
        let most = l.items.last().unwrap();
        let least = l.items.first().unwrap();
        if l.items.len() > 1 && most.facing - least.facing > 0.1 {
            assert!(most.radius > least.radius);
        }
    }

    #[test]
    fn hit_prefers_the_closest_and_honours_the_minimum_touch_target() {
        let (_, c) = compose_default(&caps());
        let l = layout(&c.entries, Camera::home(), stage(1.0));
        let it = l.items.last().unwrap();
        assert_eq!(l.hit(it.x, it.y, 66.0).map(|i| &i.id), Some(&it.id));
        let outside = l.hit(it.x + it.radius + 5.0, it.y, 0.0);
        assert!(outside.is_none_or(|o| o.id != it.id));
        assert!(l.hit(-500.0, -500.0, 66.0).is_none());
        let mut closed = l.clone();
        closed.interactive = false;
        assert!(closed.hit(it.x, it.y, 66.0).is_none());
    }

    #[test]
    fn drifting_into_empty_space_is_detected_and_an_anchor_exists() {
        let (_, c) = compose_default(&caps());
        let lost = Camera {
            center: Geo::from_degrees(150.0, 0.0),
            zoom: 1.0,
        };
        let l = layout(&c.entries, lost, stage(1.0));
        assert!(l.is_lost());
        let anchor = nearest_anchor(&c.entries, lost).unwrap();
        assert!(c.entries.iter().any(|e| e.geo == anchor));
        let home = layout(&c.entries, Camera::home(), stage(1.0));
        assert!(!home.is_lost());
    }
}
