use anyhow::{anyhow, bail, Context, Result};
use saai_displayd::panther::{PantherOutput, PantherTouch, TouchEvent};
use saai_displayd::{
    expected_frame_hash, sha256_hex, HeadlessReport, FRAME_HEIGHT, FRAME_STRIDE, FRAME_WIDTH,
};
use std::{
    collections::HashSet,
    fs::{self, File},
    os::unix::fs::FileExt,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use wayland_protocols::xdg::shell::server::{
    xdg_popup, xdg_positioner, xdg_surface, xdg_toplevel, xdg_wm_base,
};
use wayland_server::{
    backend::{ClientData, ClientId, DisconnectReason, ObjectId},
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_keyboard, wl_pointer, wl_region, wl_seat, wl_shm,
        wl_shm_pool, wl_surface, wl_touch,
    },
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, ListeningSocket, New,
    Resource, WEnum,
};

#[derive(Debug)]
struct Args {
    socket: String,
    report: PathBuf,
    timeout_ms: u64,
    backend: Backend,
    drm_card: PathBuf,
    touch: PathBuf,
    ready: PathBuf,
    heartbeat: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Backend {
    Headless,
    Panther,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut args = Self {
            socket: "wayland-saaios-s02".into(),
            report: PathBuf::new(),
            timeout_ms: 0,
            backend: Backend::Headless,
            drm_card: "/dev/dri/card0".into(),
            touch: "/dev/input/touchscreen".into(),
            ready: "/run/saai-displayd.ready".into(),
            heartbeat: "/run/saai-displayd.heartbeat".into(),
        };
        let mut values = std::env::args().skip(1);
        while let Some(flag) = values.next() {
            let value = values
                .next()
                .ok_or_else(|| anyhow!("missing value for {flag}"))?;
            match flag.as_str() {
                "--socket" => args.socket = value,
                "--report" => args.report = value.into(),
                "--timeout-ms" => args.timeout_ms = value.parse().context("parse --timeout-ms")?,
                "--backend" => {
                    args.backend = match value.as_str() {
                        "headless" => Backend::Headless,
                        "panther" => Backend::Panther,
                        _ => bail!("unknown backend {value}"),
                    }
                }
                "--drm-card" => args.drm_card = value.into(),
                "--touch" => args.touch = value.into(),
                "--ready" => args.ready = value.into(),
                "--heartbeat" => args.heartbeat = value.into(),
                _ => bail!("unknown argument {flag}"),
            }
        }
        if args.report.as_os_str().is_empty() {
            bail!("--report is required");
        }
        Ok(args)
    }
}

struct ClientState {
    disconnected: Arc<AtomicUsize>,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        self.disconnected.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Default)]
struct SurfaceData {
    pending_buffer: Option<Option<wl_buffer::WlBuffer>>,
    current_buffer: Option<wl_buffer::WlBuffer>,
    callbacks: Vec<wl_callback::WlCallback>,
}

struct PoolData {
    file: Mutex<File>,
    size: Mutex<usize>,
}

struct BufferData {
    pool: Arc<PoolData>,
    offset: usize,
    width: i32,
    height: i32,
    stride: i32,
    format: wl_shm::Format,
}

struct XdgSurfaceData {
    wl_surface: wl_surface::WlSurface,
    expected_serial: Mutex<Option<u32>>,
    configured: Mutex<bool>,
    has_role: Mutex<bool>,
}

struct ToplevelData {
    wl_surface: wl_surface::WlSurface,
    xdg_surface: xdg_surface::XdgSurface,
}

struct PantherRuntime {
    output: PantherOutput,
    touch: PantherTouch,
}

#[derive(Default)]
struct State {
    pointers: Vec<wl_pointer::WlPointer>,
    xdg_surfaces: Vec<xdg_surface::XdgSurface>,
    toplevels: Vec<xdg_toplevel::XdgToplevel>,
    role_surfaces: HashSet<ObjectId>,
    report: Option<HeadlessReport>,
    close_at: Option<Instant>,
    next_serial: u32,
    active_surface: Option<wl_surface::WlSurface>,
    pointer_focus: HashSet<ObjectId>,
    clock_origin: Option<Instant>,
    panther: Option<PantherRuntime>,
    fatal_error: Option<String>,
}

impl State {
    fn serial(&mut self) -> u32 {
        self.next_serial = self.next_serial.wrapping_add(1).max(1);
        self.next_serial
    }

