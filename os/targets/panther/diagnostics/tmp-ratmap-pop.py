#!/usr/bin/env python3
"""Trace SupportedRatMap population before QM_MM_INIT — sources & CDMA bit."""
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
            if (hw & 0xFFF0) == 0xF890:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# --- String inventory around RatMap ---
print("=== RatMap / CDMA support related strings ===")
needles = [
    b"SupportedRatMap",
    b"InitRapMap",
    b"No CDMA in InitRapMap",
    b"No CDMA in SupportedRatMap",
    b"QM_MM_INIT_REQ",
    b"RRM_RRC_INIT_REQ",
    b"SetSupportedRat",
    b"GetSupportedRat",
    b"UpdateSupportedRat",
    b"BuildSupportedRat",
    b"FillSupportedRat",
    b"SupportedRatBitmap",
    b"rat_bitmap",
    b"RatBitmap",
    b"CDMA bit",
    b"CDMA_BIT",
    b"SUPPORT_CDMA",
    b"SupportCdma",
    b"CdmaSupported",
    b"IsCdmaSupport",
    b"bSupportCdma",
    b"cdma_support",
    b"NV_MODEM",
    b"RFNV_",
    b"rfnv_",
    b"/nv/item_files",
    b"mmode/lte",
    b"mmode/sd",
    b"rat_disabled",
    b"disabled_rat",
    b"DisabledRat",
    b"MaskRat",
    b"rat_mask",
    b"eu_band",
    b"EU_ONLY",
    b"NO_CDMA",
    b"no_cdma",
    b"CDMA_DISABLE",
    b"DisableCDMA",
    b"RemoveCdma",
    b"StripCdma",
    b"FilterCdma",
    b"ClearCdma",
    b"RapMap",
    b"gRapMap",
    b"m_SupportedRat",
    b"mSupportedRat",
    b"ucSupportedRat",
    b"dwSupportedRat",
]
for n in needles:
    c = img.count(n)
    if c:
        j = img.find(n)
        print(f"  {n!r}: n={c} @{hex(j)}")
        # neighbors
        chunk = img[max(0, j - 80) : j + 100]
        cur = bytearray()
        ss = []
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 8:
                    ss.append(cur.decode())
                cur = bytearray()
        for s in ss[:5]:
            print(f"    '{s}'")

# --- Find RRM_RRC_INIT_REQ_Handler SupportedRatMap log via MOVW r2 packed style ---
# Shannon often: MOVW r2,#log_id then MLA/CMP. Search registration of string.
# Alternative: find code that stores to a field then logs SupportedRatMap

# Search for "RRM_RRC_INIT_REQ_Handler - SupportedRatMap" - use as fmt via log id table
# Find unique nearby function names
print("\n=== Functions near SupportedRatMap / Init strings ===")
for n in [
    b"RRM_RRC_INIT_REQ_Handler",
    b"QM_MM_INIT_REQ_Handler",
    b"L1C_RRM_INIT",
    b"SendQmMmInit",
    b"Send_QM_MM_INIT",
    b"QM_MM_INIT_REQ_Sender",
    b"BuildQmMmInit",
    b"FillQmMmInit",
    b"MmInitReq",
    b"SendMmInit",
]:
    j = img.find(n)
    print(f"  {n}: {hex(j) if j>=0 else None}")
    if j and j > 0:
        chunk = img[max(0, j - 120) : j + 80]
        cur = bytearray()
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 10:
                    print(f"    '{cur.decode()}'")
                cur = bytearray()

# --- CDMA capability in RAP / band config ---
print("\n=== CDMA capability / band / cal strings ===")
for n in [
    b"CDMA1X",
    b"CDMA_1X",
    b"EVDO",
    b"HDR_",
    b"IRAT_CDMA",
    b"CDMA_IRAT",
    b"LTE_CDMA_IRAT",
    b"SupportIratCdma",
    b"IratToCdma",
    b"CdmaIrat",
    b"RAT_CDMA",
    b"eRAT_CDMA",
    b"RAT_MODE_CDMA",
    b"SYS_RAT_CDMA",
    b"SYS_SYS_MODE_CDMA",
    b"MMODE_RAT_CDMA",
    b"TRM_CDMA",
    b"RFCOM_CDMA",
    b"CDMA_BC0",
    b"BandClass",
    b"band_class",
    b"cal_cdma",
    b"CDMA_CAL",
    b"cdma_cal",
    b"RFNV_CDMA",
    b"rfnv_cdma",
    b"NV_CDMA",
    b"nv_cdma",
]:
    c = img.count(n)
    if c and c < 200:
        print(f"  {n!r}: n={c} @{hex(img.find(n))}")
    elif c >= 200:
        print(f"  {n!r}: n={c} (common)")

# Product / sales / carrier features that might strip CDMA
print("\n=== Product / SKU / feature flags ===")
for n in [
    b"bcProductCode",
    b"ProductCode",
    b"SalesCode",
    b"sales_code",
    b"FEATURE_CDMA",
    b"FEATURE_LTE_ONLY",
    b"LTE_ONLY_DEVICE",
    b"NON_CDMA",
    b"non_cdma",
    b"EU_VARIANT",
    b"ROW_VARIANT",
    b"NA_VARIANT",
    b"VZW",
    b"SPRINT",
    b"USC_",
    b"C_VARIANT",
    b"G_VARIANT",
    b"DS_TCS_GV_CDMA_SUPPORT",
    b"TCS_CDMA",
    b"GetCdmaSupport",
    b"SetCdmaSupport",
    b"CheckCdmaSupport",
]:
    j = img.find(n)
    if j >= 0:
        print(f"  {n}: @{hex(j)}")
        chunk = img[max(0, j - 60) : j + 100]
        cur = bytearray()
        ss = []
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 6:
                    ss.append(cur.decode())
                cur = bytearray()
        print(f"    {ss[:6]}")
