//! What the Orb is able to hold. Classes are open: anything the registry
//! does not know becomes `Unknown(..)` and is drawn with a generic primitive,
//! so new object types never require changes here.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ObjectClass {
    Application,
    Action,
    Task,
    Person,
    Device,
    Service,
    Document,
    Project,
    Environment,
    Media,
    Automation,
    Agent,
    RemoteComputer,
    Sensor,
    Capability,
    Notification,
    SystemState,
    Unknown(String),
}

/// Drawing vocabulary shared by every class; unknown classes fall back to
/// `Disc` so the renderer never needs per-type knowledge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    Disc,
    Square,
    Diamond,
    Ring,
}

impl ObjectClass {
    pub fn wire(&self) -> &str {
        match self {
            ObjectClass::Application => "application",
            ObjectClass::Action => "action",
            ObjectClass::Task => "task",
            ObjectClass::Person => "person",
            ObjectClass::Device => "device",
            ObjectClass::Service => "service",
            ObjectClass::Document => "document",
            ObjectClass::Project => "project",
            ObjectClass::Environment => "environment",
            ObjectClass::Media => "media",
            ObjectClass::Automation => "automation",
            ObjectClass::Agent => "agent",
            ObjectClass::RemoteComputer => "remote_computer",
            ObjectClass::Sensor => "sensor",
            ObjectClass::Capability => "capability",
            ObjectClass::Notification => "notification",
            ObjectClass::SystemState => "system_state",
            ObjectClass::Unknown(s) => s,
        }
    }

    pub fn from_wire(s: &str) -> ObjectClass {
        match s {
            "application" => ObjectClass::Application,
            "action" => ObjectClass::Action,
            "task" => ObjectClass::Task,
            "person" => ObjectClass::Person,
            "device" => ObjectClass::Device,
            "service" => ObjectClass::Service,
            "document" => ObjectClass::Document,
            "project" => ObjectClass::Project,
            "environment" => ObjectClass::Environment,
            "media" => ObjectClass::Media,
            "automation" => ObjectClass::Automation,
            "agent" => ObjectClass::Agent,
            "remote_computer" => ObjectClass::RemoteComputer,
            "sensor" => ObjectClass::Sensor,
            "capability" => ObjectClass::Capability,
            "notification" => ObjectClass::Notification,
            "system_state" => ObjectClass::SystemState,
            other => ObjectClass::Unknown(other.to_string()),
        }
    }

    pub fn primitive(&self) -> Primitive {
        match self {
            ObjectClass::Application | ObjectClass::Document | ObjectClass::Project => {
                Primitive::Square
            }
            ObjectClass::Action | ObjectClass::Automation | ObjectClass::Notification => {
                Primitive::Diamond
            }
            ObjectClass::Service
            | ObjectClass::SystemState
            | ObjectClass::Sensor
            | ObjectClass::Environment => Primitive::Ring,
            _ => Primitive::Disc,
        }
    }
}

/// Which of the four tiers an object belongs to. The fourth tier,
/// availability, is a separate axis because every tier can be unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    /// Part of the learned geography; always in the same place.
    Persistent,
    /// Present because of the current situation; remembered once seen.
    Contextual,
    /// Offered for now only; never takes a slot in the geography.
    Suggestion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unavailable {
    Offline,
    Locked,
    Disabled,
    /// Remembered in the geography but not reported right now.
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Availability {
    Available,
    Unavailable(Unavailable),
}

impl Availability {
    pub fn is_available(self) -> bool {
        matches!(self, Availability::Available)
    }
}

/// How much the current situation asks for an object. Never moves anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Prominence {
    Irrelevant,
    Deeper,
    Nearby,
    Now,
}

