//! Orb as a place (ADR-430). The pure model lives in `saai-orb`; this module
//! is the shell's side of it: which real objects exist right now, what
//! activating each one does, and how raw touch drives the sphere. Nothing in
//! here draws or talks to a service.

use saai_orb::{
    camera::REST_EXPOSURE_UNITS, compose, depth_threshold, detail, layout, lower, nearest_anchor,
    search as search_entries, Camera, Context, Delta, Entry, Geo, Geography, Hit, Inertia, Layout,
    Lift, Motion, NavInput, ObjectClass, OrbObject, Pointers, Presence, Projector, Route, Stage,
    Tier, Unavailable, ZOOM_MAX,
};
use saai_ui_core::{Keyboard, Rect};

pub const GEOGRAPHY_PATH: &str = "/data/saaios/var/orb-geography.json";

const TASK_LIMIT: usize = 6;
/// The finger has to travel this share of the viewport height to raise the
/// sphere completely.
const PULL_TRAVEL: f32 = 0.5;

/// What tapping an object does. Parsed from the object id, so the geography
/// (which only remembers ids) stays enough to act on a ghost's neighbours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activation {
    OpenSearch,
    OpenIntent,
    OpenTasks,
    OpenApps,
    OpenInbox,
    OpenSpaces,
    OpenDevices,
    OpenMe,
    LaunchApp(String),
    SelectSpace(String),
    ViewTask(String),
    OpenBluetooth,
}

pub fn activation_for(id: &str) -> Option<Activation> {
    if let Some(rest) = id.strip_prefix("app:") {
        return Some(Activation::LaunchApp(rest.to_string()));
    }
    if let Some(rest) = id.strip_prefix("space:") {
        return Some(Activation::SelectSpace(rest.to_string()));
    }
    if let Some(rest) = id.strip_prefix("task:") {
        return Some(Activation::ViewTask(rest.to_string()));
    }
    if id.starts_with("bt:") {
        return Some(Activation::OpenBluetooth);
    }
    Some(match id {
        "search" => Activation::OpenSearch,
        "sai" => Activation::OpenIntent,
        "tasks" => Activation::OpenTasks,
        "apps" => Activation::OpenApps,
        "inbox" => Activation::OpenInbox,
        "spaces" => Activation::OpenSpaces,
        "devices" => Activation::OpenDevices,
        "me" => Activation::OpenMe,
        _ => return None,
    })
}

/// Real facts the shell already holds. Nothing here is invented: an empty
/// list means an empty cluster.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub apps: Vec<(String, String)>,
    pub spaces: Vec<(String, String)>,
    pub tasks: Vec<(String, String)>,
    pub bluetooth: Vec<String>,
    pub selected_space: String,
    pub appd_connected: bool,
    pub entityd_connected: bool,
    pub attention: bool,
    pub running: bool,
}

/// The built-in capabilities. Their order is the first-seen order and so
/// the permanent layout: append only, never reorder.
const CAPABILITIES: [(&str, ObjectClass, &str, &[&str]); 8] = [
    (
        "search",
        ObjectClass::Capability,
        "Поиск",
        &["найти", "search"],
    ),
    (
        "sai",
        ObjectClass::Agent,
        "Sai",
        &["ассистент", "намерение", "intent"],
    ),
    (
        "tasks",
        ObjectClass::Capability,
        "Задачи",
        &["дела", "tasks"],
    ),
    (
        "apps",
        ObjectClass::Capability,
        "Приложения",
        &["apps", "программы"],
    ),
    (
        "inbox",
        ObjectClass::Capability,
        "Входящие",
        &["уведомления", "inbox"],
    ),
    (
        "spaces",
        ObjectClass::Capability,
        "Пространства",
        &["контекст", "spaces"],
    ),
    (
        "devices",
        ObjectClass::Capability,
        "Устройства",
        &["bluetooth", "блютуз"],
    ),
    (
        "me",
        ObjectClass::Capability,
        "Я",
        &["настройки", "settings"],
    ),
];

/// The on-screen and USB keyboards type Latin only, so every name that can
/// be Cyrillic also carries its Latin spelling as a search keyword.
pub fn latin_spelling(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        let t = match c {
            'а' => "a",
            'б' => "b",
            'в' => "v",
            'г' => "g",
            'д' => "d",
            'е' | 'э' => "e",
            'ё' => "yo",
            'ж' => "zh",
            'з' => "z",
            'и' => "i",
            'й' | 'ы' => "y",
            'к' => "k",
            'л' => "l",
            'м' => "m",
            'н' => "n",
            'о' => "o",
            'п' => "p",
            'р' => "r",
            'с' => "s",
            'т' => "t",
            'у' => "u",
            'ф' => "f",
            'х' => "kh",
            'ц' => "ts",
            'ч' => "ch",
            'ш' => "sh",
            'щ' => "shch",
            'ъ' | 'ь' => "",
            'ю' => "yu",
            'я' => "ya",
            other => {
                out.push(other);
                continue;
            }
        };
        out.push_str(t);
    }
    out
}

fn with_latin(object: OrbObject, name: &str) -> OrbObject {
    let latin = latin_spelling(name);
    if latin == name.to_lowercase() {
        object
    } else {
        object.keywords(&[latin.as_str()])
    }
}

