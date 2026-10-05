#!/usr/bin/env python3
"""RO: NAS start without GET_APP in {1,4,5}; SET_APP#1 vs Pin1Verified clear."""
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()

GET_APP = 0x18EC8C0
START_NET = 0x18E831A
SET1_FN = 0x146A99C
SET1_STORE = 0x146AABA
SET_APP = 0x19916D2
# Pin1Verified obj+20 STRB sites known: FirstPIN ~0x18e / verify path
# Prior: sole Pin1Verified=1 writers via VerifyPin; clear via reset 0xbda string


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def bl_target(o):
    if o + 4 > len(IMG):
        return None
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


def find_all(needle, limit=40):
    out, off = [], 0
    while len(out) < limit:
        i = IMG.find(needle, off)
        if i < 0:
            break
        out.append(i)
        off = i + 1
    return out


print("=== emergency / limited / CSFB / attach strings ===")
needles = [
    b"Emergency",
    b"EMERGENCY",
    b"emergency",
    b"CSFB",
    b"Limited",
    b"LIMITED",
    b"limited service",
    b"START_NETWORK",
    b"StartNetwork",
    b"NS_START",
    b"ATTACH",
    b"AttachReq",
    b"IMSI attach",
    b"Emergency attach",
    b"Emergency call",
    b"EmergencyMode",
    b"SIM is not ready",
    b"Not camped",
]
for n in needles:
    hits = find_all(n, 12)
    if not hits:
        continue
    print(f"-- {n!r} count~{len(hits)} first={hex(hits[0])}")
    for h in hits[:6]:
        ctx = IMG[h : h + 72].split(b"\x00", 1)[0]
        print(f"   {hex(h)} {ctx[:64]!r}")

# Scan for BL GET_APP in NET/NAS-ish ranges; collect following CMP immediates
print("\n=== BL GET_APP then CMP #imm (NET/SIM/NAS windows) ===")
ranges = [
    (0x18E0000, 0x1920000),
    (0x14C0000, 0x1520000),
    (0x1A20000, 0x1A40000),
    (0x2600000, 0x2680000),
]
sites = []
for lo, hi in ranges:
    o = lo
    while o + 4 < hi:
        t = bl_target(o)
        if t == GET_APP:
            cmps = []
            for j in range(0, 0x60, 2):
                p = o + 4 + j
                if p + 2 > len(IMG):
                    break
                hw = u16(p)
                # CMP rn,#imm8  (0x28xx / 0x29xx)
                if (hw & 0xFF00) in (0x2800, 0x2900, 0x2A00, 0x2B00):
                    cmps.append(hw & 0xFF)
                # CMP.W encoding F5Bx / F1Bx rough skip if complex
            sites.append((o, cmps))
        o += 2

# Classify: sites whose CMP set does NOT require subset of {1,4,5} only
# i.e. accepts 2 (PIN) or has empty/no CMP in window
print(f"BL GET_APP sites in windows: {len(sites)}")
accept_pin = []
no_cmp = []
only_145 = []
other = []
for o, cmps in sites:
    s = set(cmps)
    if not cmps:
        no_cmp.append(o)
    elif 2 in s and not s.issubset({1, 2, 4, 5}):
        accept_pin.append((o, sorted(s)))
    elif s.issubset({1, 4, 5}) and s:
        only_145.append((o, sorted(s)))
    elif 2 in s:
        accept_pin.append((o, sorted(s)))
    else:
        other.append((o, sorted(s)))

print(f" only {{1,4,5}} sites: {len(only_145)}")
for o, s in only_145[:8]:
    print(f"  {hex(o)} cmps={s}")
print(f" sites with CMP#2 (PIN) in window: {len(accept_pin)}")
for o, s in accept_pin[:20]:
    print(f"  {hex(o)} cmps={s}")
print(f" no CMP in +0x60: {len(no_cmp)}")
for o in no_cmp[:15]:
    print(f"  {hex(o)}")
print(f" other CMP sets: {len(other)}")
for o, s in other[:15]:
    print(f"  {hex(o)} cmps={s}")

# START_NETWORK region dump confirmation
print("\n=== START_NETWORK gate dump around 0x18e831a ===")
for o in range(0x18E8300, 0x18E8380, 2):
    t = bl_target(o)
    hw = u16(o)
    extra = f" BL->{hex(t)}" if t else ""
    if (hw & 0xFF00) in (0x2800, 0x2900) or t:
        print(f"  {hex(o)}: {hw:04x}{extra}")

