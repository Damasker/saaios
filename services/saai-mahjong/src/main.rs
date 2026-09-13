use std::io::Write;
use std::os::fd::AsFd;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{
        wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface,
        wl_touch,
    },
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

mod install;
mod render;

use render::{BoardLayout, Tile};

const COLUMNS: u32 = 6;
const ROWS: u32 = 4;
const PAIR_COUNT: usize = (COLUMNS * ROWS / 2) as usize;
const DEFAULT_WIDTH: i32 = 1080;
const DEFAULT_HEIGHT: i32 = 2400;

/// A tiny xorshift64* PRNG seeded from the wall clock -- this app has
/// exactly one use for randomness (shuffling the board), so pulling
/// in the `rand` crate for it isn't worth the dependency, the same
/// "avoid a heavy crate for three lines of use" reasoning `saai-
/// shell` already applies elsewhere in this project (e.g. `system-
/// tools` for S14's device info).
struct Rng(u64);

impl Rng {
    fn seeded() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15);
        Self(nanos | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next_u64() as usize) % (i + 1);
            items.swap(i, j);
        }
    }
}

fn new_board() -> Vec<Tile> {
    let mut symbols: Vec<u8> = (0..PAIR_COUNT)
        .flat_map(|pair| [b'A' + pair as u8, b'A' + pair as u8])
        .collect();
    Rng::seeded().shuffle(&mut symbols);
    symbols
        .into_iter()
        .map(|symbol| Tile {
            symbol,
            removed: false,
        })
        .collect()
}

struct AppState {
    running: bool,
    configured: bool,
    configured_width: i32,
    configured_height: i32,
    touch: Option<wl_touch::WlTouch>,
    touch_started: bool,
    touch_position: (f64, f64),
    surface: Option<wl_surface::WlSurface>,
    shm: Option<wl_shm::WlShm>,
    tiles: Vec<Tile>,
    selected: Option<usize>,
    needs_redraw: bool,
}

impl AppState {
    fn won(&self) -> bool {
        self.tiles.iter().all(|tile| tile.removed)
    }

    /// A tap resolves to exactly one of: a live tile, "Новая игра",
    /// or "Закрыть" -- never more than one, since none of these three
    /// regions overlap (see `render::BoardLayout`/`restart_button_
    /// rect`/`close_button_rect`).
    fn handle_tap(&mut self, position: (f64, f64)) {
        let width = self.configured_width.max(1) as u32;
        let height = self.configured_height.max(1) as u32;
        let layout = BoardLayout::compute(width, height, COLUMNS, ROWS);

        if let Some(index) = layout.tile_at(position.0, position.1, self.tiles.len()) {
            if self.tiles[index].removed {
                return;
            }
            match self.selected {
                None => self.selected = Some(index),
                Some(current) if current == index => self.selected = None,
                Some(current) => {
                    if self.tiles[current].symbol == self.tiles[index].symbol {
                        self.tiles[current].removed = true;
                        self.tiles[index].removed = true;
                    }
                    self.selected = None;
                }
            }
            self.needs_redraw = true;
            return;
        }

        let (rx, ry, rw, rh) = render::restart_button_rect(width, height);
        if in_rect(position, rx, ry, rw, rh) {
            self.tiles = new_board();
            self.selected = None;
            self.needs_redraw = true;
            return;
        }

        let (cx, cy, cw, ch) = render::close_button_rect(width, height);
        if in_rect(position, cx, cy, cw, ch) {
            println!("saai-mahjong: close tapped, exiting");
            self.running = false;
        }
    }
}

