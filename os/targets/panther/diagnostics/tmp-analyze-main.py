#!/usr/bin/env python3
"""RO analysis of Shannon MAIN: app_state PIN vs pin1 DISABLED gates."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
MAIN_SZ = 0x05917ACC
img = PATH.read_bytes()
main = img[MAIN_OFF : MAIN_OFF + MAIN_SZ]
assert len(main) == MAIN_SZ


def u32(off):
    return struct.unpack_from("<I", img, off)[0]


def find(s: bytes):
    return img.find(s)


def is_movw(insn):
    if (insn & 0x0FF00000) != 0x03000000:
        return None
    imm = ((insn & 0xF0000) >> 4) | (insn & 0xFFF)
    rd = (insn >> 12) & 0xF
    return imm, rd


def movw_sites(imm_target, limit=20):
    hits = []
    for o in range(MAIN_OFF, MAIN_OFF + MAIN_SZ - 4, 4):
        insn = u32(o)
        r = is_movw(insn)
        if r and r[0] == imm_target:
            hits.append((o, r[1], insn))
            if len(hits) >= limit:
                break
    return hits


# --- key strings ---
keys = {
    "tx_app_state": b"Tx SIM Status(app_state=%d, perso_substate=%d)",
    "pin_disabled_fcp": b"Pin Status Disabled in APP FCP so changing PinStatus as PIN_DISABLED",
    "determine": b">> DetermineSimStatus",
    "pin_action_not": b"PIN Action Not Required",
    "pin1_enabled_notif": b"PIN1 Enabled, sending Notification to AP",
    "sit_set_pin1": b"sitSetPin1Status:- PIN 1 Status",
    "get_multi_pin1": b"GetMultiPin1Status - AppType",
    "ps_do_adm": b"PS_DO Parsing Got ADM PIN",
    "pin_skip_esim": b"PIN SKIP FAILED: NOT eSIM",
    "src_rcm": b"sitSimRcmHandle.c",
    "src_ns": b"sitSimNsHandle.c",
}

print("=== string file offsets ===")
for name, s in keys.items():
    off = find(s)
    print(f"  {name}: 0x{off:x}" if off >= 0 else f"  {name}: MISSING")

# log record header: typically [id:u32][ctx:u32][fmt...]
# For tx_app_state, string is preceded by 'A' then the format — find full log start

tx = find(b"A[SIT_0_SIM] Tx SIM Status(app_state=%d")
print(f"\ntx full log @ 0x{tx:x}")
print(f"  -8: {u32(tx-8):08x}  -4: {u32(tx-4):08x}")
log_id = u32(tx - 8)
ctx = u32(tx - 4)
print(f"  log_id=0x{log_id:x} ctx=0x{ctx:x}")

print(f"\n=== MOVW #0x{log_id:x} (tx app_state log id) ===")
for o, rd, insn in movw_sites(log_id):
    print(f"  @0x{o:x} MOVW r{rd},#0x{log_id:x} insn={insn:08x}")
    # next few words
    words = [f"{u32(o+4+4*i):08x}" for i in range(6)]
    print(f"    next: {' '.join(words)}")

# pin_disabled_fcp log id
pdis = find(b"A[USIM_%d] RemainingAttemptCount %d and Pin Status Disabled")
print(f"\npin_disabled_fcp log @ 0x{pdis:x}")
print(f"  -8: {u32(pdis-8):08x}  -4: {u32(pdis-4):08x}")
# USIM logs may have different header — walk back for small id
for back in range(4, 32, 4):
    w = u32(pdis - back)
    if 0 < w < 0x10000:
        print(f"  candidate id 0x{w:x} at -{back}")
        hits = movw_sites(w, limit=8)
        for o, rd, insn in hits:
            print(f"    MOVW r{rd} @0x{o:x}")

# DetermineSimStatus
det = find(b"A[USIM_%d] >> DetermineSimStatus")
print(f"\nDetermineSimStatus log @ 0x{det:x}")
print(f"  -8: {u32(det-8):08x}  -4: {u32(det-4):08x}")
for back in range(4, 24, 4):
    w = u32(det - back)
    if 0 < w < 0x10000:
        print(f"  candidate id 0x{w:x} at -{back}")
        for o, rd, insn in movw_sites(w, limit=8):
            print(f"    MOVW r{rd} @0x{o:x}")
            words = [f"{u32(o+4+4*i):08x}" for i in range(8)]
            print(f"      next: {' '.join(words)}")

# Search for app_state enum table: PIN=2 READY=5 nearby strings
print("\n=== nearby enum-ish strings ===")
for s in [
    b"PIN_REQUIRED",
    b"PUK_REQUIRED",
    b"SIMAPP_STATE",
    b"APPSTATE_PIN",
    b"APPSTATE_READY",
    b"RIL_APPSTATE",
    b"SIM_PIN",
    b"SIM_READY",
    b"NOT_READY",
    b"READY_STATE",
    b"PIN_STATE_DISABLED",
    b"PIN_STATE_ENABLED",
    b"PIN_STATE_UNKNOWN",
]:
    off = find(s)
    if off >= 0:
        print(f"  0x{off:x}: {s.decode()}")

# Factory AOSP mapping: app_state 2=PIN 5=READY — search dword sequences 2,3,4,5
# More useful: find "Tx SIM Status(app_state" caller via log_id MOVW then BL

print("\n=== sitSetPin1Status log ===")
sp = find(b"A[SIT_0_SIM] sitSetPin1Status")
print(f"  @0x{sp:x} -8={u32(sp-8):x} -4={u32(sp-4):x}")
sid = u32(sp - 8)
for o, rd, insn in movw_sites(sid, limit=10):
    print(f"  MOVW r{rd} @0x{o:x}")
    words = [f"{u32(o+4+4*i):08x}" for i in range(10)]
    print(f"    next: {' '.join(words)}")
