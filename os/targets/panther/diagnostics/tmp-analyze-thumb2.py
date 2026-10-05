#!/usr/bin/env python3
"""Fast Thumb-2 MOVW index then query log IDs."""
import struct
from collections import defaultdict
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
img = PATH.read_bytes()
MAIN_OFF = 0x16C10
MAIN_END = MAIN_OFF + 0x05917ACC


def u16(off):
    return struct.unpack_from("<H", img, off)[0]


def u32(off):
    return struct.unpack_from("<I", img, off)[0]


def dump_thumb(off, n=20):
    parts = []
    o = off
    for _ in range(n):
        if o + 2 > len(img):
            break
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800) and o + 4 <= len(img):
            parts.append(f"{hw:04x}{u16(o+2):04x}")
            o += 4
        else:
            parts.append(f"{hw:04x}")
            o += 2
    return " ".join(parts)


# Build MOVW index: only check halfwords starting with F24x / F64x (movw/movt family)
index = defaultdict(list)
print("indexing MOVW...")
for o in range(MAIN_OFF, MAIN_END - 4, 2):
    hw1 = u16(o)
    if (hw1 & 0xFBF0) != 0xF240:
        continue
    hw2 = u16(o + 2)
    if hw2 & 0x8000:
        continue
    i = (hw1 >> 10) & 1
    imm4 = hw1 & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    imm16 = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
    index[imm16].append((o, rd))
print(f"unique_imms={len(index)} total_movw={sum(len(v) for v in index.values())}")

records = [
    ("tx_app_state", b"[SIT_0_SIM] Tx SIM Status(app_state=%d, perso_substate=%d)"),
    ("sit_set_pin1", b"[SIT_0_SIM] sitSetPin1Status:- PIN 1 Status"),
    ("sit_get_pin1", b"[SIT_0_SIM] sitGetPin1Status:- PIN 1 Status"),
    ("determine", b"[USIM_%d] >> DetermineSimStatus"),
    ("pin_disabled_fcp", b"[USIM_%d] RemainingAttemptCount %d and Pin Status Disabled in APP FCP so changing PinStatus as PIN_DISABLED"),
    ("pin1_en_notif", b"[USIM_%d] PIN1 Enabled, sending Notification to AP"),
    ("pin_skip_esim", b"[USIM_%d] PIN SKIP FAILED: NOT eSIM"),
    ("pin_skip_state", b"[USIM_%d] PIN SKIP FAILED: Invalid SimState"),
    ("pin_action_not", b"ACPIN: PIN Action Not Required"),
    ("get_multi_pin1", b"[USIM_%d] >> GetMultiPin1Status - AppType"),
]

for name, s in records:
    off = img.find(s)
    if off < 0:
        print(f"\n{name}: MISSING")
        continue
    log_id = u32(off - 8) & 0xFFFF
    ctx = u32(off - 4)
    print(f"\n{name}: str@0x{off:x} id=0x{log_id:x} ctx=0x{ctx:x}")
    hits = index.get(log_id, [])
    print(f"  MOVW hits={len(hits)}")
    for o, rd in hits[:6]:
        print(f"  @0x{o:x} rd={rd}")
        print(f"    {dump_thumb(max(MAIN_OFF, o-8), 24)}")

# Deep dive: pin_disabled_fcp and determine — find function start (push.w)
print("\n=== function windows ===")
for label, imm in [("pin_disabled_fcp", 0x7AB), ("determine", 0x2C5E), ("tx_app_state", 0x347), ("sit_set_pin1", 0xC6B)]:
    hits = index.get(imm, [])
    print(f"\n-- {label} imm=0x{imm:x} --")
    for o, rd in hits[:3]:
        # walk back for push {..} / push.w
        entry = None
        for b in range(0, 0x180, 2):
            p = o - b
            if p < MAIN_OFF:
                break
            hw = u16(p)
            # push.w = E92D
            if hw == 0xE92D:
                entry = p
                break
            # push = B5xx / B4xx
            if (hw & 0xFE00) == 0xB400 or (hw & 0xFF00) == 0xB500:
                entry = p
                # keep looking for wider push.w
        print(f"  site@0x{o:x} entry~0x{entry:x}" if entry else f"  site@0x{o:x} entry=?")
        if entry:
            print(f"    from_entry: {dump_thumb(entry, 40)}")
        # scan ±0x120 for movs #2/#5 and cmp #2/#5/#3
        marks = []
        for p in range(max(MAIN_OFF, o - 0x120), min(MAIN_END, o + 0x120), 2):
            hw = u16(p)
            if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (0, 1, 2, 3, 4, 5):
                marks.append(f"movs_r{(hw>>8)&7}_#{hw&0xff}@0x{p:x}")
            if (hw & 0xFF00) == 0x2800 and (hw & 0xFF) in (0, 1, 2, 3, 4, 5):
                marks.append(f"cmp_r{(hw>>8)&7}_#{hw&0xff}@0x{p:x}")
        print(f"    imm_marks: {marks[:25]}")
