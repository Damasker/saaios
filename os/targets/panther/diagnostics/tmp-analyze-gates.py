#!/usr/bin/env python3
"""Dump Thumb around PIN_DISABLED FCP log and DetermineSimStatus real call sites."""
import struct
from pathlib import Path

img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def dis(start, length=0x180):
    o = start
    end = start + length
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            op = f"{hw:04x} {hw2:04x}"
            note = ""
            # movw
            if (hw & 0xFBF0) == 0xF240 and (hw2 & 0x8000) == 0:
                i = (hw >> 10) & 1
                imm4 = hw & 0xF
                imm3 = (hw2 >> 12) & 7
                rd = (hw2 >> 8) & 0xF
                imm8 = hw2 & 0xFF
                imm = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                note = f"  ; MOVW r{rd},#0x{imm:x}"
            elif (hw & 0xFBF0) == 0xF2C0 and (hw2 & 0x8000) == 0:
                i = (hw >> 10) & 1
                imm4 = hw & 0xF
                imm3 = (hw2 >> 12) & 7
                rd = (hw2 >> 8) & 0xF
                imm8 = hw2 & 0xFF
                imm = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                note = f"  ; MOVT r{rd},#0x{imm:x}"
            elif hw == 0xF44F:
                note = "  ; MOV.W ?"
            lines.append(f"  0x{o:08x}: {op}{note}")
            o += 4
        else:
            note = ""
            if (hw & 0xFF00) == 0x2000:
                note = f"  ; MOVS r{(hw>>8)&7}, #{hw&0xff}"
            elif (hw & 0xFF00) == 0x2800:
                note = f"  ; CMP r{(hw>>8)&7}, #{hw&0xff}"
            elif (hw & 0xF800) == 0xD000:
                note = f"  ; Bcond"
            elif (hw & 0xF800) == 0xE000:
                note = f"  ; B"
            elif hw == 0xB510 or hw == 0xB570 or hw == 0xB5F0 or hw == 0xB580:
                note = "  ; PUSH"
            elif hw == 0xBD10 or hw == 0xBD70 or hw == 0xBDF0:
                note = "  ; POP/BX LR"
            lines.append(f"  0x{o:08x}: {hw:04x}{note}")
            o += 2
    return "\n".join(lines)


# Region with movs #1..#5 near first pin_disabled MOVW
print("=== around 0x114ec00 (app_state imm ladder near pin_disabled log?) ===")
print(dis(0x114EC00, 0x120))

print("\n=== DetermineSimStatus call-ish @0x14c7b80 ===")
print(dis(0x14C7B80, 0x100))

print("\n=== pin_disabled MOVW site @0x114ece0 ===")
print(dis(0x114ECE0, 0x80))

# Search for unique string "changing PinStatus as PIN_DISABLED" - find BL targets
# by looking for code that does MOVW #0x7ab NOT in a dense ID table.
# Dense tables have sequential MOVW #0x7ab, #0x7ac, #0x7ad...
# Real call: isolated MOVW #0x7ab

print("\n=== isolated MOVW #0x7ab (not followed by #0x7ac within 32B) ===")
hits = []
for o in range(0x16C10, 0x16C10 + 0x05917ACC - 8, 2):
    hw = u16(o)
    if (hw & 0xFBF0) != 0xF240:
        continue
    hw2 = u16(o + 2)
    if hw2 & 0x8000:
        continue
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    imm = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
    if imm != 0x7AB:
        continue
    # check next 48 bytes for MOVW #0x7AC
    followed = False
    for p in range(o + 4, o + 48, 2):
        h1 = u16(p)
        if (h1 & 0xFBF0) != 0xF240:
            continue
        h2 = u16(p + 2)
        if h2 & 0x8000:
            continue
        i2 = (h1 >> 10) & 1
        imm42 = h1 & 0xF
        imm32 = (h2 >> 12) & 7
        imm82 = h2 & 0xFF
        imm2 = (i2 << 11) | (imm42 << 12) | (imm32 << 8) | imm82
        if imm2 == 0x7AC:
            followed = True
            break
    if not followed:
        hits.append((o, rd))

print(f"isolated_count={len(hits)}")
for o, rd in hits[:15]:
    print(f"\nISOLATED @0x{o:x} rd={rd}")
    print(dis(o - 0x40, 0xC0))
