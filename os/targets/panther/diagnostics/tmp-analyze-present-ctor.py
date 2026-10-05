#!/usr/bin/env python3
"""PresentObj ctor/INSERT + exhaustive Present=2 stores."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, END, VA = 0x16C10, 0x16C10 + 0x05917ACC, 0x40010000
FN_A, GETOBJ = 0x14F692C, 0x20EA040


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def bl(o):
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


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def find(n: bytes):
    out, i = [], 0
    while True:
        j = img.find(n, i)
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


def cstr_va(v, n=100):
    o = MAIN + (v - VA)
    if not (0 <= o < len(img)):
        return None
    b = o
    while b < o + n and 32 <= img[b] < 127:
        b += 1
    return img[o:b].decode() if b > o else None


def dump(start, end):
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t, b = movw(o), movt(o), bl(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b:
                extra = f" ;BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                rd = ((hw >> 7) & 1) << 3 | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2


# --- 1) FN_A getobj signature: r1/r2/r3 immediates before BL ---
print("=== FN_A alloc args (r1/r2/r3) ===")
dump(0x14F692C, 0x14F6960)
# strings at those VAs if they look like names
for lo, hi in [(0x636C, 0x410C), (0x18EE, 0x410C), (0x205D, 0x410C)]:
    pass
# From dump: MOVW r2,#0x18ee MOVT r2,#0x410c → VA 0x410c18ee
# MOVW r1,#0x636c — need MOVT for r1
print("try resolve:")
for o in range(0x14F6930, 0x14F6960, 2):
    r = movw(o)
    if not r:
        continue
    for q in range(o - 16, o + 24, 2):
        t = movt(q)
        if t and t[1] == r[1]:
            v = (t[0] << 16) | r[0]
            s = cstr_va(v)
            print(f"  {hex(o)} r{r[1]} VA={hex(v)} {s}")

# --- 2) Same getobj signature elsewhere: MOVW matching 0x636c / 0x18ee / 0x205d near BL getobj ---
print("\n=== getobj calls with FN_A-like imm nearby ===")
# Scan for MOVW #0x636c then nearby BL getobj
hits = []
for o in range(MAIN, END - 4, 2):
    r = movw(o)
    if not r or r[0] != 0x636C:
        continue
    # window for BL getobj
    for p in range(o, min(o + 0x40, END - 4), 2):
        if bl(p) == GETOBJ:
            hits.append((o, p))
            break
print(f"n={len(hits)}")
for a, b in hits[:30]:
    print(f"  imm@{hex(a)} getobj@{hex(b)}")

# --- 3) Exhaustive Present=2: MOVS rt,#2 then STRB rt,[rn,#0] (.W or T16) whole MAIN ---
print("\n=== Exhaustive MOVS#2 + STRB [*,#0] ===")
cands = []
o = MAIN
while o < END - 8:
    hw = u16(o)
    # MOVS rt,#2
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
        rt = (hw >> 8) & 7
        for p in range(o + 2, min(o + 16, END - 4), 2):
            h = u16(p)
            # T16 STRB rt,[rn,#0]
            if (h & 0xF800) == 0x7000 and ((h >> 6) & 0x1F) == 0 and (h & 7) == rt:
                cands.append((o, p, "t16", (h >> 3) & 7, rt))
                break
            # STRB.W rt,[rn,#0]
            if (h & 0xF800) in (0xE800, 0xF000, 0xF800):
                h2 = u16(p + 2)
                if (h & 0xFFF0) == 0xF880 and (h2 & 0xFFF) == 0 and ((h2 >> 12) & 0xF) == rt:
                    cands.append((o, p, "w", h & 0xF, rt))
                    break
                # also IT EQ MOVS#2 STRB.W pattern where rt in high
                if (h & 0xFFF0) == 0xF880 and (h2 & 0xFFF) == 0:
                    # rt may differ if MOVS was r0 and STRB uses r0 encoded in hw2
                    if ((h2 >> 12) & 0xF) == 0 and rt == 0:
                        cands.append((o, p, "w0", h & 0xF, 0))
                        break
    o += 2

print(f"total cands={len(cands)}")
# Classify known
known = {0x14F6A14, 0x14F9578}
for mov, st, kind, rn, rt in cands:
    tag = "KNOWN" if mov in known or st in (0x14F6A16, 0x14F957C) else "NEW"
    # context: nearby log imm / BL FN_A
    logs = []
    for p in range(mov - 0x40, mov + 0x20, 2):
        r = movw(p)
        if r and r[0] in (0x106A, 0x1068, 0x7AB, 0x18E, 0x105A, 0x5B5, 0xC6B):
            logs.append(hex(r[0]))
        if bl(p) == FN_A:
            logs.append("FN_A")
    if tag == "NEW" or mov in known:
        print(f"  {tag} {hex(mov)}->{hex(st)} {kind} [r{rn},#0] logs={logs}")

# Print ALL including filtering noise: only in 0x14xxxxxx-0x1affffff SIM-ish
print("\n=== NEW in 0x14c0000..0x1a80000 ===")
for mov, st, kind, rn, rt in cands:
    if not (0x14C0000 <= mov <= 0x1A80000):
        continue
    if mov in known or st in (0x14F6A16, 0x14F957C):
        continue
    print(f"  {hex(mov)}->{hex(st)} {kind} [r{rn},#0]")

print("\nDONE phase1")
