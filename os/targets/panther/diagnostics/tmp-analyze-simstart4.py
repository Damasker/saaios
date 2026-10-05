#!/usr/bin/env python3
"""Deep RO: SIM START / PIN_STATUS IND builders vs Present=2."""
import struct
from pathlib import Path
from collections import defaultdict

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
PIN1V = (0x1F04576, 0x1F046A4)
PRESENT_STRB = 0x14FB380  # STATUS copy to +0xBF6
TARGETS = {FN_A, FN_B, WRAP_SIM, SET_APP, 0x14C380E, 0x14C3986, 0x14F6A14, 0x14F9578}


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def off_from_va(v):
    return MAIN_OFF + (v - VA_BASE)


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


def find_str(needle: bytes):
    out = []
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        out.append((a, img[a:b].decode("ascii", "replace")))
        i = j + 1
    return out


def real_log_sites(log_id):
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
        hits.append((o, ctx, r[1]))
    return hits


def scan_window(lo, hi, label=""):
    hits = []
    for o in range(lo, min(hi, END - 4), 2):
        t = bl_target(o)
        if t in TARGETS:
            hits.append((o, f"BL->{hex(t)}"))
        r = movw(o)
        if r and r[0] == 2:
            # look ahead for STRB Present patterns / SET_APP nearby
            for p in range(o, min(o + 32, END - 4), 2):
                bt = bl_target(p)
                if bt == SET_APP:
                    hits.append((o, f"MOVS#2..BL SET_APP@{hex(p)}"))
                hw = u16(p)
                # STRB rt,[rn,#imm] T1: 0x7000 | ...
                if (hw & 0xF800) == 0x7000:
                    imm5 = (hw >> 6) & 0x1F
                    rt = hw & 7
                    rn = (hw >> 3) & 7
                    if imm5 in (0, 1, 20) or True:
                        # annotate only if previous movs #2 in r0-ish
                        pass
        # STRB.W rt,[rn,#imm12] encoding T3: F88x
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) == 0xF880:
            imm12 = hw2 & 0xFFF
            rt = (hw2 >> 12) & 0xF
            if imm12 in (0xBF6, 0xBF5, 0xBF4, 20, 0x18E):
                hits.append((o, f"STRB.W rt={rt} imm=#{hex(imm12)}"))
        if (hw & 0xFFF0) == 0xF8C0 and (hw2 & 0x0FFF) in (0xBF6, 0xBF5, 0xBF4):
            hits.append((o, f"STR.W imm=#{hex(hw2 & 0xFFF)}"))
        # MOVS rd,#imm8
        if (hw & 0xFF00) == 0x2000:
            imm8 = hw & 0xFF
            rd = (hw >> 8) & 7
            if imm8 in (2, 3, 5) and rd == 0:
                # check next few for STRB to [r?,#?]
                for p in range(o + 2, min(o + 16, END - 2), 2):
                    h = u16(p)
                    if (h & 0xF800) == 0x7000:
                        imm5 = (h >> 6) & 0x1F
                        hits.append((o, f"MOVS r0,#{imm8}; STRB [r{(h>>3)&7},#{imm5}] @{hex(p)}"))
                        break
                    if (h & 0xFFF0) == 0xF880 and (u16(p + 2) & 0xFFF) in (
                        0xBF6,
                        0xBF5,
                        0xBF4,
                        20,
                    ):
                        hits.append(
                            (
                                o,
                                f"MOVS r0,#{imm8}; STRB.W #{hex(u16(p+2)&0xFFF)} @{hex(p)}",
                            )
                        )
                        break
    return hits


def fn_start(site):
    for p in range(site, max(MAIN_OFF, site - 0x1000), -2):
        hw = u16(p)
        if (hw & 0xFFF0) == 0xE92D:  # PUSH.W
            return p
        if hw == 0xB570 or hw == 0xB5F0 or hw == 0xB580 or hw == 0xB5B0:
            return p
    return None


def bl_callers(target):
    out = []
    for o in range(MAIN_OFF, END - 4, 2):
        t = bl_target(o)
        if t == target:
            out.append(o)
    return out


def literal_pool_xrefs(str_off):
    """Find LDR Rn,[PC,#imm] loading pointer to str, or .word VA in pools."""
    target_va = va(str_off)
    needle = struct.pack("<I", target_va)
    hits = []
    i = MAIN_OFF
    while True:
        j = img.find(needle, i, END)
        if j < 0:
            break
        if j % 2 == 0 or True:
            hits.append(j)
        i = j + 1
    return hits