pub fn build_objects(f: &Facts) -> (Vec<OrbObject>, Context) {
    let mut objects: Vec<OrbObject> = CAPABILITIES
        .iter()
        .map(|(id, class, label, words)| {
            OrbObject::new(id, class.clone(), label, 1).keywords(words)
        })
        .collect();
    for (id, name) in &f.apps {
        let mut o = with_latin(
            OrbObject::new(&format!("app:{id}"), ObjectClass::Application, name, 2)
                .child_of("apps")
                .tier(Tier::Contextual),
            name,
        );
        if !f.appd_connected {
            o = o.unavailable(Unavailable::Offline);
        }
        objects.push(o);
    }
    for (id, name) in &f.spaces {
        let mut o = with_latin(
            OrbObject::new(&format!("space:{id}"), ObjectClass::Environment, name, 2)
                .child_of("spaces"),
            name,
        );
        if !f.entityd_connected {
            o = o.unavailable(Unavailable::Offline);
        }
        objects.push(o);
    }
    for (id, title) in f.tasks.iter().take(TASK_LIMIT) {
        objects.push(with_latin(
            OrbObject::new(&format!("task:{id}"), ObjectClass::Task, title, 2)
                .child_of("tasks")
                .tier(Tier::Suggestion),
            title,
        ));
    }
    for name in &f.bluetooth {
        objects.push(with_latin(
            OrbObject::new(&format!("bt:{name}"), ObjectClass::Device, name, 2)
                .child_of("devices")
                .tier(Tier::Contextual),
            name,
        ));
    }
    let mut ctx = Context {
        space: (!f.selected_space.is_empty()).then(|| f.selected_space.clone()),
        ..Context::default()
    };
    if f.attention {
        ctx.now.insert("inbox".into());
    }
    if f.running {
        ctx.now.insert("tasks".into());
    }
    (objects, ctx)
}

/// Geometry of the resting point inside `viewport`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointGeometry {
    pub disc: (f32, f32, f32),
    pub half_angle: f32,
    pub hit: Rect,
}

pub fn point_geometry(viewport: Rect, unit: f32) -> PointGeometry {
    let stage = Stage::new(viewport.width as f32, viewport.height as f32, 0.0, unit);
    let r = stage.radius(1.0);
    let (cx, cy) = stage.disc_center();
    let exposure = REST_EXPOSURE_UNITS * unit;
    let half_angle = ((r - exposure) / r).clamp(-1.0, 1.0).acos();
    let chord = 2.0 * (r * r - (r - exposure).powi(2)).max(0.0).sqrt();
    let pad = 8.0 * unit;
    let left = (cx - chord / 2.0 - pad).max(0.0);
    let top = (viewport.height as f32 - exposure - pad).max(0.0);
    let hit = Rect::new(
        left as u32,
        viewport.y + top as u32,
        (chord + pad * 2.0).min(viewport.width as f32) as u32,
        (viewport.height as f32 - top) as u32,
    );
    PointGeometry {
        disc: (cx, cy + viewport.y as f32, r),
        half_angle,
        hit,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grab {
    None,
    /// A finger on the resting point: it will raise the sphere.
    Point,
    /// A finger on the point while risen: a tap sinks the sphere.
    Sink,
    /// A finger on the sphere itself.
    Sphere,
}

/// What the shell should do after a touch ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    Redraw,
    Activate(Activation),
}

/// Where the sphere's search stands. The text lives here so that closing the
/// sphere closes the search with it.
pub struct SearchState {
    pub buffer: String,
    pub keyboard: Keyboard,
    /// The hit the trail points at; the best hit until the user moves it.
    target: Option<String>,
}

impl SearchState {
    #[cfg(test)]
    pub fn new_for_test(buffer: String, keyboard: Keyboard) -> SearchState {
        SearchState {
            buffer,
            keyboard,
            target: None,
        }
    }
}

pub const SEARCH_LIMIT: usize = 8;

pub fn compass_ru(c: saai_orb::Compass) -> &'static str {
    use saai_orb::Compass as C;
    match c {
        C::Here => "здесь",
        C::North => "С",
        C::NorthEast => "СВ",
        C::East => "В",
        C::SouthEast => "ЮВ",
        C::South => "Ю",
        C::SouthWest => "ЮЗ",
        C::West => "З",
        C::NorthWest => "СЗ",
    }
}

/// "СВ · 42°" from where the camera looks now, or "здесь" when it is already
/// under the camera.
pub fn route_hint(hit: &Hit) -> String {
    let direction = saai_orb::access::compass(hit.route.from, hit.route.to);
    if direction == saai_orb::Compass::Here {
        return compass_ru(direction).to_string();
    }
    format!(
        "{} · {}°",
        compass_ru(direction),
        hit.route.distance().to_degrees().round() as i32
    )
}
const TRAIL_POINTS: usize = 28;

/// The strip above the sphere while searching: the field, then a fixed
/// number of result rows. The count never changes with the results, so the
/// sphere below does not jump as the text does.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchPanel {
    pub backdrop: Rect,
    pub field: Rect,
    pub rows: Vec<Rect>,
    /// What is left for the sphere.
    pub viewport: Rect,
}

pub fn search_panel(width: u32, top: u32, bottom: u32, rows: usize, unit: f32) -> SearchPanel {
    let u = |v: f32| (v * unit).round() as u32;
    let margin = u(16.0);
    let (field_h, row_h, gap) = (u(52.0), u(48.0), u(4.0));
    let min_viewport = u(160.0);
    let mut rows = rows.max(1);
    loop {
        let field_y = top + u(8.0);
        let rows_y = field_y + field_h + u(8.0);
        let rows_end = rows_y + rows as u32 * (row_h + gap);
        let view_y = rows_end + u(8.0);
        if bottom.saturating_sub(view_y) >= min_viewport || rows == 1 {
            return SearchPanel {
                backdrop: Rect::new(0, 0, width, view_y.min(bottom)),
                field: Rect::new(margin, field_y, width.saturating_sub(margin * 2), field_h),
                rows: (0..rows as u32)
                    .map(|i| {
                        Rect::new(
                            margin,
                            rows_y + i * (row_h + gap),
                            width.saturating_sub(margin * 2),
                            row_h,
                        )
                    })
                    .collect(),
                viewport: Rect::new(0, view_y.min(bottom), width, bottom.saturating_sub(view_y)),
            };
        }
        rows -= 1;
    }
}

