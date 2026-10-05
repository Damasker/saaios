#!/usr/bin/env python3
"""Full RO: every app_state READY(5)/PIN(2) store in SIM STATUS update region around 0x106a."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
SET_APP = 0x19916D2
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
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


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


def bcond_target(o):
    hw = u16(o)
    if (hw & 0xF000) != 0xD000 or (hw & 0x0F00) == 0x0E00 or (hw & 0x0F00) == 0x0F00:
        return None  # not bcond (SVC/AL reserved)
    cond = (hw >> 8) & 0xF
    imm8 = hw & 0xFF
    if imm8 >= 0x80:
        imm8 -= 0x100
    return o + 4 + (imm8 << 1), cond


COND = {
    0: "EQ", 1: "NE", 2: "CS", 3: "CC", 4: "MI", 5: "PL",
    6: "VS", 7: "VC", 8: "HI", 9: "LS", 10: "GE", 11: "LT",
    12: "GT", 13: "LE",
}


def dis(start, length, mark_set_app=True):
    o = start
    end = start + length
    lines = []
    while o < end:
        r = movw(o)
        t = movt(o)
        b = bl_target(o)
        bc = bcond_target(o)
        hw = u16(o)
        if b is not None and (hw & 0xF800) == 0xF000:
            tag = "  <<SET_APP" if b == SET_APP else ""
            lines.append(f"  {o:#010x} va={va(o):#010x}: BL->{b:#x}{tag}")
            o += 4
            continue
        if r:
            note = ""
            if r[0] in (0x106A, 0x106B, 0x7AB, 0xC6B, 0xC5B, 0x18E, 0xBDA):
                note = f"  ;LOG_{r[0]:#x}"
            lines.append(f"  {o:#010x} va={va(o):#010x}: MOVW r{r[1]},#{r[0]:#x}{note}")
            o += 4
            continue
        if t:
            lines.append(f"  {o:#010x}: MOVT r{t[1]},#{t[0]:#x}")
            o += 4
            continue
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            # B.W
            if (hw & 0xF800) == 0xF000 and (u16(o + 2) & 0xD000) == 0x9000:
                hw2 = u16(o + 2)
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
                lines.append(f"  {o:#010x}: B.W ->{tgt:#x}")
            o += 4
            continue
        if (hw & 0xFF00) == 0x2000:
            imm = hw & 0xFF
            tag = {0: "0", 1: "1/Present?", 2: "PIN", 3: "DISABLED/PUK?", 4: "SUBLOADED?", 5: "READY", 6: "?", 7: "?"}.get(imm, str(imm))
            lines.append(f"  {o:#010x} va={va(o):#010x}: MOVS r{(hw>>8)&7},#{imm}  ;<<{tag}")
            o += 2
            continue
        if (hw & 0xFF00) == 0x2800:
            lines.append(f"  {o:#010x}: CMP r{(hw>>8)&7},#{hw&0xff}")
            o += 2
            continue
        if bc:
            tgt, cond = bc
            lines.append(f"  {o:#010x}: B{COND.get(cond,'?')} ->{tgt:#x}")
            o += 2
            continue
        if (hw & 0xF800) == 0xE000:
            imm11 = hw & 0x7FF
            if imm11 >= 0x400:
                imm11 -= 0x800
            tgt = o + 4 + (imm11 << 1)
            lines.append(f"  {o:#010x}: B ->{tgt:#x}")
            o += 2
            continue
        if hw in (0xB5F0, 0xB570, 0xB580, 0xB5B0, 0xB510, 0xB5F8, 0xB5B8):
            lines.append(f"  {o:#010x} va={va(o):#010x}: PUSH <<fn?")
            o += 2
            continue
        if (hw & 0xFF00) == 0xBD00 or hw in (0xBD10, 0xBD70, 0xBDF0, 0xBD80):
            lines.append(f"  {o:#010x}: POP/BX LR")
            o += 2
            continue
        # LDRB T1 0111 1 imm5 Rn Rt
        if (hw & 0xF800) == 0x7800:
            lines.append(f"  {o:#010x}: LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]")
            o += 2
            continue
        if (hw & 0xF800) == 0x7000:
            lines.append(f"  {o:#010x}: STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]")
            o += 2
            continue
        o += 2
    return "\n".join(lines)


# 1) ALL SET_APP callers with preceding MOVS imm
print("=== ALL BL SET_APP with preceding MOVS imm (whole MAIN) ===")
by_imm = {2: [], 5: [], "other": []}
for o in range(MAIN_OFF, END - 4, 2):
    if bl_target(o) != SET_APP:
        continue
    imm = None
    for back in range(2, 20, 2):
        hw = u16(o - back)
        if (hw & 0xFF00) == 0x2000:
            imm = hw & 0xFF
            break
    rec = (o, imm, va(o))
    if imm == 2:
        by_imm[2].append(rec)
    elif imm == 5:
        by_imm[5].append(rec)
    else:
        by_imm["other"].append(rec)

print(f"PIN(2) count={len(by_imm[2])}")
for o, imm, v in by_imm[2]:
    print(f"  PIN  BL@{o:#x}/va{v:#x}")
print(f"READY(5) count={len(by_imm[5])}")
for o, imm, v in by_imm[5]:
    print(f"  READY BL@{o:#x}/va{v:#x}")
print(f"other count={len(by_imm['other'])}")
from collections import Counter
print("  imm hist:", Counter(i for _, i, _ in by_imm["other"]).most_common())

# 2) Find function containing STATUS_UPD — walk back from first READY writer 0x14fb5c6
ready_site = by_imm[5][0][0] if by_imm[5] else 0x14FB5C6
fn = ready_site
for p in range(ready_site, ready_site - 0x800, -2):
    if u16(p) in (0xB5F0, 0xB570, 0xB5F8, 0xB580, 0xB5B0, 0xB5B8):
        # check if this is a plausible fn start (not mid-function push)
        fn = p
print(f"\n=== STATUS_UPD fn guess start={fn:#x} va={va(fn):#x} (near READY writer) ===")

# Dump from earliest PIN writer cluster to past READY — use 0x14fb200..0x14fb700
print("\n=== FULL disasm STATUS cluster 0x14fb200..0x14fb700 ===")
print(dis(0x14FB200, 0x500))

# Also dump any OTHER READY sites' ±0x100 context
print("\n=== Context for EVERY READY(5) SET_APP site ===")
for o, imm, v in by_imm[5]:
    print(f"\n---- READY site {o:#x} ----")
    print(dis(o - 0x80, 0xC0))

print("\n=== Context for EVERY PIN(2) SET_APP site ===")
for o, imm, v in by_imm[2]:
    print(f"\n---- PIN site {o:#x} ----")
    print(dis(o - 0x80, 0xC0))

# 3) Search for CMP #3 (DISABLED) near any SET_APP READY or in STATUS cluster
print("\n=== CMP #3 / MOVS#3 near READY writers (±0x200) ===")
for o, imm, v in by_imm[5]:
    for p in range(o - 0x200, o + 0x40, 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2800 and (hw & 0xFF) == 3:
            print(f"  CMP #3 @{p:#x} near READY {o:#x}")
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 3:
            # only if followed soon by SET_APP or near
            print(f"  MOVS #3 @{p:#x} near READY {o:#x}")

# 4) All 0x106a real log sites — dump ±0x100 looking for SET_APP
print("\n=== All real 0x106a sites + SET_APP within ±0x200 ===")


def real_106a():
    hits = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != 0x106A:
            continue
        dense = False
        for p in range(o + 4, o + 32, 2):
            r2 = movw(p)
            if r2 and r2[0] == 0x106B:
                dense = True
                break
        if dense:
            continue
        ctx = None
        for p in range(o - 24, o + 28, 2):
            t = movt(p)
            if t and 0x4000 <= t[0] <= 0x45FF:
                ctx = t[0]
                break
        if ctx is None:
            continue
        hits.append((o, ctx))
    return hits


for o, ctx in real_106a():
    sets = []
    for p in range(o - 0x100, o + 0x200, 2):
        if bl_target(p) == SET_APP:
            imm = None
            for back in range(2, 16, 2):
                hw = u16(p - back)
                if (hw & 0xFF00) == 0x2000:
                    imm = hw & 0xFF
                    break
            sets.append(f"SET_APP@{p:#x} imm={imm}")
    print(f"0x106a @{o:#x}/va{va(o):#x} ctx={ctx:#x} nearby: {sets}")
