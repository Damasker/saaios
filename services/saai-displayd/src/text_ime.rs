//! ADR-267 / APP-03 / ADR-319: `zwp_text_input_manager_v3` (GTK) plus
//! KDE `zwp_text_input_manager_v2` (Qt 5.15 / Qt 6.6) plus
//! `zwp_input_method_manager_v2` without `Seat::get_keyboard()` /
//! xkbcommon.
//!
//! Smithay's `InputMethodManagerState::GetInputMethod` unwraps a keyboard
//! (ADR-022). Enable on smithay's text-input is discarded unless that
//! IME instance exists. This module owns the globals so commit_string
//! can reach an enabled field on a seat that has only touch. Qt packed
//! on panther does not contain `zwp_text_input_v3` at all (ADR-319).
//! IME `DeleteSurroundingText` is also forwarded to v2 (ADR-328).
//! v2 IME commit/delete are applied after the current dispatch so Qt's
//! `update_state` + `wl_display.sync` (`m_resetCallback`) run first
//! (ADR-339).

use std::sync::Mutex;

use smithay::{
    input::{Seat, SeatHandler},
    reexports::{
        wayland_protocols::wp::text_input::zv3::server::{
            zwp_text_input_manager_v3::{self, ZwpTextInputManagerV3},
            zwp_text_input_v3::{self, ZwpTextInputV3},
        },
        wayland_protocols_misc::zwp_input_method_v2::server::{
            zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2,
            zwp_input_method_manager_v2::{self, ZwpInputMethodManagerV2},
            zwp_input_method_v2::{self, ZwpInputMethodV2},
            zwp_input_popup_surface_v2::ZwpInputPopupSurfaceV2,
        },
        wayland_server::{
            backend::{ClientId, GlobalId, ObjectId},
            protocol::wl_surface::WlSurface,
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
        },
    },
};
use wayland_protocols_plasma::text_input::v2::server::{
    zwp_text_input_manager_v2::{self, ZwpTextInputManagerV2},
    zwp_text_input_v2::{self, ZwpTextInputV2},
};

#[derive(Default)]
struct SeatTextIme {
    text_inputs: Mutex<Vec<ZwpTextInputV3>>,
    text_inputs_v2: Mutex<Vec<ZwpTextInputV2>>,
    input_method: Mutex<Option<ZwpInputMethodV2>>,
    focus: Mutex<Option<WlSurface>>,
    active: Mutex<Option<ObjectId>>,
    serial: Mutex<u32>,
    pending_v2: Mutex<Vec<PendingV2>>,
}

enum PendingV2 {
    CommitString { active_id: ObjectId, text: String },
    DeleteSurrounding {
        active_id: ObjectId,
        before_length: u32,
        after_length: u32,
    },
}

fn seat_ime<D: SeatHandler + 'static>(seat: &Seat<D>) -> &SeatTextIme {
    let user_data = seat.user_data();
    user_data.insert_if_missing(SeatTextIme::default);
    user_data.get::<SeatTextIme>().expect("just inserted")
}

/// Send leave/enter on text-input objects for the focused surface.
pub fn on_focus<D: SeatHandler + 'static>(seat: &Seat<D>, surface: Option<WlSurface>) {
    let ime = seat_ime(seat);
    if let Some(old) = ime.focus.lock().expect("focus").take() {
        for ti in ime.text_inputs.lock().expect("text_inputs").iter() {
            if old.id().same_client_as(&ti.id()) {
                ti.leave(&old);
            }
        }
        let serial = {
            let mut serial = ime.serial.lock().expect("serial");
            *serial = serial.wrapping_add(1);
            *serial
        };
        for ti in ime.text_inputs_v2.lock().expect("text_inputs_v2").iter() {
            if old.id().same_client_as(&ti.id()) {
                ti.leave(serial, &old);
            }
        }
        if let Some(im) = ime.input_method.lock().expect("input_method").as_ref() {
            im.deactivate();
            im.done();
        }
        *ime.active.lock().expect("active") = None;
    }
    *ime.focus.lock().expect("focus") = surface.clone();
    let Some(surf) = surface else {
        return;
    };
    for ti in ime.text_inputs.lock().expect("text_inputs").iter() {
        if surf.id().same_client_as(&ti.id()) {
            ti.enter(&surf);
        }
    }
    let serial = {
        let mut serial = ime.serial.lock().expect("serial");
        *serial = serial.wrapping_add(1);
        *serial
    };
    for ti in ime.text_inputs_v2.lock().expect("text_inputs_v2").iter() {
        if surf.id().same_client_as(&ti.id()) {
            ti.enter(serial, &surf);
        }
    }
}