    fn event_time(&mut self) -> u32 {
        let origin = self.clock_origin.get_or_insert_with(Instant::now);
        origin.elapsed().as_millis() as u32
    }

    fn xdg_for_surface(&self, surface: &wl_surface::WlSurface) -> Option<&xdg_surface::XdgSurface> {
        self.xdg_surfaces.iter().find(|xdg| {
            xdg.data::<XdgSurfaceData>()
                .is_some_and(|data| data.wl_surface == *surface)
        })
    }

    fn configure_surface(&mut self, surface: &wl_surface::WlSurface) {
        let Some(xdg_surface) = self.xdg_for_surface(surface).cloned() else {
            return;
        };
        let Some(data) = xdg_surface.data::<XdgSurfaceData>() else {
            return;
        };
        if !*data.has_role.lock().unwrap() || data.expected_serial.lock().unwrap().is_some() {
            return;
        }
        let Some(toplevel) = self
            .toplevels
            .iter()
            .find(|toplevel| {
                toplevel
                    .data::<ToplevelData>()
                    .is_some_and(|data| data.xdg_surface == xdg_surface)
            })
            .cloned()
        else {
            return;
        };

        let serial = self.serial();
        toplevel.configure(
            FRAME_WIDTH as i32,
            FRAME_HEIGHT as i32,
            (xdg_toplevel::State::Activated as u32)
                .to_ne_bytes()
                .to_vec(),
        );
        xdg_surface.configure(serial);
        *data.expected_serial.lock().unwrap() = Some(serial);
    }

    fn present(&mut self, surface: &wl_surface::WlSurface, buffer: &wl_buffer::WlBuffer) {
        let xdg_surface = self.xdg_for_surface(surface).cloned();
        let configured = xdg_surface
            .as_ref()
            .and_then(|xdg| xdg.data::<XdgSurfaceData>())
            .is_some_and(|data| *data.configured.lock().unwrap());
        if !configured {
            if let Some(xdg_surface) = xdg_surface {
                xdg_surface.post_error(
                    xdg_surface::Error::UnconfiguredBuffer,
                    "buffer committed before xdg configure acknowledgement",
                );
            }
            return;
        }

        let Some(data) = buffer.data::<BufferData>() else {
            return;
        };
        if data.width != FRAME_WIDTH as i32
            || data.height != FRAME_HEIGHT as i32
            || data.stride != FRAME_STRIDE as i32
            || data.format != wl_shm::Format::Argb8888
        {
            buffer.post_error(0u32, "unsupported S02 test buffer");
            return;
        }
        let frame_len = (FRAME_STRIDE * FRAME_HEIGHT) as usize;
        let size = *data.pool.size.lock().unwrap();
        if data
            .offset
            .checked_add(frame_len)
            .is_none_or(|end| end > size)
        {
            buffer.post_error(0u32, "wl_shm buffer exceeds pool");
            return;
        }
        let mut bytes = vec![0; frame_len];
        if data
            .pool
            .file
            .lock()
            .unwrap()
            .read_exact_at(&mut bytes, data.offset as u64)
            .is_err()
        {
            buffer.post_error(0u32, "cannot read wl_shm buffer");
            return;
        }

        if let Some(panther) = &mut self.panther {
            if let Err(error) =
                panther
                    .output
                    .present(&bytes, FRAME_WIDTH, FRAME_HEIGHT, FRAME_STRIDE)
            {
                self.fatal_error = Some(format!("{error:#}"));
                return;
            }
            if self.active_surface.as_ref() != Some(surface) {
                self.pointer_focus.clear();
                self.active_surface = Some(surface.clone());
            }
            buffer.release();
            self.report.get_or_insert(HeadlessReport {
                schema: 1,
                configured: true,
                frame_hash: sha256_hex(&bytes),
                input_events_sent: 0,
                close_sent: false,
                disconnected_clients: 0,
            });
            return;
        }

        let enter_serial = self.serial();
        let button_serial = self.serial();
        let mut input_recipients = 0;
        for pointer in &self.pointers {
            if pointer.id().same_client_as(&surface.id()) {
                pointer.enter(enter_serial, surface, 12.0, 8.0);
                pointer.motion(1, 12.0, 8.0);
                pointer.button(button_serial, 2, 0x110, wl_pointer::ButtonState::Pressed);
                if pointer.version() >= wl_pointer::EVT_FRAME_SINCE {
                    pointer.frame();
                }
                input_recipients += 1;
            }
        }
        buffer.release();

        for toplevel in &self.toplevels {
            if toplevel
                .data::<ToplevelData>()
                .is_some_and(|data| data.wl_surface == *surface)
            {
                toplevel.close();
            }
        }
        self.report = Some(HeadlessReport {
            schema: 1,
            configured: true,
            frame_hash: sha256_hex(&bytes),
            input_events_sent: input_recipients,
            close_sent: true,
            disconnected_clients: 0,
        });
        self.close_at = Some(Instant::now());
    }

