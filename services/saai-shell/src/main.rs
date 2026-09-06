use anyhow::{bail, Context, Result};
use memmap2::{MmapMut, MmapOptions};
use saai_shell::{frame_hash, shell_frame, FRAME_HEIGHT, FRAME_STRIDE, FRAME_WIDTH};
use std::{os::fd::AsFd, time::Instant};
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer, wl_compositor::WlCompositor, wl_registry::WlRegistry, wl_shm::WlShm,
        wl_shm_pool::WlShmPool, wl_surface::WlSurface,
    },
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::shell::client::{
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::{self, XdgWmBase},
};

struct ShellState {
    shm: WlShm,
    surface: WlSurface,
    xdg_surface: XdgSurface,
    configured: bool,
    closed: bool,
    pixels: Option<MmapMut>,
    submitted_hash: Option<String>,
}

impl ShellState {
    fn submit_frame(&mut self, qh: &QueueHandle<Self>) -> Result<()> {
        let frame = shell_frame();
        let size = frame.len();
        let file = tempfile::tempfile().context("create shell wl_shm file")?;
        file.set_len(size as u64)
            .context("size shell wl_shm file")?;
        let mut map =
            unsafe { MmapOptions::new().len(size).map_mut(&file) }.context("map shell frame")?;
        map.copy_from_slice(&frame);
        map.flush().context("flush shell frame")?;

        let pool = self.shm.create_pool(file.as_fd(), size as i32, qh, ());
        let buffer = pool.create_buffer(
            0,
            FRAME_WIDTH as i32,
            FRAME_HEIGHT as i32,
            FRAME_STRIDE as i32,
            wayland_client::protocol::wl_shm::Format::Xrgb8888,
            qh,
            (),
        );
        pool.destroy();
        self.surface.attach(Some(&buffer), 0, 0);
        self.surface
            .damage_buffer(0, 0, FRAME_WIDTH as i32, FRAME_HEIGHT as i32);
        self.surface.commit();
        self.submitted_hash = Some(frame_hash(&frame));
        self.pixels = Some(map);
        Ok(())
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for ShellState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &GlobalListContents,
        _connection: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

wayland_client::delegate_noop!(ShellState: ignore WlCompositor);
wayland_client::delegate_noop!(ShellState: ignore WlShm);
wayland_client::delegate_noop!(ShellState: ignore WlShmPool);
wayland_client::delegate_noop!(ShellState: ignore WlBuffer);
wayland_client::delegate_noop!(ShellState: ignore WlSurface);

impl Dispatch<XdgWmBase, ()> for ShellState {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<XdgSurface, ()> for ShellState {
    fn event(
        state: &mut Self,
        _proxy: &XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _connection: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            state.xdg_surface.ack_configure(serial);
            state
                .submit_frame(qh)
                .expect("submit configured shell frame");
            state.configured = true;
        }
    }
}

impl Dispatch<XdgToplevel, ()> for ShellState {
    fn event(
        state: &mut Self,
        _proxy: &XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Close = event {
            state.closed = true;
        }
    }
}

fn main() -> Result<()> {
    let connection = Connection::connect_to_env().context("connect shell to Wayland compositor")?;
    let (globals, mut event_queue) =
        registry_queue_init::<ShellState>(&connection).context("read Wayland globals")?;
    let qh = event_queue.handle();
    let compositor = globals
        .bind::<WlCompositor, _, _>(&qh, 4..=6, ())
        .context("bind wl_compositor")?;
    let shm = globals
        .bind::<WlShm, _, _>(&qh, 1..=1, ())
        .context("bind required wl_shm")?;
    let wm_base = globals
        .bind::<XdgWmBase, _, _>(&qh, 1..=6, ())
        .context("bind xdg_wm_base")?;
    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("SaaiOS Shell".to_owned());
    toplevel.set_app_id("org.saaios.shell".to_owned());

    let mut state = ShellState {
        shm,
        surface,
        xdg_surface,
        configured: false,
        closed: false,
        pixels: None,
        submitted_hash: None,
    };
    event_queue.roundtrip(&mut state)?;
    state.surface.commit();

    let started = Instant::now();
    loop {
        event_queue
            .blocking_dispatch(&mut state)
            .context("dispatch shell Wayland event")?;
        if state.closed {
            break;
        }
        if started.elapsed().as_secs() >= 5 {
            bail!("shell timed out");
        }
    }

    let hash = state
        .submitted_hash
        .context("compositor closed shell before frame submission")?;
    if !state.configured {
        bail!("shell exited without configure");
    }
    println!("SHELL_OK configured=true transport=wl_shm hash={hash}");
    Ok(())
}
