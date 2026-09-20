//! Layer-shell placement for APP-04. Status bar is top; OSK is bottom.
//! Touch hits the topmost layer whose destination contains the point.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct LayerGeom {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl LayerGeom {
    #[allow(dead_code)]
    pub fn contains(self, x: f64, y: f64) -> bool {
        let right = f64::from(self.x.saturating_add(self.width));
        let bottom = f64::from(self.y.saturating_add(self.height));
        x >= f64::from(self.x) && x < right && y >= f64::from(self.y) && y < bottom
    }
}

/// Width 0 means the output width. Height 0 means the 120px status-bar
/// default (unset at GetLayerSurface). A positive height is honored so
/// an OSK is not forced to 120.
pub fn configure_size(
    output_w: i32,
    output_h: i32,
    requested_w: i32,
    requested_h: i32,
) -> (i32, i32) {
    let width = if requested_w <= 0 {
        output_w.max(1)
    } else {
        requested_w.min(output_w).max(1)
    };
    let height = if requested_h <= 0 {
        120.min(output_h).max(1)
    } else {
        requested_h.min(output_h).max(1)
    };
    (width, height)
}

#[allow(dead_code)]
pub fn destination(
    output_w: i32,
    output_h: i32,
    width: i32,
    height: i32,
    top: bool,
    bottom: bool,
    left: bool,
    right: bool,
) -> LayerGeom {
    let x = if right && !left {
        output_w.saturating_sub(width)
    } else {
        0
    };
    let y = if bottom && !top {
        output_h.saturating_sub(height)
    } else {
        0
    };
    LayerGeom {
        x,
        y,
        width,
        height,
    }
}

/// Intersection of a source blit placed at `(dst_x, dst_y)` with the panel.
/// `None` if it is completely off-screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlitWindow {
    pub src_x: u32,
    pub src_y: u32,
    pub dst_x: u32,
    pub dst_y: u32,
    pub width: u32,
    pub height: u32,
}

#[allow(dead_code)]
pub fn blit_window(
    dst_x: i32,
    dst_y: i32,
    src_w: i32,
    src_h: i32,
    panel_w: i32,
    panel_h: i32,
) -> Option<BlitWindow> {
    if src_w <= 0 || src_h <= 0 || panel_w <= 0 || panel_h <= 0 {
        return None;
    }
    let x0 = dst_x.max(0);
    let y0 = dst_y.max(0);
    let x1 = dst_x.saturating_add(src_w).min(panel_w);
    let y1 = dst_y.saturating_add(src_h).min(panel_h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some(BlitWindow {
        src_x: u32::try_from(x0 - dst_x).unwrap_or(0),
        src_y: u32::try_from(y0 - dst_y).unwrap_or(0),
        dst_x: x0 as u32,
        dst_y: y0 as u32,
        width: (x1 - x0) as u32,
        height: (y1 - y0) as u32,
    })
}

/// Last matching layer is on top (same order as recomposite).
#[allow(dead_code)]
pub fn hit_layer(layers: &[LayerGeom], x: f64, y: f64) -> Option<usize> {
    layers
        .iter()
        .enumerate()
        .rev()
        .find(|(_, geom)| geom.contains(x, y))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_bar_stays_a_top_strip() {
        let (w, h) = configure_size(1080, 2400, 0, 120);
        assert_eq!((w, h), (1080, 120));
        let geom = destination(1080, 2400, w, h, true, false, true, true);
        assert_eq!(
            geom,
            LayerGeom {
                x: 0,
                y: 0,
                width: 1080,
                height: 120
            }
        );
        assert!(geom.contains(540.0, 60.0));
        assert!(!geom.contains(540.0, 200.0));
    }

    #[test]
    fn osk_docks_at_the_bottom() {
        let (w, h) = configure_size(1080, 2400, 0, 800);
        assert_eq!((w, h), (1080, 800));
        let geom = destination(1080, 2400, w, h, false, true, true, true);
        assert_eq!(
            geom,
            LayerGeom {
                x: 0,
                y: 1600,
                width: 1080,
                height: 800
            }
        );
        assert!(geom.contains(100.0, 2000.0));
        assert!(!geom.contains(100.0, 100.0));
    }

    #[test]
    fn later_layer_wins_when_rects_overlap() {
        let bar = destination(1080, 2400, 1080, 120, true, false, true, true);
        let osk = destination(1080, 2400, 1080, 800, false, true, true, true);
        assert_eq!(hit_layer(&[bar, osk], 540.0, 2000.0), Some(1));
        assert_eq!(hit_layer(&[bar, osk], 540.0, 60.0), Some(0));
        assert_eq!(hit_layer(&[bar, osk], 540.0, 800.0), None);
    }

    #[test]
    fn unset_size_defaults_to_the_status_bar_strip() {
        assert_eq!(configure_size(1080, 2400, 0, 0), (1080, 120));
    }

    #[test]
    fn status_bar_blit_stays_at_the_origin() {
        assert_eq!(
            blit_window(0, 0, 1080, 120, 1080, 2400),
            Some(BlitWindow {
                src_x: 0,
                src_y: 0,
                dst_x: 0,
                dst_y: 0,
                width: 1080,
                height: 120
            })
        );
    }

    #[test]
    fn osk_blit_starts_at_the_bottom() {
        let geom = destination(1080, 2400, 1080, 800, false, true, true, true);
        assert_eq!(
            blit_window(geom.x, geom.y, geom.width, geom.height, 1080, 2400),
            Some(BlitWindow {
                src_x: 0,
                src_y: 0,
                dst_x: 0,
                dst_y: 1600,
                width: 1080,
                height: 800
            })
        );
    }

    #[test]
    fn offscreen_blit_is_skipped() {
        assert_eq!(blit_window(0, 2400, 1080, 120, 1080, 2400), None);
        assert_eq!(blit_window(-1080, 0, 1080, 120, 1080, 2400), None);
    }
}
