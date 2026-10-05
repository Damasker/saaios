#!/usr/bin/env python3
"""Deep: post-VerifyPin BLs; alternate READY; PresentObj identity of USIM STRB#2."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000


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


def dump(start, end, lab, max_lines=120):
    print(f"\n=== {lab} ===")
    o = start
    n = 0
    while o < end and n < max_lines:
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
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                extra = f" ;LDR.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                extra = f" ;STR.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            # STRB.W [Rn, Rm]
            if (hw & 0xFFF0) == 0xF800 and (hw2 & 0x0FC0) == 0x0000:
                extra = f" ;STRB.W [r{hw & 0xf},r{(hw2 >> 12) & 0xf}]?"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB [r{hw & 7},r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB [r{hw & 7},r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            if (hw & 0xFFC0) == 0x4600:
                rd = (((hw >> 7) & 1) << 3) | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            if (hw & 0xF000) == 0xD000:
                cond = (hw >> 8) & 0xF
                imm = hw & 0xFF
                if imm >= 0x80:
                    imm -= 0x100
                tgt = o + 4 + imm * 2
                names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
                if cond < 14:
                    extra = f" ;B{names[cond]}->{hex(tgt)}"
            if (hw & 0xF800) == 0xE000:
                imm = hw & 0x7FF
                if imm >= 0x400:
                    imm -= 0x800
                extra = f" ;B->{hex(o + 4 + imm * 2)}"
            # BLX reg
            if (hw & 0xFF87) == 0x4780:
                extra = f" ;BLX r{(hw >> 3) & 0xF}"
            if (hw & 0xFF87) == 0x4700:
                extra = f" ;BX r{(hw >> 3) & 0xF}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2
        n += 1


# Trace BL targets from FirstPIN
for tgt in (0x1DCB028, 0x1F1458C, 0x1DC53CC, 0x1CDFAF0, 0x1F145DE):
    dump(tgt, tgt + 0x100, f"callee@{hex(tgt)}")

# Scan callee 0x1dcb028 for BL to FN_A / STATUS / Present getobj / SET_APP / SimInfo
TARGETS = {
    0x14F692C: "FN_A",
    0x14F6A14: "FN_A_p2",
    0x14FB380: "STATUS_BF6",
    0x19916D2: "SET_APP",
    0x14C3986: "SIMWRAP_LATCH",
    0x14C6626: "STATUS_WRAP",
    0x20EA040: "getobj",
}


def scan_bls(start, end, lab):
    print(f"\n=== BL scan {lab} {hex(start)}-{hex(end)} ===")
    found = []
    o = start
    while o < end - 4:
        b = bl(o)
        if b is not None:
            name = TARGETS.get(b)
            if name or (0x14F0000 <= b <= 0x1500000) or (0x1990000 <= b <= 0x19A0000):
                found.append((o, b, name or "?"))
                print(f"  {hex(o)} -> {hex(b)} {name or ''}")
        o += 2
    return found


# Walk FirstPIN function body BLs
scan_bls(0x1F0452C, 0x1F04720, "FirstPIN_fn")
scan_bls(0x1DCB028, 0x1DCB200, "SimInfoish")
scan_bls(0x1F1458C, 0x1F14700, "post_siminfo")

# Recursively: does 0x1dcb028 call sitSendNsSimInfoReq-like?
# Look for string sitSendNsSimInfoReq near FirstPIN string
j = img.find(b"sitSendNsSimInfoReq")
print(f"\nsitSendNsSimInfoReq off={hex(j)} nearby ascii:")
# print surrounding strings
a = j - 200
print(img[a : j + 80])

# Find ALL MOVS r0,#5 ; BL SET_APP (19916d2) — alternate READY writers
print("\n=== all BL->SET_APP with preceding MOVS #5 nearby ===")
set_app = 0x19916D2
o = MAIN
ready_sites = []
while o < len(img) - 8:
    b = bl(o)
    if b == set_app:
        # look back 16 bytes for MOVS #5
        window = img[max(MAIN, o - 16) : o]
        # MOVS rX,#5 = 0x20X5 where X is reg
        has5 = False
        for i in range(0, len(window) - 1, 2):
            hw = struct.unpack_from("<H", window, i)[0]
            if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 5:
                has5 = True
                break
            # MOVW immediate 5 rare
        ready_sites.append((o, has5))
    o += 2
print(f"BL SET_APP count={len(ready_sites)}")
ready5 = [s for s in ready_sites if s[1]]
print(f"with MOVS#5 nearby: {len(ready5)} {[hex(s[0]) for s in ready5[:30]]}")
for s, _ in ready5[:15]:
    dump(s - 0x20, s + 0x10, f"SET_APP#5@{hex(s)}", max_lines=40)

# Present=2 writers: MOVS #2 + STRB.W [rN,#0] where getobj #636c nearby in fn
# Broader: all STRB.W [rN,#0] preceded by MOVS #2 within 8 bytes in 0x14f0000-0x1500000 and 0x1d00000-0x1f20000 and 0x2b00000-0x2c00000
print("\n=== MOVS#2 + STRB.W [#0] in SIM/USIM windows ===")
windows = [
    (0x14F0000, 0x1505000, "L1_STATUS"),
    (0x1D00000, 0x1F80000, "SIT_USIM"),
    (0x2B00000, 0x2C00000, "USIM2"),
    (0x18C0000, 0x18E0000, "mid"),
]
for start, end, lab in windows:
    hits = []
    o = start
    while o < end - 6:
        hw = u16(o)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            # next instr STRB.W [rN,#0] = F88N 0000
            for d in (2, 4):
                if o + d + 4 > end:
                    continue
                hw2 = u16(o + d)
                hw3 = u16(o + d + 2)
                if (hw2 & 0xFFF0) == 0xF880 and hw3 == 0x0000:
                    hits.append(o)
                    break
                # thumb STRB [rN,#0] after MOVS into rM: 70XN
                if (hw2 & 0xF800) == 0x7000 and ((hw2 >> 6) & 0x1F) == 0:
                    # STRB [Rd, Rn, #0] — Rd from movs?
                    hits.append(o)
                    break
        o += 2
    print(f"{lab}: {len(hits)} {[hex(h) for h in hits[:40]]}")

# Check if getobj #636c appears near USIM hits 0x2b51dea / 0x2b11b0e / 0x18cb010
for addr in (0x2B51DEA, 0x2B11B0E, 0x2B299EE, 0x18CB010, 0x18CB048, 0x1D27100):
    # scan back 0x200 for MOVW #0x636c
    found = False
    for o in range(addr - 0x400, addr, 2):
        if o < MAIN:
            continue
        r = movw(o)
        if r and r[0] == 0x636C:
            found = True
            print(f"{hex(addr)}: getobj#636c nearby at {hex(o)}")
            break
    if not found:
        print(f"{hex(addr)}: NO getobj#636c in -0x400")
    dump(addr - 0x30, addr + 0x30, f"cand@{hex(addr)}", max_lines=50)