/// Apply IME→v2 commits queued during dispatch, after Qt has sent
/// `update_state` + sync in the same wakeup.
pub fn flush_pending_v2<D: SeatHandler + 'static>(seat: &Seat<D>) {
    let ime = seat_ime(seat);
    let pending: Vec<PendingV2> = {
        let mut q = ime.pending_v2.lock().expect("pending_v2");
        q.drain(..).collect()
    };
    if pending.is_empty() {
        return;
    }
    let active = ime.active.lock().expect("active").clone();
    let focus = ime.focus.lock().expect("focus").clone();
    let Some(active_id) = active else {
        return;
    };
    let Some(focus) = focus else {
        return;
    };
    let inputs = ime.text_inputs_v2.lock().expect("text_inputs_v2");
    for op in pending {
        match op {
            PendingV2::CommitString {
                active_id: queued,
                text,
            } => {
                if queued != active_id {
                    continue;
                }
                for ti in inputs.iter() {
                    if ti.id() == active_id && focus.id().same_client_as(&ti.id()) {
                        println!("saai-displayd: text-input-v2 commit_string");
                        ti.commit_string(text.clone());
                    }
                }
            }
            PendingV2::DeleteSurrounding {
                active_id: queued,
                before_length,
                after_length,
            } => {
                if queued != active_id {
                    continue;
                }
                for ti in inputs.iter() {
                    if ti.id() == active_id {
                        println!("saai-displayd: text-input-v2 delete_surrounding");
                        ti.delete_surrounding_text(before_length, after_length);
                    }
                }
            }
        }
    }
}

pub struct SaaiTextInputManager {
    _global_v3: GlobalId,
    _global_v2: GlobalId,
}

impl SaaiTextInputManager {
    pub fn new<D>(display: &DisplayHandle) -> Self
    where
        D: GlobalDispatch<ZwpTextInputManagerV3, ()>
            + Dispatch<ZwpTextInputManagerV3, ()>
            + Dispatch<ZwpTextInputV3, TextInputData<D>>
            + GlobalDispatch<ZwpTextInputManagerV2, ()>
            + Dispatch<ZwpTextInputManagerV2, ()>
            + Dispatch<ZwpTextInputV2, TextInputData<D>>
            + SeatHandler
            + 'static,
    {
        Self {
            _global_v3: display.create_global::<D, ZwpTextInputManagerV3, ()>(1, ()),
            _global_v2: display.create_global::<D, ZwpTextInputManagerV2, ()>(1, ()),
        }
    }
}

pub struct TextInputData<D: SeatHandler> {
    seat: Seat<D>,
}

impl<D> GlobalDispatch<ZwpTextInputManagerV3, (), D> for SaaiTextInputManager
where
    D: GlobalDispatch<ZwpTextInputManagerV3, ()>
        + Dispatch<ZwpTextInputManagerV3, ()>
        + Dispatch<ZwpTextInputV3, TextInputData<D>>
        + SeatHandler
        + 'static,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ZwpTextInputManagerV3>,
        _global_data: &(),
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }
}

