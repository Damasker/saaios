//! Privileged IME object bound to a `Field` (ADR-222).
//!
//! Android-style: the Field does not embed keys. `Keyboard` is a
//! separate entity with a swappable source. `OnScreen` is the proven
//! ADR-029 touch OSK. `Hardware` is a USB HID evdev device with
//! `KEY_A`, not gpio volume/power, not the touchscreen, not haptic.
//! No libxkbcommon and no new daemon. Keys from either source apply
//! to the bound Field's value.

use crate::Field;

/// Linux `KEY_A`. Used to tell a typing keyboard from volume/power.
pub const EVDEV_KEY_A: u16 = 30;
const EVDEV_KEY_ESC: u16 = 1;
const EVDEV_KEY_1: u16 = 2;
const EVDEV_KEY_0: u16 = 11;
const EVDEV_KEY_MINUS: u16 = 12;
const EVDEV_KEY_EQUAL: u16 = 13;
const EVDEV_KEY_BACKSPACE: u16 = 14;
const EVDEV_KEY_Q: u16 = 16;
const EVDEV_KEY_P: u16 = 25;
const EVDEV_KEY_ENTER: u16 = 28;
const EVDEV_KEY_L: u16 = 38;
const EVDEV_KEY_SEMICOLON: u16 = 39;
const EVDEV_KEY_APOSTROPHE: u16 = 40;
const EVDEV_KEY_Z: u16 = 44;
const EVDEV_KEY_M: u16 = 50;
const EVDEV_KEY_COMMA: u16 = 51;
const EVDEV_KEY_DOT: u16 = 52;
const EVDEV_KEY_SLASH: u16 = 53;
const EVDEV_KEY_SPACE: u16 = 57;
const EVDEV_KEY_KP7: u16 = 71;
const EVDEV_KEY_KP9: u16 = 73;
const EVDEV_KEY_KP4: u16 = 75;
const EVDEV_KEY_KP6: u16 = 77;
const EVDEV_KEY_KP1: u16 = 79;
const EVDEV_KEY_KP3: u16 = 81;
const EVDEV_KEY_KP0: u16 = 82;
const EVDEV_KEY_KPENTER: u16 = 96;

const QWERTY_LETTER_ROW_CHARS: [char; 26] = [
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l',
    'z', 'x', 'c', 'v', 'b', 'n', 'm',
];

/// On-screen panel vs USB HID. Hardware replaces the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardSource {
    OnScreen,
    Hardware,
}

/// Letter/symbol OSK vs digit PIN pad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardLayout {
    Qwerty,
    Pin,
}

/// Letters/symbols page of the on-screen QWERTY. Hardware ignores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KeyboardMode {
    #[default]
    Letters,
    Symbols,
}

impl KeyboardMode {
    pub fn toggled(self) -> Self {
        match self {
            KeyboardMode::Letters => KeyboardMode::Symbols,
            KeyboardMode::Symbols => KeyboardMode::Letters,
        }
    }
}

/// One input from either source, applied to the bound Field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keystroke {
    Char(char),
    Backspace,
    Enter,
    Escape,
    ModeToggle,
}

/// Result of applying a keystroke to the bound value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardCommand {
    Edited,
    Submit,
    Cancel,
    Ignored,
}

/// USB typing device discovered in `/proc/bus/input/devices`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareKeyboardDevice {
    pub name: String,
    pub event_node: String,
}

/// Privileged composite: IME bound to a Field loc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keyboard {
    pub field_id: String,
    pub source: KeyboardSource,
    pub layout: KeyboardLayout,
    pub mode: KeyboardMode,
}