def decode_near(o, before=0x40, after=0x80):
    lines = []
    for p in range(o - before, o + after, 2):
        if p < MAIN_OFF or p >= END - 4:
            continue
        r = movw(p)
        t = movt(p)
        bt = bl_target(p)
        note = ""
        if r:
            note = f";MOVW r{r[1]},#{hex(r[0])}"
            if r[0] == 0x105A:
                note += " ;LOG_START?"
        if t:
            note = f";MOVT r{t[1]},#{hex(t[0])}"
        if bt is not None:
            note = f";BL->{hex(bt)}"
            if bt in TARGETS:
                note += " ***TARGET***"
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000:
            note += f" ;MOVS r{(hw>>8)&7},#{hw&0xFF}"
        if (hw & 0xFFF0) == 0xF880:
            note += f" ;STRB.W #{hex(u16(p+2)&0xFFF)}"
        if note:
            lines.append(f"  {hex(p)}: {hw:04x} {note}")
    return lines


print("=== Message name strings + literal VA pools ===")
for name in (
    b"SIM_START_IND",
    b"SIM_PIN_STATUS_IND",
    b"SIM_PRESENT_IND",
    b"PrepareSimStartIndParameters",
    b"SIM START IND",
    b"PIN_STATUS_IND",
):
    for off, s in find_str(name):
        print(f"  @{hex(off)} VA={hex(va(off))}: {s[:80]}")
        pools = literal_pool_xrefs(off)
        print(f"    litpool .word count={len(pools)} first={list(map(hex, pools[:8]))}")

print("\n=== Nearby strings around SIM_PIN_STATUS / SIM_START ===")
for needle in (b"USIM ==> SIM_PIN_STATUS_IND", b"USIM ==> SIM_START_IND"):
    for off, s in find_str(needle):
        # dump surrounding string table
        region = img[off - 0x200 : off + 0x200]
        # extract printable runs
        cur = b""
        runs = []
        base = off - 0x200
        for i, b in enumerate(region):
            if 32 <= b < 127:
                cur += bytes([b])
            else:
                if len(cur) >= 8:
                    runs.append((base + i - len(cur), cur.decode()))
                cur = b""
        print(f"\n  around {s}:")
        for ro, rs in runs:
            mark = " <<" if needle.decode() in rs else ""
            print(f"    @{hex(ro)}: {rs[:90]}{mark}")

print("\n=== Log 0x105a sites (SIM START) — function windows ===")
sites_105a = real_log_sites(0x105A)
print(f"count={len(sites_105a)}")
usim_sites = [s for s in sites_105a if s[1] == 0x4107]
print("USIM ctx 0x4107:", [(hex(a), hex(c)) for a, c, _ in usim_sites])

# Find likely PIN_STATUS log id: strings near PIN_STATUS often share packing
# Search for "PIN_STATUS" in log format strings
print("\n=== Log-format strings mentioning PIN_STATUS / START IND ===")
for needle in (
    b"PIN_STATUS",
    b"SIM START",
    b"SimStart",
    b"sim_start",
    b"PinStatus",
    b"pin_status",
    b"SIM_PIN",
):
    for off, s in find_str(needle):
        if "USIM" in s or "SIM" in s or "Pin" in s or "PIN" in s:
            if off >= MAIN_OFF and off < END:
                continue  # code region false positive rare
            print(f"  @{hex(off)}: {s[:100]}")

# Analyze each USIM 0x105a function for Present-related
print("\n=== USIM 0x105a functions: BL targets + Present stores ===")
for site, ctx, reg in usim_sites:
    fs = fn_start(site)
    print(f"\n-- site {hex(site)} ctx={hex(ctx)} fn_start={hex(fs) if fs else None}")
    if not fs:
        continue
    # scan whole function until next PUSH.W or 0x400 bytes
    end = min(fs + 0x800, END)
    for p in range(fs + 4, min(fs + 0x800, END - 2), 2):
        if (u16(p) & 0xFFF0) == 0xE92D and p - fs > 0x40:
            end = p
            break
    hits = scan_window(fs, end)
    print(f"  fn_span {hex(fs)}..{hex(end)} hits={hits}")
    # collect interesting BLs
    bls = defaultdict(int)
    for p in range(fs, end - 4, 2):
        t = bl_target(p)
        if t:
            bls[t] += 1
    interesting = {
        t: n
        for t, n in bls.items()
        if t in TARGETS
        or t
        in (
            0x20E184E,
            0x189BCF6,
            0x19A2B54,
            0x18D2258,
            0x18A1BE8,
            0x18C5DB6,
            0x197490E,
            0x179CBE2,
        )
    }
    print(f"  interesting BLs: { {hex(k):v for k,v in interesting.items()} }")
    # any STRB.W #0xBF6 / #0xBF5 in fn?
    for p in range(fs, end - 4, 2):
        if (u16(p) & 0xFFF0) == 0xF880:
            imm = u16(p + 2) & 0xFFF
            if imm in (0xBF4, 0xBF5, 0xBF6, 0x18E):
                print(f"  STRB.W #{hex(imm)} @ {hex(p)}")