impl<D> Dispatch<ZwpTextInputManagerV3, (), D> for SaaiTextInputManager
where
    D: Dispatch<ZwpTextInputManagerV3, ()>
        + Dispatch<ZwpTextInputV3, TextInputData<D>>
        + SeatHandler
        + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &ZwpTextInputManagerV3,
        request: zwp_text_input_manager_v3::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwp_text_input_manager_v3::Request::GetTextInput { id, seat } => {
                let Some(seat) = Seat::<D>::from_resource(&seat) else {
                    return;
                };
                let instance = data_init.init(id, TextInputData { seat: seat.clone() });
                println!("saai-displayd: text-input-v3 get {:?}", instance.id());
                let ime = seat_ime(&seat);
                ime.text_inputs
                    .lock()
                    .expect("text_inputs")
                    .push(instance.clone());
                let focus = ime.focus.lock().expect("focus").clone();
                if let Some(focus) = focus {
                    if focus.id().same_client_as(&instance.id()) {
                        instance.enter(&focus);
                    }
                }
            }
            zwp_text_input_manager_v3::Request::Destroy => {}
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ZwpTextInputV3, TextInputData<D>, D> for SaaiTextInputManager
where
    D: Dispatch<ZwpTextInputV3, TextInputData<D>> + SeatHandler + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        resource: &ZwpTextInputV3,
        request: zwp_text_input_v3::Request,
        data: &TextInputData<D>,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let ime = seat_ime(&data.seat);
        match request {
            zwp_text_input_v3::Request::Enable => {
                println!("saai-displayd: text-input-v3 enable {:?}", resource.id());
                *ime.active.lock().expect("active") = Some(resource.id());
                if let Some(im) = ime.input_method.lock().expect("input_method").as_ref() {
                    im.activate();
                    im.done();
                }
            }
            zwp_text_input_v3::Request::Disable => {
                println!("saai-displayd: text-input-v3 disable {:?}", resource.id());
                *ime.active.lock().expect("active") = None;
                if let Some(im) = ime.input_method.lock().expect("input_method").as_ref() {
                    im.deactivate();
                    im.done();
                }
            }
            zwp_text_input_v3::Request::Destroy => {
                ime.text_inputs
                    .lock()
                    .expect("text_inputs")
                    .retain(|ti| ti.id() != resource.id());
            }
            zwp_text_input_v3::Request::SetSurroundingText { text, .. } => {
                println!(
                    "saai-displayd: text-input-v3 surrounding {:?} bytes={}",
                    resource.id(),
                    text.len()
                );
            }
            zwp_text_input_v3::Request::Commit => {
                println!("saai-displayd: text-input-v3 commit {:?}", resource.id());
            }
            zwp_text_input_v3::Request::SetCursorRectangle {
                x,
                y,
                width,
                height,
            } => {
                println!(
                    "saai-displayd: text-input-v3 cursor {:?} {width}x{height}+{x}+{y}",
                    resource.id()
                );
            }
            zwp_text_input_v3::Request::SetTextChangeCause { .. }
            | zwp_text_input_v3::Request::SetContentType { .. } => {}
            _ => {}
        }
    }

    fn destroyed(
        _state: &mut D,
        _client: ClientId,
        resource: &ZwpTextInputV3,
        data: &TextInputData<D>,
    ) {
        let ime = seat_ime(&data.seat);
        ime.text_inputs
            .lock()
            .expect("text_inputs")
            .retain(|ti| ti.id() != resource.id());
    }
}

impl<D> GlobalDispatch<ZwpTextInputManagerV2, (), D> for SaaiTextInputManager
where
    D: GlobalDispatch<ZwpTextInputManagerV2, ()>
        + Dispatch<ZwpTextInputManagerV2, ()>
        + Dispatch<ZwpTextInputV2, TextInputData<D>>
        + SeatHandler
        + 'static,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ZwpTextInputManagerV2>,
        _global_data: &(),
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }
}

