use fontdue::{Font, FontSettings};
use saai_ui_core::Rect;
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

/// S25: accessibility text scale, read inside `draw_text`/`draw_
/// text_centered` rather than threaded through this file's ~40
/// existing call sites as one more parameter each. This process only
/// ever renders one shell UI on one thread, so there's no
/// concurrency hazard from process-global state here -- the tradeoff
/// is one small piece of shared mutable state in exchange for not
/// touching every already-working draw call's signature. `AtomicU32`
/// holding the `f32`'s bits because `std` has no `AtomicF32`.
static TEXT_SCALE_BITS: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0f32.to_bits()

/// `scale_pct` is a percent of every literal `size` argument already
/// in this file (`100` = unchanged). Clamped to a sane range so a
/// corrupt settings file can't blow up glyph rasterization with a
/// zero or enormous size.
pub fn set_text_scale(scale_pct: u8) {
    let scale = (f32::from(scale_pct) / 100.0).clamp(0.5, 2.0);
    TEXT_SCALE_BITS.store(scale.to_bits(), Ordering::Relaxed);
}

fn text_scale() -> f32 {
    f32::from_bits(TEXT_SCALE_BITS.load(Ordering::Relaxed))
}

/// S25: a post-process contrast stretch applied once, from `main.rs`,
/// over an entire already-drawn frame's raw pixel bytes -- not a
/// second color palette threaded through this file's ~90 individual
/// `fill_rect`/`draw_text` call sites. `boost_pct` `0` is a byte-for-
/// byte no-op; higher values push every channel further from a
/// mid-grey pivot toward black or white. Operates on all 4 bytes per
/// pixel including the always-zero first byte (see `rgb`'s doc
/// comment) -- harmless, since stretching a value already at 0 away
/// from the 128 pivot only ever clamps back down to 0.
pub fn apply_contrast_boost(pixels: &mut [u8], boost_pct: u8) {
    if boost_pct == 0 {
        return;
    }
    let factor = 1.0 + (f32::from(boost_pct.min(100)) / 100.0) * 3.0;
    for channel in pixels.iter_mut() {
        let value = f32::from(*channel);
        let stretched = (value - 128.0) * factor + 128.0;
        *channel = stretched.clamp(0.0, 255.0) as u8;
    }
}

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
        // S13 Change 5: swapped from Inter, by explicit user request
        // after seeing Inter on-device. First tried Manrope, which
        // physically turned out to read as visually indistinguishable
        // from Inter at these render sizes -- Montserrat is the
        // second, confirmed-distinctive choice. Both Space Grotesk and
        // Sora were considered and rejected before that: neither ships
        // any Cyrillic glyphs at all (checked via `fontTools`'s cmap),
        // which would have broken every Russian label in this UI.
        // Montserrat ships as a variable font on Google Fonts now (no
        // separate static weight files) -- `fontdue` has no
        // variable-font support, so the two weights here are static
        // instances pre-generated with `fonttools varLib.instancer`
        // (wght=400, wght=600) rather than loaded from the variable
        // file directly.
        Self::load(
            "/saaios/fonts/Montserrat-Regular.ttf",
            "/saaios/fonts/Montserrat-SemiBold.ttf",
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
/// `title` is a parameter (S19) rather than a hardcoded "Новое
/// намерение" -- `saai-shell` reuses this same keyboard tree verbatim
/// for the Wi-Fi password screen (see `WifiPasswordState`'s doc
/// comment), which needs its own header text.
pub fn draw_intent_input(
    canvas: &mut Canvas<'_>,
    title: &str,
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
        title,
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

/// S09 Change 3: the confirmation screen for a dangerous `saaios.task`
/// (ADR-031's follow-up). `title` is already the task's own
/// human-readable title (e.g. "Подтвердите: удалить объект a1b2c3d4")
/// -- `saai-shell` shows it verbatim rather than interpreting the
/// Action's `kind`/`input`, so it never needs to know `saai-taskd`'s
/// vocabulary (ADR-030's no-cross-runtime-dependency principle applies
/// here too, not just to the daemon split itself).
pub fn draw_task_confirm(
    canvas: &mut Canvas<'_>,
    title: &str,
    header: Rect,
    accept_button: Rect,
    decline_button: Rect,
    fonts: Option<&Fonts>,
) {
    canvas.fill(BACKGROUND);

    let Some(fonts) = fonts else {
        canvas.fill_rect(accept_button, ACCENT);
        canvas.fill_rect(decline_button, SURFACE);
        return;
    };

    let margin = header.width / 22;
    draw_text(
        canvas,
        &fonts.semibold,
        "Требуется подтверждение",
        46.0,
        header.x + margin,
        header.y + 220,
        TEXT,
    );
    draw_text(
        canvas,
        &fonts.regular,
        title,
        32.0,
        header.x + margin,
        header.y + 340,
        TEXT_MUTED,
    );

    canvas.fill_rect(accept_button, ACCENT);
    canvas.fill_rect(decline_button, SURFACE);
    draw_text_centered(
        canvas,
        &fonts.semibold,
        "Подтвердить",
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

/// S19: "Wi-Fi сети" -- one row per `wifi_scan_results()` entry plus
/// the fixed trailing control rows already baked into `rows` by the
/// caller (see `wifi_list_action_at`'s doc comment for why the row
/// count is runtime-sized rather than a `root.sui` screen). Same
/// simple header-plus-list shape as `draw_task_confirm`, just with N
/// rows instead of two buttons. S20 generalized this from a
/// Wi-Fi-only `draw_wifi_list` to also draw "Bluetooth устройства" --
/// same shape both times, only the title and row contents differ.
pub fn draw_row_list(
    canvas: &mut Canvas<'_>,
    title: &str,
    status_line: &str,
    header: Rect,
    rows: &[(Rect, String)],
    fonts: Option<&Fonts>,
) {
    canvas.fill(BACKGROUND);
    canvas.fill_rect(header, SURFACE);

    let Some(fonts) = fonts else {
        for (rect, _) in rows {
            canvas.fill_rect(*rect, SURFACE_SELECTED);
        }
        return;
    };

    draw_text(
        canvas,
        &fonts.semibold,
        title,
        42.0,
        header.x + 30,
        header.y + 40,
        TEXT,
    );
    draw_text(
        canvas,
        &fonts.regular,
        status_line,
        30.0,
        header.x + 30,
        header.y + 130,
        TEXT_MUTED,
    );

    for (rect, label) in rows {
        canvas.fill_rect(*rect, SURFACE);
        draw_text(
            canvas,
            &fonts.regular,
            label,
            32.0,
            rect.x + 30,
            rect.y + rect.height / 2 - 18,
            TEXT,
        );
    }
}

/// S24: "Изменить PIN" on "Я" -- same header-plus-keys shape as
/// `draw_intent_input`, but the preview is masked (a PIN is a secret,
/// same reasoning as `WifiPasswordInput`'s masked preview) and the
/// keys come from `pin_keypad_rect`'s numeric layout instead of
/// ADR-029's letters.
pub fn draw_pin_setup(
    canvas: &mut Canvas<'_>,
    buffer: &str,
    header: Rect,
    keys: &[(Rect, &str)],
    fonts: Option<&Fonts>,
) {
    canvas.fill(BACKGROUND);
    canvas.fill_rect(header, SURFACE);

    let Some(fonts) = fonts else {
        for (rect, _) in keys {
            canvas.fill_rect(*rect, SURFACE_SELECTED);
        }
        return;
    };

    draw_text(
        canvas,
        &fonts.semibold,
        "Новый PIN-код",
        42.0,
        header.x + 30,
        header.y + 40,
        TEXT,
    );
    let masked: String = buffer.chars().map(|_| '•').collect();
    let (preview, preview_color) = if masked.is_empty() {
        ("Введите новый PIN (минимум 4 цифры)".to_string(), TEXT_MUTED)
    } else {
        (masked, TEXT)
    };
    draw_text(
        canvas,
        &fonts.regular,
        &preview,
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

/// S24: the lock surface's own keypad, shown instead of a flat
/// `LOCK_SCREEN_COLOR` fill whenever `ShellSettings.pin_code` is set
/// (`present_lock_pin_entry` in `main.rs`). `entered_len` dots are
/// filled (`ACCENT`), the rest of `pin_len` stay `MUTED` outlines --
/// no digits are ever drawn, only progress, since this is what
/// protects the lock in the first place.
pub fn draw_lock_pin_entry(
    canvas: &mut Canvas<'_>,
    width: u32,
    entered_len: usize,
    pin_len: usize,
    keys: &[(Rect, &str)],
    fonts: Option<&Fonts>,
) {
    canvas.fill(BACKGROUND);

    let dot_size = 36u32;
    let gap = 30u32;
    let count = pin_len.max(1) as u32;
    let total_width = count * dot_size + count.saturating_sub(1) * gap;
    let start_x = width.saturating_sub(total_width) / 2;
    let dot_y = 420u32;
    for index in 0..pin_len {
        let x = start_x + index as u32 * (dot_size + gap);
        let color = if index < entered_len { ACCENT } else { MUTED };
        canvas.fill_rect(Rect::new(x, dot_y, dot_size, dot_size), color);
    }

    let Some(fonts) = fonts else {
        for (rect, _) in keys {
            canvas.fill_rect(*rect, SURFACE_SELECTED);
        }
        return;
    };

    draw_text_centered(
        canvas,
        &fonts.regular,
        "Введите PIN",
        32.0,
        width / 2,
        330,
        TEXT_MUTED,
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
            36.0,
            key.x + key.width / 2,
            key.y + key.height / 2 - 20,
            TEXT,
        );
    }
}

// S23 added `is_grid` as the 8th plain draw-time knob on an already
// data-only function (no behavior to extract into a struct without
// inventing one purely to appease this lint) -- same call shape as
// `draw_consent`/`draw_task_confirm`, just with one more page-shaped
// screen to describe.
#[allow(clippy::too_many_arguments)]
pub fn draw_root(
    canvas: &mut Canvas<'_>,
    content: Rect,
    tabs: &[(Rect, &str)],
    selected: usize,
    context_label: &str,
    fonts: Option<&Fonts>,
    content_actions: &[(Rect, ActionCardView)],
    is_grid: bool,
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
    let first_placeholder = if content_actions.is_empty() { 0 } else { row_count };
    if !is_grid {
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
    }

    // S23: "Сейчас"'s icon grid (phone-style: square icon, label
    // below, no description/button) -- every other page keeps the
    // original single-column card list below. No real per-app icon
    // asset exists anywhere in the project (no icon pipeline was ever
    // built), so the "icon" is a colored square with the app's own
    // first letter, same honest placeholder spirit as `MUTED`'s
    // loading skeleton above.
    if is_grid {
        for (rect, card) in content_actions {
            let icon_size = rect.width.min(rect.height.saturating_sub(70)).min(180);
            let icon_x = rect.x + rect.width.saturating_sub(icon_size) / 2;
            canvas.fill_rect(
                Rect::new(icon_x, rect.y, icon_size, icon_size),
                if card.selected { SURFACE_SELECTED } else { ACCENT },
            );
            if let Some(fonts) = fonts {
                let initial = card
                    .label
                    .chars()
                    .next()
                    .map(|ch| ch.to_uppercase().to_string())
                    .unwrap_or_default();
                draw_text_centered(
                    canvas,
                    &fonts.semibold,
                    &initial,
                    54.0,
                    icon_x + icon_size / 2,
                    rect.y + icon_size / 2 - 27,
                    BACKGROUND,
                );
                draw_text_centered(
                    canvas,
                    &fonts.regular,
                    &card.label,
                    26.0,
                    rect.x + rect.width / 2,
                    rect.y + icon_size + 16,
                    TEXT,
                );
            }
        }
    } else {
        for (rect, card) in content_actions {
            canvas.fill_rect(
                *rect,
                if card.selected {
                    SURFACE_SELECTED
                } else {
                    SURFACE
                },
            );
            // S13 Change 5: an empty `action` means this card has nothing to
            // tap (an info summary -- "Это устройство", "Я"'s app grants,
            // "Входящие"'s empty state) -- the icon block and the
            // accent-colored button were drawn unconditionally before, which
            // made every such card look clickable even though nothing
            // happened when tapped. Text starts at the icon's own left edge
            // instead of after it when there's no icon to make room for.
            let has_action = !card.action.is_empty();
            let text_left = if has_action {
                canvas.fill_rect(Rect::new(rect.x + 34, rect.y + 52, 104, 104), ACCENT);
                rect.x + 174
            } else {
                rect.x + 34
            };
            let button = has_action.then(|| {
                let button_width = 250.min(rect.width / 3);
                let button = Rect::new(
                    rect.x + rect.width.saturating_sub(button_width + 34),
                    rect.y + 58,
                    button_width,
                    88,
                );
                canvas.fill_rect(button, ACCENT);
                button
            });
            if let Some(fonts) = fonts {
                draw_text(
                    canvas,
                    &fonts.semibold,
                    &card.label,
                    38.0,
                    text_left,
                    rect.y + 48,
                    TEXT,
                );
                draw_text(
                    canvas,
                    &fonts.regular,
                    &card.status,
                    27.0,
                    text_left,
                    rect.y + 108,
                    TEXT_MUTED,
                );
                if let Some(button) = button {
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

/// The permanent system layer's real content (S13 Change 1) -- time on
/// the left, network and battery state on the right. Replaces the
/// solid-color placeholder that namespace's own `"...-test"` suffix
/// (`main.rs`) had been honestly admitting to since ADR-015.
pub fn draw_status_bar(
    canvas: &mut Canvas<'_>,
    width: u32,
    height: u32,
    time_text: &str,
    wifi_up: bool,
    battery: Option<(u8, bool)>,
    fonts: Option<&Fonts>,
) {
    canvas.fill(BACKGROUND);
    let Some(fonts) = fonts else {
        return;
    };
    let baseline = height / 2 - 22;
    let margin = width / 30;
    draw_text(canvas, &fonts.semibold, time_text, 44.0, margin, baseline, TEXT);

    let wifi_label = if wifi_up { "Wi-Fi" } else { "Нет сети" };
    let wifi_color = if wifi_up { ACCENT } else { TEXT_MUTED };
    let battery_label = battery
        .map(|(percent, charging)| {
            if charging {
                format!("{percent}% +")
            } else {
                format!("{percent}%")
            }
        })
        .unwrap_or_default();

    // Right-aligned: battery flush with the margin, Wi-Fi immediately to
    // its left with a fixed gap -- same "measure, then place" approach
    // `draw_text_centered` already uses, just anchored from the right
    // edge instead of a center point.
    let text_width = |font: &Font, text: &str, size: f32| -> f32 {
        text.chars()
            .map(|character| font.metrics(character, size).advance_width)
            .sum()
    };
    let gap = 40.0;
    let battery_width = text_width(&fonts.semibold, &battery_label, 40.0);
    let battery_left = width as f32 - margin as f32 - battery_width;
    if !battery_label.is_empty() {
        draw_text(
            canvas,
            &fonts.semibold,
            &battery_label,
            40.0,
            battery_left.round() as u32,
            baseline,
            TEXT,
        );
    }
    let wifi_width = text_width(&fonts.regular, wifi_label, 36.0);
    let wifi_left = battery_left - gap - wifi_width;
    draw_text(
        canvas,
        &fonts.regular,
        wifi_label,
        36.0,
        wifi_left.round() as u32,
        baseline + 4,
        wifi_color,
    );
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
    let size = size * text_scale();
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
    let size = size * text_scale();
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
    use super::{apply_contrast_boost, draw_root, Canvas, ACCENT, SURFACE};
    use saai_ui_core::Rect;

    #[test]
    fn contrast_boost_zero_is_a_byte_for_byte_no_op() {
        let original = vec![10u8, 90, 128, 200, 250, 0];
        let mut pixels = original.clone();
        apply_contrast_boost(&mut pixels, 0);
        assert_eq!(pixels, original);
    }

    #[test]
    fn contrast_boost_pushes_values_away_from_the_midpoint() {
        let mut pixels = vec![100u8, 128, 200];
        apply_contrast_boost(&mut pixels, 100);
        // Below the 128 pivot moves further down, above it moves
        // further up, exactly at the pivot stays put.
        assert!(pixels[0] < 100);
        assert_eq!(pixels[1], 128);
        assert!(pixels[2] > 200);
    }

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
            true,
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
            false,
        );
        assert_eq!(canvas.pixel(135, 2125), SURFACE);
        assert_eq!(canvas.pixel(945, 2125), ACCENT);
    }
}
