#!/usr/bin/env python3
"""Find MOVW log-id sites for Tx SIM Status / PIN_DISABLED / DetermineSimStatus."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
MAIN_SZ = 0x05917ACC
img = memoryview(PATH.read_bytes())


def u32(off):
    return struct.unpack_from("<I", img, off)[0]


def is_movw(insn):
    if (insn & 0x0FF00000) != 0x03000000:
        return None
    imm = ((insn & 0xF0000) >> 4) | (insn & 0xFFF)
    rd = (insn >> 12) & 0xF
    return imm, rd


def movw_sites(imm_target, limit=30):
    hits = []
    end = MAIN_OFF + MAIN_SZ
    for o in range(MAIN_OFF, end - 4, 4):
        r = is_movw(u32(o))
        if r and r[0] == imm_target:
            hits.append((o, r[1]))
            if len(hits) >= limit:
                break
    return hits


def dump_fn_around(file_off, radius=64):
    """Walk back for STMFD/PUSH (e92d....) as rough function entry."""
    start = max(MAIN_OFF, (file_off - 0x200) & ~3)
    entries = []
    for o in range(start, file_off, 4):
        insn = u32(o)
        # STMFD sp!, {...}  = e92dxxxx or push
        if (insn & 0xFFFF0000) == 0xE92D0000:
            entries.append(o)
    return entries[-3:] if entries else []


# Correct aligned headers: string starts with '[' after ctx word
records = [
    ("tx_app_state_s0", b"[SIT_0_SIM] Tx SIM Status(app_state=%d, perso_substate=%d)"),
    ("tx_app_state_s1", b"[SIT_1_SIM] Tx SIM Status(app_state=%d, perso_substate=%d)"),
    ("sit_set_pin1_s0", b"[SIT_0_SIM] sitSetPin1Status:- PIN 1 Status"),
    ("sit_get_pin1_s0", b"[SIT_0_SIM] sitGetPin1Status:- PIN 1 Status"),
    ("determine", b"[USIM_%d] >> DetermineSimStatus"),
    ("pin_disabled_fcp", b"[USIM_%d] RemainingAttemptCount %d and Pin Status Disabled in APP FCP so changing PinStatus as PIN_DISABLED"),
    ("pin1_en_notif", b"[USIM_%d] PIN1 Enabled, sending Notification to AP"),
    ("pin_action_not", b"ACPIN: PIN Action Not Required"),
    ("pin_skip_esim", b"[USIM_%d] PIN SKIP FAILED: NOT eSIM"),
    ("get_multi_pin1", b"[USIM_%d] >> GetMultiPin1Status - AppType"),
]

print("=== log records (id @ str-8, ctx @ str-4) ===")
for name, s in records:
    off = bytes(img).find(s)
    if off < 0:
        print(f"{name}: MISSING")
        continue
    # ensure 8-byte aligned header immediately before string
    hid = u32(off - 8)
    ctx = u32(off - 4)
    print(f"\n{name}: str@0x{off:x} id=0x{hid:x} ctx=0x{ctx:x}")
    if hid == 0 or hid > 0xFFFF:
        # try alternate: some records pack differently
        for back in (8, 12, 16):
            w = u32(off - back)
            if 0 < w < 0x10000:
                print(f"  alt id 0x{w:x} at -{back}")
                hid = w
                break
    hits = movw_sites(hid & 0xFFFF) if hid else []
    # if id looks like 0x00000347 use full; if garbage try low 16
    if not hits and 0 < (hid & 0xFFFF) < 0x10000:
        hits = movw_sites(hid & 0xFFFF)
    print(f"  MOVW hits for 0x{hid & 0xFFFF:x}: {len(hits)}")
    for o, rd in hits[:8]:
        nxt = " ".join(f"{u32(o+4+4*i):08x}" for i in range(8))
        print(f"  @0x{o:x} r{rd}  next[{nxt}]")
        ents = dump_fn_around(o)
        if ents:
            print(f"    recent STMFD: " + ", ".join(f"0x{e:x}" for e in ents))

# Special: known good id 0x347 from manual hexdump
print("\n=== FORCE log_id 0x347 (manual from hexdump) ===")
for o, rd in movw_sites(0x347):
    nxt = " ".join(f"{u32(o+4+4*i):08x}" for i in range(10))
    print(f"@0x{o:x} r{rd} next[{nxt}]")
    ents = dump_fn_around(o)
    print(f"  STMFD: " + ", ".join(f"0x{e:x}" for e in ents))

# Also search MOVT+MOVW pairs constructing 0x410259f5 (common SIT_0 ctx)
# movw imm low, movt imm high
print("\n=== refs constructing ctx 0x410259f5 ===")
# low=0x59f5 high=0x4102
count = 0
end = MAIN_OFF + MAIN_SZ
for o in range(MAIN_OFF, end - 8, 4):
    r = is_movw(u32(o))
    if not r or r[0] != 0x59F5:
        continue
    # look ahead 1..6 insns for MOVT same Rd with 0x4102
    rd = r[1]
    for k in range(1, 8):
        insn2 = u32(o + 4 * k)
        # MOVT: cond 00110100 imm4 Rd imm12 — opcode 0x34
        if (insn2 & 0x0FF00000) != 0x03400000:
            continue
        imm = ((insn2 & 0xF0000) >> 4) | (insn2 & 0xFFF)
        rd2 = (insn2 >> 12) & 0xF
        if rd2 == rd and imm == 0x4102:
            print(f"ctx pair @0x{o:x}+{4*k} r{rd}")
            count += 1
            break
    if count >= 15:
        break
print(f"ctx_pairs={count}")