#[derive(Debug, Clone)]
struct Flight {
    route: Route,
    elapsed: f32,
}

pub struct OrbSpace {
    geography: Geography,
    geography_dirty: bool,
    pub presence: Presence,
    pub camera: Camera,
    inertia: Inertia,
    pointers: Pointers,
    grab: Grab,
    flight: Option<Flight>,
    entries: Vec<Entry>,
    pub selected: Option<String>,
    search: Option<SearchState>,
    dirty: bool,
}

impl OrbSpace {
    pub fn new(geography: Geography, reduced_motion: bool) -> OrbSpace {
        let mut presence = Presence::resting();
        presence.reduced_motion = reduced_motion;
        OrbSpace {
            geography,
            geography_dirty: false,
            presence,
            camera: Camera::home(),
            inertia: Inertia::default(),
            pointers: Pointers::default(),
            grab: Grab::None,
            flight: None,
            entries: Vec::new(),
            selected: None,
            search: None,
            dirty: false,
        }
    }

    pub fn load() -> OrbSpace {
        let geography = std::fs::read_to_string(GEOGRAPHY_PATH)
            .map(|text| Geography::from_json(&text))
            .unwrap_or_default();
        OrbSpace::new(geography, false)
    }

    /// Atomic replace, so a power cut never leaves half a geography.
    pub fn save_if_changed(&mut self) {
        if !self.geography_dirty {
            return;
        }
        self.geography_dirty = false;
        let tmp = format!("{GEOGRAPHY_PATH}.tmp");
        if std::fs::write(&tmp, self.geography.to_json()).is_ok() {
            let _ = std::fs::rename(&tmp, GEOGRAPHY_PATH);
        }
    }

    #[cfg(test)]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    #[cfg(test)]
    pub fn geography(&self) -> &Geography {
        &self.geography
    }

    pub fn refresh(&mut self, objects: &[OrbObject], ctx: &Context) {
        let c = compose(&mut self.geography, objects, ctx);
        self.geography_dirty |= c.changed;
        self.entries = c.entries;
    }

    pub fn stage(&self, viewport: Rect, unit: f32) -> Stage {
        Stage::new(
            viewport.width as f32,
            viewport.height as f32,
            self.presence.rise(),
            unit,
        )
    }

    pub fn layout(&self, viewport: Rect, unit: f32) -> Layout {
        layout(&self.entries, self.camera, self.stage(viewport, unit))
    }

    /// True while anything wants another frame.
    pub fn needs_frame(&self) -> bool {
        self.dirty
            || self.flight.is_some()
            || self.inertia.is_moving()
            || self.presence.needs_frames()
    }

    pub fn is_pointer_down(&self) -> bool {
        self.pointers.is_down()
    }

    pub fn is_visible(&self) -> bool {
        self.presence.is_visible()
    }

    pub fn is_open(&self) -> bool {
        self.presence.wants_open()
    }

    pub fn set_reduced_motion(&mut self, on: bool) {
        self.presence.reduced_motion = on;
    }

    pub fn dismiss(&mut self) {
        self.search = None;
        self.presence.dismiss();
        self.flight = None;
        self.inertia.stop();
        self.selected = None;
        self.dirty = true;
    }

    /// Opens the search field over the risen sphere. Nothing leaves the
    /// device: it only reads the entries already on the sphere.
    pub fn open_search(&mut self, keyboard: Keyboard) {
        self.presence.summon();
        self.search = Some(SearchState {
            buffer: String::new(),
            keyboard,
            target: None,
        });
        self.selected = None;
        self.dirty = true;
    }

    pub fn close_search(&mut self) {
        if self.search.take().is_some() {
            self.selected = None;
            self.dirty = true;
        }
    }

    pub fn search(&self) -> Option<&SearchState> {
        self.search.as_ref()
    }

    pub fn search_mut(&mut self) -> Option<&mut SearchState> {
        self.search.as_mut()
    }

    /// The text changed: the best hit becomes the target again.
    pub fn search_edited(&mut self) {
        if let Some(s) = self.search.as_mut() {
            s.target = None;
        }
        self.selected = None;
        self.dirty = true;
    }

    pub fn search_hits(&self) -> Vec<Hit> {
        match self.search.as_ref() {
            Some(s) => search_entries(&self.entries, &s.buffer, self.camera, SEARCH_LIMIT),
            None => Vec::new(),
        }
    }

    /// Index of the hit the trail points at.
    pub fn search_focus(&self, hits: &[Hit]) -> Option<usize> {
        let target = self.search.as_ref()?.target.as_deref();
        match target {
            Some(id) => hits.iter().position(|h| h.id == id).or(Some(0)),
            None => (!hits.is_empty()).then_some(0),
        }
        .filter(|i| *i < hits.len())
    }

    /// Moves the trail to the next or previous hit without moving the camera.
    pub fn step_search(&mut self, delta: i32) {
        let hits = self.search_hits();
        let Some(current) = self.search_focus(&hits) else {
            return;
        };
        let next = (current as i32 + delta).clamp(0, hits.len() as i32 - 1) as usize;
        if let Some(s) = self.search.as_mut() {
            s.target = Some(hits[next].id.clone());
        }
        self.dirty = true;
    }

    /// First time: fly there along the route. Once there: open it. A hit
    /// that is a cluster, offline or only remembered is shown, never forced.
    pub fn choose(&mut self, id: &str) -> Outcome {
        let hits = self.search_hits();
        let Some(hit) = hits.iter().find(|h| h.id == id) else {
            return Outcome::Nothing;
        };
        if let Some(s) = self.search.as_mut() {
            s.target = Some(id.to_string());
        }
        self.dirty = true;
        let arrived = self.selected.as_deref() == Some(id) && self.flight.is_none();
        if !arrived {
            self.selected = Some(id.to_string());
            self.inertia.stop();
            self.fly_to(hit.route.clone());
            return Outcome::Redraw;
        }
        match activation_for(id) {
            Some(action) if hit.availability.is_available() && !hit.ghost => {
                Outcome::Activate(action)
            }
            _ => Outcome::Redraw,
        }
    }

