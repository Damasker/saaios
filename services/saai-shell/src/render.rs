use saai_ui_core::Rect;

pub type Pixel = [u8; 4];

// Saai-displayd currently blits client pixels into the panel's native BGRX
// scanout buffer without conversion. The physically calibrated panel packing
// is [X, R, G, B] in little-endian memory (the same mapping as
// drm-splash.c::panel_color). Keep that quirk behind one logical RGB helper.
pub const fn rgb(red: u8, green: u8, blue: u8) -> Pixel {
    [0, red, green, blue]
}

pub const BACKGROUND: Pixel = rgb(14, 20, 24);
pub const SURFACE: Pixel = rgb(25, 33, 38);
pub const SURFACE_SELECTED: Pixel = rgb(38, 51, 57);
pub const MUTED: Pixel = rgb(76, 91, 98);
pub const ACCENT: Pixel = rgb(116, 211, 190);

pub struct Canvas<'a> {
    pixels: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u8], width: u32, height: u32) -> Self {
        assert_eq!(pixels.len(), width as usize * height as usize * 4);
        Self {
            pixels,
            width,
            height,
        }
    }

    pub fn fill(&mut self, color: Pixel) {
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Pixel) {
        let left = rect.x.min(self.width);
        let top = rect.y.min(self.height);
        let right = rect.x.saturating_add(rect.width).min(self.width);
        let bottom = rect.y.saturating_add(rect.height).min(self.height);
        for y in top..bottom {
            let start = (y as usize * self.width as usize + left as usize) * 4;
            let end = (y as usize * self.width as usize + right as usize) * 4;
            for pixel in self.pixels[start..end].chunks_exact_mut(4) {
                pixel.copy_from_slice(&color);
            }
        }
    }

    #[cfg(test)]
    fn pixel(&self, x: u32, y: u32) -> Pixel {
        let start = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels[start..start + 4].try_into().unwrap()
    }
}

pub fn draw_root(canvas: &mut Canvas<'_>, content: Rect, tabs: &[Rect], selected: usize) {
    canvas.fill(BACKGROUND);

    // A stable phone-like content surface. The number of rows changes per
    // root page so page transitions remain visible even if a display pipeline
    // maps two colors too similarly.
    let margin = content.width / 22;
    let card_width = content.width.saturating_sub(margin * 2);
    canvas.fill_rect(Rect::new(margin, 150, card_width, 190), SURFACE);
    canvas.fill_rect(Rect::new(margin, 150, 14, 190), ACCENT);

    let row_count = selected.saturating_add(2).min(5);
    for row in 0..row_count {
        let y = 430 + row as u32 * 230;
        if y >= content.height {
            break;
        }
        canvas.fill_rect(Rect::new(margin, y, card_width, 170), SURFACE);
        canvas.fill_rect(Rect::new(margin + 34, y + 42, 86, 86), MUTED);
        canvas.fill_rect(
            Rect::new(margin + 154, y + 52, card_width.saturating_sub(210), 24),
            MUTED,
        );
        canvas.fill_rect(
            Rect::new(margin + 154, y + 96, card_width.saturating_sub(290), 18),
            MUTED,
        );
    }

    if let Some(tab_bar) = tabs.first().and_then(|first| {
        tabs.last().map(|last| {
            Rect::new(
                first.x,
                first.y,
                last.x.saturating_add(last.width).saturating_sub(first.x),
                first.height.max(last.height),
            )
        })
    }) {
        canvas.fill_rect(tab_bar, SURFACE);
    }

    for (index, rect) in tabs.iter().copied().enumerate() {
        let is_selected = index == selected;
        if is_selected {
            canvas.fill_rect(
                Rect::new(
                    rect.x.saturating_add(12),
                    rect.y.saturating_add(12),
                    rect.width.saturating_sub(24),
                    rect.height.saturating_sub(24),
                ),
                SURFACE_SELECTED,
            );
            canvas.fill_rect(
                Rect::new(
                    rect.x.saturating_add(rect.width.saturating_sub(112) / 2),
                    rect.y.saturating_add(18),
                    112.min(rect.width),
                    14,
                ),
                ACCENT,
            );
        }

        let icon_size = if is_selected { 76 } else { 54 };
        canvas.fill_rect(
            Rect::new(
                rect.x
                    .saturating_add(rect.width.saturating_sub(icon_size) / 2),
                rect.y.saturating_add(70),
                icon_size,
                icon_size,
            ),
            if is_selected { ACCENT } else { MUTED },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{draw_root, Canvas, ACCENT, SURFACE};
    use saai_ui_core::Rect;

    #[test]
    fn selected_indicator_moves_between_edge_tabs() {
        let mut pixels = vec![0; 1080 * 2400 * 4];
        let tabs = [
            Rect::new(0, 2100, 270, 300),
            Rect::new(270, 2100, 270, 300),
            Rect::new(540, 2100, 270, 300),
            Rect::new(810, 2100, 270, 300),
        ];
        let mut canvas = Canvas::new(&mut pixels, 1080, 2400);
        draw_root(&mut canvas, Rect::new(0, 0, 1080, 2100), &tabs, 0);
        assert_eq!(canvas.pixel(135, 2125), ACCENT);
        assert_eq!(canvas.pixel(945, 2125), SURFACE);

        draw_root(&mut canvas, Rect::new(0, 0, 1080, 2100), &tabs, 3);
        assert_eq!(canvas.pixel(135, 2125), SURFACE);
        assert_eq!(canvas.pixel(945, 2125), ACCENT);
    }
}