impl Prominence {
    pub fn alpha(self) -> f32 {
        match self {
            Prominence::Now => 1.0,
            Prominence::Nearby => 0.92,
            Prominence::Deeper => 0.66,
            Prominence::Irrelevant => 0.3,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrbObject {
    pub id: String,
    pub class: ObjectClass,
    pub label: String,
    /// E1 is `1`. Higher echelons organise, they are not submenus.
    pub echelon: u8,
    pub parent: Option<String>,
    pub tier: Tier,
    pub availability: Availability,
    pub keywords: Vec<String>,
    /// Situations in which the object is relevant. Empty means always.
    pub spaces: Vec<String>,
}

impl OrbObject {
    pub fn new(id: &str, class: ObjectClass, label: &str, echelon: u8) -> OrbObject {
        OrbObject {
            id: id.to_string(),
            class,
            label: label.to_string(),
            echelon: echelon.max(1),
            parent: None,
            tier: Tier::Persistent,
            availability: Availability::Available,
            keywords: Vec::new(),
            spaces: Vec::new(),
        }
    }

    pub fn child_of(mut self, parent: &str) -> OrbObject {
        self.parent = Some(parent.to_string());
        self
    }

    pub fn tier(mut self, tier: Tier) -> OrbObject {
        self.tier = tier;
        self
    }

    pub fn unavailable(mut self, why: Unavailable) -> OrbObject {
        self.availability = Availability::Unavailable(why);
        self
    }

    pub fn keywords(mut self, words: &[&str]) -> OrbObject {
        self.keywords = words.iter().map(|w| w.to_string()).collect();
        self
    }

    pub fn spaces(mut self, spaces: &[&str]) -> OrbObject {
        self.spaces = spaces.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// The situation the Orb is read in. Context changes prominence only.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Context {
    pub space: Option<String>,
    /// Objects that matter this very moment (needs attention, running).
    pub now: BTreeSet<String>,
    /// The object the user is looking into.
    pub focus: Option<String>,
}

pub fn prominence_of(o: &OrbObject, ctx: &Context) -> Prominence {
    if ctx.now.contains(&o.id) {
        return Prominence::Now;
    }
    if o.tier == Tier::Suggestion {
        return Prominence::Nearby;
    }
    if let Some(f) = &ctx.focus {
        if *f == o.id || o.parent.as_deref() == Some(f.as_str()) {
            return Prominence::Nearby;
        }
    }
    let top = o.echelon <= 1;
    if let Some(space) = &ctx.space {
        let foreign = !o.spaces.is_empty() && !o.spaces.iter().any(|s| s == space);
        if foreign && !(top && o.tier == Tier::Persistent) {
            return Prominence::Irrelevant;
        }
    }
    if top {
        Prominence::Nearby
    } else {
        Prominence::Deeper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_roundtrip_covers_every_known_class_and_keeps_unknown() {
        let all = [
            "application",
            "action",
            "task",
            "person",
            "device",
            "service",
            "document",
            "project",
            "environment",
            "media",
            "automation",
            "agent",
            "remote_computer",
            "sensor",
            "capability",
            "notification",
            "system_state",
        ];
        for w in all {
            let c = ObjectClass::from_wire(w);
            assert!(!matches!(c, ObjectClass::Unknown(_)), "{w}");
            assert_eq!(c.wire(), w);
        }
        let u = ObjectClass::from_wire("quantum_toaster");
        assert_eq!(u, ObjectClass::Unknown("quantum_toaster".into()));
        assert_eq!(u.wire(), "quantum_toaster");
        assert_eq!(u.primitive(), Primitive::Disc);
    }

    #[test]
    fn now_beats_everything_and_persistent_top_level_is_never_irrelevant() {
        let ctx = Context {
            space: Some("home".into()),
            now: ["a".to_string()].into_iter().collect(),
            focus: None,
        };
        let a = OrbObject::new("a", ObjectClass::Task, "A", 2).spaces(&["work"]);
        assert_eq!(prominence_of(&a, &ctx), Prominence::Now);
        let b = OrbObject::new("b", ObjectClass::Task, "B", 2).spaces(&["work"]);
        assert_eq!(prominence_of(&b, &ctx), Prominence::Irrelevant);
        let c = OrbObject::new("c", ObjectClass::Capability, "C", 1).spaces(&["work"]);
        assert_eq!(prominence_of(&c, &ctx), Prominence::Nearby);
        let d = OrbObject::new("d", ObjectClass::Device, "D", 1)
            .tier(Tier::Contextual)
            .spaces(&["work"]);
        assert_eq!(prominence_of(&d, &ctx), Prominence::Irrelevant);
    }

    #[test]
    fn focus_raises_children_and_deep_objects_default_to_deeper() {
        let ctx = Context {
            focus: Some("apps".into()),
            ..Context::default()
        };
        let child = OrbObject::new("mail", ObjectClass::Application, "Mail", 2).child_of("apps");
        assert_eq!(prominence_of(&child, &ctx), Prominence::Nearby);
        let other = OrbObject::new("x", ObjectClass::Application, "X", 2).child_of("tasks");
        assert_eq!(prominence_of(&other, &ctx), Prominence::Deeper);
        let s = OrbObject::new("s", ObjectClass::Action, "S", 2).tier(Tier::Suggestion);
        assert_eq!(prominence_of(&s, &Context::default()), Prominence::Nearby);
    }

    #[test]
    fn unavailability_is_orthogonal_to_prominence() {
        let o = OrbObject::new("srv", ObjectClass::RemoteComputer, "Server", 1)
            .unavailable(Unavailable::Offline);
        assert!(!o.availability.is_available());
        assert_eq!(prominence_of(&o, &Context::default()), Prominence::Nearby);
    }
}
