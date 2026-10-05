#!/usr/bin/env python3
"""Find log-id for No-CDMA InitRapMap; TCS CDMA dump; RRM rat map build."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


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


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, b, t = movw(o), bl(o), movt(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" ;BL->{hex(b)}"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# Shannon registration often: MOVW r1, #offset_into_pool; MOVT r1, #pool_hi; MOVW r0, #id; BL reg
# For strings in 0x44a3xxx, offset from a base - try find MOVW of low 16 of FILE offset
# as r1 with nearby BL 0x20d1afa (seen in PinSkip reg)

j = img.find(b"No CDMA in InitRapMap!")
# Try file-offset low as registration r1
off_lo = j & 0xFFFF
off_hi = (j >> 16) & 0xFFFF
print(f"str file_off={hex(j)} lo={hex(off_lo)} hi={hex(off_hi)}")

# Search MOVW r1,#off_lo with MOVT matching - or just MOVW r1,#off_lo near BL 0x20d1afa
hits = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == off_lo and r[1] == 1:
        # check nearby BL 0x20d1afa or 0x20e943c within ±0x20
        for p in range(max(0, o - 0x20), min(len(img) - 4, o + 0x30), 2):
            t = bl(p)
            if t in (0x20D1AFA, 0x20E943C, 0x20DED94, 0x217039C):
                hits.append((o, p, t))
                break
    o += 2
print(f"reg-like MOVW r1,#{hex(off_lo)}: {len(hits)}")
for o, p, t in hits[:5]:
    dump(o - 0x30, o + 0x20, f"reg@{hex(o)}")

# Alternative: VA low of string as r1
sva = VA + (j - MAIN)
vlo, vhi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
print(f"\nVA={hex(sva)} vlo={hex(vlo)}")
hits = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == vlo:
        t = movt(o + 4)
        ok = t and t[0] == vhi and t[1] == r[1]
        t2 = movt(o - 4) if o >= 4 else None
        ok2 = t2 and t2[0] == vhi and t2[1] == r[1]
        if ok or ok2:
            hits.append(o)
    o += 2
print(f"VA MOVW hits: {len(hits)}")

# Broader: find "No CDMA in InitRapMap" by searching for unique 4-byte substring
# as immediate? Unlikely.

# Find QM_MM_INIT_REQ_Handler code via C++ mangled / unique log cluster
# Search for MOVW of "SupportedRatMap(0x%X)" file offset similarly
j2 = img.find(b"@QM_MM_INIT_REQ_Handler: SupportedRatMap(0x%X)")
print(f"\nQM INIT SuppRat fmt file={hex(j2)}")
# Look at bytes BEFORE the string - sometimes there's a log header with id
print("bytes before InitRapMap NoCDMA:", img[j - 16 : j].hex())
print("bytes before SuppRat fmt:", img[j2 - 16 : j2].hex())

# Shannon TOC log: often uint16 id before string in a separate table
# Search for word arrays where consecutive entries point near these strings
# Try: find 0x44a3360-related by scanning for ADR.W PC-relative

# PC-relative ADR.W T3: ADDW rn, pc, #imm / SUBW
# Or LDR rn, [pc, #imm] then that literal pool has VA

# Scan for literal pool words == sva near code that might be QM handler
# Search entire image for sva as little-endian - already 0 ptrs.
# Maybe strings are referenced by index into a string table, not VA.

# === Approach: find "GmcM_CommSendMmInitReq" as debug name in .rodata next to function ptr ===
j = img.find(b"GmcM_CommSendMmInitReq")
print(f"\nGmcM name @{hex(j)} neighbors as words:")
for i in range(-8, 8):
    off = j + i * 4
    if 0 <= off < len(img) - 4:
        v = u32(off)
        mark = ""
        if 0x41000000 <= v <= 0x46000000:
            fo = v - VA + MAIN
            mark = f" ->file {hex(fo)}"
        print(f"  [{i:+d}] {hex(off)}={hex(v)}{mark}")

# Search for function that contains both "SendMmInit" logic - look for
# string "MmInit" in more forms
for n in [
    b"SendMmInitReq",
    b"MmInitReq",
    b"MM_INIT_REQ",
    b"SupportedRatMap",
    b"InitRap",
]:
    # count in GMC region 0x17b0000-0x1800000 as ASCII only already done
    pass

# Dump around GmcM string - might be in a function name table with code ptr BEFORE name
# Common: {fptr, name} or {name} only in assert
# Search backwards for code-looking VA
print("\n=== scan back 0x100 from GmcM name for code VAs ===")
base = j - 0x100
for off in range(base, j, 4):
    v = u32(off)
    if 0x41700000 <= v <= 0x41800000:  # near GmcM VA 0x417b8e00
        fo = v - VA + MAIN
        print(f"  {hex(off)}={hex(v)} -> {hex(fo)}")

# === TCS: find dump of A[I][[...]] features ===
# Search "A[I][[" count and find code that formats it
print(f"\nA[I][[ count={img.count(b'A[I][[')}")
# Unique: "A[I][[TCS_CDMA_SUPPORT]]" is one of many consecutive feature name strings
# Find start of feature name table
j = img.find(b"A[I][[TCS_CDMA_SUPPORT]]")
# Walk backwards collecting A[I][[ names
names = []
pos = j
for _ in range(30):
    # find previous A[I][[
    prev = img.rfind(b"A[I][[", 0, pos)
    if prev < 0 or prev < j - 0x2000:
        break
    end = img.find(b"]]", prev)
    if end > 0:
        names.append(img[prev : end + 2].decode())
    pos = prev
print("TCS feature names before CDMA:")
for n in reversed(names[-15:]):
    print(f"  {n}")
# And after
pos = j + 1
for _ in range(10):
    nxt = img.find(b"A[I][[", pos)
    if nxt < 0 or nxt > j + 0x800:
        break
    end = img.find(b"]]", nxt)
    if end > 0:
        print(f"  after: {img[nxt:end+2].decode()}")
    pos = nxt + 1

# Find "GV updated from reg" - TCS loads from NV registry
j = img.find(b"GV updated from reg")
print(f"\nGV updated from reg @{hex(j) if j>=0 else None}")
if j and j > 0:
    chunk = img[max(0, j - 100) : j + 150]
    cur = bytearray()
    for b in chunk:
        if 32 <= b < 127:
            cur.append(b)
        else:
            if len(cur) >= 8:
                print(f"  '{cur.decode()}'")
            cur = bytearray()

# Search for getter of TCS GV by id - "GetTcs" / "TcsGet" / "TCS_Get"
print("\n=== TCS get/set APIs ===")
for n in [
    b"TcsGet",
    b"TCS_Get",
    b"GetTcsGv",
    b"TcsGvGet",
    b"GetGvValue",
    b"TcsFeature",
    b"ReadTcs",
    b"TcsRead",
    b"LoadTcs",
    b"TcsLoad",
    b"InitTcs",
    b"TcsInit",
    b"TCS_Init",
]:
    j = img.find(n)
    if j >= 0:
        print(f"  {n}: @{hex(j)}")
        chunk = img[max(0, j - 40) : j + 60]
        cur = bytearray()
        ss = []
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 5:
                    ss.append(cur.decode())
                cur = bytearray()
        print(f"    {ss[:5]}")