impl<D> Dispatch<ZwpTextInputManagerV2, (), D> for SaaiTextInputManager
where
    D: Dispatch<ZwpTextInputManagerV2, ()>
        + Dispatch<ZwpTextInputV2, TextInputData<D>>
        + SeatHandler
        + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &ZwpTextInputManagerV2,
        request: zwp_text_input_manager_v2::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwp_text_input_manager_v2::Request::GetTextInput { id, seat } => {
                println!("saai-displayd: text-input-v2 get");
                let Some(seat) = Seat::<D>::from_resource(&seat) else {
                    return;
                };
                let instance = data_init.init(id, TextInputData { seat: seat.clone() });
                let ime = seat_ime(&seat);
                ime.text_inputs_v2
                    .lock()
                    .expect("text_inputs_v2")
                    .push(instance.clone());
                let focus = ime.focus.lock().expect("focus").clone();
                if let Some(focus) = focus {
                    if focus.id().same_client_as(&instance.id()) {
                        let serial = {
                            let mut serial = ime.serial.lock().expect("serial");
                            *serial = serial.wrapping_add(1);
                            *serial
                        };
                        instance.enter(serial, &focus);
                    }
                }
            }
            zwp_text_input_manager_v2::Request::Destroy => {}
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ZwpTextInputV2, TextInputData<D>, D> for SaaiTextInputManager
where
    D: Dispatch<ZwpTextInputV2, TextInputData<D>> + SeatHandler + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        resource: &ZwpTextInputV2,
        request: zwp_text_input_v2::Request,
        data: &TextInputData<D>,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let ime = seat_ime(&data.seat);
        match request {
            zwp_text_input_v2::Request::Enable { surface: _ }
            | zwp_text_input_v2::Request::ShowInputPanel => {
                println!("saai-displayd: text-input-v2 enable");
                *ime.active.lock().expect("active") = Some(resource.id());
                if let Some(im) = ime.input_method.lock().expect("input_method").as_ref() {
                    im.activate();
                    im.done();
                }
            }
            zwp_text_input_v2::Request::Disable { surface: _ }
            | zwp_text_input_v2::Request::HideInputPanel => {
                println!("saai-displayd: text-input-v2 disable");
                *ime.active.lock().expect("active") = None;
                if let Some(im) = ime.input_method.lock().expect("input_method").as_ref() {
                    im.deactivate();
                    im.done();
                }
            }
            zwp_text_input_v2::Request::Destroy => {
                ime.text_inputs_v2
                    .lock()
                    .expect("text_inputs_v2")
                    .retain(|ti| ti.id() != resource.id());
            }
            zwp_text_input_v2::Request::UpdateState { .. } => {
                println!("saai-displayd: text-input-v2 update_state");
            }
            zwp_text_input_v2::Request::SetSurroundingText { text, .. } => {
                println!(
                    "saai-displayd: text-input-v2 surrounding bytes={}",
                    text.len()
                );
            }
            zwp_text_input_v2::Request::SetCursorRectangle {
                x,
                y,
                width,
                height,
            } => {
                println!("saai-displayd: text-input-v2 cursor {width}x{height}+{x}+{y}");
            }
            _ => {}
        }
    }

    fn destroyed(
        _state: &mut D,
        _client: ClientId,
        resource: &ZwpTextInputV2,
        data: &TextInputData<D>,
    ) {
        let ime = seat_ime(&data.seat);
        ime.text_inputs_v2
            .lock()
            .expect("text_inputs_v2")
            .retain(|ti| ti.id() != resource.id());
    }
}

pub struct SaaiInputMethodManager {
    _global: GlobalId,
}

impl SaaiInputMethodManager {
    pub fn new<D>(display: &DisplayHandle) -> Self
    where
        D: GlobalDispatch<ZwpInputMethodManagerV2, ()>
            + Dispatch<ZwpInputMethodManagerV2, ()>
            + Dispatch<ZwpInputMethodV2, InputMethodData<D>>
            + Dispatch<ZwpInputMethodKeyboardGrabV2, ()>
            + Dispatch<ZwpInputPopupSurfaceV2, ()>
            + SeatHandler
            + 'static,
    {
        Self {
            _global: display.create_global::<D, ZwpInputMethodManagerV2, ()>(1, ()),
        }
    }
}

pub struct InputMethodData<D: SeatHandler> {
    seat: Seat<D>,
}

impl<D> GlobalDispatch<ZwpInputMethodManagerV2, (), D> for SaaiInputMethodManager
where
    D: GlobalDispatch<ZwpInputMethodManagerV2, ()>
        + Dispatch<ZwpInputMethodManagerV2, ()>
        + Dispatch<ZwpInputMethodV2, InputMethodData<D>>
        + Dispatch<ZwpInputMethodKeyboardGrabV2, ()>
        + Dispatch<ZwpInputPopupSurfaceV2, ()>
        + SeatHandler
        + 'static,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ZwpInputMethodManagerV2>,
        _global_data: &(),
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }
}