    /// The route to the target as a screen polyline of what is on the
    /// visible hemisphere, shortening as the camera arrives.
    pub fn trail(&self, viewport: Rect, unit: f32) -> Vec<(f32, f32)> {
        let hits = self.search_hits();
        let Some(i) = self.search_focus(&hits) else {
            return Vec::new();
        };
        let route = Route {
            from: self.camera.center,
            ..hits[i].route.clone()
        };
        let proj = Projector::new(self.camera, self.stage(viewport, unit));
        route
            .trail(TRAIL_POINTS)
            .into_iter()
            .filter_map(|g| proj.project(g))
            .map(|p| (p.x, p.y))
            .collect()
    }

    fn has_children(&self, id: &str) -> bool {
        self.entries.iter().any(|e| e.parent.as_deref() == Some(id))
    }

    fn fly_to(&mut self, route: Route) {
        if self.presence.reduced_motion {
            self.camera = route.at(1.0);
            self.flight = None;
        } else {
            self.flight = Some(Flight {
                route,
                elapsed: 0.0,
            });
        }
        self.dirty = true;
    }

    pub fn fly_to_entry(&mut self, id: &str, zoom: f32) {
        let Some(e) = self.entries.iter().find(|e| e.id == id) else {
            return;
        };
        let route = Route {
            target: id.to_string(),
            from: self.camera.center,
            to: e.geo,
            zoom_from: self.camera.zoom,
            zoom_to: zoom.clamp(saai_orb::ZOOM_MIN, ZOOM_MAX),
        };
        self.fly_to(route);
    }

    /// A finger came down. Returns whether the Orb claims the touch, in
    /// which case no other surface should see it.
    pub fn touch_down(
        &mut self,
        id: i32,
        pos: (f32, f32),
        t: f32,
        viewport: Rect,
        unit: f32,
    ) -> bool {
        let inside = pos.1 >= viewport.y as f32 && pos.1 < (viewport.y + viewport.height) as f32;
        if self.pointers.is_down() {
            if self.grab == Grab::Sphere {
                self.pointers.down(id, pos.0, pos.1, t);
                return true;
            }
            return self.grab != Grab::None;
        }
        let point = point_geometry(viewport, unit);
        let on_point = point.hit.contains(f64::from(pos.0), f64::from(pos.1));
        let grab = if self.presence.rise() <= 0.0 && !self.presence.wants_open() {
            if on_point {
                Grab::Point
            } else {
                Grab::None
            }
        } else if on_point {
            Grab::Sink
        } else if inside {
            Grab::Sphere
        } else {
            Grab::None
        };
        if grab == Grab::None {
            return false;
        }
        self.grab = grab;
        self.pointers.down(id, pos.0, pos.1, t);
        self.inertia.stop();
        self.flight = None;
        if grab == Grab::Point {
            self.presence.begin_pull(pos.1);
        }
        true
    }

    pub fn touch_motion(&mut self, id: i32, pos: (f32, f32), t: f32, viewport: Rect, unit: f32) {
        let Some(delta) = self.pointers.motion(id, pos.0, pos.1, t) else {
            return;
        };
        match (self.grab, delta) {
            (Grab::Point, Delta::Pan { .. }) => {
                self.presence
                    .pull_to(pos.1, viewport.height as f32 * PULL_TRAVEL);
                self.dirty = true;
            }
            (Grab::Sphere, Delta::Pan { dx, dy }) => {
                let stage = self.stage(viewport, unit);
                if let Motion::Pan { dx, dy } = lower(NavInput::Drag { dx, dy }, unit) {
                    self.camera.pan_px(dx, dy, &stage);
                    self.dirty = true;
                }
            }
            (Grab::Sphere, Delta::Pinch { scale, ax, ay }) => {
                let stage = self.stage(viewport, unit);
                self.camera.zoom_about(scale, ax, ay, &stage);
                self.dirty = true;
            }
            _ => {}
        }
    }

    pub fn touch_up(&mut self, id: i32, t: f32, viewport: Rect, unit: f32) -> Outcome {
        let grab = self.grab;
        let lift = self.pointers.up(id, t, saai_orb::TAP_SLOP_UNITS * unit);
        if self.pointers.is_down() {
            return Outcome::Nothing;
        }
        self.grab = Grab::None;
        self.dirty = true;
        match (grab, lift) {
            (Grab::Point, Lift::Tap { .. }) => {
                self.presence.summon();
                Outcome::Redraw
            }
            (Grab::Point, Lift::Release { vy, .. }) => {
                self.presence.release(vy);
                Outcome::Redraw
            }
            (Grab::Point, _) => {
                self.presence.release(0.0);
                Outcome::Redraw
            }
            (Grab::Sink, Lift::Tap { .. }) => {
                self.dismiss();
                Outcome::Redraw
            }
            (Grab::Sink, Lift::Release { vy, .. }) if vy > 600.0 => {
                self.dismiss();
                Outcome::Redraw
            }
            (Grab::Sphere, Lift::Tap { x, y }) => self.tap_sphere(x, y, viewport, unit),
            (Grab::Sphere, Lift::Release { vx, vy }) => {
                self.inertia = Inertia::fling(vx, vy, unit);
                Outcome::Redraw
            }
            _ => Outcome::Redraw,
        }
    }

    pub fn touch_cancel(&mut self) {
        self.pointers.cancel();
        if self.grab == Grab::Point {
            self.presence.release(0.0);
        }
        self.grab = Grab::None;
        self.dirty = true;
    }

