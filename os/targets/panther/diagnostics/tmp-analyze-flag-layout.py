#!/usr/bin/env python3
"""Confirm compared byte at +0xBF6 is NOT pin1 status; map LDRB [r5,#0/#1] to flags."""
import struct
from pathlib import Path

img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


# In STATUS cluster, find all LDRB [r5,#imm] and LDRB.W [rN,#0xbf6]
print("=== LDRB from r5 (flag struct) in STATUS cluster ===")
for o in range(0x14FB200, 0x14FB700, 2):
    hw = u16(o)
    if (hw & 0xF800) == 0x7800:
        rt, rn, imm = hw & 7, (hw >> 3) & 7, (hw >> 6) & 0x1F
        if rn == 5:
            print(f"  {o:#x}: LDRB r{rt},[r5,#{imm}]")
    # LDRB.W F89x
    if (hw & 0xFFF0) == 0xF890:
        hw2 = u16(o + 2)
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm12 = hw2 & 0xFFF
        if imm12 == 0xBF6:
            print(f"  {o:#x}: LDRB.W r{rt},[r{rn},#0xBF6]")

# Search strings about app state values near SIM STATUS
print("\n=== related log 0x106b ===")
# hdr packed: id<<8 | 0x44
for off in range(0x4D4E000, 0x4D51000):
    if off + 8 >= len(img):
        break
    w = struct.unpack_from("<I", img, off)[0]
    if (w & 0xFF) == 0x44 and ((w >> 8) & 0xFFFF) == 0x106B:
        s = off + 8
        # maybe ctx at off+4, string at off+8 — or string immediately
        # try both
        for cand in (off + 8, off + 4):
            if 32 <= img[cand] < 127:
                end = img.find(b"\0", cand)
                print(f"  {cand:#x}: {img[cand:end].decode()[:160]}")
                break

# Any string mentioning pin disabled + ready / app state
print("\n=== DISABLED+READY / pin status app state strings ===")
for n in [
    b"PIN_DISABLED",
    b"pin disabled",
    b"Pin Disabled",
    b"PinStatus as PIN_DISABLED",
    b"app_state.*READY",
    b"changing.*READY",
    b"Set App State",
    b"set app state",
    b"AppState to READY",
    b"APP STATE READY",
    b"SIM is ready",
    b"Sim is READY",
]:
    j = img.find(n) if b".*" not in n else -1
    if j >= 0:
        s0 = j
        while s0 > 0 and 32 <= img[s0 - 1] < 127:
            s0 -= 1
        s1 = j
        while s1 < len(img) and 32 <= img[s1] < 127:
            s1 += 1
        print(f"  {s0:#x}: {img[s0:s1].decode()[:140]}")
