#!/usr/bin/env python3
"""RO: who sets app_state PIN(2) vs READY(5) when pin1 DISABLED; PinSkip gates."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
MAIN_SZ = 0x05917ACC
VA_BASE = 0x40010000  # TOC m_off
img = PATH.read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def file_to_va(o):
    return VA_BASE + (o - MAIN_OFF)


def find_str(s: bytes):
    return img.find(s)


def log_id_before(soff):
    """Shannon log: id @ -8, ctx @ -4 before printable string."""
    return u32(soff - 8), u32(soff - 4)


def thumb_movw_imm(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    imm = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
    return imm, rd


def isolated_movw(imm_target, exclude_next=None, limit=40):
    if exclude_next is None:
        exclude_next = imm_target + 1
    hits = []
    end = MAIN_OFF + MAIN_SZ - 8
    for o in range(MAIN_OFF, end, 2):
        r = thumb_movw_imm(o)
        if not r or r[0] != imm_target:
            continue
        followed = False
        for p in range(o + 4, o + 48, 2):
            r2 = thumb_movw_imm(p)
            if r2 and r2[0] == exclude_next:
                followed = True
                break
        if not followed:
            hits.append((o, r[1]))
            if len(hits) >= limit:
                break
    return hits


def dump_near(o, back=0x60, ahead=0xA0):
    start = max(MAIN_OFF, (o - back) & ~1)
    end = min(MAIN_OFF + MAIN_SZ, o + ahead)
    lines = []
    p = start
    while p < end:
        r = thumb_movw_imm(p)
        hw = u16(p)
        if r:
            tag = ""
            if r[0] in (2, 3, 5):
                tag = {2: "PIN?", 3: "DISABLED?", 5: "READY?"}[r[0]]
                tag = f"  <<{tag}"
            if r[0] in interesting_ids:
                tag += f"  <<LOG_{interesting_ids[r[0]]}"
            lines.append(f"  0x{p:08x} va=0x{file_to_va(p):08x}: MOVW r{r[1]},#0x{r[0]:x}{tag}")
            p += 4
            continue
        # MOVT
        if (hw & 0xFBF0) == 0xF2C0 and (u16(p + 2) & 0x8000) == 0:
            hw2 = u16(p + 2)
            i = (hw >> 10) & 1
            imm4 = hw & 0xF
            imm3 = (hw2 >> 12) & 7
            rd = (hw2 >> 8) & 0xF
            imm8 = hw2 & 0xFF
            imm = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
            lines.append(f"  0x{p:08x}: MOVT r{rd},#0x{imm:x}")
            p += 4
            continue
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            p += 4
            continue
        if (hw & 0xFF00) == 0x2000:  # MOVS Rd, #imm8
            rd, imm = (hw >> 8) & 7, hw & 0xFF
            tag = ""
            if imm in (2, 3, 5):
                tag = {2: "PIN?", 3: "DISABLED?", 5: "READY?"}[imm]
                tag = f"  <<{tag}"
            lines.append(f"  0x{p:08x} va=0x{file_to_va(p):08x}: MOVS r{rd},#{imm}{tag}")
        elif (hw & 0xFF00) == 0x2800:
            lines.append(f"  0x{p:08x}: CMP r{(hw>>8)&7},#{hw&0xff}")
        elif hw in (0xB510, 0xB570, 0xB5F0, 0xB580, 0xB5B0):
            lines.append(f"  0x{p:08x} va=0x{file_to_va(p):08x}: PUSH  <<fn?")
        p += 2
    return "\n".join(lines)


# --- string / log id catalog ---
STRINGS = {
    "tx_app": b"[SIT_0_SIM] Tx SIM Status(app_state=%d, perso_substate=%d)",
    "determine": b"[USIM_%d] >> DetermineSimStatus",
    "pin_disabled_fcp": b"[USIM_%d] RemainingAttemptCount %d and Pin Status Disabled in APP FCP so changing PinStatus as PIN_DISABLED",
    "sit_set_pin1": b"[SIT_0_SIM] sitSetPin1Status:- PIN 1 Status",
    "sit_get_pin1": b"[SIT_0_SIM] sitGetPin1Status:- PIN 1 Status",
    "pin_skip_fail": b"[USIM_%d] PIN SKIP FAILED: NOT eSIM",
    "pin_skip_inv": b"Invalid SimState",
    "sim_pin_skip": b"SimPinSkip",
    "decode_skip": b"DecodeSimPinSkipReq",
    "ns_skip_req": b"NS_SIM_PIN_SKIP_REQ",
    "acpin_not": b"ACPIN: PIN Action Not Required",
    "pin1_en_notif": b"[USIM_%d] PIN1 Enabled, sending Notification to AP",
    "get_multi": b"[USIM_%d] >> GetMultiPin1Status - AppType",
    "app_state_pin": b"APPSTATE_PIN",
    "app_state_ready": b"APPSTATE_READY",
    "sim_status_ready": b"SIM STATUS is READY",
    "set_app_state": b"SetAppState",
    "sim_app_state": b"SimAppState",
}

print("=== MAIN image ===")
print(f"path={PATH}")
print(f"MAIN_OFF=0x{MAIN_OFF:x} MAIN_SZ=0x{MAIN_SZ:x} VA_BASE=0x{VA_BASE:x}")
print(f"file_size=0x{len(img):x}")

print("\n=== log records ===")
interesting_ids = {}
for name, s in STRINGS.items():
    off = find_str(s)
    if off < 0:
        print(f"{name}: MISSING")
        continue
    hid, ctx = log_id_before(off)
    if not (0 < hid < 0x10000):
        for back in (8, 12, 16, 4):
            w = u32(off - back)
            if 0 < w < 0x10000:
                hid = w
                break
    interesting_ids[hid] = name
    print(f"{name}: str@0x{off:x} id=0x{hid:x} ctx=0x{ctx:x} va_str=0x{file_to_va(off) if MAIN_OFF<=off<MAIN_OFF+MAIN_SZ else 0:x}")

# Known from prior session
KNOWN = {
    0x347: "tx_app_state",
    0x7AB: "pin_disabled_fcp",
    0x2C5E: "determine",
    0xC6B: "sit_set_pin1",
}
for k, v in KNOWN.items():
    interesting_ids[k] = v

print("\n=== isolated MOVW log sites (key ids) ===")
for imm, label in [(0x347, "tx_app"), (0x7AB, "pin_dis"), (0x2C5E, "determine"), (0xC6B, "set_pin1")]:
    hits = isolated_movw(imm, limit=12)
    print(f"\n--- {label} id=0x{imm:x} isolated={len(hits)} ---")
    for o, rd in hits[:8]:
        print(f"SITE file=0x{o:x} va=0x{file_to_va(o):x} rd={rd}")
        # show MOVS #2/#5 nearby
        near_imm = []
        for p in range(max(MAIN_OFF, o - 0x100), min(MAIN_OFF + MAIN_SZ, o + 0x100), 2):
            hw = u16(p)
            if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (1, 2, 3, 4, 5, 6):
                near_imm.append(f"0x{p:x}:MOVS#{hw&0xff}")
        if near_imm:
            print("  near MOVS#1..6:", ", ".join(near_imm[:12]))

# Focus: DetermineSimStatus + pin_disabled real USIM sites (ctx high nibble often 0x41xx)
print("\n=== USIM pin_disabled sites (MOVT ctx 0x41xx near MOVW #0x7ab) ===")
usim_sites = []
for o, rd in isolated_movw(0x7AB, limit=30):
    # look ±0x20 for MOVT #0x41xx
    ctxish = False
    for p in range(o - 0x20, o + 0x30, 2):
        hw = u16(p)
        if (hw & 0xFBF0) != 0xF2C0:
            continue
        hw2 = u16(p + 2)
        if hw2 & 0x8000:
            continue
        i = (hw >> 10) & 1
        imm4 = hw & 0xF
        imm3 = (hw2 >> 12) & 7
        imm8 = hw2 & 0xFF
        imm = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
        if (imm & 0xFF00) == 0x4100 or imm in (0x4102, 0x4107, 0x44BD):
            ctxish = True
            break
    if ctxish:
        usim_sites.append(o)
        print(f"\nUSIM-ish @0x{o:x} va=0x{file_to_va(o):x}")
        print(dump_near(o, 0x80, 0xC0))

print("\n=== DetermineSimStatus isolated sites ===")
for o, rd in isolated_movw(0x2C5E, limit=10):
    print(f"\nDET @0x{o:x} va=0x{file_to_va(o):x}")
    print(dump_near(o, 0x40, 0x120))

print("\n=== Tx SIM Status (app_state log) isolated ===")
for o, rd in isolated_movw(0x347, limit=8):
    print(f"\nTX @0x{o:x} va=0x{file_to_va(o):x}")
    print(dump_near(o, 0x80, 0x80))

# PinSkip strings and nearby code refs via ADR-ish: search LDR literal pools pointing to string
print("\n=== PinSkip / DecodeSimPinSkipReq string offsets ===")
for name in ("pin_skip_fail", "decode_skip", "ns_skip_req", "sim_pin_skip", "acpin_not"):
    s = STRINGS[name]
    off = find_str(s)
    print(f"{name}: 0x{off:x}" if off >= 0 else f"{name}: MISSING")

# Search more specific USIM app_state strings
print("\n=== targeted app_state / pin strings ===")
for needle in [
    b"app_state",
    b"AppState",
    b"APP_STATE",
    b"SimStatus",
    b"SIM_STATE",
    b"PinStatus",
    b"PIN_DISABLED",
    b"PIN_ENABLED",
    b"CHV1",
    b"PS_DO",
    b"changing PinStatus",
    b"SIM STATUS",
    b"SetSimStatus",
    b"UpdateSimStatus",
    b"sitTxSimStatus",
    b"sitBuildSimStatus",
    b"RilAppState",
]:
    hits = []
    start = 0
    while len(hits) < 5:
        j = img.find(needle, start)
        if j < 0:
            break
        # print surrounding printable
        a = max(0, j - 40)
        chunk = img[a : j + 80]
        # extract C string containing needle
        s0 = j
        while s0 > 0 and 32 <= img[s0 - 1] < 127:
            s0 -= 1
        s1 = j
        while s1 < len(img) and 32 <= img[s1] < 127:
            s1 += 1
        text = img[s0:s1].decode("ascii", "replace")
        if len(text) >= 8:
            hits.append((s0, text[:140]))
        start = j + 1
    if hits:
        print(f"\n[{needle.decode()}]")
        for o, t in hits:
            print(f"  0x{o:x}: {t}")