impl<D> Dispatch<ZwpInputMethodManagerV2, (), D> for SaaiInputMethodManager
where
    D: Dispatch<ZwpInputMethodManagerV2, ()>
        + Dispatch<ZwpInputMethodV2, InputMethodData<D>>
        + Dispatch<ZwpInputMethodKeyboardGrabV2, ()>
        + Dispatch<ZwpInputPopupSurfaceV2, ()>
        + SeatHandler
        + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &ZwpInputMethodManagerV2,
        request: zwp_input_method_manager_v2::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwp_input_method_manager_v2::Request::GetInputMethod { seat, input_method } => {
                let Some(seat) = Seat::<D>::from_resource(&seat) else {
                    return;
                };
                // Deliberately never calls seat.get_keyboard() (ADR-022/267).
                let instance = data_init.init(input_method, InputMethodData { seat: seat.clone() });
                let ime = seat_ime(&seat);
                *ime.input_method.lock().expect("input_method") = Some(instance.clone());
                if ime.active.lock().expect("active").is_some() {
                    instance.activate();
                    instance.done();
                }
            }
            zwp_input_method_manager_v2::Request::Destroy => {}
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ZwpInputMethodV2, InputMethodData<D>, D> for SaaiInputMethodManager
where
    D: Dispatch<ZwpInputMethodV2, InputMethodData<D>>
        + Dispatch<ZwpInputMethodKeyboardGrabV2, ()>
        + Dispatch<ZwpInputPopupSurfaceV2, ()>
        + SeatHandler
        + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &ZwpInputMethodV2,
        request: zwp_input_method_v2::Request,
        data: &InputMethodData<D>,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let ime = seat_ime(&data.seat);
        match request {
            zwp_input_method_v2::Request::CommitString { text } => {
                let active = ime.active.lock().expect("active").clone();
                let focus = ime.focus.lock().expect("focus").clone();
                let Some(active_id) = active else {
                    println!("saai-displayd: text-input-v3 commit_string dropped no-active");
                    return;
                };
                let Some(focus) = focus else {
                    println!("saai-displayd: text-input-v3 commit_string dropped no-focus");
                    return;
                };
                let mut forwarded = false;
                for ti in ime.text_inputs.lock().expect("text_inputs").iter() {
                    if ti.id() == active_id && focus.id().same_client_as(&ti.id()) {
                        println!("saai-displayd: text-input-v3 commit_string {:?}", active_id);
                        ti.commit_string(Some(text.clone()));
                        forwarded = true;
                    }
                }
                if !forwarded {
                    println!("saai-displayd: text-input-v3 commit_string dropped no-match");
                }
                ime.pending_v2.lock().expect("pending_v2").push(
                    PendingV2::CommitString {
                        active_id,
                        text,
                    },
                );
            }
            zwp_input_method_v2::Request::SetPreeditString {
                text,
                cursor_begin,
                cursor_end,
            } => {
                let active = ime.active.lock().expect("active").clone();
                let Some(active_id) = active else {
                    return;
                };
                for ti in ime.text_inputs.lock().expect("text_inputs").iter() {
                    if ti.id() == active_id {
                        ti.preedit_string(Some(text.clone()), cursor_begin, cursor_end);
                    }
                }
            }
            zwp_input_method_v2::Request::DeleteSurroundingText {
                before_length,
                after_length,
            } => {
                let active = ime.active.lock().expect("active").clone();
                let Some(active_id) = active else {
                    return;
                };
                for ti in ime.text_inputs.lock().expect("text_inputs").iter() {
                    if ti.id() == active_id {
                        ti.delete_surrounding_text(before_length, after_length);
                    }
                }
                ime.pending_v2.lock().expect("pending_v2").push(
                    PendingV2::DeleteSurrounding {
                        active_id,
                        before_length,
                        after_length,
                    },
                );
            }
            zwp_input_method_v2::Request::Commit { serial: _ } => {
                let mut serial = ime.serial.lock().expect("serial");
                *serial = serial.wrapping_add(1);
                let done_serial = *serial;
                drop(serial);
                let active = ime.active.lock().expect("active").clone();
                let Some(active_id) = active else {
                    println!("saai-displayd: text-input-v3 done dropped no-active");
                    return;
                };
                for ti in ime.text_inputs.lock().expect("text_inputs").iter() {
                    if ti.id() == active_id {
                        println!("saai-displayd: text-input-v3 done {:?}", active_id);
                        ti.done(done_serial);
                    }
                }
            }
            zwp_input_method_v2::Request::GrabKeyboard { keyboard } => {
                // Accept the object so the IME client is not disconnected.
                // Do not send a keymap — that is the ADR-022 crash.
                data_init.init(keyboard, ());
            }
            zwp_input_method_v2::Request::GetInputPopupSurface { id, surface: _ } => {
                data_init.init(id, ());
            }
            zwp_input_method_v2::Request::Destroy => {
                *ime.input_method.lock().expect("input_method") = None;
            }
            _ => {}
        }
    }
}

