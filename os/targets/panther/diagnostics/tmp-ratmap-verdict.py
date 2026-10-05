#!/usr/bin/env python3
"""Find TCS_CDMA default; confirm only No-CDMA log paths; product capability source."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


# Positive vs negative CDMA-in-map strings
print("=== CDMA-in-map log polarity ===")
for n in [
    b"No CDMA in InitRapMap",
    b"No CDMA in SupportedRatMap",
    b"CDMA in InitRapMap!",  # would be positive if exists without No
    b"Found CDMA",
    b"CDMA supported in",
    b"CdmaRatEnabled",
    b"EnableCdmaRat",
]:
    # for positive, ensure not substring of No CDMA
    j = 0
    hits = []
    while True:
        k = img.find(n, j)
        if k < 0:
            break
        # check not preceded by "No "
        pref = img[max(0, k - 3) : k]
        hits.append((hex(k), pref))
        j = k + 1
        if len(hits) > 5:
            break
    print(f"  {n!r}: {hits}")

# TCS feature table: names are consecutive; values often in parallel array
# Find "A[I][[TCS_CDMA_SUPPORT]]" and look for nearby numeric defaults in a struct
j = img.find(b"A[I][[TCS_CDMA_SUPPORT]]")
print(f"\nTCS_CDMA_SUPPORT dump str @{hex(j)}")
# Surrounding 64 bytes as hex + ascii
print(img[j - 32 : j + 64].hex())

# DS_TCS_GV_CDMA_SUPPORT - feature id from registration at 0x1763582: r3=#0x127
# That registration: MOVW r0, #string; BL hash; MOVW r1; MOVW r3,#0x127; BL register
# Feature id 0x127 may be the GV index
print("\nFeature id 0x127 near CDMA_SUPPORT registration (from prior)")
print("  DS_TCS_GV_CDMA_SUPPORT registered with r3=#0x127 at 0x1763594")

# Search for CMP/MOV with #0x127 near TCS feature reads (not reg table)
print("\n=== MOVW #0x127 outside reg tables (possible GV readers) ===")
hits = []
o = 0x1400000
while o < 0x3C00000 - 4:
    hw, hw2 = u16(o), u16(o + 2)
    # MOVW
    if (hw & 0xFBF0) == 0xF240 and (hw2 & 0x8000) == 0:
        i = (hw >> 10) & 1
        imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
        if imm == 0x127:
            # skip if looks like reg table (nearby BL 0x20d1afa)
            hits.append(o)
    o += 2
# filter: not in 0x1763xxx cluster
filt = [h for h in hits if not (0x1763000 <= h <= 0x1764000) and not (0x1100000 <= h <= 0x1200000)]
print(f"total MOVW #0x127: {len(hits)}; outside common reg: {len(filt)}")
print([hex(h) for h in filt[:20]])

# Capabilities exercised this goal (for MODEM-06 doc)
print("""
=== CAPABILITIES EXERCISED (inventory for MODEM-06) ===
RO MAIN: app_state SET_APP/+0xBF4; Present/+0xBF6; FN_A×2 CDMA-only; FN_B dead;
  SIT 0x0200 mirror; Pin1Verified VerifyPin-only; PIN_SKIP eSIM stub;
  InitRapMap/SupportedRatMap init-only; TCS_CDMA_SUPPORT GV (reg/NV);
  handover no RAP; preferred 11/12 live-neg; modem_a same CDMA Present=2.
Live (prior): 0x0200 PIN+DISABLED; radio ON; reg0; rmnet 0; STATUS APDU;
  OpenChannel; preferred 11/12; cold sysrq; handover ONLINE.
BANNED/not done: EFS RW; CDMA preferred; slot switch; PinSkip; PIN spray.
""")