    fn dispatch_touch(&mut self) {
        let events = match self.panther.as_mut().map(|panther| panther.touch.drain()) {
            Some(Ok(events)) => events,
            Some(Err(error)) => {
                self.fatal_error = Some(format!("{error:#}"));
                return;
            }
            None => return,
        };
        let Some(surface) = self.active_surface.clone() else {
            return;
        };
        for event in events {
            let serial = self.serial();
            let event_time = self.event_time();
            let (x, y) = match event {
                TouchEvent::Down { x, y }
                | TouchEvent::Motion { x, y }
                | TouchEvent::Up { x, y } => (x, y),
            };
            let mut recipients = 0;
            for pointer in &self.pointers {
                if !pointer.id().same_client_as(&surface.id()) {
                    continue;
                }
                match event {
                    TouchEvent::Down { .. } => {
                        if self.pointer_focus.insert(pointer.id()) {
                            pointer.enter(serial, &surface, x, y);
                        }
                        pointer.motion(event_time, x, y);
                        pointer.button(serial, event_time, 0x110, wl_pointer::ButtonState::Pressed);
                    }
                    TouchEvent::Motion { .. } => pointer.motion(event_time, x, y),
                    TouchEvent::Up { .. } => {
                        pointer.button(
                            serial,
                            event_time,
                            0x110,
                            wl_pointer::ButtonState::Released,
                        );
                    }
                }
                if pointer.version() >= wl_pointer::EVT_FRAME_SINCE {
                    pointer.frame();
                }
                recipients += 1;
            }
            if let Some(report) = &mut self.report {
                report.input_events_sent += recipients;
            }
        }
    }
}

macro_rules! simple_global {
    ($interface:ty) => {
        impl GlobalDispatch<$interface, ()> for State {
            fn bind(
                _state: &mut State,
                _handle: &DisplayHandle,
                _client: &Client,
                resource: New<$interface>,
                _global_data: &(),
                data_init: &mut DataInit<'_, State>,
            ) {
                data_init.init(resource, ());
            }
        }
    };
}

simple_global!(wl_compositor::WlCompositor);
simple_global!(xdg_wm_base::XdgWmBase);

impl GlobalDispatch<wl_shm::WlShm, ()> for State {
    fn bind(
        _state: &mut State,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<wl_shm::WlShm>,
        _global_data: &(),
        data_init: &mut DataInit<'_, State>,
    ) {
        let shm = data_init.init(resource, ());
        shm.format(wl_shm::Format::Argb8888);
    }
}