impl<D> Dispatch<ZwpInputMethodKeyboardGrabV2, (), D> for SaaiInputMethodManager
where
    D: Dispatch<ZwpInputMethodKeyboardGrabV2, ()> + SeatHandler + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &ZwpInputMethodKeyboardGrabV2,
        _request: <ZwpInputMethodKeyboardGrabV2 as Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
    }
}

impl<D> Dispatch<ZwpInputPopupSurfaceV2, (), D> for SaaiInputMethodManager
where
    D: Dispatch<ZwpInputPopupSurfaceV2, ()> + SeatHandler + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &ZwpInputPopupSurfaceV2,
        _request: <ZwpInputPopupSurfaceV2 as Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
    }
}

#[macro_export]
macro_rules! delegate_saai_text_ime {
    ($ty:ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($ty: [
            smithay::reexports::wayland_protocols::wp::text_input::zv3::server::zwp_text_input_manager_v3::ZwpTextInputManagerV3: ()
        ] => $crate::text_ime::SaaiTextInputManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_protocols::wp::text_input::zv3::server::zwp_text_input_manager_v3::ZwpTextInputManagerV3: ()
        ] => $crate::text_ime::SaaiTextInputManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_protocols::wp::text_input::zv3::server::zwp_text_input_v3::ZwpTextInputV3: $crate::text_ime::TextInputData<Self>
        ] => $crate::text_ime::SaaiTextInputManager);
        smithay::reexports::wayland_server::delegate_global_dispatch!($ty: [
            wayland_protocols_plasma::text_input::v2::server::zwp_text_input_manager_v2::ZwpTextInputManagerV2: ()
        ] => $crate::text_ime::SaaiTextInputManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            wayland_protocols_plasma::text_input::v2::server::zwp_text_input_manager_v2::ZwpTextInputManagerV2: ()
        ] => $crate::text_ime::SaaiTextInputManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            wayland_protocols_plasma::text_input::v2::server::zwp_text_input_v2::ZwpTextInputV2: $crate::text_ime::TextInputData<Self>
        ] => $crate::text_ime::SaaiTextInputManager);

        smithay::reexports::wayland_server::delegate_global_dispatch!($ty: [
            smithay::reexports::wayland_protocols_misc::zwp_input_method_v2::server::zwp_input_method_manager_v2::ZwpInputMethodManagerV2: ()
        ] => $crate::text_ime::SaaiInputMethodManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_protocols_misc::zwp_input_method_v2::server::zwp_input_method_manager_v2::ZwpInputMethodManagerV2: ()
        ] => $crate::text_ime::SaaiInputMethodManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_protocols_misc::zwp_input_method_v2::server::zwp_input_method_v2::ZwpInputMethodV2: $crate::text_ime::InputMethodData<Self>
        ] => $crate::text_ime::SaaiInputMethodManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_protocols_misc::zwp_input_method_v2::server::zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2: ()
        ] => $crate::text_ime::SaaiInputMethodManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_protocols_misc::zwp_input_method_v2::server::zwp_input_popup_surface_v2::ZwpInputPopupSurfaceV2: ()
        ] => $crate::text_ime::SaaiInputMethodManager);
    };
}
