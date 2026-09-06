use anyhow::{bail, Context, Result};
use memmap2::{Mmap, MmapOptions};
use saai_displayd::{
    BufferTransport, DmabufDescriptor, DmabufImportCheck, FocusState, ProtocolFault,
    RuntimeBackendCapabilities, SoftwareCompositionBackend, SurfaceLifecycle, FRAME_HEIGHT,
    FRAME_STRIDE, FRAME_WIDTH,
};
use std::{
    collections::HashSet,
    fs::File,
    io::Write,
    os::fd::AsFd,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use wayland_protocols::xdg::shell::server::{
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::{self, XdgWmBase},
};
use wayland_server::{
    backend::{ClientData, ClientId, DisconnectReason},
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_compositor::{self, WlCompositor},
        wl_keyboard::{self, WlKeyboard},
        wl_region::{self, WlRegion},
        wl_seat::{self, WlSeat},
        wl_shm::{self, WlShm},
        wl_shm_pool::{self, WlShmPool},
        wl_surface::{self, WlSurface},
    },
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, ListeningSocket, New,
    Resource,
};

#[derive(Default)]
struct Shared {
    events: Mutex<Vec<String>>,
    successful: Mutex<HashSet<ClientId>>,
    completed: Mutex<usize>,
}

impl Shared {
    fn record(&self, event: impl Into<String>) {
        self.events.lock().unwrap().push(event.into());
    }
}

struct ClientMeta {
    shared: Arc<Shared>,
}

impl ClientData for ClientMeta {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
        let successful = self.shared.successful.lock().unwrap().remove(&client_id);
        self.shared
            .record(format!("CLIENT_DISCONNECTED reason={reason:?}"));
        if successful {
            *self.shared.completed.lock().unwrap() += 1;
        }
    }
}

struct CompositorState {
    shared: Arc<Shared>,
    next_serial: u32,
    focus: FocusState,
    keyboards: Vec<(ClientId, WlKeyboard)>,
    backend: SoftwareCompositionBackend<UnavailableDmabufImport>,
}

impl CompositorState {
    fn serial(&mut self) -> u32 {
        self.next_serial += 1;
        self.next_serial
    }

    fn protocol_fault(resource: &impl Resource, fault: ProtocolFault) {
        resource.post_error(1_u32, format!("{fault:?}"));
    }
}

#[derive(Default)]
struct SurfaceInfo {
    lifecycle: SurfaceLifecycle,
    buffer: Option<WlBuffer>,
    xdg_surface: Option<XdgSurface>,
    toplevel: Option<XdgToplevel>,
}

struct SurfaceData(Mutex<SurfaceInfo>);
struct XdgSurfaceData(WlSurface);
struct XdgToplevelData;

struct PoolData {
    map: Arc<Mmap>,
    size: usize,
}

struct BufferData {
    map: Arc<Mmap>,
    offset: usize,
    width: u32,
    height: u32,
    stride: u32,
    format: wl_shm::Format,
}

struct UnavailableDmabufImport;

impl DmabufImportCheck for UnavailableDmabufImport {
    type Error = &'static str;

    fn check_import(&self, _descriptor: &DmabufDescriptor) -> Result<(), Self::Error> {
        Err("headless software backend has no dmabuf importer")
    }
}

macro_rules! simple_global {
    ($interface:ty) => {
        impl GlobalDispatch<$interface, ()> for CompositorState {
            fn bind(
                _state: &mut Self,
                _handle: &DisplayHandle,
                _client: &Client,
                resource: New<$interface>,
                _global_data: &(),
                data_init: &mut DataInit<'_, Self>,
            ) {
                data_init.init(resource, ());
            }
        }
    };
}

simple_global!(WlCompositor);
simple_global!(XdgWmBase);

impl GlobalDispatch<WlShm, ()> for CompositorState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<WlShm>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let shm = data_init.init(resource, ());
        shm.format(wl_shm::Format::Xrgb8888);
    }
}

impl GlobalDispatch<WlSeat, ()> for CompositorState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<WlSeat>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let seat = data_init.init(resource, ());
        seat.capabilities(wl_seat::Capability::Keyboard);
        if seat.version() >= 2 {
            seat.name("saai-synthetic-seat".to_owned());
        }
    }
}

