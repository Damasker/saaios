/// Files copied into the model bundle. Verbose USB and full `dmesg` stay out.
pub const REDACTED_FILES: &[&str] = &[
    "uname.txt",
    "cpu.txt",
    "pci.txt",
    "pci_drivers.txt",
    "pci_tree.txt",
    "usb.txt",
    "usb_tree.txt",
    "block.txt",
    "modules.txt",
    "drm.txt",
    "pci_sysfs.txt",
    "dmi.txt",
    "kernel_warnings.txt",
];

/// Drop identifier lines and replace MAC, UUID, and serial values.
pub fn redact_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for line in input.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let ending = &line[body.len()..];
        if contains_iserial(body) {
            continue;
        }
        let cleaned = redact_serial_assignment(&replace_macs(&replace_uuids(body)));
        out.push_str(&cleaned);
        out.push_str(ending);
    }
    out
}

fn contains_iserial(line: &str) -> bool {
    line.to_ascii_lowercase().contains("iserial")
}

fn replace_macs(input: &str) -> String {
    replace_spans(input, mac_end, "[mac]")
}

fn mac_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start;
    for part in 0..6 {
        if part > 0 {
            if cursor >= bytes.len() || bytes[cursor] != b':' {
                return None;
            }
            cursor += 1;
        }
        if cursor + 2 > bytes.len()
            || !bytes[cursor].is_ascii_hexdigit()
            || !bytes[cursor + 1].is_ascii_hexdigit()
        {
            return None;
        }
        cursor += 2;
    }
    Some(cursor)
}

fn replace_uuids(input: &str) -> String {
    replace_spans(input, uuid_end, "[uuid]")
}

fn replace_spans(input: &str, detect: fn(&[u8], usize) -> Option<usize>, token: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut index = 0;
    while index < bytes.len() {
        if let Some(end) = detect(bytes, index) {
            out.push_str(token);
            index = end;
            continue;
        }
        let ch = input[index..]
            .chars()
            .next()
            .expect("index is on a char boundary");
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

fn uuid_end(bytes: &[u8], start: usize) -> Option<usize> {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let mut cursor = start;
    for (group, width) in GROUPS.iter().enumerate() {
        if group > 0 {
            if cursor >= bytes.len() || bytes[cursor] != b'-' {
                return None;
            }
            cursor += 1;
        }
        if cursor + width > bytes.len()
            || !bytes[cursor..cursor + width]
                .iter()
                .all(|b| b.is_ascii_hexdigit())
        {
            return None;
        }
        cursor += width;
    }
    Some(cursor)
}

fn redact_serial_assignment(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let Some(key_at) = lower.find("serial").or_else(|| lower.find("uuid")) else {
        return line.to_string();
    };
    let tail = &line[key_at..];
    let Some(split) = tail.find(['=', ':']) else {
        return line.to_string();
    };
    let mut rebuilt = String::new();
    rebuilt.push_str(&line[..key_at + split + 1]);
    let value = tail[split + 1..].trim_start();
    let skipped = tail.len() - value.len();
    rebuilt.push_str(&line[key_at + split + 1..key_at + skipped]);
    if value.starts_with('"') {
        rebuilt.push_str("\"[redacted]\"");
    } else if !value.is_empty() {
        rebuilt.push_str("[redacted]");
    }
    rebuilt
}
