use crate::{
    scale_argb_to_panther_bgrx, FrameGeometry, TouchBounds, FRAME_HEIGHT, FRAME_WIDTH,
    PANTHER_HEIGHT, PANTHER_WIDTH,
};
use anyhow::{anyhow, bail, Context, Result};
use drm::{
    buffer::{Buffer, DrmFourcc},
    control::{
        connector, crtc, dumbbuffer::DumbBuffer, framebuffer, Device as ControlDevice, FbCmd2Flags,
        Mode,
    },
    Device,
};
use evdev::{AbsoluteAxisCode, Device as InputDevice, EventSummary, SynchronizationCode};
use std::{
    fs::{File, OpenOptions},
    os::fd::{AsFd, BorrowedFd},
    path::Path,
};

#[derive(Debug)]
struct Card(File);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for Card {}
impl ControlDevice for Card {}

struct PlanarDumb<'a>(&'a DumbBuffer);

impl drm::buffer::PlanarBuffer for PlanarDumb<'_> {
    fn size(&self) -> (u32, u32) {
        self.0.size()
    }

    fn format(&self) -> DrmFourcc {
        self.0.format()
    }

    fn modifier(&self) -> Option<drm::buffer::DrmModifier> {
        None
    }

    fn pitches(&self) -> [u32; 4] {
        [self.0.pitch(), 0, 0, 0]
    }

    fn handles(&self) -> [Option<drm::buffer::Handle>; 4] {
        [Some(self.0.handle()), None, None, None]
    }

    fn offsets(&self) -> [u32; 4] {
        [0; 4]
    }
}

pub struct PantherOutput {
    card: Card,
    connector: connector::Handle,
    crtc: crtc::Handle,
    mode: Mode,
    buffer: DumbBuffer,
    framebuffer: framebuffer::Handle,
    active: bool,
}

impl PantherOutput {
    pub fn open(path: &Path) -> Result<Self> {
        let card = Card(
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .with_context(|| format!("open DRM card {}", path.display()))?,
        );
        let resources = card.resource_handles().context("read DRM resources")?;
        let connector_info = resources
            .connectors()
            .iter()
            .filter_map(|handle| card.get_connector(*handle, true).ok())
            .find(|info| {
                info.interface() == connector::Interface::DSI
                    && info.state() == connector::State::Connected
            })
            .ok_or_else(|| anyhow!("no connected DSI connector"))?;
        let mode = connector_info
            .modes()
            .iter()
            .copied()
            .find(|mode| {
                mode.size() == (PANTHER_WIDTH as u16, PANTHER_HEIGHT as u16)
                    && mode.vrefresh() == 60
            })
            .ok_or_else(|| anyhow!("DSI mode 1080x2400x60 unavailable"))?;

        let crtc = connector_info
            .current_encoder()
            .and_then(|handle| card.get_encoder(handle).ok())
            .and_then(|encoder| encoder.crtc())
            .or_else(|| {
                connector_info.encoders().iter().find_map(|handle| {
                    let encoder = card.get_encoder(*handle).ok()?;
                    resources
                        .filter_crtcs(encoder.possible_crtcs())
                        .first()
                        .copied()
                })
            })
            .or_else(|| resources.crtcs().first().copied())
            .ok_or_else(|| anyhow!("no usable CRTC"))?;

        let buffer = card
            .create_dumb_buffer((PANTHER_WIDTH, PANTHER_HEIGHT), DrmFourcc::Xrgb8888, 32)
            .context("create DRM dumb buffer")?;
        let framebuffer = card
            .add_planar_framebuffer(&PlanarDumb(&buffer), FbCmd2Flags::empty())
            .context("add DRM framebuffer")?;

        Ok(Self {
            card,
            connector: connector_info.handle(),
            crtc,
            mode,
            buffer,
            framebuffer,
            active: false,
        })
    }