fn in_rect(position: (f64, f64), x: u32, y: u32, w: u32, h: u32) -> bool {
    position.0 >= f64::from(x)
        && position.0 < f64::from(x + w)
        && position.1 >= f64::from(y)
        && position.1 < f64::from(y + h)
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for AppState {
    fn event(
        _state: &mut Self,
        proxy: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            proxy.ack_configure(serial);
            if !state.configured {
                state.configured = true;
                println!("saai-mahjong: received first configure, drawing board");
            }
            state.needs_redraw = true;
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Close => {
                println!("saai-mahjong: compositor requested close");
                state.running = false;
            }
            xdg_toplevel::Event::Configure { width, height, .. } if width > 0 && height > 0 => {
                state.configured_width = width;
                state.configured_height = height;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: wayland_client::WEnum::Value(caps),
        } = event
        {
            if caps.contains(wl_seat::Capability::Touch) && state.touch.is_none() {
                state.touch = Some(proxy.get_touch(qh, ()));
            }
        }
    }
}

impl Dispatch<wl_touch::WlTouch, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &wl_touch::WlTouch,
        event: wl_touch::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_touch::Event::Down { x, y, .. } => {
                state.touch_started = true;
                state.touch_position = (x, y);
            }
            wl_touch::Event::Motion { x, y, .. } => state.touch_position = (x, y),
            wl_touch::Event::Up { .. } if state.touch_started => {
                state.touch_started = false;
                state.handle_tap(state.touch_position);
            }
            wl_touch::Event::Cancel => state.touch_started = false,
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        _event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(AppState: ignore wl_compositor::WlCompositor);
delegate_noop!(AppState: ignore wl_surface::WlSurface);
delegate_noop!(AppState: ignore wl_shm::WlShm);
delegate_noop!(AppState: wl_shm_pool::WlShmPool);
delegate_noop!(AppState: ignore wayland_client::protocol::wl_buffer::WlBuffer);

/// Builds a fresh `wl_shm` pool/buffer for the current board state and
/// commits it. Not double-buffered (a new tempfile/pool per redraw,
/// same shape as `saai-demo-surface`'s own `attach_test_pattern`) --
/// a tile game redraws on taps, not every frame, so the extra
/// allocation per tap is not a real cost.
fn redraw(state: &AppState, shm: &wl_shm::WlShm, surface: &wl_surface::WlSurface, qh: &QueueHandle<AppState>) {
    let width = state.configured_width.max(1);
    let height = state.configured_height.max(1);
    let stride = width * 4;
    let size = (stride * height) as usize;

    let mut pixels = vec![0u8; size];
    let layout = BoardLayout::compute(width as u32, height as u32, COLUMNS, ROWS);
    render::draw_board(
        &mut pixels,
        width as u32,
        height as u32,
        &state.tiles,
        state.selected,
        &layout,
        state.won(),
    );

    let mut file = tempfile::tempfile().expect("failed to create anonymous shm file");
    file.write_all(&pixels).expect("failed to write pixel data");
    file.flush().ok();

    let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
    let buffer = pool.create_buffer(0, width, height, stride, wl_shm::Format::Xrgb8888, qh, ());

    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, width, height);
    surface.commit();
}

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--install") {
        std::process::exit(if install::run(args) { 0 } else { 1 });
    }

    let conn = Connection::connect_to_env().expect(
        "failed to connect to Wayland display -- set WAYLAND_DISPLAY to saai-displayd's socket",
    );
    let (globals, mut queue) =
        registry_queue_init::<AppState>(&conn).expect("registry init failed");
    let qh = queue.handle();

    let compositor: wl_compositor::WlCompositor = globals
        .bind(&qh, 1..=6, ())
        .expect("wl_compositor not advertised");
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).expect("wl_shm not advertised");
    let wm_base: xdg_wm_base::XdgWmBase = globals
        .bind(&qh, 1..=6, ())
        .expect("xdg_wm_base not advertised");
    let _seat: wl_seat::WlSeat = globals
        .bind(&qh, 1..=1, ())
        .expect("wl_seat not advertised");

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("Маджонг".to_string());
    toplevel.set_app_id("org.saaios.mahjong".to_string());

    let mut state = AppState {
        running: true,
        configured: false,
        configured_width: DEFAULT_WIDTH,
        configured_height: DEFAULT_HEIGHT,
        touch: None,
        touch_started: false,
        touch_position: (0.0, 0.0),
        surface: Some(surface),
        shm: Some(shm),
        tiles: new_board(),
        selected: None,
        needs_redraw: false,
    };

    // Initial commit with no buffer attached triggers the first
    // configure, per xdg-shell's configure/ack/commit lifecycle.
    state.surface.as_ref().unwrap().commit();

    let deadline = Instant::now() + Duration::from_secs(8);
    while state.running && !state.configured && Instant::now() < deadline {
        queue.blocking_dispatch(&mut state).expect("dispatch failed");
    }
    if !state.configured {
        eprintln!("saai-mahjong: no configure received within 8s, exiting");
        return;
    }

    while state.running {
        if state.needs_redraw {
            state.needs_redraw = false;
            let shm = state.shm.as_ref().unwrap().clone();
            let surface = state.surface.as_ref().unwrap().clone();
            redraw(&state, &shm, &surface, &qh);
        }
        queue
            .blocking_dispatch(&mut state)
            .expect("dispatch failed");
    }

    println!("saai-mahjong: exiting cleanly");
}
