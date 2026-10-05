#!/usr/bin/env python3
"""Xref 0x105a SIM START IND; find PIN_STATUS_IND handlers; Present=2 links."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()

FN_A = 0x14F692C
FN_B = 0x14F9108
WRAP_SIM = 0x14F6D02
SET_APP = 0x19916D2
PIN1V_SITES = (0x1F04576, 0x1F046A4)


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


def real_sites(log_id):
    hits = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != log_id:
            continue
        dense = False
        for p in range(o + 4, o + 28, 2):
            r2 = movw(p)
            if r2 and abs(r2[0] - log_id) <= 2:
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


def fn_start(site):
    for p in range(site, site - 0x800, -2):
        hw = u16(p)
        if (hw & 0xFFF0) == 0xE92D:
            return p
        if hw in (0xB5F0, 0xB5F8, 0xB570, 0xB5B0, 0xB580, 0xB5B8):
            return p
    return site - 0x100


def scan_fn_for_targets(fn, size=0x600):
    found = []
    stores = []
    for o in range(fn, min(fn + size, END - 4), 2):
        b = bl_target(o)
        if b in (FN_A, FN_B, WRAP_SIM, SET_APP) or b in PIN1V_SITES:
            found.append((o, b))
        if b and abs(b - 0x1F04576) < 0x100:
            found.append((o, b))
        hw = u16(o)
        if (hw & 0xFFF0) == 0xF880:
            imm = u16(o + 2) & 0xFFF
            if imm in (0xBF4, 0xBF5, 0xBF6):
                stores.append((o, imm))
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            for q in range(o + 2, o + 12, 2):
                h = u16(q)
                if (h & 0xFFF0) == 0xF880 and (u16(q + 2) & 0xFFF) == 0:
                    stores.append((o, "Present=2"))
                if (h & 0xF800) == 0x7000 and ((h >> 6) & 0x1F) == 0:
                    stores.append((o, "STRB#0"))
                if (h & 0xF800) == 0x7000 and ((h >> 6) & 0x1F) == 20:
                    stores.append((o, "STRB#20 Pin1V?"))
    return found, stores


def dump(start, end):
    o = start
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r = movw(o)
            if r:
                note = ""
                if r[0] in (0x105A, 0x107D, 0x7AB, 0x18E, 0x106A, 0xC6B, 0x347):
                    note = f" ;LOG_{r[0]:#x}"
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                t = bl_target(o)
                tag = ""
                if t == FN_A:
                    tag = " <<FN_A"
                elif t == FN_B:
                    tag = " <<FN_B"
                elif t == WRAP_SIM:
                    tag = " <<WRAP_SIM"
                elif t == SET_APP:
                    tag = " <<SET_APP"
                extra = f" ;BL->{t:#x}{tag}"
            elif (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W [r{hw&0xf},#{hw2&0xfff:#x}]"
            lines.append(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS #{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP #{hw&0xff}"
        elif (hw & 0xF800) == 0x7000:
            extra = f" ;STRB [r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        lines.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(lines)


print("=== Real MOVW #0x105a (SIM START IND) ===")
sites_105a = real_sites(0x105A)
for o, ctx in sites_105a:
    print(f"  @{o:#x}/va{va(o):#x} ctx={ctx:#x}")

print("\n=== Analyze each 0x105a function ===")
for o, ctx in sites_105a:
    fn = fn_start(o)
    bls, stores = scan_fn_for_targets(fn, 0x800)
    print(f"\n-- site {o:#x} fn≈{fn:#x} --")
    print(f"  BLs to targets: {[(hex(a), hex(b)) for a,b in bls]}")
    print(f"  stores: {stores[:20]}")
    # also list unique BL targets in fn (first 20 interesting)
    all_bls = []
    for p in range(fn, fn + 0x400, 2):
        b = bl_target(p)
        if b:
            all_bls.append(b)
    # check if any BL into Present builders region
    near = [b for b in all_bls if 0x14F6000 <= b <= 0x14FB800 or 0x14C3700 <= b <= 0x14C3A00]
    print(f"  BLs into Present/STATUS region: {[hex(x) for x in sorted(set(near))][:15]}")

# PIN_STATUS: search strings and message dispatch
print("\n=== SIM_PIN_STATUS_IND related ===")
# symbol at 0x1035939 — find pointer xrefs (ADR/MOVW+MOVT to that VA)
sym_va = VA_BASE + (0x1035939 - MAIN_OFF)
print(f"  sym VA for 'USIM ==> SIM_PIN_STATUS_IND' = {sym_va:#x}")
# Also find log formats mentioning Pin1Status
for needle in [
    b"Pin1Status=%d",
    b"PIN_STATUS_IND",
    b"Pin Status Ind",
    b"sitRx.*Pin",
    b"Rx SIM_PIN",
    b"SIM_PIN_STATUS",
    b"HandlePinStatus",
    b"PinStatusInd",
]:
    idx = 0
    while True:
        j = img.find(needle if b".*" not in needle else b"PIN_STATUS", idx)
        if j < 0:
            break
        if b".*" in needle:
            idx = j + 1
            continue
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        s = img[a:b].decode(errors="replace")
        if "PIN" in s.upper() or "Pin" in s:
            print(f"  @{a:#x}: {s[:120]}")
        idx = j + 1
        if idx > j + 500000:
            break

# Find SIT handlers that log SIM START or receive USIM start
print("\n=== SIT strings for start/pin status ===")
for needle in [
    b"[SIT_0_SIM] Rx",
    b"[SIT_0_SIM] SIM START",
    b"SimStart",
    b"sim_start",
    b"NS_SIM_START",
    b"USIM_START",
    b"SIM_START_IND",
    b"SIM_PIN_STATUS_IND",
]:
    idx = 0
    n = 0
    while n < 8:
        j = img.find(needle, idx)
        if j < 0:
            break
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        print(f"  @{a:#x}: {img[a:b].decode(errors='replace')[:130]}")
        idx = j + 1
        n += 1
