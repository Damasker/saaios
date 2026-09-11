use fontdue::{Font, FontSettings};
use saai_ui_core::Rect;
use std::fs;

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
pub const TEXT: Pixel = rgb(232, 241, 239);
pub const TEXT_MUTED: Pixel = rgb(141, 158, 164);

pub struct Fonts {
    regular: Font,
    semibold: Font,
}

#[derive(Clone)]
pub struct ActionCardView {
    pub label: String,
    pub status: String,
    pub action: String,
    pub selected: bool,
}

impl ActionCardView {
    pub fn new(
        label: impl Into<String>,
        status: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            status: status.into(),
            action: action.into(),
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Fonts {
    pub fn load_system() -> Result<Self, String> {
        Self::load(
            "/saaios/fonts/Inter-Regular.ttf",
            "/saaios/fonts/Inter-SemiBold.ttf",
        )
    }

    fn load(regular_path: &str, semibold_path: &str) -> Result<Self, String> {
        let regular =
            fs::read(regular_path).map_err(|error| format!("read {regular_path}: {error}"))?;
        let semibold =
            fs::read(semibold_path).map_err(|error| format!("read {semibold_path}: {error}"))?;
        Ok(Self {
            regular: Font::from_bytes(regular, FontSettings::default())
                .map_err(|error| format!("parse {regular_path}: {error}"))?,
            semibold: Font::from_bytes(semibold, FontSettings::default())
                .map_err(|error| format!("parse {semibold_path}: {error}"))?,
        })
    }
}

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

    fn blend(&mut self, x: i32, y: i32, color: Pixel, alpha: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 || alpha == 0 {
            return;
        }
        let start = (y as usize * self.width as usize + x as usize) * 4;
        let inverse = 255 - alpha as u16;
        self.pixels[start] = 0;
        for (channel, &src) in color.iter().enumerate().skip(1) {
            self.pixels[start + channel] =
                ((src as u16 * alpha as u16 + self.pixels[start + channel] as u16 * inverse + 127)
                    / 255) as u8;
        }
    }

    #[cfg(test)]
    fn pixel(&self, x: u32, y: u32) -> Pixel {
        let start = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels[start..start + 4].try_into().unwrap()
    }
}

/// The ADR-020 consent screen (S07 Change 4): full-screen, replaces
/// `draw_root` entirely while a launch is blocked on consent -- same
/// `Canvas`/`Fonts` primitives, a second top-level entry point rather than
/// a mode bolted onto `draw_root`, since the two share no layout beyond
/// both being full-screen.
#[allow(clippy::too_many_arguments)]
pub fn draw_consent(
    canvas: &mut Canvas<'_>,
    app_name: &str,
    capabilities: &[String],
    header: Rect,
    accept_button: Rect,
    decline_button: Rect,
    fonts: Option<&Fonts>,
) {
    canvas.fill(BACKGROUND);

    let Some(fonts) = fonts else {
        // No font asset available (missing/unreadable on this build) --
        // still show the two buttons in distinct colors so the screen is
        // at least operable without text, matching this client's existing
        // "color as the physically-verifiable signal" fallback used
        // elsewhere before Inter rendering landed.
        canvas.fill_rect(accept_button, ACCENT);
        canvas.fill_rect(decline_button, SURFACE);
        return;
    };

    let margin = header.width / 22;
    draw_text(
        canvas,
        &fonts.semibold,
        &format!("{app_name} запрашивает доступ"),
        46.0,
        header.x + margin,
        header.y + 220,
        TEXT,
    );

    let mut row_top = header.y + 340;
    if capabilities.is_empty() {
        draw_text(
            canvas,
            &fonts.regular,
            "Без дополнительных разрешений",
            32.0,
            header.x + margin,
            row_top,
            TEXT_MUTED,
        );
    }
    for capability in capabilities {
        canvas.fill_rect(Rect::new(header.x + margin, row_top + 10, 16, 16), ACCENT);
        draw_text(
            canvas,
            &fonts.regular,
            capability,
            32.0,
            header.x + margin + 44,
            row_top,
            TEXT_MUTED,
        );
        row_top += 70;
    }

    canvas.fill_rect(accept_button, ACCENT);
    canvas.fill_rect(decline_button, SURFACE);
    draw_text_centered(
        canvas,
        &fonts.semibold,
        "Разрешить",
        40.0,
        accept_button.x + accept_button.width / 2,
        accept_button.y + accept_button.height / 2 - 20,
        BACKGROUND,
    );
    draw_text_centered(
        canvas,
        &fonts.semibold,
        "Отклонить",
        40.0,
        decline_button.x + decline_button.width / 2,
        decline_button.y + decline_button.height / 2 - 20,
        TEXT,
    );
}

/// S09 Change 2 / ADR-030: the bespoke touch-hit-test on-screen keyboard
/// ADR-029 proved on a throwaway spike, now the real, committed way to
/// type a `saaios.intent`'s text. `keys` is already laid out and
/// hit-tested by `main.rs`'s `intent_view()` -- this only draws the
/// rectangles it's handed, the same "no second set of rectangles" rule
/// `draw_root()` follows for the tab bar and content cards.
pub fn draw_intent_input(
    canvas: &mut Canvas<'_>,
    buffer: &str,
    header: Rect,
    keys: &[(Rect, String)],
    fonts: Option<&Fonts>,
) {
    canvas.fill(BACKGROUND);
    canvas.fill_rect(header, SURFACE);

    let Some(fonts) = fonts else {
        // Same no-font fallback draw_consent() uses: every key still gets
        // a distinct, tappable rectangle even with no label rendered.
        for (rect, _) in keys {
            canvas.fill_rect(*rect, SURFACE_SELECTED);
        }
        return;
    };

    draw_text(
        canvas,
        &fonts.semibold,
        "Новое намерение",
        42.0,
        header.x + 30,
        header.y + 40,
        TEXT,
    );
    let (preview, preview_color) = if buffer.is_empty() {
        ("Наберите текст…", TEXT_MUTED)
    } else {
        (buffer, TEXT)
    };
    draw_text(
        canvas,
        &fonts.regular,
        preview,
        34.0,
        header.x + 30,
        header.y + 130,
        preview_color,
    );

    for (rect, label) in keys {
        let key = Rect::new(
            rect.x.saturating_add(4),
            rect.y.saturating_add(4),
            rect.width.saturating_sub(8),
            rect.height.saturating_sub(8),
        );
        canvas.fill_rect(key, SURFACE);
        draw_text_centered(
            canvas,
            &fonts.semibold,
            label,
            32.0,
            key.x + key.width / 2,
            key.y + key.height / 2 - 18,
            TEXT,
        );
    }
}

pub fn draw_root(
    canvas: &mut Canvas<'_>,
    content: Rect,
    tabs: &[(Rect, &str)],
    selected: usize,
    context_label: &str,
    fonts: Option<&Fonts>,
    content_actions: &[(Rect, ActionCardView)],
) {
    canvas.fill(BACKGROUND);

    // A stable phone-like content surface. The number of rows changes per
    // root page so page transitions remain visible even if a display pipeline
    // maps two colors too similarly.
    let margin = content.width / 22;
    let card_width = content.width.saturating_sub(margin * 2);
    canvas.fill_rect(Rect::new(margin, 150, card_width, 190), SURFACE);
    canvas.fill_rect(Rect::new(margin, 150, 14, 190), ACCENT);
    if let (Some(fonts), Some((_, title))) = (fonts, tabs.get(selected)) {
        let header = format!("{context_label} · {title}");
        draw_text_centered(
            canvas,
            &fonts.semibold,
            &header,
            54.0,
            content.x + content.width / 2,
            210,
            TEXT,
        );
    }

    let row_count = selected.saturating_add(2).min(5);
    let first_placeholder = if content_actions.is_empty() {
        0
    } else {
        row_count
    };
    for row in first_placeholder..row_count {
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

    for (rect, card) in content_actions {
        canvas.fill_rect(
            *rect,
            if card.selected {
                SURFACE_SELECTED
            } else {
                SURFACE
            },
        );
        canvas.fill_rect(Rect::new(rect.x + 34, rect.y + 52, 104, 104), ACCENT);
        let button_width = 250.min(rect.width / 3);
        let button = Rect::new(
            rect.x + rect.width.saturating_sub(button_width + 34),
            rect.y + 58,
            button_width,
            88,
        );
        canvas.fill_rect(button, ACCENT);
        if let Some(fonts) = fonts {
            draw_text(
                canvas,
                &fonts.semibold,
                &card.label,
                38.0,
                rect.x + 174,
                rect.y + 48,
                TEXT,
            );
            draw_text(
                canvas,
                &fonts.regular,
                &card.status,
                27.0,
                rect.x + 174,
                rect.y + 108,
                TEXT_MUTED,
            );
            draw_text_centered(
                canvas,
                &fonts.semibold,
                &card.action,
                25.0,
                button.x + button.width / 2,
                button.y + 24,
                BACKGROUND,
            );
        }
    }

    if let Some(tab_bar) = tabs.first().and_then(|(first, _)| {
        tabs.last().map(|last| {
            Rect::new(
                first.x,
                first.y,
                last.0
                    .x
                    .saturating_add(last.0.width)
                    .saturating_sub(first.x),
                first.height.max(last.0.height),
            )
        })
    }) {
        canvas.fill_rect(tab_bar, SURFACE);
    }

    for (index, (rect, label)) in tabs.iter().copied().enumerate() {
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

        if let Some(fonts) = fonts {
            draw_text_centered(
                canvas,
                if is_selected {
                    &fonts.semibold
                } else {
                    &fonts.regular
                },
                label,
                if is_selected { 31.0 } else { 27.0 },
                rect.x + rect.width / 2,
                rect.y + 172,
                if is_selected { TEXT } else { TEXT_MUTED },
            );
        }
    }
}

fn draw_text_centered(
    canvas: &mut Canvas<'_>,
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
    let mut cursor = center_x as f32 - width / 2.0;
    for character in text.chars() {
        let (metrics, bitmap) = font.rasterize(character, size);
        let glyph_x = cursor.round() as i32 + metrics.xmin;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                canvas.blend(
                    glyph_x + column as i32,
                    top as i32 + row as i32,
                    color,
                    bitmap[row * metrics.width + column],
                );
            }
        }
        cursor += metrics.advance_width;
    }
}

