//! Non-spatial ways in. The geography is spatial, but nobody is required to
//! navigate it spatially: the same objects are exposed as an ordered outline
//! (screen readers, a plain list) and as directional focus steps (keyboard,
//! d-pad). Both are derived from the same entries the sphere draws.

use crate::camera::Camera;
use crate::model::{Availability, ObjectClass};
use crate::scene::{Entry, Layout};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compass {
    Here,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

pub fn compass(from: crate::geo::Geo, to: crate::geo::Geo) -> Compass {
    if from.distance(to) < 0.5f64.to_radians() {
        return Compass::Here;
    }
    let sector = ((from.bearing(to) + std::f64::consts::FRAC_PI_8) / std::f64::consts::FRAC_PI_4)
        .floor() as i64
        % 8;
    match sector {
        0 => Compass::North,
        1 => Compass::NorthEast,
        2 => Compass::East,
        3 => Compass::SouthEast,
        4 => Compass::South,
        5 => Compass::SouthWest,
        6 => Compass::West,
        _ => Compass::NorthWest,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutlineEntry {
    pub id: String,
    pub label: String,
    pub class: ObjectClass,
    pub echelon: u8,
    /// Labels of the ancestors, outermost first.
    pub path: Vec<String>,
    pub availability: Availability,
    pub ghost: bool,
    pub direction: Compass,
    pub distance_deg: f32,
}

/// Reading order: first-seen order, with children right after their parent.
pub fn outline(entries: &[Entry], camera: Camera) -> Vec<OutlineEntry> {
    let by_id: BTreeMap<&str, &Entry> = entries.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut children: BTreeMap<&str, Vec<&Entry>> = BTreeMap::new();
    let mut roots: Vec<&Entry> = Vec::new();
    for e in entries {
        match e.parent.as_deref().filter(|p| by_id.contains_key(p)) {
            Some(p) => children.entry(p).or_default().push(e),
            None => roots.push(e),
        }
    }
    let key = |e: &&Entry| (e.seq, e.id.clone());
    roots.sort_by_key(key);
    for v in children.values_mut() {
        v.sort_by_key(key);
    }
    let mut out = Vec::new();
    let mut stack: Vec<(&Entry, Vec<String>)> =
        roots.into_iter().rev().map(|e| (e, Vec::new())).collect();
    while let Some((e, path)) = stack.pop() {
        out.push(OutlineEntry {
            id: e.id.clone(),
            label: e.label.clone(),
            class: e.class.clone(),
            echelon: e.echelon,
            path: path.clone(),
            availability: e.availability,
            ghost: e.ghost,
            direction: compass(camera.center, e.geo),
            distance_deg: camera.center.distance(e.geo).to_degrees() as f32,
        });
        if let Some(kids) = children.get(e.id.as_str()) {
            let mut p = path.clone();
            p.push(e.label.clone());
            for k in kids.iter().rev() {
                stack.push((k, p.clone()));
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Move focus to the nearest touchable object in `dir`. With no current
/// focus, start from the object closest to the middle of the view.
pub fn step_focus(layout: &Layout, from: Option<&str>, dir: Direction) -> Option<String> {
    if !layout.interactive {
        return None;
    }
    let live = || {
        layout
            .items
            .iter()
            .filter(|i| i.alpha >= 0.25 && i.x >= 0.0 && i.x <= layout.bounds.0)
            .filter(|i| i.y >= 0.0 && i.y <= layout.bounds.1)
    };
    let cur = from.and_then(|id| layout.items.iter().find(|i| i.id == id));
    let Some(cur) = cur else {
        let (mx, my) = (layout.bounds.0 * 0.5, layout.bounds.1 * 0.5);
        return live()
            .min_by(|a, b| {
                (a.x - mx)
                    .hypot(a.y - my)
                    .partial_cmp(&(b.x - mx).hypot(b.y - my))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            })
            .map(|i| i.id.clone());
    };
    live()
        .filter(|i| i.id != cur.id)
        .filter_map(|i| {
            let (dx, dy) = (i.x - cur.x, i.y - cur.y);
            let (along, across) = match dir {
                Direction::Left => (-dx, dy),
                Direction::Right => (dx, dy),
                Direction::Up => (-dy, dx),
                Direction::Down => (dy, dx),
            };
            (along > 1.0 && across.abs() <= along * 1.2).then(|| (along + across.abs() * 0.5, i))
        })
        .min_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.id.cmp(&b.1.id))
        })
        .map(|(_, i)| i.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Stage;
    use crate::geo::Geo;
    use crate::geography::Geography;
    use crate::model::{Context, ObjectClass as C, OrbObject};
    use crate::scene::{compose, layout};

    fn entries() -> Vec<Entry> {
        let objs = vec![
            OrbObject::new("apps", C::Capability, "Приложения", 1),
            OrbObject::new("tasks", C::Capability, "Задачи", 1),
            OrbObject::new("mail", C::Application, "Почта", 2).child_of("apps"),
            OrbObject::new("music", C::Application, "Музыка", 2).child_of("apps"),
            OrbObject::new("t1", C::Task, "Купить хлеб", 2).child_of("tasks"),
        ];
        compose(&mut Geography::default(), &objs, &Context::default()).entries
    }

    #[test]
    fn outline_lists_everything_once_children_after_their_parent() {
        let e = entries();
        let o = outline(&e, Camera::home());
        let ids: Vec<&str> = o.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids, ["apps", "mail", "music", "tasks", "t1"]);
        let mail = &o[1];
        assert_eq!(mail.path, vec!["Приложения".to_string()]);
        assert!(o[0].path.is_empty());
    }

    #[test]
    fn outline_is_independent_of_zoom_and_rise_so_nothing_is_only_reachable_by_gesture() {
        let e = entries();
        let far = Camera {
            center: Geo::from_degrees(100.0, 0.0),
            zoom: 0.55,
        };
        assert_eq!(outline(&e, far).len(), outline(&e, Camera::home()).len());
    }

    #[test]
    fn compass_follows_bearing() {
        let h = Geo::HOME;
        assert_eq!(compass(h, h), Compass::Here);
        assert_eq!(compass(h, Geo::from_degrees(0.0, 10.0)), Compass::North);
        assert_eq!(compass(h, Geo::from_degrees(10.0, 0.0)), Compass::East);
        assert_eq!(compass(h, Geo::from_degrees(0.0, -10.0)), Compass::South);
        assert_eq!(compass(h, Geo::from_degrees(-10.0, 0.0)), Compass::West);
        assert_eq!(compass(h, Geo::from_degrees(8.0, 8.0)), Compass::NorthEast);
    }

    fn open_layout() -> Layout {
        let e = entries();
        let apps = e.iter().find(|x| x.id == "apps").unwrap().geo;
        layout(
            &e,
            Camera {
                center: apps,
                zoom: 1.8,
            },
            Stage::new(1080.0, 2400.0, 1.0, 3.0),
        )
    }

    #[test]
    fn focus_starts_in_the_middle_and_moves_in_the_pressed_direction() {
        let l = open_layout();
        let first = step_focus(&l, None, Direction::Right).unwrap();
        let cur = l.items.iter().find(|i| i.id == first).unwrap();
        for d in [Direction::Left, Direction::Right, Direction::Up, Direction::Down] {
            if let Some(n) = step_focus(&l, Some(&first), d) {
                let it = l.items.iter().find(|i| i.id == n).unwrap();
                match d {
                    Direction::Left => assert!(it.x < cur.x),
                    Direction::Right => assert!(it.x > cur.x),
                    Direction::Up => assert!(it.y < cur.y),
                    Direction::Down => assert!(it.y > cur.y),
                }
                assert_ne!(n, first);
            }
        }
    }

    #[test]
    fn every_visible_object_is_reachable_by_focus_steps() {
        let l = open_layout();
        let mut seen = std::collections::BTreeSet::new();
        let mut cur = step_focus(&l, None, Direction::Right);
        let mut guard = 0;
        'walk: while let Some(c) = cur.clone() {
            seen.insert(c.clone());
            guard += 1;
            assert!(guard < 200);
            for d in [Direction::Right, Direction::Down, Direction::Left, Direction::Up] {
                if let Some(n) = step_focus(&l, Some(&c), d) {
                    if !seen.contains(&n) {
                        cur = Some(n);
                        continue 'walk;
                    }
                }
            }
            break;
        }
        let visible = l.items.iter().filter(|i| i.alpha >= 0.25).count();
        assert!(seen.len() >= visible.min(2), "{} of {visible}", seen.len());
    }

    #[test]
    fn focus_is_inert_while_the_sphere_is_still_resting() {
        let e = entries();
        let l = layout(&e, Camera::home(), Stage::new(1080.0, 2400.0, 0.3, 3.0));
        assert_eq!(step_focus(&l, None, Direction::Right), None);
    }
}
