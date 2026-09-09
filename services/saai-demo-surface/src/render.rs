use std::fs;

use fontdue::{Font, FontSettings};

type Pixel = [u8; 4];

const fn rgb(red: u8, green: u8, blue: u8) -> Pixel {
    [0, red, green, blue]
}

const BACKGROUND: Pixel = rgb(14, 20, 24);
const SURFACE: Pixel = rgb(25, 33, 38);
const SURFACE_ALT: Pixel = rgb(38, 51, 57);
const ACCENT: Pixel = rgb(116, 211, 190);
const TEXT: Pixel = rgb(232, 241, 239);
const MUTED: Pixel = rgb(141, 158, 164);

pub fn draw(pixels: &mut [u8], width: u32, height: u32) {
    let mut canvas = Canvas::new(pixels, width, height);
    canvas.fill(BACKGROUND);
    let margin = width / 18;
    let usable = width.saturating_sub(margin * 2);

    canvas.fill_rect(margin, 150, usable, 210, SURFACE_ALT);
    canvas.fill_rect(margin, 150, 16, 210, ACCENT);
    for row in 0..3 {
        let y = 470 + row * 260;
        canvas.fill_rect(margin, y, usable, 200, SURFACE);
        canvas.fill_rect(margin + 36, y + 50, 100, 100, ACCENT);
    }

    let button_y = height.saturating_sub(300);
    canvas.fill_rect(margin, button_y, usable, 150, ACCENT);

    if let Ok(fonts) = Fonts::load() {
        canvas.text(&fonts.semibold, "Saai Demo", 58.0, margin + 54, 205, TEXT);
        canvas.text(
            &fonts.regular,
            "Отдельное приложение SaaiOS",
            29.0,
            margin + 54,
            285,
            MUTED,
        );
        for (row, (title, detail)) in [
            ("Собственный процесс", "Запущено через saai-appd"),
            ("Настоящее окно", "Обычная Wayland-поверхность"),
            ("Сенсорный ввод", "Касания получает только это окно"),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 470 + row as u32 * 260;
            canvas.text(&fonts.semibold, title, 35.0, margin + 178, y + 55, TEXT);
            canvas.text(&fonts.regular, detail, 25.0, margin + 178, y + 112, MUTED);
        }
        canvas.text_centered(
            &fonts.semibold,
            "Закрыть приложение",
            34.0,
            width / 2,
            button_y + 48,
            BACKGROUND,
        );
    }
}

struct Fonts {
    regular: Font,
    semibold: Font,
}

impl Fonts {
    fn load() -> Result<Self, String> {
        let regular = fs::read("/saaios/fonts/Inter-Regular.ttf").map_err(|e| e.to_string())?;
        let semibold = fs::read("/saaios/fonts/Inter-SemiBold.ttf").map_err(|e| e.to_string())?;
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
