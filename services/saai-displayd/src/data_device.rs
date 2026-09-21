//! ADR-294 / ADR-023: `wl_data_device_manager` without smithay's ungated
//! selection path.
//!
//! GTK4 still needs the global to open a display (ADR-021). Smithay 0.7
//! `SelectionHandler` cannot refuse SetSelection or Receive. This manager
//! advertises the protocol, accepts binds, and cancels every source. No
//! `wl_data_offer` is created, so native copy/paste cannot complete.
//! Portal clipboard (AUTH-08) stays the capability-gated channel.

use smithay::reexports::wayland_server::{
    backend::GlobalId,
    protocol::{
        wl_data_device::{self, WlDataDevice},
        wl_data_device_manager::{self, WlDataDeviceManager},
        wl_data_source::{self, WlDataSource},
    },
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New,
};

const DATA_DEVICE_MANAGER_VERSION: u32 = 3;

pub struct SaaiDataDeviceManager {
    _global: GlobalId,
}

impl SaaiDataDeviceManager {
    pub fn new<D>(display: &DisplayHandle) -> Self
    where
        D: GlobalDispatch<WlDataDeviceManager, ()>
            + Dispatch<WlDataDeviceManager, ()>
            + Dispatch<WlDataDevice, ()>
            + Dispatch<WlDataSource, ()>
            + 'static,
    {
        Self {
            _global: display
                .create_global::<D, WlDataDeviceManager, ()>(DATA_DEVICE_MANAGER_VERSION, ()),
        }
    }
}

fn cancel_source(source: Option<WlDataSource>) {
    if let Some(source) = source {
        source.cancelled();
    }
}

impl<D> GlobalDispatch<WlDataDeviceManager, (), D> for SaaiDataDeviceManager
where
    D: GlobalDispatch<WlDataDeviceManager, ()>
        + Dispatch<WlDataDeviceManager, ()>
        + Dispatch<WlDataDevice, ()>
        + Dispatch<WlDataSource, ()>
        + 'static,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<WlDataDeviceManager>,
        _global_data: &(),
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }
}

impl<D> Dispatch<WlDataDeviceManager, (), D> for SaaiDataDeviceManager
where
    D: Dispatch<WlDataDeviceManager, ()>
        + Dispatch<WlDataDevice, ()>
        + Dispatch<WlDataSource, ()>
        + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &WlDataDeviceManager,
        request: wl_data_device_manager::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            wl_data_device_manager::Request::CreateDataSource { id } => {
                data_init.init(id, ());
            }
            wl_data_device_manager::Request::GetDataDevice { id, seat: _ } => {
                data_init.init(id, ());
            }
            _ => {}
        }
    }
}

impl<D> Dispatch<WlDataDevice, (), D> for SaaiDataDeviceManager
where
    D: Dispatch<WlDataDevice, ()> + Dispatch<WlDataSource, ()> + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &WlDataDevice,
        request: wl_data_device::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            wl_data_device::Request::StartDrag {
                source,
                origin: _,
                icon: _,
                serial: _,
            } => {
                cancel_source(source);
            }
            wl_data_device::Request::SetSelection { source, serial: _ } => {
                cancel_source(source);
            }
            wl_data_device::Request::Release => {}
            _ => {}
        }
    }
}

impl<D> Dispatch<WlDataSource, (), D> for SaaiDataDeviceManager
where
    D: Dispatch<WlDataSource, ()> + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &WlDataSource,
        request: wl_data_source::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            wl_data_source::Request::Offer { mime_type: _ }
            | wl_data_source::Request::Destroy
            | wl_data_source::Request::SetActions { dnd_actions: _ } => {}
            _ => {}
        }
    }
}

#[macro_export]
macro_rules! delegate_saai_data_device {
    ($ty:ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($ty: [
            smithay::reexports::wayland_server::protocol::wl_data_device_manager::WlDataDeviceManager: ()
        ] => $crate::data_device::SaaiDataDeviceManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_server::protocol::wl_data_device_manager::WlDataDeviceManager: ()
        ] => $crate::data_device::SaaiDataDeviceManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_server::protocol::wl_data_device::WlDataDevice: ()
        ] => $crate::data_device::SaaiDataDeviceManager);
        smithay::reexports::wayland_server::delegate_dispatch!($ty: [
            smithay::reexports::wayland_server::protocol::wl_data_source::WlDataSource: ()
        ] => $crate::data_device::SaaiDataDeviceManager);
    };
}
