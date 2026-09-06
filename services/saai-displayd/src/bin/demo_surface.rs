use anyhow::{bail, Context, Result};
use memmap2::{MmapMut, MmapOptions};
use saai_displayd::{demo_frame, FRAME_HEIGHT, FRAME_STRIDE, FRAME_WIDTH};
use std::{os::fd::AsFd, time::Instant};
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer, wl_compositor::WlCompositor, wl_keyboard, wl_keyboard::WlKeyboard,
        wl_registry::WlRegistry, wl_seat, wl_seat::WlSeat, wl_shm::WlShm, wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols::xdg::shell::client::{
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::{self, XdgWmBase},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Valid,
    InvalidSerial,
    InvalidBuffer,
    InvalidRole,
    Crash,
}

struct ClientState {
    mode: Mode,
    shm: WlShm,
    surface: WlSurface,
    xdg_surface: XdgSurface,
    configured: bool,
    key_received: bool,
    closed: bool,
    pixels: Option<MmapMut>,
}

impl ClientState {
    fn submit_frame(&mut self, qh: &QueueHandle<Self>) -> Result<()> {
        let size = (FRAME_STRIDE * FRAME_HEIGHT) as usize;
        let file = tempfile::tempfile().context("create wl_shm file")?;
        file.set_len(size as u64).context("size wl_shm file")?;
        let mut map =
            unsafe { MmapOptions::new().len(size).map_mut(&file) }.context("map wl_shm file")?;
        map.copy_from_slice(&demo_frame());
        map.flush().context("flush demo pixels")?;

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
        self.pixels = Some(map);
        Ok(())
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for ClientState {
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

wayland_client::delegate_noop!(ClientState: ignore WlCompositor);
wayland_client::delegate_noop!(ClientState: ignore WlShm);
wayland_client::delegate_noop!(ClientState: ignore WlShmPool);
wayland_client::delegate_noop!(ClientState: ignore WlBuffer);
wayland_client::delegate_noop!(ClientState: ignore WlSurface);

impl Dispatch<XdgWmBase, ()> for ClientState {
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

impl Dispatch<XdgSurface, ()> for ClientState {
    fn event(
        state: &mut Self,
        _proxy: &XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _connection: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            let ack = if state.mode == Mode::InvalidSerial {
                serial.wrapping_add(1)
            } else {
                serial
            };
            state.xdg_surface.ack_configure(ack);
            if state.mode == Mode::Valid {
                state.submit_frame(qh).expect("submit configured frame");
                state.configured = true;
            }
        }
    }
}

impl Dispatch<WlSeat, ()> for ClientState {
    fn event(
        _state: &mut Self,
        proxy: &WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _connection: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            if capabilities
                .into_result()
                .is_ok_and(|caps| caps.contains(wl_seat::Capability::Keyboard))
            {
                proxy.get_keyboard(qh, ());
            }
        }
    }
}

impl Dispatch<WlKeyboard, ()> for ClientState {
    fn event(
        state: &mut Self,
        _proxy: &WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key {
            key,
            state: WEnum::Value(wl_keyboard::KeyState::Pressed),
            ..
        } = event
        {
            if key == 30 {
                state.key_received = true;
            }
        }
    }
}

impl Dispatch<XdgToplevel, ()> for ClientState {
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

fn mode_from_args() -> Result<Mode> {
    let Some(arg) = std::env::args().nth(1) else {
        return Ok(Mode::Valid);
    };
    match arg.as_str() {
        "--invalid-serial" => Ok(Mode::InvalidSerial),
        "--invalid-buffer" => Ok(Mode::InvalidBuffer),
        "--invalid-role" => Ok(Mode::InvalidRole),
        "--crash" => Ok(Mode::Crash),
        _ => bail!("unknown argument: {arg}"),
    }
}

fn main() -> Result<()> {
    let mode = mode_from_args()?;
    let connection = Connection::connect_to_env().context("connect to Wayland compositor")?;
    let (globals, mut event_queue) =
        registry_queue_init::<ClientState>(&connection).context("read Wayland globals")?;
    let qh = event_queue.handle();
    let compositor = globals
        .bind::<WlCompositor, _, _>(&qh, 4..=6, ())
        .context("bind wl_compositor")?;
    let shm = globals
        .bind::<WlShm, _, _>(&qh, 1..=1, ())
        .context("bind wl_shm")?;
    let _seat = globals
        .bind::<WlSeat, _, _>(&qh, 1..=7, ())
        .context("bind wl_seat")?;
    let wm_base = globals
        .bind::<XdgWmBase, _, _>(&qh, 1..=6, ())
        .context("bind xdg_wm_base")?;
    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("SaaiOS S01 deterministic demo".to_owned());
    toplevel.set_app_id("org.saaios.demo-surface".to_owned());

    let mut state = ClientState {
        mode,
        shm,
        surface,
        xdg_surface,
        configured: false,
        key_received: false,
        closed: false,
        pixels: None,
    };
    event_queue.roundtrip(&mut state)?;

    match mode {
        Mode::Crash => std::process::exit(23),
        Mode::InvalidRole => {
            wm_base.get_xdg_surface(&state.surface, &qh, ());
            state.surface.commit();
        }
        Mode::InvalidBuffer => {
            state.submit_frame(&qh)?;
        }
        Mode::Valid | Mode::InvalidSerial => state.surface.commit(),
    }

    let started = Instant::now();
    loop {
        match event_queue.blocking_dispatch(&mut state) {
            Ok(_) if state.closed => break,
            Ok(_) if started.elapsed().as_secs() < 5 => {}
            Ok(_) => bail!("client timed out"),
            Err(error) if mode != Mode::Valid => {
                eprintln!("EXPECTED_PROTOCOL_DISCONNECT mode={mode:?} error={error}");
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        }
    }
    if !state.configured || !state.key_received {
        bail!(
            "incomplete lifecycle: configured={}, key_received={}",
            state.configured,
            state.key_received
        );
    }
    println!("CLIENT_OK configure=true input=true close=true");
    Ok(())
}