# What is 0x20e184e? Check nearby strings / xrefs
print("\n=== Callee 0x20e184e (seen with MOVS #2/#5 near Prepare/START) ===")
cal = 0x20E184E
print(f"fn_start={hex(fn_start(cal) or 0)}")
# first 0x60 of callee
print("\n".join(decode_near(cal, 0, 0x60)))

print("\n=== Callee 0x189bcf6 (log helper near 0x105a) ===")
print("\n".join(decode_near(0x189BCF6, 0, 0x40)))

print("\n=== PrepareSimStartIndParameters log 0x5b5 — which call Present? ===")
sites_5b5 = real_log_sites(0x5B5)
print(f"0x5b5 sites: {[(hex(a),hex(c)) for a,c,_ in sites_5b5]}")
for site, ctx, _ in sites_5b5[:6]:
    fs = fn_start(site)
    if not fs:
        continue
    end = min(fs + 0x600, END)
    for p in range(fs + 4, min(fs + 0x600, END - 2), 2):
        if (u16(p) & 0xFFF0) == 0xE92D and p - fs > 0x40:
            end = p
            break
    hits = scan_window(fs, end)
    bl_hit = [(hex(o), h) for o, h in hits if "TARGET" in h or "FN" in h or "SET_APP" in h or "BL->" in h]
    print(f"  {hex(site)} fn={hex(fs)}..{hex(end)} hits={hits[:12]}")

# Cross: do any 0x105a / 0x5b5 functions BL to FN_A/FN_B?
print("\n=== Direct BL FN_A/FN_B from START-related windows (±0x200 of each site) ===")
for site, ctx, _ in usim_sites + sites_5b5:
    hits = scan_window(site - 0x200, site + 0x400)
    if hits:
        print(f"  {hex(site)}: {hits}")

# Boot-order strings
print("\n=== Boot / order strings near SIM init ===")
for needle in (
    b"SIM_INIT",
    b"SimInit",
    b"USIM_INIT",
    b"SIM POWER",
    b"CARD_STATUS",
    b"ATR",
    b"SIM_ABSENT",
    b"SIM_PRESENT",
    b"BEFORE SIM START",
    b"After SIM START",
    b"SendSimStart",
    b"Send SIM START",
    b"NS_SIM",
    b"sitSend",
    b"SIT_SEND",
):
    hits = find_str(needle)
    for off, s in hits[:3]:
        print(f"  @{hex(off)}: {s[:90]}")

# Find PIN_STATUS log id by searching movw near string usage via litpool
print("\n=== Follow litpool for SIM_PIN_STATUS_IND / SIM_START_IND ===")
for needle in (b"USIM ==> SIM_PIN_STATUS_IND", b"USIM ==> SIM_START_IND"):
    offs = find_str(needle)
    if not offs:
        continue
    soff = offs[0][0]
    pools = literal_pool_xrefs(soff)
    print(f"{needle.decode()}: pools={len(pools)}")
    for po in pools[:12]:
        # find LDR that might use this pool entry: PC-relative
        # Thumb LDR Rt,[PC,#imm] : 0x4800 | (rt<<8) | imm8 ; addr = align4(pc+4)+imm*4
        # Search backwards 0x400 for code that could load this
        for code in range(max(MAIN_OFF, po - 0x400), po, 2):
            hw = u16(code)
            if (hw & 0xF800) == 0x4800:
                imm8 = hw & 0xFF
                pc = code + 4
                base = pc & ~3
                dest = base + imm8 * 4
                if dest == po:
                    print(f"  LDR lit @{hex(code)} -> pool {hex(po)}")
                    print("\n".join(decode_near(code, 0x20, 0x40)))
