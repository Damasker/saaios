use std::fs;

use fontdue::{Font, FontSettings};

type Pixel = [u8; 4];

/// Same [0, R, G, B] byte order `saai-shell`'s `render::rgb` already
/// documents -- this panel's own pipeline reads R/G/B from bytes
/// 1/2/3, byte 0 unused.
const fn rgb(red: u8, green: u8, blue: u8) -> Pixel {
    [0, red, green, blue]
}

const BACKGROUND: Pixel = rgb(14, 20, 24);
const TILE: Pixel = rgb(38, 51, 57);
const TILE_SELECTED: Pixel = rgb(116, 211, 190);
const TILE_REMOVED: Pixel = rgb(14, 20, 24);
const TEXT: Pixel = rgb(232, 241, 239);
const TEXT_MUTED: Pixel = rgb(141, 158, 164);
const ACCENT: Pixel = rgb(116, 211, 190);

/// One board cell. `symbol` is a letter 'A'..='L' (12 distinct pairs);
/// no real tile art exists anywhere in this project (same honest gap
/// as `saai-shell`'s S23 icon-grid placeholders), so a plain letter
/// is what actually distinguishes matching pairs.
pub struct Tile {
    pub symbol: u8,
    pub removed: bool,
}

pub struct BoardLayout {
    pub columns: u32,
    pub margin: u32,
    pub gap: u32,
    pub tile_size: u32,
    pub board_top: u32,
}

impl BoardLayout {
    pub fn compute(width: u32, height: u32, columns: u32, rows: u32) -> Self {
        let margin = width / 20;
        let gap = width / 100;
        let usable_width = width.saturating_sub(margin * 2);
        let tile_from_width = (usable_width.saturating_sub(gap * (columns - 1))) / columns;
        let board_top = height / 6;
        let usable_height = height.saturating_sub(board_top + height / 6);
        let tile_from_height = (usable_height.saturating_sub(gap * (rows - 1))) / rows;
        let tile_size = tile_from_width.min(tile_from_height);
        Self {
            columns,
            margin,
            gap,
            tile_size,
            board_top,
        }
    }

    pub fn tile_rect(&self, index: usize) -> (u32, u32, u32, u32) {
        let column = index as u32 % self.columns;
        let row = index as u32 / self.columns;
        let x = self.margin + column * (self.tile_size + self.gap);
        let y = self.board_top + row * (self.tile_size + self.gap);
        (x, y, self.tile_size, self.tile_size)
    }

    pub fn tile_at(&self, x: f64, y: f64, count: usize) -> Option<usize> {
        for index in 0..count {
            let (tx, ty, tw, th) = self.tile_rect(index);
            if x >= f64::from(tx)
                && x < f64::from(tx + tw)
                && y >= f64::from(ty)
                && y < f64::from(ty + th)
            {
                return Some(index);
            }
        }
        None
    }
}

/// Bottom "Новая игра" / "Закрыть" control row -- same fixed-region
/// convention `saai-demo-surface`'s own close button already uses
/// (no window chrome exists anywhere in this shell, so every
/// installed app owns drawing and hit-testing its own exit
/// affordance).
pub fn restart_button_rect(width: u32, height: u32) -> (u32, u32, u32, u32) {
    let margin = width / 20;
    let button_height = height / 10;
    let y = height.saturating_sub(button_height * 2 + margin);
    (margin, y, width.saturating_sub(margin * 2), button_height)
}

pub fn close_button_rect(width: u32, height: u32) -> (u32, u32, u32, u32) {
    let margin = width / 20;
    let button_height = height / 10;
    let y = height.saturating_sub(button_height + margin / 2);
    (margin, y, width.saturating_sub(margin * 2), button_height)
}