# SET_APP#1 callers
print("\n=== BL -> SET1_FN / SET1_STORE ===")
callers_fn, callers_store = [], []
for o in range(0x1400000, 0x1B00000, 2):
    t = bl_target(o)
    if t == SET1_FN:
        callers_fn.append(o)
    if t == SET1_STORE:
        callers_store.append(o)
print(f"BL SET1_FN {hex(SET1_FN)}: {len(callers_fn)} {[hex(x) for x in callers_fn[:10]]}")
print(f"BL SET1_STORE {hex(SET1_STORE)}: {len(callers_store)} {[hex(x) for x in callers_store[:10]]}")

# Inside SET1_FN: does it clear Pin1Verified (+20 / +0xBF5)?
print("\n=== SET1_FN body scan for Pin1Verified clear / +0xBF5 / MOVS #0 stores ===")


def dump(lo, hi, label):
    print(f"-- {label} {hex(lo)}..{hex(hi)} --")
    o = lo
    while o < hi:
        hw = u16(o)
        t = bl_target(o)
        # STRB rt,[rn,#imm5] 0x70xx / STRB.W
        note = ""
        if t:
            note = f" BL->{hex(t)}"
        # LDRB.W / STRB.W with imm12 look for BF5/BF4/14(=+20)
        if hw == 0xF89F or (hw & 0xFFF0) == 0xF880:  # rough
            note += " STRB.W?"
        if (hw & 0xFF00) == 0x7000:
            imm5 = (hw >> 6) & 0x1F
            note += f" STRB imm5={imm5}"
        if (hw & 0xFF00) == 0x7800:
            imm5 = (hw >> 6) & 0x1F
            note += f" LDRB imm5={imm5}"
        # MOVW / MOVT for 0xBF5
        if note or (hw & 0xF800) in (0xF000, 0xF800) and t:
            if note or t:
                print(f"  {hex(o)}: {hw:04x}{note}")
                if t:
                    o += 2  # skip 2nd half already printed via bl
        o += 2


# Dump SET1 function ~0x200 bytes
dump(SET1_FN, SET1_FN + 0x120, "SET1_FN")

# Search STRB.W [rn,#0xBF5] encodings near SET1 and near Pin1Verified writers
# Thumb2 STRB.W Rt,[Rn,#imm12] = F88x / F80x family: F880|Rn , imm
print("\n=== STRB.W #+0xBF5 / #+0x14 near SET1 callers ===")


def find_strb_imm12(imm, lo, hi):
    hits = []
    o = lo
    while o + 4 <= hi:
        hw, hw2 = u16(o), u16(o + 2)
        # STRB.W Rt,[Rn,#imm12]  T3: 1111 1000 1000 nnnn  tttt 1111 iiii iiii iiii
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0x0F00) == 0x0F00:
            imm12 = hw2 & 0xFFF
            if imm12 == imm:
                hits.append(o)
        o += 2
    return hits


for imm, name in [(0xBF5, "+0xBF5 Pin1Verified STATUS"), (0x14, "+0x14 obj Pin1Verified"), (0xBF4, "+0xBF4 app"), (0xBF6, "+0xBF6 Present")]:
    # whole interesting region
    hits = find_strb_imm12(imm, 0x1450000, 0x1470000)
    hits += find_strb_imm12(imm, 0x18E0000, 0x1900000)
    hits += find_strb_imm12(imm, 0x1A20000, 0x1A30000)
    print(f"  STRB.W {name}: {len(hits)} {[hex(h) for h in hits[:12]]}")

# Does SET1_FN or its callers BL anything that clears Pin1Verified?
# Known clear: string 'Reset IsSimVerifyCompleteSent' / store Pin1Verified=0
print("\n=== Reset Pin1Verified string + nearby STRB ===")
for s in [b"Reset IsSimVerifyCompleteSent", b"Pin1Verified", b"IsSimVerifyComplete"]:
    for h in find_all(s, 5):
        print(f"  {s!r} @{hex(h)}")

# Caller context dumps
print("\n=== SET1 caller contexts (precede 0x40) ===")
for c in callers_fn[:4]:
    print(f" caller {hex(c)}")
    for o in range(c - 0x40, c + 0x20, 2):
        if o < 0:
            continue
        hw = u16(o)
        t = bl_target(o)
        if t or (hw & 0xFF00) in (0x2000, 0x2100, 0x2800) or (hw & 0xF800) == 0xF000:
            extra = f" BL->{hex(t)}" if t else ""
            if (hw & 0xFF00) == 0x2000:
                extra += f" MOVS r0,#{hw & 0xFF}"
            if (hw & 0xFF00) == 0x2800:
                extra += f" CMP r0,#{hw & 0xFF}"
            print(f"   {hex(o)}: {hw:04x}{extra}")

print("\nDONE")
