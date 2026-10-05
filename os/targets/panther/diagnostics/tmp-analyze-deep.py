#!/usr/bin/env python3
"""Deep gates: DetermineSimStatus body + Pin1Verified + app_state writers."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
VA_BASE = 0x40010000
img = PATH.read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    return (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8, rd


def dis_compact(start, length):
    o = start
    end = start + length
    out = []
    while o < end:
        r = movw(o)
        hw = u16(o)
        if r:
            note = ""
            if r[0] in (0x2C5E, 0x7AB, 0x347, 0xC6B, 0x11D6, 0x64D, 0x136F):
                note = {
                    0x2C5E: "DetermineSimStatus",
                    0x7AB: "PIN_DISABLED_FCP",
                    0x347: "TxAppState",
                    0xC6B: "sitSetPin1",
                    0x11D6: "PIN_SKIP_NOT_eSIM",
                    0x64D: "PIN1_Enabled_notif",
                    0x136F: "GetMultiPin1",
                }[r[0]]
                note = f"  ; {note}"
            out.append(f"  0x{o:08x}/va0x{va(o):08x}: MOVW r{r[1]},#0x{r[0]:x}{note}")
            o += 4
            continue
        if (hw & 0xFBF0) == 0xF2C0 and (u16(o + 2) & 0x8000) == 0:
            hw2 = u16(o + 2)
            i = (hw >> 10) & 1
            imm = ((i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF))
            out.append(f"  0x{o:08x}: MOVT r{(hw2>>8)&0xF},#0x{imm:x}")
            o += 4
            continue
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            # BL/B.W etc — compute BL target if possible
            hw2 = u16(o + 2)
            if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                # BL
                s = (hw >> 10) & 1
                imm10 = hw & 0x3FF
                j1 = (hw2 >> 13) & 1
                j2 = (hw2 >> 11) & 1
                imm11 = hw2 & 0x7FF
                i1 = ~(j1 ^ s) & 1
                i2 = ~(j2 ^ s) & 1
                imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
                if s:
                    imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
                    if imm32 >= 0x80000000:
                        imm32 -= 0x100000000
                tgt = o + 4 + imm32
                out.append(f"  0x{o:08x}/va0x{va(o):08x}: BL -> file0x{tgt:x}/va0x{va(tgt):x}")
            o += 4
            continue
        if (hw & 0xFF00) == 0x2000:
            imm = hw & 0xFF
            tag = {2: "PIN", 3: "DISABLED/PUK?", 5: "READY", 0: "UNK", 1: "PIN?"}.get(imm, "")
            tag = f"  ; <<{tag}" if tag else ""
            out.append(f"  0x{o:08x}/va0x{va(o):08x}: MOVS r{(hw>>8)&7},#{imm}{tag}")
        elif (hw & 0xFF00) == 0x2800:
            out.append(f"  0x{o:08x}: CMP r{(hw>>8)&7},#{hw&0xff}")
        elif hw in (0xB510, 0xB570, 0xB5F0, 0xB580, 0xB5B0, 0xB5F8):
            out.append(f"  0x{o:08x}/va0x{va(o):08x}: PUSH")
        elif (hw & 0xFF00) == 0xD000:
            out.append(f"  0x{o:08x}: Bcond.N")
        elif (hw & 0xF800) == 0xE000:
            out.append(f"  0x{o:08x}: B.N")
        o += 2
    return "\n".join(out)


def find_all(s, limit=8):
    hits, start = [], 0
    while len(hits) < limit:
        j = img.find(s, start)
        if j < 0:
            break
        hits.append(j)
        start = j + 1
    return hits


# 1) strings around SIM STATUS update / Pin1Verified / Determine
print("=== key USIM/SIT strings (C-string extract) ===")
needles = [
    b"SIM STATUS update",
    b"Pin1Verified",
    b"MePerVerified",
    b"DetermineSimStatus",
    b"PIN SKIP FAILED",
    b"Updating PinStatus for Appln",
    b"sitSetPin1Status",
    b"sitGetPin1Status",
    b"Tx SIM Status",
    b"Application state",
    b"App State",
    b"app state",
    b"USIM_APP_STATE",
    b"SimAppStatus",
    b"perso_substate",
    b"PIN Action Not Required",
    b"PIN SKIP",
    b"eSIM",
    b"DecodeSimPinSkip",
    b"NS_SIM_PIN_SKIP",
    b"Invalid SimState for PIN SKIP",
    b"PIN SKIP FAILED: Invalid",
    b"changing PinStatus as PIN_DISABLED",
    b"Disabling CHV1",
    b"GetCHV1Att",
    b"Pin Status Disabled in APP FCP",
]
for n in needles:
    for off in find_all(n, 3):
        s0 = off
        while s0 > 0 and 32 <= img[s0 - 1] < 127:
            s0 -= 1
        s1 = off
        while s1 < len(img) and 32 <= img[s1] < 127:
            s1 += 1
        hid = u32(s0 - 8) if s0 >= 8 else 0
        print(f"  0x{s0:x} id?=0x{hid:x}: {img[s0:s1].decode()[:160]}")

# 2) DetermineSimStatus real site — dump function window
print("\n=== DetermineSimStatus @0x14c7b9a window ===")
# find PUSH before
fn = 0x14C7B9A
for p in range(0x14C7B9A, 0x14C7B9A - 0x200, -2):
    if u16(p) in (0xB5F0, 0xB570, 0xB5F8, 0xB580, 0xB5B0):
        fn = p
        break
print(f"fn_entry_guess=0x{fn:x} va=0x{va(fn):x}")
print(dis_compact(fn, 0x200))

# 3) pin_disabled site that also has MOVS#2 nearby (0x1507a00 region)
print("\n=== pin_disabled cluster @0x15078e0 (MOVS#2 nearby) ===")
print(dis_compact(0x15078A0, 0x220))

# 4) USIM pin_disabled that sets DISABLED=3 @0x2b36700
print("\n=== USIM FCP->PIN_DISABLED @0x2b36700 ===")
print(dis_compact(0x2B366F0, 0x140))

# 5) PIN SKIP NOT eSIM log id 0x11d6 isolated sites
print("\n=== PIN_SKIP_NOT_eSIM (0x11d6) isolated ===")


def isolated(imm, lim=8):
    hits = []
    for o in range(MAIN_OFF, MAIN_OFF + 0x5917ACC - 8, 2):
        r = movw(o)
        if not r or r[0] != imm:
            continue
        fol = False
        for p in range(o + 4, o + 40, 2):
            r2 = movw(p)
            if r2 and r2[0] == imm + 1:
                fol = True
                break
        if not fol:
            hits.append(o)
            if len(hits) >= lim:
                break
    return hits


for o in isolated(0x11D6, 6):
    print(f"\nSKIP_FAIL @0x{o:x} va=0x{va(o):x}")
    print(dis_compact(o - 0x60, 0x100))

# 6) SIM STATUS update string — find refs via partial ADR (literal pool)
status_upd = img.find(b"SIM STATUS update: Present:")
print(f"\n=== SIM STATUS update str @0x{status_upd:x} ===")
if status_upd > 0:
    hid = u32(status_upd - 8)
    print(f"log_id=0x{hid:x}")
    for o in isolated(hid & 0xFFFF, 8) if 0 < (hid & 0xFFFF) < 0x10000 else []:
        print(f"\nSTATUS_UPD @0x{o:x} va=0x{va(o):x}")
        print(dis_compact(o - 0x80, 0x120))

# 7) Updating PinStatus for Appln
upd = img.find(b"Updating PinStatus for Appln pin")
print(f"\n=== Updating PinStatus str @0x{upd:x} ===")
if upd > 0:
    hid = u32(upd - 8)
    print(f"log_id=0x{hid:x}")
    if 0 < hid < 0x10000:
        for o in isolated(hid, 6):
            print(f"\nUPD_PIN @0x{o:x} va=0x{va(o):x}")
            print(dis_compact(o - 0x40, 0xC0))
