use anyhow::{bail, Context, Result};
use saai_displayd::{demo_frame, demo_frame_with_touch, FRAME_HEIGHT, FRAME_STRIDE, FRAME_WIDTH};
use serde::Serialize;
use std::{
    fs::File,
    io::Write,
    os::{fd::AsFd, unix::fs::FileExt},
};
use wayland_client::{
    delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface,
    },
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

#[derive(Debug, Default)]
struct Args {
    observer: bool,
    unconfigured_buffer: bool,
    duplicate_role: bool,
    crash_before_buffer: bool,
    reuse_role: bool,
    interactive: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut args = Self::default();
        for flag in std::env::args().skip(1) {
            match flag.as_str() {
                "--observer" => args.observer = true,
                "--unconfigured-buffer" => args.unconfigured_buffer = true,
                "--duplicate-role" => args.duplicate_role = true,
                "--crash-before-buffer" => args.crash_before_buffer = true,
                "--reuse-role" => args.reuse_role = true,
                "--interactive" => args.interactive = true,
                _ => bail!("unknown argument {flag}"),
            }
        }
        Ok(args)
    }
}

#[derive(Debug, Serialize)]
struct ClientReport {
    schema: u32,
    role: &'static str,
    configured: bool,
    pointer_enters: u32,
    pointer_buttons: u32,
    close_received: bool,
    close_events: u32,
}

struct State {
    observer: bool,
    unconfigured_buffer: bool,
    duplicate_role: bool,
    crash_before_buffer: bool,
    reuse_role: bool,
    interactive: bool,
    role_reused: bool,
    violation_sent: bool,
    running: bool,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    base_surface: Option<wl_surface::WlSurface>,
    xdg_surface: Option<xdg_surface::XdgSurface>,
    toplevel: Option<xdg_toplevel::XdgToplevel>,
    buffer: Option<wl_buffer::WlBuffer>,
    shm_file: Option<File>,
    configured: bool,
    pointer_x: f64,
    pointer_y: f64,
    pointer_enters: u32,
    pointer_buttons: u32,
    close_received: bool,
    close_events: u32,
}

impl State {
    fn maybe_create_surface(&mut self, qh: &QueueHandle<Self>) -> Result<()> {
        if self.observer || self.base_surface.is_some() {
            return Ok(());
        }
        let (Some(compositor), Some(wm_base)) = (&self.compositor, &self.wm_base) else {
            return Ok(());
        };
        let surface = compositor.create_surface(qh, ());
        let xdg_surface = wm_base.get_xdg_surface(&surface, qh, ());
        let toplevel = xdg_surface.get_toplevel(qh, ());
        if self.duplicate_role {
            let duplicate = wm_base.get_xdg_surface(&surface, qh, ());
            let _duplicate_toplevel = duplicate.get_toplevel(qh, ());
        }
        toplevel.set_title("SaaiOS S02 deterministic surface".into());
        toplevel.set_app_id("org.saaios.s02-demo".into());
        surface.commit();
        self.base_surface = Some(surface);
        self.xdg_surface = Some(xdg_surface);
        self.toplevel = Some(toplevel);
        self.maybe_send_unconfigured_buffer();
        Ok(())
    }

    fn maybe_create_buffer(&mut self, qh: &QueueHandle<Self>) -> Result<()> {
        if self.observer || self.crash_before_buffer || self.buffer.is_some() {
            return Ok(());
        }
        let Some(shm) = &self.shm else {
            return Ok(());
        };
        let mut file = tempfile::tempfile().context("create wl_shm file")?;
        let frame = demo_frame();
        file.set_len(frame.len() as u64)
            .context("size wl_shm file")?;
        file.write_all(&frame).context("draw deterministic frame")?;
        file.flush().context("flush deterministic frame")?;
        let pool = shm.create_pool(file.as_fd(), frame.len() as i32, qh, ());
        self.buffer = Some(pool.create_buffer(
            0,
            FRAME_WIDTH as i32,
            FRAME_HEIGHT as i32,
            FRAME_STRIDE as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        ));
        self.shm_file = Some(file);
        self.maybe_send_unconfigured_buffer();
        if self.configured {
            self.attach();
        }
        Ok(())
    }

    fn attach(&self) {
        if let (Some(surface), Some(buffer)) = (&self.base_surface, &self.buffer) {
            surface.attach(Some(buffer), 0, 0);
            surface.damage_buffer(0, 0, FRAME_WIDTH as i32, FRAME_HEIGHT as i32);
            surface.commit();
            // A no-op commit must not re-present or re-release the old buffer.
            surface.commit();
        }
    }

    fn redraw_touch(&mut self) -> Result<()> {
        let Some(file) = &self.shm_file else {
            return Ok(());
        };
        let x = self.pointer_x.round().clamp(0.0, (FRAME_WIDTH - 1) as f64) as u32;
        let y = self.pointer_y.round().clamp(0.0, (FRAME_HEIGHT - 1) as f64) as u32;
        let frame = demo_frame_with_touch(x, y);
        file.write_all_at(&frame, 0)
            .context("draw touch marker into wl_shm")?;
        file.sync_data().context("flush touch marker")?;
        self.attach();
        println!("CLIENT_TOUCH x={x} y={y}");
        std::io::stdout().flush().context("flush touch evidence")?;
        Ok(())
    }

