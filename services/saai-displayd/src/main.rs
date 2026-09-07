use std::collections::HashMap;
use std::io::BufRead;
use std::sync::Arc;

#[cfg(feature = "panther-hardware")]
mod hardware;
#[cfg(feature = "panther-hardware")]
mod touch;

use sha2::{Digest, Sha256};
use smithay::input::keyboard::Keycode;
use smithay::{
    delegate_compositor, delegate_seat, delegate_shm, delegate_xdg_shell,
    input::{
        keyboard::{FilterResult, XkbConfig},
        Seat, SeatHandler, SeatState,
    },
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction},
        wayland_server::{
            backend::ClientData,
            protocol::{wl_buffer::WlBuffer, wl_seat::WlSeat, wl_surface::WlSurface},
            Client, Display, DisplayHandle, Resource,
        },
    },
    utils::Serial,
    utils::SERIAL_COUNTER,
    wayland::{
        buffer::BufferHandler,
        compositor::{
            with_states, BufferAssignment, CompositorClientState, CompositorHandler,
            CompositorState, SurfaceAttributes,
        },
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
        shm::{with_buffer_contents, ShmHandler, ShmState},
        socket::ListeningSocketSource,
    },
};

#[derive(Default)]
struct SaaiClientState {
    compositor_state: CompositorClientState,
}
impl ClientData for SaaiClientState {}

struct State {
    compositor_state: CompositorState,
    shm_state: ShmState,
    xdg_shell_state: XdgShellState,
    seat_state: SeatState<State>,
    _seat: Seat<State>,
    keyboard: smithay::input::keyboard::KeyboardHandle<State>,
    /// First surface to commit a real (non-null) buffer keeps keyboard focus
    /// for the lifetime of this headless compositor -- single-window focus
    /// policy, matching the eventual fullscreen panther shell.
    focused_surface: Option<WlSurface>,
    /// wl_surface id -> its ToplevelSurface handle, so commit() can call
    /// ensure_configured() (S02 protocol-negative test: reject a buffer
    /// attached before the surface's first configure was acked).
    toplevels: HashMap<WlSurface, ToplevelSurface>,
    #[cfg(feature = "panther-hardware")]
    hardware: Option<hardware::HardwareOutput>,
    #[cfg(feature = "panther-hardware")]
    touch: smithay::input::touch::TouchHandle<State>,
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client
            .get_data::<SaaiClientState>()
            .unwrap()
            .compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        let buffer = with_states(surface, |states| {
            let mut guard = states.cached_state.get::<SurfaceAttributes>();
            match &guard.current().buffer {
                Some(BufferAssignment::NewBuffer(buffer)) => Some(buffer.clone()),
                _ => None,
            }
        });

        let Some(buffer) = buffer else {
            println!(
                "saai-displayd: commit on surface {:?} (no buffer)",
                surface.id()
            );
            return;
        };

        // S02 protocol-negative test: xdg-shell requires a surface to have
        // its initial configure acked before any buffer-carrying commit --
        // only relevant once we know this commit actually carries one.
        // ensure_configured() checks this and, on violation, posts
        // xdg_surface::Error::NotConstructed itself -- the offending client
        // gets disconnected with a protocol error, everyone else is
        // unaffected.
        if let Some(toplevel) = self.toplevels.get(surface) {
            if !toplevel.ensure_configured() {
                println!(
                    "saai-displayd: rejected commit on surface {:?} -- buffer attached before initial configure was acked",
                    surface.id()
                );
                return;
            }
        }

        let result = with_buffer_contents(&buffer, |ptr, len, data| {
            let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            #[cfg(feature = "panther-hardware")]
            let pixels = bytes.to_vec();
            #[cfg(not(feature = "panther-hardware"))]
            let pixels = ();
            (
                hasher.finalize(),
                pixels,
                data.width as u32,
                data.height as u32,
                data.stride as u32,
            )
        });

        // Must run before the hardware-blit check below: on a surface's very
        // first buffer-carrying commit, focus is still None, so checking
        // focus first would skip blitting that frame -- the display would
        // only ever show the initial fill from hardware::init() and never
        // this (or any) client's actual content.
        if self.focused_surface.is_none() {
            self.focused_surface = Some(surface.clone());
            let serial = SERIAL_COUNTER.next_serial();
            let keyboard = self.keyboard.clone();
            keyboard.set_focus(self, Some(surface.clone()), serial);
            println!(
                "saai-displayd: keyboard focus set to surface {:?}",
                surface.id()
            );
        }

