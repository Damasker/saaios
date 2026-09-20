//! USB HID on the x86 surface (PCE-25). Volume/power/touch stay out —
//! those are wave E on panther.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidDevice {
    pub name: String,
    pub event_node: String,
}

pub fn usb_hid_keyboards(proc_bus_input: &str) -> Vec<HidDevice> {
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
            let event_node = event_node(&block.handlers)?;
            Some(HidDevice {
                name: block.name,
                event_node,
            })
        })
        .collect()
}

pub fn usb_hid_pointers(proc_bus_input: &str) -> Vec<HidDevice> {
    parse_input_blocks(proc_bus_input)
        .into_iter()
        .filter_map(|block| {
            if excluded_input_name(&block.name) {
                return None;
            }
            if !block
                .handlers
                .split_whitespace()
                .any(|handler| handler == "mouse" || handler.starts_with("mouse"))
            {
                return None;
            }
            let event_node = event_node(&block.handlers)?;
            Some(HidDevice {
                name: block.name,
                event_node,
            })
        })
        .collect()
}

const EVDEV_KEY_A: u16 = 30;

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

fn event_node(handlers: &str) -> Option<String> {
    handlers
        .split_whitespace()
        .find(|handler| handler.starts_with("event"))
        .map(str::to_string)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeatInputProfile {
    pub pointer: bool,
    pub keyboard: bool,
    pub touch: bool,
}

pub fn seat_input_profile(phone_gate: bool) -> SeatInputProfile {
    if phone_gate {
        SeatInputProfile {
            pointer: false,
            keyboard: false,
            touch: true,
        }
    } else {
        SeatInputProfile {
            pointer: true,
            keyboard: true,
            touch: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const USB_FIXTURE: &str = "\
I: Bus=0003 Vendor=046d Product=c31c Version=0110
N: Name=\"Logitech USB Keyboard\"
P: Phys=usb-0000:00:14.0-1/input0
H: Handlers=sysrq kbd event4 leds
B: PROP=0
B: EV=120013
B: KEY=10000 7 ff800000 7ff febeffdf ffefffff ffffffff fffffffe

I: Bus=0003 Vendor=046d Product=c077 Version=0110
N: Name=\"Logitech USB Optical Mouse\"
H: Handlers=mouse0 event5
B: EV=17
B: KEY=70000 0 0 0 0

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
";

    #[test]
    fn usb_hid_keyboard_and_pointer_are_kept_volume_power_touch_are_not() {
        let keyboards = usb_hid_keyboards(USB_FIXTURE);
        assert_eq!(keyboards.len(), 1);
        assert_eq!(keyboards[0].name, "Logitech USB Keyboard");
        assert_eq!(keyboards[0].event_node, "event4");
        let pointers = usb_hid_pointers(USB_FIXTURE);
        assert_eq!(pointers.len(), 1);
        assert_eq!(pointers[0].name, "Logitech USB Optical Mouse");
        assert_eq!(pointers[0].event_node, "event5");
    }

    #[test]
    fn x86_seat_has_pointer_and_keyboard_panther_stays_touch() {
        assert_eq!(
            seat_input_profile(false),
            SeatInputProfile {
                pointer: true,
                keyboard: true,
                touch: false,
            }
        );
        assert_eq!(
            seat_input_profile(true),
            SeatInputProfile {
                pointer: false,
                keyboard: false,
                touch: true,
            }
        );
    }
}