impl GlobalDispatch<wl_seat::WlSeat, ()> for State {
    fn bind(
        _state: &mut State,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<wl_seat::WlSeat>,
        _global_data: &(),
        data_init: &mut DataInit<'_, State>,
    ) {
        let seat = data_init.init(resource, ());
        seat.capabilities(wl_seat::Capability::Pointer);
        if seat.version() >= wl_seat::EVT_NAME_SINCE {
            seat.name("saaios-headless".into());
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &wl_compositor::WlCompositor,
        request: wl_compositor::Request,
        _data: &(),
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, State>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                data_init.init(id, Mutex::new(SurfaceData::default()));
            }
            wl_compositor::Request::CreateRegion { id } => {
                data_init.init(id, ());
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_region::WlRegion, ()> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &wl_region::WlRegion,
        _request: wl_region::Request,
        _data: &(),
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, State>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, Mutex<SurfaceData>> for State {
    fn request(
        state: &mut State,
        _client: &Client,
        resource: &wl_surface::WlSurface,
        request: wl_surface::Request,
        data: &Mutex<SurfaceData>,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, State>,
    ) {
        match request {
            wl_surface::Request::Attach { buffer, .. } => {
                data.lock().unwrap().pending_buffer = Some(buffer);
            }
            wl_surface::Request::Frame { callback } => {
                let callback = data_init.init(callback, ());
                data.lock().unwrap().callbacks.push(callback);
            }
            wl_surface::Request::Commit => {
                let (new_buffer, initial_commit, callbacks) = {
                    let mut surface = data.lock().unwrap();
                    let pending = surface.pending_buffer.take();
                    let had_attach = pending.is_some();
                    if let Some(pending) = pending {
                        surface.current_buffer = pending;
                    }
                    let initial_commit = !had_attach && surface.current_buffer.is_none();
                    let new_buffer = if had_attach {
                        surface.current_buffer.clone()
                    } else {
                        None
                    };
                    (
                        new_buffer,
                        initial_commit,
                        std::mem::take(&mut surface.callbacks),
                    )
                };
                if initial_commit {
                    state.configure_surface(resource);
                }
                if let Some(buffer) = new_buffer {
                    state.present(resource, &buffer);
                }
                for callback in callbacks {
                    callback.done(3);
                }
            }
            _ => {}
        }
    }

    fn destroyed(
        state: &mut State,
        _client: ClientId,
        resource: &wl_surface::WlSurface,
        _data: &Mutex<SurfaceData>,
    ) {
        if state.active_surface.as_ref() == Some(resource) {
            state.active_surface = None;
            state.pointer_focus.clear();
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &wl_callback::WlCallback,
        _request: wl_callback::Request,
        _data: &(),
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, State>,
    ) {
    }
}

impl Dispatch<wl_shm::WlShm, ()> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        resource: &wl_shm::WlShm,
        request: wl_shm::Request,
        _data: &(),
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, State>,
    ) {
        if let wl_shm::Request::CreatePool { id, fd, size } = request {
            if size <= 0 {
                resource.post_error(wl_shm::Error::InvalidStride, "invalid wl_shm pool size");
                return;
            }
            data_init.init(
                id,
                Arc::new(PoolData {
                    file: Mutex::new(fd.into()),
                    size: Mutex::new(size as usize),
                }),
            );
        }
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, Arc<PoolData>> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        resource: &wl_shm_pool::WlShmPool,
        request: wl_shm_pool::Request,
        data: &Arc<PoolData>,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, State>,
    ) {
        match request {
            wl_shm_pool::Request::CreateBuffer {
                id,
                offset,
                width,
                height,
                stride,
                format,
            } => {
                let WEnum::Value(format) = format else {
                    resource.post_error(wl_shm::Error::InvalidFormat, "unknown wl_shm format");
                    return;
                };
                if offset < 0 || width <= 0 || height <= 0 || stride < width.saturating_mul(4) {
                    resource.post_error(wl_shm::Error::InvalidStride, "invalid wl_shm buffer");
                    return;
                }
                data_init.init(
                    id,
                    BufferData {
                        pool: data.clone(),
                        offset: offset as usize,
                        width,
                        height,
                        stride,
                        format,
                    },
                );
            }
            wl_shm_pool::Request::Resize { size } if size > 0 => {
                *data.size.lock().unwrap() = size as usize;
            }
            wl_shm_pool::Request::Resize { .. } => {
                resource.post_error(wl_shm::Error::InvalidFd, "invalid wl_shm pool resize");
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_buffer::WlBuffer, BufferData> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &wl_buffer::WlBuffer,
        _request: wl_buffer::Request,
        _data: &BufferData,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, State>,
    ) {
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn request(
        state: &mut State,
        _client: &Client,
        _resource: &xdg_wm_base::XdgWmBase,
        request: xdg_wm_base::Request,
        _data: &(),
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, State>,
    ) {
        match request {
            xdg_wm_base::Request::CreatePositioner { id } => {
                data_init.init(id, ());
            }
            xdg_wm_base::Request::GetXdgSurface { id, surface } => {
                if !state.role_surfaces.insert(surface.id()) {
                    let duplicate = data_init.init(
                        id,
                        XdgSurfaceData {
                            wl_surface: surface,
                            expected_serial: Mutex::new(None),
                            configured: Mutex::new(false),
                            has_role: Mutex::new(false),
                        },
                    );
                    state.xdg_surfaces.push(duplicate);
                    _resource.post_error(
                        xdg_wm_base::Error::Role,
                        "wl_surface already has an xdg role",
                    );
                    return;
                }
                let xdg = data_init.init(
                    id,
                    XdgSurfaceData {
                        wl_surface: surface,
                        expected_serial: Mutex::new(None),
                        configured: Mutex::new(false),
                        has_role: Mutex::new(false),
                    },
                );
                state.xdg_surfaces.push(xdg);
            }
            _ => {}
        }
    }
}

impl Dispatch<xdg_positioner::XdgPositioner, ()> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &xdg_positioner::XdgPositioner,
        _request: xdg_positioner::Request,
        _data: &(),
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, State>,
    ) {
    }
}

impl Dispatch<xdg_surface::XdgSurface, XdgSurfaceData> for State {
    fn request(
        state: &mut State,
        _client: &Client,
        resource: &xdg_surface::XdgSurface,
        request: xdg_surface::Request,
        data: &XdgSurfaceData,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, State>,
    ) {
        match request {
            xdg_surface::Request::GetToplevel { id } => {
                let mut has_role = data.has_role.lock().unwrap();
                if *has_role {
                    let duplicate = data_init.init(
                        id,
                        ToplevelData {
                            wl_surface: data.wl_surface.clone(),
                            xdg_surface: resource.clone(),
                        },
                    );
                    state.toplevels.push(duplicate);
                    resource.post_error(
                        xdg_surface::Error::AlreadyConstructed,
                        "xdg_surface already has a role",
                    );
                    return;
                }
                *has_role = true;
                drop(has_role);
                let toplevel = data_init.init(
                    id,
                    ToplevelData {
                        wl_surface: data.wl_surface.clone(),
                        xdg_surface: resource.clone(),
                    },
                );
                state.toplevels.push(toplevel);
            }
            xdg_surface::Request::AckConfigure { serial } => {
                if *data.expected_serial.lock().unwrap() == Some(serial) {
                    *data.configured.lock().unwrap() = true;
                } else {
                    resource.post_error(
                        xdg_surface::Error::InvalidSerial,
                        "unknown xdg configure serial",
                    );
                }
            }
            xdg_surface::Request::GetPopup { id, .. } => {
                data_init.init(id, ());
                resource.post_error(0u32, "xdg_popup is outside the S02 protocol subset");
            }
            xdg_surface::Request::Destroy if *data.has_role.lock().unwrap() => {
                resource.post_error(
                    xdg_surface::Error::DefunctRoleObject,
                    "destroy the xdg role object before xdg_surface",
                );
            }
            _ => {}
        }
    }

    fn destroyed(
        state: &mut State,
        _client: ClientId,
        resource: &xdg_surface::XdgSurface,
        data: &XdgSurfaceData,
    ) {
        state.xdg_surfaces.retain(|xdg| xdg != resource);
        state.role_surfaces.remove(&data.wl_surface.id());
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ToplevelData> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &xdg_toplevel::XdgToplevel,
        _request: xdg_toplevel::Request,
        _data: &ToplevelData,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, State>,
    ) {
    }

    fn destroyed(
        state: &mut State,
        _client: ClientId,
        resource: &xdg_toplevel::XdgToplevel,
        data: &ToplevelData,
    ) {
        if let Some(xdg_data) = data.xdg_surface.data::<XdgSurfaceData>() {
            *xdg_data.has_role.lock().unwrap() = false;
            *xdg_data.expected_serial.lock().unwrap() = None;
            *xdg_data.configured.lock().unwrap() = false;
        }
        state.toplevels.retain(|toplevel| toplevel != resource);
    }
}

impl Dispatch<xdg_popup::XdgPopup, ()> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &xdg_popup::XdgPopup,
        _request: xdg_popup::Request,
        _data: &(),
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, State>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn request(
        state: &mut State,
        _client: &Client,
        _resource: &wl_seat::WlSeat,
        request: wl_seat::Request,
        _data: &(),
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, State>,
    ) {
        match request {
            wl_seat::Request::GetPointer { id } => {
                let pointer = data_init.init(id, ());
                state.pointers.push(pointer);
            }
            wl_seat::Request::GetKeyboard { id } => {
                data_init.init(id, ());
                _resource.post_error(0u32, "keyboard is outside the S02 protocol subset");
            }
            wl_seat::Request::GetTouch { id } => {
                data_init.init(id, ());
                _resource.post_error(0u32, "touch is outside the S02 protocol subset");
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &wl_keyboard::WlKeyboard,
        _request: wl_keyboard::Request,
        _data: &(),
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, State>,
    ) {
    }
}

impl Dispatch<wl_touch::WlTouch, ()> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &wl_touch::WlTouch,
        _request: wl_touch::Request,
        _data: &(),
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, State>,
    ) {
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for State {
    fn request(
        _state: &mut State,
        _client: &Client,
        _resource: &wl_pointer::WlPointer,
        _request: wl_pointer::Request,
        _data: &(),
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, State>,
    ) {
    }

    fn destroyed(
        state: &mut State,
        _client: ClientId,
        resource: &wl_pointer::WlPointer,
        _data: &(),
    ) {
        state.pointers.retain(|pointer| pointer != resource);
        state.pointer_focus.remove(&resource.id());
    }
}

fn main() -> Result<()> {
    let args = Args::parse()?;
    if args.backend == Backend::Headless && args.timeout_ms == 0 {
        bail!("headless backend requires a non-zero --timeout-ms");
    }
    if args.backend == Backend::Panther {
        let _ = fs::remove_file(&args.ready);
        let _ = fs::remove_file(&args.heartbeat);
    }
    let mut display: Display<State> = Display::new().context("create Wayland display")?;
    let mut handle = display.handle();
    handle.create_global::<State, wl_compositor::WlCompositor, _>(4, ());
    handle.create_global::<State, wl_shm::WlShm, _>(1, ());
    handle.create_global::<State, wl_seat::WlSeat, _>(8, ());
    handle.create_global::<State, xdg_wm_base::XdgWmBase, _>(6, ());
    let listener = ListeningSocket::bind(&args.socket).context("bind private Wayland socket")?;
    println!("READY {}", args.socket);

    let disconnected = Arc::new(AtomicUsize::new(0));
    let deadline =
        (args.timeout_ms != 0).then(|| Instant::now() + Duration::from_millis(args.timeout_ms));
    let mut state = State {
        panther: if args.backend == Backend::Panther {
            Some(PantherRuntime {
                output: PantherOutput::open(&args.drm_card)?,
                touch: PantherTouch::open(&args.touch)?,
            })
        } else {
            None
        },
        ..State::default()
    };
    if args.backend == Backend::Panther {
        fs::write(&args.ready, "drm-touch-wayland-ready")
            .with_context(|| format!("write {}", args.ready.display()))?;
    }
    let mut clients = Vec::new();
    let mut heartbeat = 0u64;
    let mut last_heartbeat = Instant::now();
    while deadline.is_none_or(|deadline| Instant::now() < deadline) {
        if let Some(stream) = listener.accept().context("accept Wayland client")? {
            clients.push(
                handle
                    .insert_client(
                        stream,
                        Arc::new(ClientState {
                            disconnected: disconnected.clone(),
                        }),
                    )
                    .context("insert Wayland client")?,
            );
        }
        display
            .dispatch_clients(&mut state)
            .context("dispatch Wayland clients")?;
        state.dispatch_touch();
        if let Some(error) = state.fatal_error.take() {
            bail!("Panther backend failed: {error}");
        }
        display.flush_clients().context("flush Wayland clients")?;
        if args.backend == Backend::Panther
            && last_heartbeat.elapsed() >= Duration::from_millis(250)
        {
            heartbeat = heartbeat.wrapping_add(1);
            fs::write(&args.heartbeat, heartbeat.to_string())
                .with_context(|| format!("write {}", args.heartbeat.display()))?;
            last_heartbeat = Instant::now();
        }
        if state
            .close_at
            .is_some_and(|sent| sent.elapsed() >= Duration::from_millis(250))
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    if args.backend == Backend::Panther {
        bail!("Panther backend stopped unexpectedly");
    }
    let mut report = state.report.context("headless lifecycle timed out")?;
    report.disconnected_clients = disconnected.load(Ordering::Relaxed);
    if report.frame_hash != expected_frame_hash() {
        bail!(
            "unexpected frame hash: expected {}, got {}",
            expected_frame_hash(),
            report.frame_hash
        );
    }
    if !report.configured || report.input_events_sent != 1 || !report.close_sent {
        bail!("headless lifecycle incomplete: {report:?}");
    }
    fs::write(&args.report, serde_json::to_vec_pretty(&report)?)
        .with_context(|| format!("write {}", args.report.display()))?;
    println!("RESULT {}", serde_json::to_string(&report)?);
    Ok(())
}