fn draw_text(
    canvas: &mut Canvas<'_>,
    font: &Font,
    text: &str,
    size: f32,
    left: u32,
    top: u32,
    color: Pixel,
) {
    let mut cursor = left as f32;
    for character in text.chars() {
        let (metrics, bitmap) = font.rasterize(character, size);
        let glyph_x = cursor.round() as i32 + metrics.xmin;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                canvas.blend(
                    glyph_x + column as i32,
                    top as i32 + row as i32,
                    color,
                    bitmap[row * metrics.width + column],
                );
            }
        }
        cursor += metrics.advance_width;
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
            (Rect::new(0, 2100, 270, 300), "Сейчас"),
            (Rect::new(270, 2100, 270, 300), "Входящие"),
            (Rect::new(540, 2100, 270, 300), "Пространства"),
            (Rect::new(810, 2100, 270, 300), "Я"),
        ];
        let mut canvas = Canvas::new(&mut pixels, 1080, 2400);
        draw_root(
            &mut canvas,
            Rect::new(0, 0, 1080, 2100),
            &tabs,
            0,
            "Дом",
            None,
            &[],
        );
        assert_eq!(canvas.pixel(135, 2125), ACCENT);
        assert_eq!(canvas.pixel(945, 2125), SURFACE);

        draw_root(
            &mut canvas,
            Rect::new(0, 0, 1080, 2100),
            &tabs,
            3,
            "Дом",
            None,
            &[],
        );
        assert_eq!(canvas.pixel(135, 2125), SURFACE);
        assert_eq!(canvas.pixel(945, 2125), ACCENT);
    }
}
