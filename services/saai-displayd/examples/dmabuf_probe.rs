//! Physical linux-dmabuf acceptance probe for panther.
//!
//! Allocates a DRM dumb buffer without becoming DRM master, exports it as a
//! PRIME fd, creates a zwp_linux_dmabuf_v1 wl_buffer and presents a simple
//! full-screen pattern. saai-displayd's log must report a direct GPU
//! dma-buf blit; a synchronized staging fallback is a test failure.

use std::fs::OpenOptions;
use std::os::fd::{AsFd, OwnedFd};
use std::time::{Duration, Instant};

use smithay::backend::allocator::dumb::DumbAllocator;
use smithay::backend::allocator::{Allocator, Fourcc, Modifier};
use smithay::backend::drm::DrmDeviceFd;
use smithay::reexports::drm::buffer::Buffer as DrmBuffer;
use smithay::reexports::drm::control::{dumbbuffer::DumbBuffer as RawDumbBuffer, Device};
use smithay::reexports::drm::{CLOEXEC, RDWR};
use smithay::utils::DeviceFd;
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_buffer, wl_compositor, wl_output, wl_registry, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::{
    wp::linux_dmabuf::zv1::client::{zwp_linux_buffer_params_v1, zwp_linux_dmabuf_v1},
    xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base},
};

const WIDTH: u32 = 1080;
const HEIGHT: u32 = 2400;

struct Probe {
    configured: bool,
    closed: bool,
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Probe {
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

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for Probe {
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

impl Dispatch<xdg_surface::XdgSurface, ()> for Probe {
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
            state.configured = true;
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for Probe {
    fn event(
        state: &mut Self,
        _proxy: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if matches!(event, xdg_toplevel::Event::Close) {
            state.closed = true;
        }
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for Probe {
    fn event(
        _state: &mut Self,
        _proxy: &wl_buffer::WlBuffer,
        _event: wl_buffer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(Probe: ignore wl_compositor::WlCompositor);
delegate_noop!(Probe: ignore wl_surface::WlSurface);
delegate_noop!(Probe: ignore wl_output::WlOutput);
delegate_noop!(Probe: ignore zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1);
delegate_noop!(Probe: ignore zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1);

fn fill_pattern(bytes: &mut [u8], pitch: usize) {
    for y in 0..HEIGHT as usize {
        for x in 0..WIDTH as usize {
            let (r, g, b) = if x < WIDTH as usize / 3 {
                (0x11, 0xd4, 0xb0)
            } else if x < WIDTH as usize * 2 / 3 {
                (0x62, 0x5b, 0xe8)
            } else {
                (0xf2, 0x9f, 0x3f)
            };
            let offset = y * pitch + x * 4;
            bytes[offset..offset + 4].copy_from_slice(&[b, g, r, 0]);
        }
    }
}

fn main() {
    let drm_file: OwnedFd = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/dri/card0")
        .expect("failed to open /dev/dri/card0")
        .into();
    let drm_fd = DrmDeviceFd::new(DeviceFd::from(drm_file));
    let mut allocator = DumbAllocator::new(drm_fd.clone());
    let dumb = allocator
        .create_buffer(WIDTH, HEIGHT, Fourcc::Xrgb8888, &[Modifier::Linear])
        .expect("failed to allocate DRM dumb buffer");
    let mut raw: RawDumbBuffer = *dumb.handle();
    let pitch = raw.pitch() as usize;
    {
        let mut mapping = drm_fd
            .map_dumb_buffer(&mut raw)
            .expect("failed to map DRM dumb buffer");
        fill_pattern(mapping.as_mut(), pitch);
    }
    let prime_fd = drm_fd
        .buffer_to_prime_fd(raw.handle(), CLOEXEC | RDWR)
        .expect("failed to export PRIME fd");

    let conn = Connection::connect_to_env().expect("failed to connect to saai-displayd");
    let (globals, mut queue) = registry_queue_init::<Probe>(&conn).expect("registry init failed");
    let qh = queue.handle();
    let compositor: wl_compositor::WlCompositor = globals
        .bind(&qh, 1..=6, ())
        .expect("wl_compositor unavailable");
    let wm_base: xdg_wm_base::XdgWmBase = globals
        .bind(&qh, 1..=6, ())
        .expect("xdg_wm_base unavailable");
    let dmabuf: zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1 = globals
        .bind(&qh, 3..=3, ())
        .expect("zwp_linux_dmabuf_v1 v3 unavailable");

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("SaaiOS DMA-BUF probe".into());
    toplevel.set_fullscreen(None);
    surface.commit();

    let mut state = Probe {
        configured: false,
        closed: false,
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while !state.configured && Instant::now() < deadline {
        queue
            .blocking_dispatch(&mut state)
            .expect("Wayland dispatch failed before configure");
    }
    assert!(state.configured, "xdg_surface was not configured");

    let params = dmabuf.create_params(&qh, ());
    params.add(prime_fd.as_fd(), 0, 0, pitch as u32, 0, 0);
    let buffer = params.create_immed(
        WIDTH as i32,
        HEIGHT as i32,
        Fourcc::Xrgb8888 as u32,
        zwp_linux_buffer_params_v1::Flags::empty(),
        &qh,
        (),
    );
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, WIDTH as i32, HEIGHT as i32);
    surface.commit();
    conn.flush().expect("failed to flush dma-buf commit");
    queue.roundtrip(&mut state).expect("dma-buf commit failed");
    println!(
        "saai-dmabuf-probe: committed {}x{} DRM PRIME buffer, pitch={pitch}",
        WIDTH, HEIGHT
    );

    // Keep the allocation and wl_buffer alive long enough for physical
    // inspection and at least one page flip.
    let until = Instant::now() + Duration::from_secs(90);
    while !state.closed && Instant::now() < until {
        queue.roundtrip(&mut state).expect("probe roundtrip failed");
        std::thread::sleep(Duration::from_millis(100));
    }
    drop((buffer, params, toplevel, xdg_surface, surface, dumb));
}
