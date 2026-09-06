use std::sync::Arc;

use smithay::{
    delegate_compositor, delegate_seat, delegate_shm, delegate_xdg_shell,
    input::{Seat, SeatHandler, SeatState},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction},
        wayland_server::{
            backend::ClientData,
            protocol::{wl_buffer::WlBuffer, wl_seat::WlSeat, wl_surface::WlSurface},
            Client, Display, DisplayHandle, Resource,
        },
    },
    utils::Serial,
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState},
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
        shm::{ShmHandler, ShmState},
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
        println!("saai-displayd: commit on surface {:?}", surface.id());
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
    let seat = seat_state.new_wl_seat(&dh, "seat0");

    let mut state = State {
        compositor_state,
        shm_state,
        xdg_shell_state,
        seat_state,
        _seat: seat,
    };

    let socket = ListeningSocketSource::new_auto().expect("failed to create listening socket");
    let socket_name = socket.socket_name().to_string_lossy().into_owned();

    let mut event_loop: EventLoop<'static, State> =
        EventLoop::try_new().expect("failed to create event loop");
    let handle = event_loop.handle();

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

    println!("saai-displayd: listening on WAYLAND_DISPLAY={socket_name}");
    event_loop
        .run(None, &mut state, |_| {})
        .expect("event loop failed");
}