    fn tap_sphere(&mut self, x: f32, y: f32, viewport: Rect, unit: f32) -> Outcome {
        let l = self.layout(viewport, unit);
        let min_target = 24.0 * unit;
        let Some(hit) = l.hit(x, y, min_target) else {
            self.selected = None;
            return Outcome::Redraw;
        };
        let id = hit.id.clone();
        let zoom = self.camera.zoom;
        let dive_zoom = depth_threshold(hit.echelon.saturating_add(1)) * 1.1;
        let children_hidden = self.has_children(&id) && detail(hit.echelon + 1, zoom) < 0.6;
        if children_hidden {
            self.selected = Some(id.clone());
            self.fly_to_entry(&id, dive_zoom.max(zoom));
            return Outcome::Redraw;
        }
        match activation_for(&id) {
            Some(action) if hit.availability.is_available() && !hit.ghost => {
                self.selected = Some(id);
                Outcome::Activate(action)
            }
            _ => {
                self.selected = Some(id);
                Outcome::Redraw
            }
        }
    }

    /// Advance animations by `dt` seconds. Returns whether to repaint.
    pub fn tick(&mut self, dt: f32, viewport: Rect, unit: f32) -> bool {
        let mut repaint = std::mem::take(&mut self.dirty);
        let before = self.presence.rise();
        self.presence.step(dt);
        repaint |= (self.presence.rise() - before).abs() > f32::EPSILON;
        if let Some(f) = self.flight.as_mut() {
            f.elapsed += dt;
            let dur = f.route.duration_s();
            let t = (f.elapsed / dur).min(1.0);
            self.camera = f.route.at(t);
            if t >= 1.0 {
                self.flight = None;
            }
            repaint = true;
        }
        if let Some((dx, dy)) = self.inertia.step(dt, unit) {
            let stage = self.stage(viewport, unit);
            self.camera.pan_px(dx, dy, &stage);
            repaint = true;
        }
        if self.settle_if_lost(viewport, unit) {
            repaint = true;
        }
        repaint
    }

    /// Drifting into empty space is never a dead end: once everything has
    /// come to rest with nothing in view, glide to the nearest object.
    fn settle_if_lost(&mut self, viewport: Rect, unit: f32) -> bool {
        if self.pointers.is_down()
            || self.flight.is_some()
            || self.inertia.is_moving()
            || !self.presence.is_open()
        {
            return false;
        }
        if !self.layout(viewport, unit).is_lost() {
            return false;
        }
        let Some(to) = nearest_anchor(&self.entries, self.camera) else {
            return false;
        };
        self.fly_to(Route {
            target: String::new(),
            from: self.camera.center,
            to,
            zoom_from: self.camera.zoom,
            zoom_to: self.camera.zoom,
        });
        true
    }
}