impl Dispatch<WlCompositor, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlCompositor,
        request: wl_compositor::Request,
        _data: &(),
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                data_init.init(id, SurfaceData(Mutex::new(SurfaceInfo::default())));
            }
            wl_compositor::Request::CreateRegion { id } => {
                data_init.init(id, ());
            }
            _ => {}
        }
    }
}

impl Dispatch<WlRegion, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlRegion,
        _request: wl_region::Request,
        _data: &(),
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }
}

impl Dispatch<WlShm, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        resource: &WlShm,
        request: wl_shm::Request,
        _data: &(),
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let wl_shm::Request::CreatePool { id, fd, size } = request {
            if size <= 0 {
                resource.post_error(wl_shm::Error::InvalidStride, "pool size must be positive");
                return;
            }
            let file = File::from(fd);
            let map = unsafe { MmapOptions::new().len(size as usize).map(&file) };
            match map {
                Ok(map) => {
                    data_init.init(
                        id,
                        PoolData {
                            map: Arc::new(map),
                            size: size as usize,
                        },
                    );
                }
                Err(error) => resource.post_error(
                    wl_shm::Error::InvalidFd,
                    format!("could not map wl_shm pool: {error}"),
                ),
            }
        }
    }
}

impl Dispatch<WlShmPool, PoolData> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        resource: &WlShmPool,
        request: wl_shm_pool::Request,
        data: &PoolData,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
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
                let Ok(format) = format.into_result() else {
                    resource.post_error(wl_shm::Error::InvalidFormat, "unknown buffer format");
                    return;
                };
                let valid = offset >= 0
                    && width > 0
                    && height > 0
                    && stride >= width.saturating_mul(4)
                    && (offset as usize)
                        .checked_add((stride as usize).saturating_mul(height as usize))
                        .is_some_and(|end| end <= data.size);
                if !valid {
                    resource.post_error(wl_shm::Error::InvalidStride, "invalid buffer bounds");
                    return;
                }
                data_init.init(
                    id,
                    BufferData {
                        map: data.map.clone(),
                        offset: offset as usize,
                        width: width as u32,
                        height: height as u32,
                        stride: stride as u32,
                        format,
                    },
                );
            }
            wl_shm_pool::Request::Resize { .. } => {
                resource.post_error(wl_shm::Error::InvalidFd, "pool resize is unsupported");
            }
            wl_shm_pool::Request::Destroy => {}
            _ => {}
        }
    }
}

impl Dispatch<WlBuffer, BufferData> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlBuffer,
        _request: wl_buffer::Request,
        _data: &BufferData,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }
}

impl Dispatch<XdgWmBase, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        resource: &XdgWmBase,
        request: xdg_wm_base::Request,
        _data: &(),
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            xdg_wm_base::Request::Destroy => {}
            xdg_wm_base::Request::CreatePositioner { id } => {
                data_init.post_error(id, 1_u32, "positioners are outside S01");
            }
            xdg_wm_base::Request::GetXdgSurface { id, surface } => {
                let Some(surface_data) = surface.data::<SurfaceData>() else {
                    resource.post_error(1_u32, "unknown wl_surface");
                    return;
                };
                let mut info = surface_data.0.lock().unwrap();
                if info.lifecycle.assign_toplevel_role().is_err() {
                    CompositorState::protocol_fault(resource, ProtocolFault::RoleAlreadyAssigned);
                    return;
                }
                let xdg = data_init.init(id, XdgSurfaceData(surface.clone()));
                info.xdg_surface = Some(xdg);
            }
            xdg_wm_base::Request::Pong { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<XdgSurface, XdgSurfaceData> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        resource: &XdgSurface,
        request: xdg_surface::Request,
        data: &XdgSurfaceData,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            xdg_surface::Request::Destroy => {}
            xdg_surface::Request::GetToplevel { id } => {
                let Some(surface_data) = data.0.data::<SurfaceData>() else {
                    resource.post_error(1_u32, "unknown wl_surface");
                    return;
                };
                let mut info = surface_data.0.lock().unwrap();
                if info.toplevel.is_some() {
                    CompositorState::protocol_fault(resource, ProtocolFault::RoleAlreadyAssigned);
                    return;
                }
                let toplevel = data_init.init(id, XdgToplevelData);
                info.toplevel = Some(toplevel);
            }
            xdg_surface::Request::AckConfigure { serial } => {
                let Some(surface_data) = data.0.data::<SurfaceData>() else {
                    return;
                };
                if let Err(fault) = surface_data
                    .0
                    .lock()
                    .unwrap()
                    .lifecycle
                    .ack_configure(serial)
                {
                    CompositorState::protocol_fault(resource, fault);
                }
            }
            xdg_surface::Request::SetWindowGeometry { .. } => {}
            _ => resource.post_error(1_u32, "xdg_surface request outside S01"),
        }
    }
}