pub fn draw_board(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    tiles: &[Tile],
    selected: Option<usize>,
    layout: &BoardLayout,
    won: bool,
) {
    let mut canvas = Canvas::new(pixels, width, height);
    canvas.fill(BACKGROUND);

    let fonts = Fonts::load().ok();

    if let Some(fonts) = &fonts {
        let title = if won {
            "Победа!"
        } else {
            "Маджонг"
        };
        canvas.text(
            &fonts.semibold,
            title,
            48.0,
            layout.margin,
            layout.board_top.saturating_sub(80).max(60),
            TEXT,
        );
    }

    for (index, tile) in tiles.iter().enumerate() {
        let (x, y, w, h) = layout.tile_rect(index);
        if tile.removed {
            canvas.fill_rect(x, y, w, h, TILE_REMOVED);
            continue;
        }
        let color = if selected == Some(index) {
            TILE_SELECTED
        } else {
            TILE
        };
        canvas.fill_rect(x, y, w, h, color);
        if let Some(fonts) = &fonts {
            let label = (tile.symbol as char).to_string();
            let text_color = if selected == Some(index) {
                BACKGROUND
            } else {
                TEXT
            };
            canvas.text_centered(
                &fonts.semibold,
                &label,
                (layout.tile_size as f32 * 0.5).max(20.0),
                x + w / 2,
                y + h / 3,
                text_color,
            );
        }
    }

    let (rx, ry, rw, rh) = restart_button_rect(width, height);
    canvas.fill_rect(rx, ry, rw, rh, TILE);
    let (cx, cy, cw, ch) = close_button_rect(width, height);
    canvas.fill_rect(cx, cy, cw, ch, ACCENT);
    if let Some(fonts) = &fonts {
        canvas.text_centered(
            &fonts.semibold,
            "Новая игра",
            32.0,
            rx + rw / 2,
            ry + rh / 2 - 18,
            TEXT,
        );
        canvas.text_centered(
            &fonts.semibold,
            "Закрыть",
            32.0,
            cx + cw / 2,
            cy + ch / 2 - 18,
            BACKGROUND,
        );
        let hint = if won {
            "Все пары найдены"
        } else {
            "Найдите две одинаковые плитки"
        };
        canvas.text_centered(
            &fonts.regular,
            hint,
            26.0,
            width / 2,
            layout.board_top.saturating_sub(30).max(140),
            TEXT_MUTED,
        );
    }
}

struct Fonts {
    regular: Font,
    semibold: Font,
}

impl Fonts {
    fn load() -> Result<Self, String> {
        // Same paths ADR-070 fixed `saai-shell` to use -- the old
        // `/saaios/fonts/Inter-*.ttf` this app's own boilerplate was
        // copied from no longer exist in the image at all.
        let regular =
            fs::read("/saaios/fonts/Montserrat-Regular.ttf").map_err(|e| e.to_string())?;
        let semibold =
            fs::read("/saaios/fonts/Montserrat-SemiBold.ttf").map_err(|e| e.to_string())?;
        Ok(Self {
            regular: Font::from_bytes(regular, FontSettings::default())
                .map_err(|e| e.to_string())?,
            semibold: Font::from_bytes(semibold, FontSettings::default())
                .map_err(|e| e.to_string())?,
        })
    }
}

struct Canvas<'a> {
    pixels: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> Canvas<'a> {
    fn new(pixels: &'a mut [u8], width: u32, height: u32) -> Self {
        Self {
            pixels,
            width,
            height,
        }
    }

    fn fill(&mut self, color: Pixel) {
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
    }

    fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: Pixel) {
        let right = x.saturating_add(width).min(self.width);
        let bottom = y.saturating_add(height).min(self.height);
        for row in y.min(self.height)..bottom {
            let start = (row as usize * self.width as usize + x.min(self.width) as usize) * 4;
            let end = (row as usize * self.width as usize + right as usize) * 4;
            for pixel in self.pixels[start..end].chunks_exact_mut(4) {
                pixel.copy_from_slice(&color);
            }
        }
    }

    fn text(&mut self, font: &Font, text: &str, size: f32, left: u32, top: u32, color: Pixel) {
        let mut cursor = left as f32;
        for character in text.chars() {
            let (metrics, bitmap) = font.rasterize(character, size);
            let glyph_x = cursor.round() as i32 + metrics.xmin;
            self.glyph(
                glyph_x,
                top as i32,
                metrics.width,
                metrics.height,
                &bitmap,
                color,
            );
            cursor += metrics.advance_width;
        }
    }

    fn text_centered(
        &mut self,
        font: &Font,
        text: &str,
        size: f32,
        center_x: u32,
        top: u32,
        color: Pixel,
    ) {
        let width = text
            .chars()
            .map(|character| font.metrics(character, size).advance_width)
            .sum::<f32>();
        self.text(
            font,
            text,
            size,
            (center_x as f32 - width / 2.0).max(0.0) as u32,
            top,
            color,
        );
    }

    fn glyph(&mut self, x: i32, y: i32, width: usize, height: usize, bitmap: &[u8], color: Pixel) {
        for row in 0..height {
            for column in 0..width {
                let px = x + column as i32;
                let py = y + row as i32;
                if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 {
                    continue;
                }
                let alpha = bitmap[row * width + column] as u16;
                if alpha == 0 {
                    continue;
                }
                let start = (py as usize * self.width as usize + px as usize) * 4;
                let inverse = 255 - alpha;
                for (channel, source) in color.iter().enumerate().skip(1) {
                    self.pixels[start + channel] = ((*source as u16 * alpha
                        + self.pixels[start + channel] as u16 * inverse
                        + 127)
                        / 255) as u8;
                }
            }
        }
    }
}