    fn maybe_send_unconfigured_buffer(&mut self) {
        if self.unconfigured_buffer && !self.violation_sent {
            if let (Some(surface), Some(buffer)) = (&self.base_surface, &self.buffer) {
                surface.attach(Some(buffer), 0, 0);
                surface.commit();
                self.violation_sent = true;
            }
        }
    }

    fn report(&self) -> ClientReport {
        ClientReport {
            schema: 1,
            role: if self.observer { "observer" } else { "surface" },
            configured: self.configured,
            pointer_enters: self.pointer_enters,
            pointer_buttons: self.pointer_buttons,
            close_received: self.close_received,
            close_events: self.close_events,
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };
        match interface.as_str() {
            "wl_compositor" => {
                state.compositor = Some(registry.bind(name, version.min(6), qh, ()));
                state.maybe_create_surface(qh).expect("create surface");
            }
            "wl_shm" => {
                state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                state.maybe_create_buffer(qh).expect("create buffer");
            }
            "wl_seat" => {
                registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(8), qh, ());
            }
            "xdg_wm_base" => {
                state.wm_base = Some(registry.bind(name, version.min(6), qh, ()));
                state.maybe_create_surface(qh).expect("create xdg surface");
            }
            _ => {}
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn event(
        _state: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            xdg_surface.ack_configure(serial);
            state.configured = true;
            state.attach();
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        _toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Close = event {
            state.close_received = true;
            state.close_events += 1;
            if state.reuse_role && !state.role_reused {
                let surface = state.base_surface.as_ref().unwrap();
                surface.attach(None, 0, 0);
                surface.commit();
                state.toplevel.take().unwrap().destroy();
                let xdg_surface = state.xdg_surface.as_ref().unwrap();
                let replacement = xdg_surface.get_toplevel(qh, ());
                replacement.set_title("SaaiOS S02 replacement role".into());
                state.toplevel = Some(replacement);
                state.configured = false;
                state.role_reused = true;
                surface.commit();
            } else {
                state.running = false;
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            if capabilities.contains(wl_seat::Capability::Pointer) {
                seat.get_pointer(qh, ());
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for State {
    fn event(
        state: &mut Self,
        _pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface_x,
                surface_y,
                ..
            } => {
                state.pointer_enters += 1;
                state.pointer_x = surface_x;
                state.pointer_y = surface_y;
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.pointer_x = surface_x;
                state.pointer_y = surface_y;
            }
            wl_pointer::Event::Button {
                state: WEnum::Value(wl_pointer::ButtonState::Pressed),
                ..
            } => {
                state.pointer_buttons += 1;
                if state.interactive {
                    state.redraw_touch().expect("redraw touch marker");
                }
            }
            _ => {}
        }
    }
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_buffer::WlBuffer);

fn main() -> Result<()> {
    let args = Args::parse()?;
    let connection = Connection::connect_to_env().context("connect to Wayland socket")?;
    let mut queue = connection.new_event_queue();
    let qh = queue.handle();
    connection.display().get_registry(&qh, ());
    let mut state = State {
        observer: args.observer,
        unconfigured_buffer: args.unconfigured_buffer,
        duplicate_role: args.duplicate_role,
        crash_before_buffer: args.crash_before_buffer,
        reuse_role: args.reuse_role,
        interactive: args.interactive,
        role_reused: false,
        violation_sent: false,
        running: true,
        compositor: None,
        shm: None,
        wm_base: None,
        base_surface: None,
        xdg_surface: None,
        toplevel: None,
        buffer: None,
        shm_file: None,
        configured: false,
        pointer_x: 0.0,
        pointer_y: 0.0,
        pointer_enters: 0,
        pointer_buttons: 0,
        close_received: false,
        close_events: 0,
    };

    let initialized = queue.roundtrip(&mut state);
    if state.crash_before_buffer {
        initialized.context("initialize crash fixture")?;
        std::process::exit(42);
    }
    if state.unconfigured_buffer || state.duplicate_role {
        let disconnected = if initialized.is_err() {
            true
        } else {
            queue.blocking_dispatch(&mut state).is_err()
        };
        if state.unconfigured_buffer && !state.violation_sent {
            bail!("unconfigured buffer request was not sent");
        }
        if !disconnected {
            bail!("protocol violation was not rejected");
        }
        println!(
            "VIOLATION_RESULT {}_disconnected=true",
            if state.unconfigured_buffer {
                "unconfigured_buffer"
            } else {
                "duplicate_role"
            }
        );
        return Ok(());
    }
    initialized.context("initialize Wayland globals")?;
    println!(
        "CLIENT_READY {}",
        if state.observer {
            "observer"
        } else {
            "surface"
        }
    );
    while state.running {
        if let Err(error) = queue.blocking_dispatch(&mut state) {
            if state.observer {
                break;
            }
            return Err(error).context("dispatch Wayland events");
        }
    }

    let report = state.report();
    if state.observer {
        if report.pointer_enters != 0 || report.pointer_buttons != 0 {
            bail!("observer received focused input: {report:?}");
        }
    } else {
        let expected_events = if state.reuse_role { 2 } else { 1 };
        if !report.configured
            || report.pointer_enters != expected_events
            || report.pointer_buttons != expected_events
            || report.close_events != expected_events
            || !report.close_received
        {
            bail!("surface lifecycle incomplete: {report:?}");
        }
    }
    println!("CLIENT_RESULT {}", serde_json::to_string(&report)?);
    Ok(())
}
