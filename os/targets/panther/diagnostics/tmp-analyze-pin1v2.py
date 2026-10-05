#!/usr/bin/env python3
"""Pin1Verified writers: real log sites (ctx 0x41xx) + VerifyPin RSP / FCP coupling."""
import struct
from pathlib import Path

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


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
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


def shannon_id(soff):
    """Shannon often packs log id in bits[23:8] with low byte 0x44."""
    w = u32(soff - 8)
    if 0 < w < 0x10000:
        return w
    if (w & 0xFF) == 0x44:
        return (w >> 8) & 0xFFFF
    return w & 0xFFFF


def real_log_sites(imm, lim=15):
    """MOVW #imm with nearby MOVT to 0x41xx/0x40xx ctx-ish, not dense id table."""
    hits = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != imm:
            continue
        # skip dense tables: next MOVW is imm+1 within 32B
        dense = False
        for p in range(o + 4, o + 32, 2):
            r2 = movw(p)
            if r2 and r2[0] == imm + 1:
                dense = True
                break
        if dense:
            continue
        # require MOVT 0x40xx..0x45xx within ±24B (log ctx)
        ctx = None
        for p in range(max(MAIN_OFF, o - 24), min(END, o + 28), 2):
            t = movt(p)
            if t and 0x4000 <= t[0] <= 0x45FF:
                ctx = t[0]
                break
        if ctx is None:
            continue
        hits.append((o, r[1], ctx))
        if len(hits) >= lim:
            break
    return hits


def dump(o0, length):
    o = o0
    end = o0 + length
    out = []
    while o < end:
        r = movw(o)
        t = movt(o)
        b = bl_target(o)
        hw = u16(o)
        if b is not None and (hw & 0xF800) == 0xF000:
            out.append(f"  {o:#010x} va={va(o):#010x}: BL->{b:#x}/{va(b):#x}")
            o += 4
            continue
        if r:
            out.append(f"  {o:#010x} va={va(o):#010x}: MOVW r{r[1]},#{r[0]:#x}")
            o += 4
            continue
        if t:
            out.append(f"  {o:#010x}: MOVT r{t[1]},#{t[0]:#x}")
            o += 4
            continue
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            o += 4
            continue
        if (hw & 0xFF00) == 0x2000:
            imm = hw & 0xFF
            tag = {0: "CLR?", 1: "SET?", 2: "PIN", 3: "DIS/PUK", 5: "READY"}.get(imm, "")
            out.append(f"  {o:#010x} va={va(o):#010x}: MOVS r{(hw>>8)&7},#{imm}" + (f"  ;{tag}" if tag else ""))
        elif (hw & 0xFF00) == 0x2800:
            out.append(f"  {o:#010x}: CMP r{(hw>>8)&7},#{hw&0xff}")
        elif hw in (0xB5F0, 0xB570, 0xB580, 0xB5B0, 0xB510, 0xB5F8):
            out.append(f"  {o:#010x} va={va(o):#010x}: PUSH")
        elif (hw & 0xFF00) == 0xD000:
            out.append(f"  {o:#010x}: Bcond")
        # STRB Rt,[Rn,#imm]  T1: 0111 0 imm5 Rn Rt
        elif (hw & 0xF800) == 0x7000:
            rt = hw & 7
            rn = (hw >> 3) & 7
            imm5 = (hw >> 6) & 0x1F
            out.append(f"  {o:#010x} va={va(o):#010x}: STRB r{rt},[r{rn},#{imm5}]")
        o += 2
    return "\n".join(out)


# Resolve IDs from strings
strs = {
    "first_pin_s0": b"[SIT_0_SIM] First PIN1 Verification is done",
    "first_pin_s1": b"[SIT_1_SIM] First PIN1 Verification is done",
    "first_unblk_s0": b"[SIT_0_SIM] First PIN1 Unblock is done",
    "vp_rsp_s0": b"[SIT_0_SIM] Rx NS_USIM_VERIFYPIN_RSP",
    "vp_rsp_s1": b"[SIT_1_SIM] Rx NS_USIM_VERIFYPIN_RSP",
    "vp_req_s0": b"[SIT_0_SIM] Tx NS_USIM_VERIFYPIN_REQ",
    "status_upd": b"SIM STATUS update: Present:%d, Pin1Verified: %d, MePerVerified:%d",
    "reset_both": b"Reset IsSimVerifyCompleteSent :%d,Pin1Verified :%d",
    "reset_only": b"Reset IsSimVerifyCompleteSent :%d",
    "pin_dis_fcp": b"changing PinStatus as PIN_DISABLED",
    "pin_verify_ind": b"NS_SIM_PIN_VERIFY_IND : PinCmdStatus",
}

print("=== resolved log ids ===")
ids = {}
for name, s in strs.items():
    off = img.find(s)
    # prefer start of full C string
    s0 = off
    while s0 > 0 and 32 <= img[s0 - 1] < 127:
        s0 -= 1
    hid = shannon_id(s0)
    ids[name] = hid
    print(f"{name}: str@0x{s0:x} id=0x{hid:x}")

# Real sites for first-pin / verify-rsp / reset
for name in ("first_pin_s0", "first_pin_s1", "first_unblk_s0", "vp_rsp_s0", "reset_both", "pin_verify_ind"):
    imm = ids[name]
    sites = real_log_sites(imm, 8)
    print(f"\n======== {name} id=0x{imm:x} real_sites={len(sites)} ========")
    for o, rd, ctx in sites:
        print(f"\n-- SITE 0x{o:x} va=0x{va(o):x} rd={rd} ctx_hi=0x{ctx:x} --")
        print(dump(o - 0xA0, 0x160))

# Cross-check: does FCP PIN_DISABLED site set flag #1 before/after 0x7ab?
print("\n======== FCP PIN_DISABLED real sites vs MOVS #1 ========")
for o, rd, ctx in real_log_sites(0x7AB, 10):
    ones = []
    for p in range(o - 0x80, o + 0xC0, 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 1:
            ones.append(hex(p))
    print(f"FCP 0x{o:x}/va0x{va(o):x} ctx=0x{ctx:x} MOVS#1 nearby: {ones[:8]}")

# Around STATUS_UPD READY writer: what is tested for Pin1Verified?
# Previously: 0x14fb5c4 MOVS#5 SET_APP. Dump broader for loads of verified flag.
print("\n======== STATUS_UPD PIN/READY cluster @0x14fb280..0x14fb5e0 ========")
print(dump(0x14FB280, 0x380))

# Look for STRB #1 near first_pin real sites — scan all real first_pin_s0
print("\n======== STRB near first_pin_s0 sites ========")
for o, rd, ctx in real_log_sites(ids["first_pin_s0"], 6):
    for p in range(o - 0x120, o + 0x80, 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (0, 1):
            # look ahead 8B for STRB
            for q in range(p, p + 12, 2):
                h2 = u16(q)
                if (h2 & 0xF800) == 0x7000:
                    print(f"  near log@0x{o:x}: MOVS#{hw&0xff}@0x{p:x} STRB@0x{q:x} [r{(h2>>3)&7},#{(h2>>6)&0x1f}]")