        match result {
            Ok((digest, _pixels, _width, _height, _stride)) => {
                println!(
                    "saai-displayd: commit on surface {:?}, frame sha256={:x}",
                    surface.id(),
                    digest
                );
                #[cfg(feature = "panther-hardware")]
                if self.focused_surface.as_ref() == Some(surface) {
                    if let Some(hw) = self.hardware.as_mut() {
                        hw.blit(&_pixels, _width, _height, _stride);
                        if let Err(err) = hw.present(false) {
                            eprintln!("saai-displayd: hardware present failed: {err}");
                        }
                    }
                }
            }
            Err(err) => eprintln!("saai-displayd: failed to hash committed buffer: {err}"),
        }
    }
}
delegate_compositor!(State);

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_shm!(State);

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<State> {
        &mut self.seat_state
    }
}
delegate_seat!(State);

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        println!("saai-displayd: new xdg_toplevel");
        surface.with_pending_state(|state| {
            state.size = Some((800, 480).into());
        });
        surface.send_configure();
        self.toplevels.insert(surface.wl_surface().clone(), surface);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        self.toplevels.remove(surface.wl_surface());
        if self.focused_surface.as_ref() == Some(surface.wl_surface()) {
            self.focused_surface = None;
        }
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {}

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
    }
}
delegate_xdg_shell!(State);

