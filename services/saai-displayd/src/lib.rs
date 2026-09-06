//! Protocol state shared by the headless compositor and its tests.

use sha2::{Digest, Sha256};
use std::fmt::Write;

pub const FRAME_WIDTH: u32 = 64;
pub const FRAME_HEIGHT: u32 = 48;
pub const FRAME_STRIDE: u32 = FRAME_WIDTH * 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBackendCapabilities {
    dmabuf_import: bool,
}

impl RuntimeBackendCapabilities {
    pub const fn software_only() -> Self {
        Self {
            dmabuf_import: false,
        }
    }

    pub const fn with_dmabuf_import() -> Self {
        Self {
            dmabuf_import: true,
        }
    }

    pub const fn supports_wl_shm(self) -> bool {
        true
    }

    pub const fn supports_dmabuf_import(self) -> bool {
        self.dmabuf_import
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmabufDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub modifier: u64,
}

pub trait DmabufImportCheck {
    type Error: std::fmt::Display;

    fn check_import(&self, descriptor: &DmabufDescriptor) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferTransport {
    WlShm,
    Dmabuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WlShmFallback {
    DmabufCapabilityUnavailable,
    NoDmabufCandidate,
    DmabufImportRejected(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionPlan {
    pub transport: BufferTransport,
    pub dmabuf_available: bool,
    pub fallback: Option<WlShmFallback>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareComposedFrame {
    pub transport: BufferTransport,
    pub byte_len: usize,
    pub frame_hash: String,
}

pub struct SoftwareCompositionBackend<C> {
    capabilities: RuntimeBackendCapabilities,
    import_check: C,
}

impl<C> SoftwareCompositionBackend<C>
where
    C: DmabufImportCheck,
{
    pub const fn new(capabilities: RuntimeBackendCapabilities, import_check: C) -> Self {
        Self {
            capabilities,
            import_check,
        }
    }

    pub const fn capabilities(&self) -> RuntimeBackendCapabilities {
        self.capabilities
    }

    pub fn compose_wl_shm(&self, frame: &[u8]) -> SoftwareComposedFrame {
        SoftwareComposedFrame {
            transport: BufferTransport::WlShm,
            byte_len: frame.len(),
            frame_hash: frame_hash(frame),
        }
    }

    pub fn plan(&self, candidate: Option<&DmabufDescriptor>) -> CompositionPlan {
        if !self.capabilities.supports_dmabuf_import() {
            return CompositionPlan {
                transport: BufferTransport::WlShm,
                dmabuf_available: false,
                fallback: Some(WlShmFallback::DmabufCapabilityUnavailable),
            };
        }

        let Some(candidate) = candidate else {
            return CompositionPlan {
                transport: BufferTransport::WlShm,
                dmabuf_available: false,
                fallback: Some(WlShmFallback::NoDmabufCandidate),
            };
        };

        match self.import_check.check_import(candidate) {
            Ok(()) => CompositionPlan {
                transport: BufferTransport::Dmabuf,
                dmabuf_available: true,
                fallback: None,
            },
            Err(error) => CompositionPlan {
                transport: BufferTransport::WlShm,
                dmabuf_available: false,
                fallback: Some(WlShmFallback::DmabufImportRejected(error.to_string())),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolFault {
    BufferBeforeConfigure,
    InvalidConfigureSerial,
    RoleAlreadyAssigned,
}

#[derive(Debug, Default)]
pub struct SurfaceLifecycle {
    role_assigned: bool,
    configure_serial: Option<u32>,
    acked_serial: Option<u32>,
}

impl SurfaceLifecycle {
    pub fn assign_toplevel_role(&mut self) -> Result<(), ProtocolFault> {
        if self.role_assigned {
            return Err(ProtocolFault::RoleAlreadyAssigned);
        }
        self.role_assigned = true;
        Ok(())
    }

    pub fn configure(&mut self, serial: u32) {
        self.configure_serial = Some(serial);
        self.acked_serial = None;
    }

    pub fn ack_configure(&mut self, serial: u32) -> Result<(), ProtocolFault> {
        if self.configure_serial != Some(serial) {
            return Err(ProtocolFault::InvalidConfigureSerial);
        }
        self.acked_serial = Some(serial);
        Ok(())
    }

    pub fn validate_buffer_commit(&self) -> Result<(), ProtocolFault> {
        match (self.configure_serial, self.acked_serial) {
            (Some(configured), Some(acked)) if configured == acked => Ok(()),
            _ => Err(ProtocolFault::BufferBeforeConfigure),
        }
    }
}

#[derive(Debug, Default)]
pub struct FocusState {
    focused_surface: Option<u64>,
}

impl FocusState {
    pub fn focus(&mut self, surface: u64) {
        self.focused_surface = Some(surface);
    }

    pub fn receives_input(&self, surface: u64) -> bool {
        self.focused_surface == Some(surface)
    }
}

pub fn demo_frame() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((FRAME_STRIDE * FRAME_HEIGHT) as usize);
    for y in 0..FRAME_HEIGHT {
        for x in 0..FRAME_WIDTH {
            let checker = ((x / 8) + (y / 8)) % 2 == 0;
            let (red, green, blue) = if checker {
                (0x16, 0xc7, 0x84)
            } else {
                (0x21, 0x2a, 0x3a)
            };
            pixels.extend_from_slice(&[blue, green, red, 0xff]);
        }
    }
    pixels
}

pub fn frame_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            write!(out, "{byte:02x}").expect("writing to String cannot fail");
            out
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct ImportCheck<'a> {
        called: &'a Cell<bool>,
        result: Result<(), &'static str>,
    }

    impl DmabufImportCheck for ImportCheck<'_> {
        type Error = &'static str;

        fn check_import(&self, _descriptor: &DmabufDescriptor) -> Result<(), Self::Error> {
            self.called.set(true);
            self.result
        }
    }

    fn dmabuf() -> DmabufDescriptor {
        DmabufDescriptor {
            width: FRAME_WIDTH,
            height: FRAME_HEIGHT,
            format: 0x3432_5258,
            modifier: 0,
        }
    }

    #[test]
    fn wl_shm_is_always_available_and_dmabuf_needs_capability() {
        let called = Cell::new(false);
        let backend = SoftwareCompositionBackend::new(
            RuntimeBackendCapabilities::software_only(),
            ImportCheck {
                called: &called,
                result: Ok(()),
            },
        );

        assert!(backend.capabilities().supports_wl_shm());
        assert_eq!(
            backend.plan(Some(&dmabuf())),
            CompositionPlan {
                transport: BufferTransport::WlShm,
                dmabuf_available: false,
                fallback: Some(WlShmFallback::DmabufCapabilityUnavailable),
            }
        );
        assert!(!called.get(), "import must not run without capability");
    }

    #[test]
    fn rejected_dmabuf_import_falls_back_without_error() {
        let called = Cell::new(false);
        let backend = SoftwareCompositionBackend::new(
            RuntimeBackendCapabilities::with_dmabuf_import(),
            ImportCheck {
                called: &called,
                result: Err("unsupported modifier"),
            },
        );

        assert_eq!(
            backend.plan(Some(&dmabuf())),
            CompositionPlan {
                transport: BufferTransport::WlShm,
                dmabuf_available: false,
                fallback: Some(WlShmFallback::DmabufImportRejected(
                    "unsupported modifier".to_owned()
                )),
            }
        );
        assert!(called.get());
    }

    #[test]
    fn dmabuf_is_available_only_after_successful_import_check() {
        let called = Cell::new(false);
        let backend = SoftwareCompositionBackend::new(
            RuntimeBackendCapabilities::with_dmabuf_import(),
            ImportCheck {
                called: &called,
                result: Ok(()),
            },
        );

        assert_eq!(
            backend.plan(Some(&dmabuf())),
            CompositionPlan {
                transport: BufferTransport::Dmabuf,
                dmabuf_available: true,
                fallback: None,
            }
        );
        assert!(called.get());
    }

    #[test]
    fn software_composition_accepts_wl_shm_without_dmabuf() {
        let called = Cell::new(false);
        let backend = SoftwareCompositionBackend::new(
            RuntimeBackendCapabilities::software_only(),
            ImportCheck {
                called: &called,
                result: Err("must not be called"),
            },
        );
        let frame = demo_frame();

        assert_eq!(
            backend.compose_wl_shm(&frame),
            SoftwareComposedFrame {
                transport: BufferTransport::WlShm,
                byte_len: frame.len(),
                frame_hash: frame_hash(&frame),
            }
        );
        assert!(!called.get());
    }

    #[test]
    fn configure_ack_commit_is_ordered() {
        let mut lifecycle = SurfaceLifecycle::default();
        lifecycle.assign_toplevel_role().unwrap();
        assert_eq!(
            lifecycle.validate_buffer_commit(),
            Err(ProtocolFault::BufferBeforeConfigure)
        );
        lifecycle.configure(7);
        assert_eq!(
            lifecycle.ack_configure(8),
            Err(ProtocolFault::InvalidConfigureSerial)
        );
        lifecycle.ack_configure(7).unwrap();
        assert_eq!(lifecycle.validate_buffer_commit(), Ok(()));
    }

    #[test]
    fn a_surface_cannot_receive_two_roles() {
        let mut lifecycle = SurfaceLifecycle::default();
        lifecycle.assign_toplevel_role().unwrap();
        assert_eq!(
            lifecycle.assign_toplevel_role(),
            Err(ProtocolFault::RoleAlreadyAssigned)
        );
    }

    #[test]
    fn input_only_reaches_focus() {
        let mut focus = FocusState::default();
        focus.focus(11);
        assert!(focus.receives_input(11));
        assert!(!focus.receives_input(12));
    }

    #[test]
    fn demo_frame_hash_is_stable() {
        assert_eq!(
            frame_hash(&demo_frame()),
            "9b05ff34424f63e620a88baecc12950fe13e242ca1645ec9d8d57d939186f99d"
        );
    }
}
