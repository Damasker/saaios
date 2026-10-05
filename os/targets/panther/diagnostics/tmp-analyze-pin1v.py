#!/usr/bin/env python3
"""RO: who sets/clears Pin1Verified; relation to VerifyPin/FCP/DISABLED."""
import struct
from pathlib import Path
from collections import Counter

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
MAIN_SZ = 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()
END = MAIN_OFF + MAIN_SZ


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


def bl_target(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000:
        return None
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
    return o + 4 + imm32


def cstr_at(off):
    s0 = off
    while s0 > 0 and 32 <= img[s0 - 1] < 127:
        s0 -= 1
    s1 = off
    while s1 < len(img) and 32 <= img[s1] < 127:
        s1 += 1
    return s0, img[s0:s1].decode("ascii", "replace")


def log_id(soff):
    for back in (8, 12, 16, 4):
        w = u32(soff - back)
        if 0 < w < 0x10000:
            return w, back
    return u32(soff - 8), 8


def isolated_movw(imm, lim=20):
    hits = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != imm:
            continue
        fol = False
        for p in range(o + 4, o + 48, 2):
            r2 = movw(p)
            if r2 and r2[0] == imm + 1:
                fol = True
                break
        if not fol:
            hits.append((o, r[1]))
            if len(hits) >= lim:
                break
    return hits


def dump_window(start, length):
    o = start
    end = start + length
    lines = []
    while o < end:
        r = movw(o)
        hw = u16(o)
        t = bl_target(o)
        if t is not None and (hw & 0xF800) == 0xF000:
            lines.append(f"  0x{o:08x}/va0x{va(o):08x}: BL -> 0x{t:x}/va0x{va(t):x}")
            o += 4
            continue
        if r:
            tag = {
                0x106A: "STATUS_UPD",
                0x7AB: "PIN_DISABLED_FCP",
                0xC6B: "sitSetPin1",
                0x347: "TxAppState",
                0x2C5E: "DetermineSimStatus",
            }.get(r[0], "")
            # try first-pin log ids later
            extra = f"  ; {tag}" if tag else ""
            lines.append(f"  0x{o:08x}/va0x{va(o):08x}: MOVW r{r[1]},#0x{r[0]:x}{extra}")
            o += 4
            continue
        if (hw & 0xFBF0) == 0xF2C0 and (u16(o + 2) & 0x8000) == 0:
            hw2 = u16(o + 2)
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            lines.append(f"  0x{o:08x}: MOVT r{(hw2>>8)&0xF},#0x{imm:x}")
            o += 4
            continue
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            o += 4
            continue
        if (hw & 0xFF00) == 0x2000:
            imm = hw & 0xFF
            note = ""
            if imm in (0, 1):
                note = f"  ; <<flag={imm}"
            elif imm in (2, 3, 5):
                note = {2: "PIN", 3: "DISABLED?", 5: "READY"}[imm]
                note = f"  ; <<{note}"
            lines.append(f"  0x{o:08x}/va0x{va(o):08x}: MOVS r{(hw>>8)&7},#{imm}{note}")
        elif (hw & 0xFF00) == 0x2800:
            lines.append(f"  0x{o:08x}: CMP r{(hw>>8)&7},#{hw&0xff}")
        elif hw in (0xB510, 0xB570, 0xB5F0, 0xB580, 0xB5B0, 0xB5F8):
            lines.append(f"  0x{o:08x}/va0x{va(o):08x}: PUSH")
        elif (hw & 0xFF00) == 0xD000:
            lines.append(f"  0x{o:08x}: Bcond")
        o += 2
    return "\n".join(lines)


# --- catalog strings ---
print("=== Pin1Verified-related strings ===")
needles = [
    b"Pin1Verified",
    b"IsSimVerifyCompleteSent",
    b"MePerVerified",
    b"First PIN1 Verification is done",
    b"sitSendNsSimInfoReq",
    b"SIM STATUS update: Present",
    b"VerifyPin",
    b"VERIFY PIN",
    b"Pin Verified",
    b"PIN1 Verified",
    b"PIN verified",
    b"CHV1 verified",
    b"CHV verified",
    b"Verify CHV",
    b"VERIFY CHV",
    b"changing PinStatus as PIN_DISABLED",
    b"sitSetPin1Status",
    b"NS_SIM_PIN",
    b"SIM_VERIFY",
    b"VerifyComplete",
]
seen = set()
for n in needles:
    start = 0
    while True:
        j = img.find(n, start)
        if j < 0:
            break
        s0, text = cstr_at(j)
        if s0 not in seen and len(text) >= 6:
            seen.add(s0)
            hid, back = log_id(s0)
            print(f"  0x{s0:x} id=0x{hid:x}(-{back}): {text[:150]}")
        start = j + 1

# First PIN1 Verification log
fp = img.find(b"First PIN1 Verification is done")
# walk back to [SIT_0
s0, text = cstr_at(fp)
hid, _ = log_id(s0)
print(f"\n=== First PIN1 Verification log_id=0x{hid:x} sites ===")
# also SIT_1 variant
fp1 = img.find(b"[SIT_1_SIM] First PIN1 Verification is done")
hid1, _ = log_id(fp1) if fp1 >= 0 else (0, 0)
print(f"SIT_0 str@0x{s0:x} id=0x{hid:x}")
if fp1 >= 0:
    print(f"SIT_1 str@0x{fp1:x} id=0x{hid1:x}")

for label, imm in [("SIT0_first_pin", hid), ("SIT1_first_pin", hid1)]:
    if not (0 < imm < 0x10000):
        continue
    hits = isolated_movw(imm, 10)
    print(f"\n--- {label} isolated MOVW #0x{imm:x}: {len(hits)} ---")
    for o, rd in hits:
        print(f"\nSITE file=0x{o:x} va=0x{va(o):x} rd={rd}")
        print(dump_window(o - 0x80, 0x140))

# STATUS update 0x106a region — already known; dump Pin1Verified store pattern
# Look for STRB/STR of #1 near STATUS_UPD and First PIN sites
print("\n=== STATUS_UPD 0x106a sites with nearby MOVS #0/#1 ===")
for o, rd in isolated_movw(0x106A, 12):
    flags = []
    for p in range(o - 0x100, o + 0x100, 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (0, 1):
            flags.append(f"0x{p:x}:MOVS#{hw&0xff}")
    print(f"0x{o:x}/va0x{va(o):x} near0/1: {', '.join(flags[:16])}")

# Reset IsSimVerifyCompleteSent log
reset_s = img.find(b"Reset IsSimVerifyCompleteSent :%d,Pin1Verified :%d")
if reset_s < 0:
    reset_s = img.find(b"Reset IsSimVerifyCompleteSent")
rs0, rtext = cstr_at(reset_s)
rhid, _ = log_id(rs0)
print(f"\n=== Reset Pin1Verified log @0x{rs0:x} id=0x{rhid:x}: {rtext[:120]} ===")
if 0 < rhid < 0x10000:
    for o, rd in isolated_movw(rhid, 8):
        print(f"\nRESET @0x{o:x} va=0x{va(o):x}")
        print(dump_window(o - 0x60, 0x100))

# Search more strings about verification done / pin ok
print("\n=== more verify-done strings ===")
for n in [
    b"PIN1 Verification",
    b"Pin1 Verified",
    b"pin1 verified",
    b"gPin1Verified",
    b"m_Pin1Verified",
    b"bPin1Verified",
    b"IsPin1Verified",
    b"SetPin1Verified",
    b"Pin1Verify",
    b"SIM_PIN_VERIFIED",
    b"PIN_VERIFIED",
    b"Verification is done",
    b"Verify Success",
    b"VERIFY SUCCESS",
    b"verify success",
    b"CHV1 OK",
    b"CHV OK",
    b"PIN OK",
]:
    j = img.find(n)
    if j >= 0:
        s0, text = cstr_at(j)
        hid, _ = log_id(s0)
        print(f"  0x{s0:x} id=0x{hid:x}: {text[:140]}")