/// Meridians and parallels for the depth cue, as screen polylines of the
/// visible hemisphere. Pure so it can be tested without a canvas.
pub fn graticule(camera: Camera, stage: Stage) -> Vec<Vec<(f32, f32)>> {
    use saai_orb::Projector;
    let proj = Projector::new(camera, stage);
    let step = 15.0f64;
    let sample = 3.0f64;
    let mut lines = Vec::new();
    let mut walk = |points: &mut dyn Iterator<Item = Geo>| {
        let mut current: Vec<(f32, f32)> = Vec::new();
        for g in points {
            match proj.project(g) {
                Some(p) if p.facing > 0.04 => current.push((p.x, p.y)),
                _ => {
                    if current.len() >= 2 {
                        lines.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                }
            }
        }
        if current.len() >= 2 {
            lines.push(current);
        }
    };
    let mut lon = -180.0;
    while lon < 180.0 {
        let mut pts = (0..=((180.0 / sample) as i32))
            .map(|i| Geo::from_degrees(lon, -90.0 + f64::from(i) * sample));
        walk(&mut pts);
        lon += step;
    }
    let mut lat = -75.0;
    while lat <= 75.0 {
        let mut pts = (0..=((360.0 / sample) as i32))
            .map(|i| Geo::from_degrees(-180.0 + f64::from(i) * sample, lat));
        walk(&mut pts);
        lat += step;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    const VP: Rect = Rect {
        x: 0,
        y: 0,
        width: 1080,
        height: 2000,
    };
    const U: f32 = 3.0;

    fn facts() -> Facts {
        Facts {
            apps: vec![
                ("mail".into(), "Почта".into()),
                ("notes".into(), "Заметки".into()),
            ],
            spaces: vec![
                ("home".into(), "Дом".into()),
                ("work".into(), "Работа".into()),
            ],
            tasks: vec![("t1".into(), "Купить хлеб".into())],
            bluetooth: vec!["Наушники".into()],
            selected_space: "home".into(),
            appd_connected: true,
            entityd_connected: true,
            attention: false,
            running: false,
        }
    }

    fn open_space() -> OrbSpace {
        let mut s = OrbSpace::new(Geography::default(), false);
        let (o, c) = build_objects(&facts());
        s.refresh(&o, &c);
        s.presence.summon();
        for _ in 0..120 {
            s.presence.step(1.0 / 60.0);
        }
        s
    }

    #[test]
    fn capability_order_is_the_permanent_layout_and_is_pinned() {
        let ids: Vec<&str> = CAPABILITIES.iter().map(|c| c.0).collect();
        assert_eq!(
            ids,
            ["search", "sai", "tasks", "apps", "inbox", "spaces", "devices", "me"]
        );
        let (a, _) = build_objects(&Facts::default());
        let (b, _) = build_objects(&facts());
        assert_eq!(a.len(), 8, "no facts, no children: truthful empty clusters");
        assert!(b.len() > a.len());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.id, y.id);
        }
    }

    #[test]
    fn every_object_the_shell_can_build_has_a_real_activation() {
        let (objects, _) = build_objects(&facts());
        for o in &objects {
            assert!(activation_for(&o.id).is_some(), "{}", o.id);
        }
        assert_eq!(
            activation_for("app:mail"),
            Some(Activation::LaunchApp("mail".into()))
        );
        assert_eq!(activation_for("nope"), None);
    }

    #[test]
    fn offline_services_dim_their_objects_instead_of_removing_them() {
        let mut f = facts();
        f.appd_connected = false;
        f.entityd_connected = false;
        let (objects, _) = build_objects(&f);
        let mail = objects.iter().find(|o| o.id == "app:mail").unwrap();
        assert!(!mail.availability.is_available());
        let space = objects.iter().find(|o| o.id == "space:home").unwrap();
        assert!(!space.availability.is_available());
    }

    #[test]
    fn tasks_are_temporary_suggestions_and_never_pile_up_in_the_geography() {
        let mut f = facts();
        f.tasks = (0..20)
            .map(|i| (format!("t{i}"), format!("Дело {i}")))
            .collect();
        let (objects, ctx) = build_objects(&f);
        let tasks: Vec<_> = objects
            .iter()
            .filter(|o| o.id.starts_with("task:"))
            .collect();
        assert_eq!(tasks.len(), TASK_LIMIT);
        assert!(tasks.iter().all(|t| t.tier == Tier::Suggestion));
        let mut s = OrbSpace::new(Geography::default(), false);
        s.refresh(&objects, &ctx);
        assert!(s.geography().place("task:t0").is_none());
    }

    #[test]
    fn context_marks_what_matters_now_without_moving_anything() {
        let mut f = facts();
        let (o1, c1) = build_objects(&f);
        f.attention = true;
        f.running = true;
        let (o2, c2) = build_objects(&f);
        assert!(c2.now.contains("inbox") && c2.now.contains("tasks"));
        let mut s = OrbSpace::new(Geography::default(), false);
        s.refresh(&o1, &c1);
        let before: Vec<_> = s.entries().iter().map(|e| (e.id.clone(), e.geo)).collect();
        s.refresh(&o2, &c2);
        for (id, geo) in before {
            assert_eq!(s.entries().iter().find(|e| e.id == id).unwrap().geo, geo);
        }
    }

    #[test]
    fn point_sits_at_the_bottom_centre_and_is_a_small_target() {
        let g = point_geometry(VP, U);
        assert!((g.disc.0 - 540.0).abs() < 1.0);
        assert!(g.hit.contains(540.0, 1990.0));
        assert!(!g.hit.contains(40.0, 1990.0));
        assert!(
            !g.hit.contains(540.0, 1700.0),
            "the rest of the page keeps its taps"
        );
        assert!(g.hit.width < VP.width / 2);
        assert!(g.half_angle > 0.5 && g.half_angle < 1.5);
        assert!(g.hit.y + g.hit.height <= VP.y + VP.height);
    }

    #[test]
    fn nothing_claims_a_touch_while_resting_unless_it_lands_on_the_point() {
        let mut s = OrbSpace::new(Geography::default(), false);
        assert!(!s.touch_down(1, (100.0, 300.0), 0.0, VP, U));
        assert!(s.touch_down(1, (540.0, 1990.0), 0.0, VP, U));
    }

    #[test]
    fn tapping_the_point_raises_the_sphere_and_tapping_it_again_sinks_it() {
        let mut s = OrbSpace::new(Geography::default(), false);
        assert!(s.touch_down(1, (540.0, 1990.0), 0.0, VP, U));
        assert_eq!(s.touch_up(1, 0.05, VP, U), Outcome::Redraw);
        assert!(s.is_open());
        for _ in 0..90 {
            s.tick(1.0 / 60.0, VP, U);
        }
        assert!(s.presence.is_open());
        assert!(s.touch_down(1, (540.0, 1990.0), 1.0, VP, U));
        s.touch_up(1, 1.05, VP, U);
        assert!(!s.is_open());
    }

    #[test]
    fn pulling_up_follows_the_finger_and_a_short_pull_settles_back() {
        let mut s = OrbSpace::new(Geography::default(), false);
        s.touch_down(1, (540.0, 1990.0), 0.0, VP, U);
        s.touch_motion(1, (540.0, 1990.0 - 500.0), 0.1, VP, U);
        assert!(
            (s.presence.rise() - 0.5).abs() < 0.01,
            "{}",
            s.presence.rise()
        );
        s.touch_up(1, 0.6, VP, U);
        assert!(s.is_open());

        let mut t = OrbSpace::new(Geography::default(), false);
        t.touch_down(1, (540.0, 1990.0), 0.0, VP, U);
        t.touch_motion(1, (540.0, 1990.0 - 120.0), 0.1, VP, U);
        t.touch_up(1, 1.0, VP, U);
        assert!(!t.is_open());
    }

    #[test]
    fn dragging_the_risen_sphere_pans_it_and_a_flick_keeps_gliding() {
        let mut s = open_space();
        let start = s.camera.center;
        assert!(s.touch_down(1, (540.0, 900.0), 0.0, VP, U));
        let mut x = 540.0;
        for i in 1..=6 {
            x += 40.0;
            s.touch_motion(1, (x, 900.0), 0.016 * i as f32, VP, U);
        }
        assert!(s.camera.center != start);
        s.touch_up(1, 0.1, VP, U);
        let after_release = s.camera.center;
        for _ in 0..10 {
            s.tick(1.0 / 60.0, VP, U);
        }
        assert!(s.camera.center != after_release, "momentum");
    }

    #[test]
    fn pinching_zooms_and_never_leaves_the_range() {
        let mut s = open_space();
        s.touch_down(1, (400.0, 900.0), 0.0, VP, U);
        s.touch_down(2, (600.0, 900.0), 0.0, VP, U);
        for i in 1..=30 {
            s.touch_motion(2, (600.0 + 20.0 * i as f32, 900.0), 0.01 * i as f32, VP, U);
        }
        assert!(s.camera.zoom > 1.0);
        assert!(s.camera.zoom <= ZOOM_MAX);
    }

    #[test]
    fn tapping_a_cluster_dives_into_it_and_tapping_a_leaf_opens_it() {
        let mut s = open_space();
        let apps = s.entries().iter().find(|e| e.id == "apps").unwrap().geo;
        s.camera = Camera {
            center: apps,
            zoom: 0.9,
        };
        let l = s.layout(VP, U);
        let it = l.items.iter().find(|i| i.id == "apps").unwrap();
        s.touch_down(1, (it.x, it.y), 0.0, VP, U);
        let out = s.touch_up(1, 0.05, VP, U);
        assert_eq!(
            out,
            Outcome::Redraw,
            "children are hidden: dive, don't open"
        );
        assert!(s.needs_frame());
        for _ in 0..120 {
            s.tick(1.0 / 60.0, VP, U);
        }
        assert!(s.camera.zoom >= depth_threshold(2));
        let l = s.layout(VP, U);
        let mail = l.items.iter().find(|i| i.id == "app:mail").unwrap();
        s.touch_down(1, (mail.x, mail.y), 1.0, VP, U);
        assert_eq!(
            s.touch_up(1, 1.05, VP, U),
            Outcome::Activate(Activation::LaunchApp("mail".into()))
        );
        let apps_item = s
            .layout(VP, U)
            .items
            .iter()
            .find(|i| i.id == "apps")
            .cloned();
        if let Some(it) = apps_item {
            s.touch_down(1, (it.x, it.y), 2.0, VP, U);
            assert_eq!(
                s.touch_up(1, 2.05, VP, U),
                Outcome::Activate(Activation::OpenApps),
                "once the apps are in view, the cluster itself opens"
            );
        }
    }

    #[test]
    fn unavailable_objects_are_selected_but_never_activated() {
        let mut f = facts();
        f.appd_connected = false;
        let mut s = OrbSpace::new(Geography::default(), false);
        let (o, c) = build_objects(&f);
        s.refresh(&o, &c);
        s.presence.summon();
        for _ in 0..90 {
            s.presence.step(1.0 / 60.0);
        }
        let apps = s.entries().iter().find(|e| e.id == "apps").unwrap().geo;
        s.camera = Camera {
            center: apps,
            zoom: 1.8,
        };
        let l = s.layout(VP, U);
        let mail = l.items.iter().find(|i| i.id == "app:mail").unwrap();
        s.touch_down(1, (mail.x, mail.y), 0.0, VP, U);
        assert_eq!(s.touch_up(1, 0.05, VP, U), Outcome::Redraw);
        assert_eq!(s.selected.as_deref(), Some("app:mail"));
    }

    #[test]
    fn drifting_into_empty_space_glides_back_to_the_nearest_object() {
        let mut s = open_space();
        s.camera = Camera {
            center: Geo::from_degrees(150.0, 0.0),
            zoom: 1.0,
        };
        assert!(s.layout(VP, U).is_lost());
        for _ in 0..240 {
            s.tick(1.0 / 60.0, VP, U);
        }
        assert!(
            !s.layout(VP, U).is_lost(),
            "settled where something is in view"
        );
    }

    #[test]
    fn reduced_motion_jumps_instead_of_flying() {
        let mut s = open_space();
        s.set_reduced_motion(true);
        s.fly_to_entry("me", 1.0);
        assert!(s.flight.is_none());
        let me = s.entries().iter().find(|e| e.id == "me").unwrap().geo;
        assert!(s.camera.center.distance(me) < 1e-9);
    }

    #[test]
    fn touches_outside_the_viewport_are_left_to_the_navigation_strip() {
        let mut s = open_space();
        let vp = Rect::new(0, 0, 1080, 2000);
        assert!(!s.touch_down(1, (200.0, 2100.0), 0.0, vp, U));
    }

    #[test]
    fn cancel_leaves_a_consistent_state() {
        let mut s = OrbSpace::new(Geography::default(), false);
        s.touch_down(1, (540.0, 1990.0), 0.0, VP, U);
        s.touch_motion(1, (540.0, 1700.0), 0.05, VP, U);
        s.touch_cancel();
        assert!(!s.presence.is_pulling());
        assert!(s.touch_down(2, (540.0, 1990.0), 1.0, VP, U));
    }

    #[test]
    fn graticule_lines_stay_on_the_visible_hemisphere() {
        let stage = Stage::new(1080.0, 2000.0, 1.0, U);
        let lines = graticule(Camera::home(), stage);
        assert!(lines.len() > 10);
        let (cx, cy, r) = {
            let p = saai_orb::Projector::new(Camera::home(), stage);
            p.disc()
        };
        for line in &lines {
            for (x, y) in line {
                assert!((x - cx).hypot(y - cy) <= r + 1.0);
            }
        }
    }

    fn typed(s: &mut OrbSpace, text: &str) {
        s.open_search(Keyboard::bind(
            "orb-search",
            saai_ui_core::KeyboardLayout::Qwerty,
        ));
        s.search_mut().unwrap().buffer = text.into();
        s.search_edited();
    }

    fn settle(s: &mut OrbSpace) {
        for _ in 0..120 {
            s.tick(1.0 / 60.0, VP, U);
        }
    }

    #[test]
    fn cyrillic_names_can_be_found_by_typing_latin() {
        assert_eq!(latin_spelling("Почта"), "pochta");
        assert_eq!(latin_spelling("Щётка"), "shchyotka");
        assert_eq!(latin_spelling("Wi-Fi 6"), "wi-fi 6");
        let mut s = open_space();
        typed(&mut s, "pochta");
        let hits = s.search_hits();
        assert_eq!(hits.first().map(|h| h.id.as_str()), Some("app:mail"));
        typed(&mut s, "bluetooth");
        assert_eq!(s.search_hits()[0].id, "devices");
    }

    #[test]
    fn search_finds_but_does_not_move_until_asked() {
        let mut s = open_space();
        let before = s.camera;
        typed(&mut s, "pochta");
        assert_eq!(s.camera, before, "typing never moves the camera");
        assert!(!s.trail(VP, U).is_empty() || s.search_hits()[0].route.distance() > 1.2);
        assert!(s.search_focus(&s.search_hits()).is_some());
    }

    #[test]
    fn choosing_flies_along_the_route_then_opens_on_the_second_choice() {
        let mut s = open_space();
        typed(&mut s, "pochta");
        let id = s.search_hits()[0].id.clone();
        assert_eq!(s.choose(&id), Outcome::Redraw);
        assert_eq!(s.selected.as_deref(), Some("app:mail"));
        assert!(s.needs_frame());
        assert_eq!(
            s.choose(&id),
            Outcome::Redraw,
            "still flying: a second press does not skip the journey"
        );
        settle(&mut s);
        let target = s.entries().iter().find(|e| e.id == "app:mail").unwrap().geo;
        assert!(s.camera.center.distance(target) < 0.01, "arrived");
        assert_eq!(
            s.choose(&id),
            Outcome::Activate(Activation::LaunchApp("mail".into()))
        );
    }

    #[test]
    fn the_trail_shortens_as_the_camera_arrives() {
        let mut s = open_space();
        typed(&mut s, "pochta");
        let far = s.search_hits()[0].route.distance();
        let id = s.search_hits()[0].id.clone();
        s.choose(&id);
        settle(&mut s);
        let near = s.search_hits()[0].route.distance();
        assert!(near < far.max(0.02));
        assert!(near < 0.01);
    }

    #[test]
    fn offline_and_remembered_objects_are_found_and_shown_but_not_opened() {
        let mut off = facts();
        off.appd_connected = false;
        let mut s = OrbSpace::new(Geography::default(), false);
        let (o, c) = build_objects(&off);
        s.refresh(&o, &c);
        s.presence.summon();
        for _ in 0..120 {
            s.presence.step(1.0 / 60.0);
        }
        typed(&mut s, "pochta");
        let id = s.search_hits()[0].id.clone();
        assert_eq!(
            s.search_hits()[0].id,
            "app:mail",
            "an offline app still exists"
        );
        s.choose(&id);
        settle(&mut s);
        assert_eq!(s.choose(&id), Outcome::Redraw, "shown, never forced open");
    }

    #[test]
    fn stepping_moves_the_trail_to_the_next_hit_without_flying() {
        let mut s = open_space();
        typed(&mut s, "a");
        let hits = s.search_hits();
        assert!(hits.len() >= 2);
        let camera = s.camera;
        assert_eq!(s.search_focus(&hits), Some(0));
        s.step_search(1);
        assert_eq!(s.search_focus(&s.search_hits()), Some(1));
        s.step_search(-5);
        assert_eq!(s.search_focus(&s.search_hits()), Some(0));
        s.step_search(100);
        assert_eq!(
            s.search_focus(&s.search_hits()),
            Some(s.search_hits().len() - 1)
        );
        assert_eq!(s.camera, camera);
    }

    #[test]
    fn no_text_and_no_match_are_honestly_empty() {
        let mut s = open_space();
        typed(&mut s, "");
        assert!(s.search_hits().is_empty());
        assert!(s.trail(VP, U).is_empty());
        typed(&mut s, "zzzzqqqq");
        assert!(s.search_hits().is_empty());
        assert_eq!(s.search_focus(&[]), None);
        assert_eq!(s.choose("search"), Outcome::Nothing);
    }

    #[test]
    fn hints_say_which_way_and_how_far() {
        let mut s = open_space();
        typed(&mut s, "pochta");
        let hit = s.search_hits().remove(0);
        let hint = route_hint(&hit);
        assert!(hint == "здесь" || hint.ends_with('°'), "{hint}");
        let mut here = hit.clone();
        here.route.from = here.route.to;
        assert_eq!(route_hint(&here), "здесь");
        let mut east = hit.clone();
        east.route.from = Geo::from_degrees(0.0, 0.0);
        east.route.to = Geo::from_degrees(40.0, 0.0);
        assert_eq!(route_hint(&east), "В · 40°");
    }

    #[test]
    fn dismissing_the_sphere_closes_the_search() {
        let mut s = open_space();
        typed(&mut s, "pochta");
        s.dismiss();
        assert!(s.search().is_none());
        assert!(s.search_hits().is_empty());
    }

    #[test]
    fn search_panel_reserves_a_fixed_strip_and_leaves_the_rest_to_the_sphere() {
        let p3 = search_panel(1080, 100, 1700, 3, U);
        let p3_again = search_panel(1080, 100, 1700, 3, U);
        assert_eq!(p3, p3_again);
        assert_eq!(p3.rows.len(), 3);
        assert!(p3.field.y >= 100);
        assert!(p3.rows[0].y >= p3.field.y + p3.field.height);
        assert!(p3.rows[1].y >= p3.rows[0].y + p3.rows[0].height);
        assert_eq!(p3.viewport.y, p3.backdrop.height);
        assert_eq!(p3.viewport.y + p3.viewport.height, 1700);
        assert!(p3.rows.iter().all(|r| r.height >= (44.0 * U) as u32));
        let p4 = search_panel(1080, 100, 1700, 4, U);
        assert!(p4.viewport.y > p3.viewport.y);
    }

    #[test]
    fn search_panel_gives_up_rows_before_it_squeezes_the_sphere_out() {
        let tight = search_panel(1080, 100, 1200, 4, U);
        assert!(tight.rows.len() < 4);
        assert!(tight.viewport.height >= (160.0 * U) as u32);
        let hopeless = search_panel(1080, 100, 300, 4, U);
        assert_eq!(hopeless.rows.len(), 1);
    }
}