impl Dispatch<XdgToplevel, XdgToplevelData> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &XdgToplevel,
        request: xdg_toplevel::Request,
        _data: &XdgToplevelData,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            xdg_toplevel::Request::SetTitle { .. }
            | xdg_toplevel::Request::SetAppId { .. }
            | xdg_toplevel::Request::Destroy => {}
            _ => {}
        }
    }
}

impl Dispatch<WlSurface, SurfaceData> for CompositorState {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &WlSurface,
        request: wl_surface::Request,
        data: &SurfaceData,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_surface::Request::Attach { buffer, .. } => {
                data.0.lock().unwrap().buffer = buffer;
            }
            wl_surface::Request::Commit => {
                let mut info = data.0.lock().unwrap();
                if info.buffer.is_none() {
                    if let (Some(xdg), Some(toplevel)) =
                        (info.xdg_surface.clone(), info.toplevel.clone())
                    {
                        let serial = state.serial();
                        toplevel.configure(
                            FRAME_WIDTH as i32,
                            FRAME_HEIGHT as i32,
                            Vec::<u8>::new(),
                        );
                        xdg.configure(serial);
                        info.lifecycle.configure(serial);
                        state.shared.record(format!("CONFIGURE serial={serial}"));
                    }
                    return;
                }
                if let Err(fault) = info.lifecycle.validate_buffer_commit() {
                    CompositorState::protocol_fault(resource, fault);
                    return;
                }
                let buffer = info.buffer.take().unwrap();
                let Some(buffer_data) = buffer.data::<BufferData>() else {
                    resource.post_error(1_u32, "buffer is not wl_shm");
                    return;
                };
                if buffer_data.width != FRAME_WIDTH
                    || buffer_data.height != FRAME_HEIGHT
                    || buffer_data.stride != FRAME_STRIDE
                    || buffer_data.format != wl_shm::Format::Xrgb8888
                {
                    resource.post_error(1_u32, "unexpected demo buffer layout");
                    return;
                }
                let length = (buffer_data.stride * buffer_data.height) as usize;
                let bytes = &buffer_data.map[buffer_data.offset..buffer_data.offset + length];
                let composed = state.backend.compose_wl_shm(bytes);
                debug_assert_eq!(composed.transport, BufferTransport::WlShm);
                state.shared.record(format!(
                    "FRAME hash={} width=64 height=48",
                    composed.frame_hash
                ));
                buffer.release();

                let surface_id = resource.id().protocol_id() as u64;
                state.focus.focus(surface_id);
                let keyboard = state
                    .focus
                    .receives_input(surface_id)
                    .then(|| {
                        state
                            .keyboards
                            .iter()
                            .find(|(id, _)| *id == client.id())
                            .map(|(_, keyboard)| keyboard.clone())
                    })
                    .flatten();
                if let Some(keyboard) = keyboard {
                    let serial = state.serial();
                    keyboard.enter(serial, resource, Vec::new());
                    keyboard.key(serial, 1, 30, wl_keyboard::KeyState::Pressed);
                    state
                        .shared
                        .record(format!("INPUT key=30 focused=true surface={surface_id}"));
                }
                if let Some(toplevel) = info.toplevel.as_ref() {
                    toplevel.close();
                }
                state.shared.successful.lock().unwrap().insert(client.id());
            }
            wl_surface::Request::Destroy
            | wl_surface::Request::Damage { .. }
            | wl_surface::Request::DamageBuffer { .. }
            | wl_surface::Request::SetBufferScale { .. }
            | wl_surface::Request::SetBufferTransform { .. }
            | wl_surface::Request::SetOpaqueRegion { .. }
            | wl_surface::Request::SetInputRegion { .. }
            | wl_surface::Request::Offset { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<WlSeat, ()> for CompositorState {
    fn request(
        state: &mut Self,
        client: &Client,
        _resource: &WlSeat,
        request: wl_seat::Request,
        _data: &(),
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_seat::Request::GetKeyboard { id } => {
                let keyboard = data_init.init(id, ());
                let keymap = tempfile::tempfile().expect("create empty keymap");
                keyboard.keymap(wl_keyboard::KeymapFormat::NoKeymap, keymap.as_fd(), 0);
                state.keyboards.push((client.id(), keyboard));
            }
            wl_seat::Request::Release => {}
            wl_seat::Request::GetPointer { id } => {
                data_init.post_error(id, 1_u32, "pointer is outside S01");
            }
            wl_seat::Request::GetTouch { id } => {
                data_init.post_error(id, 1_u32, "touch is outside S01");
            }
            _ => {}
        }
    }
}

impl Dispatch<WlKeyboard, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlKeyboard,
        _request: wl_keyboard::Request,
        _data: &(),
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }
}