impl Keyboard {
    pub const QWERTY_LETTER_ROWS: [&'static str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
    pub const QWERTY_SYMBOL_ROWS: [&'static str; 3] = ["1234567890", "-_/:;()$&@\"", ".,?!'#%^*+="];
    pub const PIN_DIGIT_ROWS: [&'static str; 4] = ["123", "456", "789", " 0⌫"];

    pub fn bind(field_id: impl Into<String>, layout: KeyboardLayout) -> Self {
        Self {
            field_id: field_id.into(),
            source: KeyboardSource::OnScreen,
            layout,
            mode: KeyboardMode::Letters,
        }
    }

    pub fn with_source(mut self, source: KeyboardSource) -> Self {
        self.source = source;
        self
    }

    pub fn set_source(&mut self, source: KeyboardSource) {
        self.source = source;
    }

    /// Android hides the IME panel when a hardware keyboard is attached.
    pub fn shows_panel(&self) -> bool {
        self.source == KeyboardSource::OnScreen
    }

    pub fn qwerty_rows(mode: KeyboardMode) -> &'static [&'static str; 3] {
        match mode {
            KeyboardMode::Letters => &Self::QWERTY_LETTER_ROWS,
            KeyboardMode::Symbols => &Self::QWERTY_SYMBOL_ROWS,
        }
    }

    pub fn handle(&mut self, stroke: Keystroke, value: &mut String) -> KeyboardCommand {
        match stroke {
            Keystroke::Char(ch) => {
                if self.layout == KeyboardLayout::Pin && !ch.is_ascii_digit() {
                    return KeyboardCommand::Ignored;
                }
                value.push(ch);
                KeyboardCommand::Edited
            }
            Keystroke::Backspace => {
                let _ = value.pop();
                KeyboardCommand::Edited
            }
            Keystroke::ModeToggle
                if self.layout == KeyboardLayout::Qwerty
                    && self.source == KeyboardSource::OnScreen =>
            {
                self.mode = self.mode.toggled();
                KeyboardCommand::Edited
            }
            Keystroke::Enter => KeyboardCommand::Submit,
            Keystroke::Escape => KeyboardCommand::Cancel,
            Keystroke::ModeToggle => KeyboardCommand::Ignored,
        }
    }

    pub fn apply_to_field(&mut self, stroke: Keystroke, field: &mut Field) -> KeyboardCommand {
        if field.disabled {
            return KeyboardCommand::Ignored;
        }
        self.handle(stroke, &mut field.value)
    }

    pub fn keystroke_from_osk_action(action: &str) -> Option<Keystroke> {
        match action {
            "intent:cancel" | "Отмена" => Some(Keystroke::Escape),
            "intent:send" | "Готово" => Some(Keystroke::Enter),
            "intent:mode:toggle" => Some(Keystroke::ModeToggle),
            "intent:space" => Some(Keystroke::Char(' ')),
            "intent:backspace" | "⌫" => Some(Keystroke::Backspace),
            other => {
                if let Some(key) = other.strip_prefix("intent:key:") {
                    return key.chars().next().map(Keystroke::Char);
                }
                let mut chars = other.chars();
                match (chars.next(), chars.next()) {
                    (Some(ch), None) if ch.is_ascii_digit() => Some(Keystroke::Char(ch)),
                    _ => None,
                }
            }
        }
    }

    pub fn keystroke_from_evdev(code: u16, layout: KeyboardLayout) -> Option<Keystroke> {
        match code {
            EVDEV_KEY_ESC => Some(Keystroke::Escape),
            EVDEV_KEY_BACKSPACE => Some(Keystroke::Backspace),
            EVDEV_KEY_ENTER | EVDEV_KEY_KPENTER => Some(Keystroke::Enter),
            EVDEV_KEY_SPACE if layout == KeyboardLayout::Qwerty => Some(Keystroke::Char(' ')),
            other => {
                let ch = evdev_char(other)?;
                match layout {
                    KeyboardLayout::Pin if ch.is_ascii_digit() => Some(Keystroke::Char(ch)),
                    KeyboardLayout::Qwerty => Some(Keystroke::Char(ch)),
                    KeyboardLayout::Pin => None,
                }
            }
        }
    }
}

/// Devices that can type `KEY_A` and are not volume/power/touch/haptic.
pub fn hardware_keyboards(proc_bus_input: &str) -> Vec<HardwareKeyboardDevice> {
    parse_input_blocks(proc_bus_input)
        .into_iter()
        .filter_map(|block| {
            if excluded_input_name(&block.name) {
                return None;
            }
            if !block
                .handlers
                .split_whitespace()
                .any(|handler| handler == "kbd")
            {
                return None;
            }
            if !key_bit_set(&block.key, EVDEV_KEY_A) {
                return None;
            }
            let event_node = block
                .handlers
                .split_whitespace()
                .find(|handler| handler.starts_with("event"))
                .map(str::to_string)?;
            Some(HardwareKeyboardDevice {
                name: block.name,
                event_node,
            })
        })
        .collect()
}

pub fn hardware_keyboard_present(proc_bus_input: &str) -> bool {
    !hardware_keyboards(proc_bus_input).is_empty()
}

#[derive(Default)]
struct InputBlock {
    name: String,
    handlers: String,
    key: String,
}

fn parse_input_blocks(text: &str) -> Vec<InputBlock> {
    let mut blocks = Vec::new();
    let mut current = InputBlock::default();
    let mut any = false;
    for line in text.lines() {
        if line.trim().is_empty() {
            if any {
                blocks.push(std::mem::take(&mut current));
                any = false;
            }
            continue;
        }
        any = true;
        if let Some(rest) = line.strip_prefix("N: Name=") {
            current.name = rest.trim().trim_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("H: Handlers=") {
            current.handlers = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("B: KEY=") {
            current.key = rest.trim().to_string();
        }
    }
    if any {
        blocks.push(current);
    }
    blocks
}

fn excluded_input_name(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    [
        "gpio-keys",
        "gpio_keys",
        "gpio keys",
        "s2mpg",
        "power-keys",
        "power_keys",
        "power-button",
        "fts",
        "touchscreen",
        "goodix",
        "cs40",
        "haptic",
        "aw869",
        "volume",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn key_bit_set(key_hex: &str, code: u16) -> bool {
    let words: Vec<&str> = key_hex.split_whitespace().collect();
    if words.is_empty() {
        return false;
    }
    let width = if words.iter().any(|word| word.len() > 8) {
        64u32
    } else {
        32
    };
    let parsed: Vec<u64> = words
        .iter()
        .filter_map(|word| u64::from_str_radix(word, 16).ok())
        .collect();
    let word_from_low = u32::from(code) / width;
    let bit = u32::from(code) % width;
    let index = word_from_low as usize;
    if index >= parsed.len() {
        return false;
    }
    let word = parsed[parsed.len() - 1 - index];
    (word & (1u64 << bit)) != 0
}

fn evdev_char(code: u16) -> Option<char> {
    match code {
        EVDEV_KEY_1..=EVDEV_KEY_0 => {
            let digits = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'];
            Some(digits[(code - EVDEV_KEY_1) as usize])
        }
        EVDEV_KEY_Q..=EVDEV_KEY_P => Some(QWERTY_LETTER_ROW_CHARS[(code - EVDEV_KEY_Q) as usize]),
        EVDEV_KEY_A..=EVDEV_KEY_L => {
            Some(QWERTY_LETTER_ROW_CHARS[(10 + code - EVDEV_KEY_A) as usize])
        }
        EVDEV_KEY_Z..=EVDEV_KEY_M => {
            Some(QWERTY_LETTER_ROW_CHARS[(19 + code - EVDEV_KEY_Z) as usize])
        }
        EVDEV_KEY_MINUS => Some('-'),
        EVDEV_KEY_EQUAL => Some('='),
        EVDEV_KEY_SEMICOLON => Some(';'),
        EVDEV_KEY_APOSTROPHE => Some('\''),
        EVDEV_KEY_COMMA => Some(','),
        EVDEV_KEY_DOT => Some('.'),
        EVDEV_KEY_SLASH => Some('/'),
        EVDEV_KEY_KP1..=EVDEV_KEY_KP3 => Some(char::from(b'1' + (code - EVDEV_KEY_KP1) as u8)),
        EVDEV_KEY_KP4..=EVDEV_KEY_KP6 => Some(char::from(b'4' + (code - EVDEV_KEY_KP4) as u8)),
        EVDEV_KEY_KP7..=EVDEV_KEY_KP9 => Some(char::from(b'7' + (code - EVDEV_KEY_KP7) as u8)),
        EVDEV_KEY_KP0 => Some('0'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FieldKind;

    const USB_KEYBOARD: &str = "\
I: Bus=0003 Vendor=046d Product=c31c Version=0110
N: Name=\"Logitech USB Keyboard\"
P: Phys=usb-0000:00:14.0-1/input0
H: Handlers=sysrq kbd event4 leds
B: PROP=0
B: EV=120013
B: KEY=10000 7 ff800000 7ff febeffdf ffefffff ffffffff fffffffe

I: Bus=0019 Vendor=0001 Product=0001 Version=0100
N: Name=\"gpio-keys\"
H: Handlers=kbd event1
B: EV=3
B: KEY=1c0000 0 0 0 0 0 0 0 0 0

I: Bus=0019 Vendor=0000 Product=0000 Version=0000
N: Name=\"s2mpg12-power-keys\"
H: Handlers=kbd event2
B: EV=3
B: KEY=100000 0 0 0 0

I: Bus=0018 Vendor=0000 Product=0000 Version=0000
N: Name=\"fts_ts\"
H: Handlers=event0
B: EV=b
B: KEY=400 0 0 0 0 0 0 0 0 0 0
";

    #[test]
    fn keyboard_binds_a_field_and_applies_on_screen_keystrokes() {
        let mut keyboard = Keyboard::bind("intent-field", KeyboardLayout::Qwerty);
        let mut field = Field::new("Новое намерение", FieldKind::Text);
        assert!(keyboard.shows_panel());
        assert_eq!(keyboard.field_id, "intent-field");
        assert_eq!(
            keyboard.apply_to_field(Keystroke::Char('h'), &mut field),
            KeyboardCommand::Edited
        );
        assert_eq!(
            keyboard.apply_to_field(Keystroke::Char('i'), &mut field),
            KeyboardCommand::Edited
        );
        assert_eq!(field.value, "hi");
        assert_eq!(
            keyboard.apply_to_field(Keystroke::Backspace, &mut field),
            KeyboardCommand::Edited
        );
        assert_eq!(field.value, "h");
        assert_eq!(
            keyboard.handle(Keystroke::Enter, &mut field.value),
            KeyboardCommand::Submit
        );
        assert_eq!(
            keyboard.handle(Keystroke::Escape, &mut field.value),
            KeyboardCommand::Cancel
        );
    }

    #[test]
    fn hardware_source_hides_the_on_screen_panel() {
        let keyboard = Keyboard::bind("intent-field", KeyboardLayout::Qwerty)
            .with_source(KeyboardSource::Hardware);
        assert!(!keyboard.shows_panel());
    }

    #[test]
    fn pin_layout_rejects_letters_and_accepts_digits() {
        let mut keyboard = Keyboard::bind("pin-field", KeyboardLayout::Pin);
        let mut value = String::new();
        assert_eq!(
            keyboard.handle(Keystroke::Char('a'), &mut value),
            KeyboardCommand::Ignored
        );
        assert_eq!(
            keyboard.handle(Keystroke::Char('4'), &mut value),
            KeyboardCommand::Edited
        );
        assert_eq!(value, "4");
    }

    #[test]
    fn usb_keyboard_is_detected_and_volume_power_touch_are_not() {
        let found = hardware_keyboards(USB_KEYBOARD);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Logitech USB Keyboard");
        assert_eq!(found[0].event_node, "event4");
        assert!(hardware_keyboard_present(USB_KEYBOARD));
        assert!(!hardware_keyboard_present(
            "N: Name=\"gpio-keys\"\nH: Handlers=kbd event1\nB: KEY=1c0000 0 0 0 0\n"
        ));
    }

    #[test]
    fn evdev_key_a_types_into_the_bound_field() {
        let mut keyboard = Keyboard::bind("intent-field", KeyboardLayout::Qwerty)
            .with_source(KeyboardSource::Hardware);
        let mut value = String::new();
        let stroke =
            Keyboard::keystroke_from_evdev(EVDEV_KEY_A, KeyboardLayout::Qwerty).expect("KEY_A");
        assert_eq!(stroke, Keystroke::Char('a'));
        assert_eq!(keyboard.handle(stroke, &mut value), KeyboardCommand::Edited);
        assert_eq!(value, "a");
        let digit = Keyboard::keystroke_from_evdev(EVDEV_KEY_1, KeyboardLayout::Pin);
        assert_eq!(digit, Some(Keystroke::Char('1')));
        let letter_on_pin = Keyboard::keystroke_from_evdev(EVDEV_KEY_A, KeyboardLayout::Pin);
        assert!(letter_on_pin.is_none());
    }

    #[test]
    fn osk_actions_map_onto_the_same_keystrokes() {
        assert_eq!(
            Keyboard::keystroke_from_osk_action("intent:key:q"),
            Some(Keystroke::Char('q'))
        );
        assert_eq!(
            Keyboard::keystroke_from_osk_action("intent:backspace"),
            Some(Keystroke::Backspace)
        );
        assert_eq!(
            Keyboard::keystroke_from_osk_action("5"),
            Some(Keystroke::Char('5'))
        );
        assert_eq!(Keyboard::keystroke_from_osk_action("Убрать PIN"), None);
    }
}