fn main() {
    let mut display: Display<State> = Display::new().expect("failed to create display");
    let dh: DisplayHandle = display.handle();

    let compositor_state = CompositorState::new::<State>(&dh);
    let shm_state = ShmState::new::<State>(&dh, Vec::new());
    let xdg_shell_state = XdgShellState::new::<State>(&dh);
    let mut seat_state = SeatState::<State>::new();
    let mut seat = seat_state.new_wl_seat(&dh, "seat0");
    let keyboard = seat
        .add_keyboard(XkbConfig::default(), 200, 25)
        .expect("failed to add keyboard capability");
    #[cfg(feature = "panther-hardware")]
    let touch = seat.add_touch();

    let mut event_loop: EventLoop<'static, State> =
        EventLoop::try_new().expect("failed to create event loop");
    let handle = event_loop.handle();

    #[cfg(feature = "panther-hardware")]
    let hardware = match hardware::init() {
        Ok((output, drm_notifier)) => {
            if let Err(err) = handle.insert_source(drm_notifier, |_event, _, _state| {}) {
                eprintln!("saai-displayd: failed to register DRM notifier: {err}");
            }
            println!("saai-displayd: hardware output initialized");
            Some(output)
        }
        Err(err) => {
            eprintln!("saai-displayd: hardware output unavailable: {err}");
            None
        }
    };

    #[cfg(feature = "panther-hardware")]
    match touch::open() {
        Ok(touch_file) => {
            let mut touch_state = touch::TouchState::new();
            let source = Generic::new(touch_file, Interest::READ, Mode::Level);
            if let Err(err) = handle.insert_source(source, move |_, file, state: &mut State| {
                use std::io::Read;
                use smithay::input::touch::{DownEvent, MotionEvent, UpEvent};
                use smithay::utils::Point;

                let mut buf = [0u8; touch::RAW_EVENT_SIZE];
                let touch = state.touch.clone();
                // `file`'s metadata type only derefs to `&File` (no
                // `DerefMut`), but `std::fs::File` also implements `Read`
                // for a shared reference (it's just an fd, no internal
                // buffering state that would need exclusive access).
                let mut f: &std::fs::File = file;
                loop {
                    match f.read(&mut buf) {
                        Ok(n) if n == buf.len() => {
                            let event = touch::parse(buf);
                            let Some(update) = touch_state.feed(&event) else {
                                continue;
                            };
                            let serial = SERIAL_COUNTER.next_serial();
                            let time = 0;
                            // Computed once as an owned value (not a
                            // closure over `state`): `touch.down(state, ...)`
                            // needs `state` by mutable reference, which
                            // would conflict with a closure still borrowing
                            // it for this same call's other argument.
                            let focus = state
                                .focused_surface
                                .clone()
                                .map(|s| (s, Point::from((0.0, 0.0))));
                            match update {
                                touch::TouchUpdate::Down { x, y } => {
                                    println!("saai-displayd: touch down at ({x}, {y})");
                                    let location = Point::from((x as f64, y as f64));
                                    touch.down(
                                        state,
                                        focus,
                                        &DownEvent { slot: (None::<u32>).into(), location, serial, time },
                                    );
                                    touch.frame(state);
                                }
                                touch::TouchUpdate::Motion { x, y } => {
                                    let location = Point::from((x as f64, y as f64));
                                    touch.motion(
                                        state,
                                        focus,
                                        &MotionEvent { slot: (None::<u32>).into(), location, time },
                                    );
                                    touch.frame(state);
                                }
                                touch::TouchUpdate::Up => {
                                    println!("saai-displayd: touch up");
                                    touch.up(state, &UpEvent { slot: (None::<u32>).into(), serial, time });
                                    touch.frame(state);
                                }
                            }
                        }
                        _ => break,
                    }
                }
                Ok(PostAction::Continue)
            }) {
                eprintln!("saai-displayd: failed to register touch input source: {err}");
            } else {
                println!("saai-displayd: touch input initialized");
            }
        }
        Err(err) => eprintln!("saai-displayd: touch input unavailable: {err}"),
    }

    let mut state = State {
        compositor_state,
        shm_state,
        xdg_shell_state,
        seat_state,
        _seat: seat,
        keyboard: keyboard.clone(),
        focused_surface: None,
        toplevels: HashMap::new(),
        #[cfg(feature = "panther-hardware")]
        touch,
        #[cfg(feature = "panther-hardware")]
        hardware,
    };

    let socket = ListeningSocketSource::new_auto().expect("failed to create listening socket");
    let socket_name = socket.socket_name().to_string_lossy().into_owned();

    let mut dh_for_socket = dh.clone();
    handle
        .insert_source(socket, move |client_stream, _, _state| match dh_for_socket
            .insert_client(client_stream, Arc::new(SaaiClientState::default()))
        {
            Ok(_) => println!("saai-displayd: client connected"),
            Err(err) => eprintln!("saai-displayd: failed to insert client: {err}"),
        })
        .expect("failed to insert socket source");

    let display_fd = display
        .backend()
        .poll_fd()
        .try_clone_to_owned()
        .expect("failed to clone display fd");
    handle
        .insert_source(
            Generic::new(display_fd, Interest::READ, Mode::Level),
            move |_, _, state: &mut State| {
                display
                    .dispatch_clients(state)
                    .expect("dispatch_clients failed");
                display.flush_clients().expect("flush_clients failed");
                Ok(PostAction::Continue)
            },
        )
        .expect("failed to insert display source");

    // Debug/test-only synthetic input trigger: `echo inject-key | saai-displayd`
    // sends a press+release of a fixed key to whichever surface currently
    // holds keyboard focus, so the S02 "synthetic input delivered only to
    // the focused client" acceptance test can be driven from a shell script
    // without real hardware.
    let stdin_source = Generic::new(std::io::stdin(), Interest::READ, Mode::Level);
    match handle.insert_source(stdin_source, move |_, _, state: &mut State| {
        let mut line = String::new();
        if std::io::stdin().lock().read_line(&mut line).unwrap_or(0) == 0 {
            return Ok(PostAction::Remove);
        }
        if line.trim() == "inject-key" {
            if state.focused_surface.is_some() {
                let time = 0;
                // evdev KEY_A (30) + 8 = xkb keycode 38.
                let keycode = Keycode::new(38);
                keyboard.input::<(), _>(
                    state,
                    keycode,
                    smithay::backend::input::KeyState::Pressed,
                    SERIAL_COUNTER.next_serial(),
                    time,
                    |_, _, _| FilterResult::Forward,
                );
                keyboard.input::<(), _>(
                    state,
                    keycode,
                    smithay::backend::input::KeyState::Released,
                    SERIAL_COUNTER.next_serial(),
                    time,
                    |_, _, _| FilterResult::Forward,
                );
                println!("saai-displayd: injected synthetic key press+release");
            } else {
                println!("saai-displayd: inject-key requested but no surface is focused yet");
            }
        }
        Ok(PostAction::Continue)
    }) {
        Ok(_) => {}
        Err(err) => {
            // Best-effort debug convenience only -- stdin is not always
            // pollable depending on how this process was launched (backgrounded
            // with an inherited fd, a plain file, etc). Losing it must never
            // take the compositor down.
            eprintln!(
                "saai-displayd: inject-key debug trigger unavailable, continuing without it: {err}"
            );
        }
    }

    println!("saai-displayd: listening on WAYLAND_DISPLAY={socket_name}");
    event_loop
        .run(None, &mut state, |_| {})
        .expect("event loop failed");
}