fn parse_args() -> Result<(String, usize, Option<PathBuf>)> {
    let mut socket = "wayland-saai-s01".to_owned();
    let mut expected = 1;
    let mut report = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--socket" => socket = args.next().context("--socket needs a value")?,
            "--expected-successes" => {
                expected = args
                    .next()
                    .context("--expected-successes needs a value")?
                    .parse()
                    .context("invalid success count")?;
            }
            "--report" => report = Some(args.next().context("--report needs a value")?.into()),
            _ => bail!("unknown argument: {arg}"),
        }
    }
    Ok((socket, expected, report))
}

fn main() -> Result<()> {
    let (socket_name, expected, report_path) = parse_args()?;
    let shared = Arc::new(Shared::default());
    let backend = SoftwareCompositionBackend::new(
        RuntimeBackendCapabilities::software_only(),
        UnavailableDmabufImport,
    );
    let backend_plan = backend.plan(None);
    debug_assert_eq!(backend_plan.transport, BufferTransport::WlShm);
    shared.record(format!(
        "BACKEND wl_shm={} dmabuf_available={} fallback={:?}",
        backend.capabilities().supports_wl_shm(),
        backend_plan.dmabuf_available,
        backend_plan.fallback
    ));
    let mut display = Display::<CompositorState>::new().context("create Wayland display")?;
    let handle = display.handle();
    handle.create_global::<CompositorState, WlCompositor, _>(6, ());
    handle.create_global::<CompositorState, WlShm, _>(1, ());
    handle.create_global::<CompositorState, WlSeat, _>(7, ());
    handle.create_global::<CompositorState, XdgWmBase, _>(6, ());
    let socket = ListeningSocket::bind(&socket_name).context("bind Wayland socket")?;
    let mut state = CompositorState {
        shared: shared.clone(),
        next_serial: 40,
        focus: FocusState::default(),
        keyboards: Vec::new(),
        backend,
    };
    shared.record(format!("READY socket={socket_name}"));
    let started = Instant::now();

    while *shared.completed.lock().unwrap() < expected {
        while let Some(stream) = socket.accept().context("accept Wayland client")? {
            display
                .handle()
                .insert_client(
                    stream,
                    Arc::new(ClientMeta {
                        shared: shared.clone(),
                    }),
                )
                .context("insert Wayland client")?;
        }
        display
            .dispatch_clients(&mut state)
            .context("dispatch Wayland clients")?;
        display.flush_clients().context("flush Wayland clients")?;
        if started.elapsed() > Duration::from_secs(15) {
            bail!("timed out waiting for {expected} successful client(s)");
        }
        thread::sleep(Duration::from_millis(2));
    }

    let output = {
        let events = shared.events.lock().unwrap();
        format!("{}\n", events.join("\n"))
    };
    print!("{output}");
    std::io::stdout().flush()?;
    if let Some(path) = report_path {
        std::fs::write(path, output)?;
    }
    Ok(())
}
