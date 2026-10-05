#!/usr/bin/env python3
"""Thumb-2 MOVW xref for SIM app_state / PIN_DISABLED log IDs in Shannon MAIN."""
import struct
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


def thumb_movw_imm(off):
    """If halfword pair at off is MOVW Rd,#imm16, return (rd, imm16)."""
    if off + 4 > len(img):
        return None
    hw1 = u16(off)
    hw2 = u16(off + 2)
    # 11110 i 100100 imm4 | 0 imm3 Rd imm8
    if (hw1 & 0xFBF0) != 0xF240:
        return None
    if (hw2 & 0x8000) != 0:
        return None
    i = (hw1 >> 10) & 1
    imm4 = hw1 & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    imm16 = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
    return rd, imm16


def find_movw(imm, limit=20):
    hits = []
    # Thumb can be 2-byte aligned
    for o in range(MAIN_OFF, MAIN_END - 4, 2):
        r = thumb_movw_imm(o)
        if r and r[1] == imm:
            hits.append((o, r[0]))
            if len(hits) >= limit:
                break
    return hits


def dump_thumb(off, n=12):
    parts = []
    o = off
    for _ in range(n):
        hw = u16(o)
        # wide?
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            parts.append(f"{hw:04x}{hw2:04x}")
            o += 4
        else:
            parts.append(f"{hw:04x}")
            o += 2
    return " ".join(parts)


records = [
    ("tx_app_state", b"[SIT_0_SIM] Tx SIM Status(app_state=%d, perso_substate=%d)", 8),
    ("sit_set_pin1", b"[SIT_0_SIM] sitSetPin1Status:- PIN 1 Status", 8),
    ("determine", b"[USIM_%d] >> DetermineSimStatus", 8),
    ("pin_disabled_fcp", b"[USIM_%d] RemainingAttemptCount %d and Pin Status Disabled in APP FCP so changing PinStatus as PIN_DISABLED", 8),
    ("pin1_en_notif", b"[USIM_%d] PIN1 Enabled, sending Notification to AP", 8),
    ("pin_skip_esim", b"[USIM_%d] PIN SKIP FAILED: NOT eSIM", 8),
    ("pin_skip_state", b"[USIM_%d] PIN SKIP FAILED: Invalid SimState", 8),
]

print("=== Thumb-2 MOVW sites for log IDs ===")
for name, s, back in records:
    off = img.find(s)
    if off < 0:
        print(f"{name}: MISSING")
        continue
    log_id = u32(off - back)
    ctx = u32(off - 4)
    print(f"\n{name}: str@0x{off:x} id=0x{log_id:x} ctx=0x{ctx:x}")
    hits = find_movw(log_id & 0xFFFF)
    print(f"  MOVW #0x{log_id & 0xFFFF:x} hits={len(hits)}")
    for o, rd in hits:
        print(f"  @0x{o:x} rd={rd}")
        print(f"    {dump_thumb(o, 16)}")

# Also search for stores of immediate 2 vs 5 near pin_disabled / determine sites
# Look at code around first good hit for pin_disabled and determine

print("\n=== PIN/READY immediate ops near pin_disabled MOVW ===")
hits = find_movw(0x7AB)
for o, rd in hits[:3]:
    # scan ±0x80 for mov #2 / #5 (Thumb: 2002 = movs r0,#2; 2005 = movs r0,#5)
    lo, hi = max(MAIN_OFF, o - 0x100), min(MAIN_END, o + 0x100)
    imms = []
    for p in range(lo, hi, 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000:  # movs rd, #imm8
            imms.append((p, hw & 0xFF, (hw >> 8) & 7))
        r = thumb_movw_imm(p)
        if r and r[1] in (2, 3, 4, 5, 0, 1):
            imms.append((p, r[1], f"movw_r{r[0]}"))
    print(f"around 0x{o:x}: small_imms={imms[:20]}")
