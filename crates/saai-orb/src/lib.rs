//! Orb: a continuous spatial navigation space (ADR-430).
//!
//! The screen is a viewport; Orb is the geography behind it. This crate is
//! pure — no I/O, no clock, no randomness, no rendering — so everything the
//! concept promises (stable places, semantic zoom, route-aware search,
//! device-agnostic input, accessible alternatives) is testable on the host.

pub mod access;
pub mod camera;
pub mod geo;
pub mod geography;
pub mod gesture;
pub mod model;
pub mod nav;
pub mod scene;
pub mod search;

pub use access::{compass, outline, step_focus, Compass, Direction, OutlineEntry};
pub use camera::{
    depth_threshold, detail, Camera, Projected, Projector, Stage, MAX_CENTER_LAT, ZOOM_MAX,
    ZOOM_MIN,
};
pub use geo::{smoothstep, Geo};
pub use geography::{Geography, Place, DEPTH_FACTOR, E1_SCALE};
pub use gesture::{Delta, Lift, Pointers, VelocityTracker, TAP_SLOP_UNITS};
pub use model::{
    prominence_of, Availability, Context, ObjectClass, OrbObject, Primitive, Prominence, Tier,
    Unavailable,
};
pub use nav::{lower, Inertia, Motion, NavInput, Presence, WheelAxis};
pub use scene::{compose, layout, nearest_anchor, Composition, Entry, Item, Layout};
pub use search::{browse, search, Hit, Route, DEFAULT_LIMIT};