    pub fn present(
        &mut self,
        source: &[u8],
        source_width: u32,
        source_height: u32,
        source_stride: u32,
    ) -> Result<()> {
        if source_width != FRAME_WIDTH || source_height != FRAME_HEIGHT {
            bail!(
                "S03 surface must be {}x{}, got {}x{}",
                FRAME_WIDTH,
                FRAME_HEIGHT,
                source_width,
                source_height
            );
        }
        let pitch = self.buffer.pitch();
        {
            let mut mapping = self
                .card
                .map_dumb_buffer(&mut self.buffer)
                .context("map DRM dumb buffer")?;
            scale_argb_to_panther_bgrx(
                source,
                FrameGeometry {
                    width: source_width,
                    height: source_height,
                    stride: source_stride,
                },
                mapping.as_mut(),
                FrameGeometry {
                    width: PANTHER_WIDTH,
                    height: PANTHER_HEIGHT,
                    stride: pitch,
                },
            )
            .map_err(|error| anyhow!("transform Wayland frame: {error:?}"))?;
        }
        self.card
            .set_crtc(
                self.crtc,
                Some(self.framebuffer),
                (0, 0),
                &[self.connector],
                Some(self.mode),
            )
            .context("set Pixel 7 CRTC")?;
        self.active = true;
        Ok(())
    }
}

impl Drop for PantherOutput {
    fn drop(&mut self) {
        if self.active {
            let _ = self.card.set_crtc(self.crtc, None, (0, 0), &[], None);
        }
        let _ = self.card.destroy_framebuffer(self.framebuffer);
        let _ = self.card.destroy_dumb_buffer(self.buffer);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TouchEvent {
    Down { x: f64, y: f64 },
    Motion { x: f64, y: f64 },
    Up { x: f64, y: f64 },
}

pub struct PantherTouch {
    device: InputDevice,
    bounds: TouchBounds,
    tracking: bool,
    pointer_down: bool,
    changed: bool,
    x: i32,
    y: i32,
}

impl PantherTouch {
    pub fn open(path: &Path) -> Result<Self> {
        let device = InputDevice::open(path)
            .with_context(|| format!("open touch input {}", path.display()))?;
        device.set_nonblocking(true)?;
        let abs = device
            .get_absinfo()
            .context("read touch absolute axis bounds")?;
        let mut x = None;
        let mut y = None;
        for (code, info) in abs {
            if code == AbsoluteAxisCode::ABS_MT_POSITION_X {
                x = Some((info.minimum(), info.maximum()));
            } else if code == AbsoluteAxisCode::ABS_MT_POSITION_Y {
                y = Some((info.minimum(), info.maximum()));
            }
        }
        let (min_x, max_x) = x.ok_or_else(|| anyhow!("touch has no ABS_MT_POSITION_X"))?;
        let (min_y, max_y) = y.ok_or_else(|| anyhow!("touch has no ABS_MT_POSITION_Y"))?;
        Ok(Self {
            device,
            bounds: TouchBounds {
                min_x,
                max_x,
                min_y,
                max_y,
            },
            tracking: false,
            pointer_down: false,
            changed: false,
            x: min_x,
            y: min_y,
        })
    }

    pub fn drain(&mut self) -> Result<Vec<TouchEvent>> {
        let events = match self.device.fetch_events() {
            Ok(events) => events.collect::<Vec<_>>(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(Vec::new()),
            Err(error) => return Err(error).context("read touch events"),
        };
        let mut output = Vec::new();
        for event in events {
            match event.destructure() {
                EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_TRACKING_ID, value) => {
                    self.tracking = value >= 0;
                }
                EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_POSITION_X, value) => {
                    self.x = value;
                    self.changed = true;
                }
                EventSummary::AbsoluteAxis(_, AbsoluteAxisCode::ABS_MT_POSITION_Y, value) => {
                    self.y = value;
                    self.changed = true;
                }
                EventSummary::Synchronization(_, SynchronizationCode::SYN_REPORT, _) => {
                    let (x, y) = self
                        .bounds
                        .normalize(self.x, self.y, FRAME_WIDTH, FRAME_HEIGHT);
                    if self.tracking && !self.pointer_down {
                        self.pointer_down = true;
                        output.push(TouchEvent::Down { x, y });
                    } else if self.tracking && self.changed {
                        output.push(TouchEvent::Motion { x, y });
                    } else if !self.tracking && self.pointer_down {
                        self.pointer_down = false;
                        output.push(TouchEvent::Up { x, y });
                    }
                    self.changed = false;
                }
                _ => {}
            }
        }
        Ok(output)
    }
}
