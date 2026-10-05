#!/usr/bin/env python3
"""RO: compare modem A/B MAIN for pin1==DISABLED(#3) → SET_APP#5 path.
No secrets; file offsets only."""
from __future__ import annotations
import struct
import sys
from pathlib import Path

SET_APP = 0x19916D2  # B known; re-find per image via string/prolog if needed
GET_APP = 0x18EC8C0

def bl_target(off: int, hw: int, hw2: int) -> int | None:
    # Thumb BL: F000..F7FF / F800..FFFF encoding
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000:
        return None
    s = (hw >> 10) & 1
    imm10 = hw & 0x3FF
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    i1 = 1 - (j1 ^ s)
    i2 = 1 - (j2 ^ s)
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        imm32 = imm32 - (1 << 32) if imm32 >= (1 << 31) else imm32
    return (off + 4 + imm32) & 0xFFFFFFFF

def find_movw_imm(data: bytes, imm: int) -> list[int]:
    # MOVW Rd, #imm16: F240|... complicated; search CMP.W Rn,#3 near STATUS
    hits = []
    # CMP.W Rn,#imm8 : F1Bx 0Fimm  (imm8)
    # also CMP Rn,#imm : 2ximm for low regs
    pat = bytes([0x0F, imm & 0xFF])  # second half of CMP.W ... #imm — too loose
    return hits

def scan_image(path: Path, label: str) -> None:
    data = path.read_bytes()
    print(f"\n=== {label} {path.name} size={len(data)} ===")
    # TOC: first entry count
    if len(data) < 64:
        print("too small"); return
    n = struct.unpack_from("<I", data, 0)[0]
    if n == 0 or n > 16:
        # sometimes first field is magic; try string TOC
        print("TOC idx unusual", n)
    # Find MAIN by name in TOC entries (32-byte records typical)
    main_off = main_size = None
    # Shannon TOC: each entry 32 bytes, name at +0
    for i in range(1, 17):
        e = i * 32
        if e + 32 > len(data):
            break
        name = data[e:e+12].split(b"\0", 1)[0]
        if name == b"MAIN":
            # layout varies: b_off, m_off, size, ...
            # Known panther: b_off at +12 or use prior knowledge
            vals = struct.unpack_from("<8I", data, e)
            print(f"MAIN toc raw: {[hex(v) for v in vals]}")
            # common: name(12) pad, load/b_off, m_off, size, ...
            # From probe: boot b_off, MAIN b_off known from live
            break
    # Locate MAIN by searching TOC name and reading official layout from probe docs:
    # toc_entry: name[12], b_off, m_off, size, ...
    for i in range(0, 16):
        e = i * 32
        name = data[e:e+12].split(b"\0", 1)[0]
        if name != b"MAIN":
            continue
        b_off, m_off, size = struct.unpack_from("<III", data, e + 12)
        print(f"MAIN b_off={hex(b_off)} m_off={hex(m_off)} size={hex(size)}")
        main_off, main_size = b_off, size
        break
    if main_off is None:
        # fallback known B offsets from probe
        print("MAIN TOC not found; try string scan for SET_APP cluster only")
        main_off, main_size = 0, len(data)
    main = data[main_off:main_off + main_size]
    base = main_off

    # Find all BL → candidates: search for STRB.W #+0xBF4 pattern near SET_APP
    # STRB.W Rt,[Rn,#0xBF4] = F88x BF4? encoding F880|Rt<<12 ... 
    # Prior: SET_APP @ file 0x19916d2 relative to MAIN start in B
    # Search MOV imm #5 then BL nearby, and CMP #3 then BL SET#5

    # Find SET_APP by unique STRB +0xBF4 (sole site)
    bf4_sites = []
    for o in range(0, len(main) - 4, 2):
        hw = main[o] | (main[o+1] << 8)
        hw2 = main[o+2] | (main[o+3] << 8)
        # STRB.W Rt, [Rn, #imm12] : F880|Rn , (Rt<<12)|imm12
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0x0FFF) == 0xBF4:
            bf4_sites.append(base + o)
    print(f"STRB.W #+0xBF4 sites: {[hex(x) for x in bf4_sites]}")
    set_app = None
    if len(bf4_sites) == 1:
        # function usually starts a bit before store
        set_app = bf4_sites[0] - 0x62  # B: 0x1991734-0x19916d2=0x62
        print(f"inferred SET_APP entry ~{hex(set_app)} (store-0x62)")
    elif bf4_sites:
        set_app = bf4_sites[0] - 0x62

    # Collect all BL targets matching set_app
    bl_to_set = []
    if set_app is not None:
        for o in range(0, len(main) - 4, 2):
            hw = main[o] | (main[o+1] << 8)
            hw2 = main[o+2] | (main[o+3] << 8)
            t = bl_target(base + o, hw, hw2)
            if t == set_app:
                # look back 32 bytes for MOV imm into arg (=app_state)
                window = main[max(0, o - 48):o]
                imm = None
                # MOVS Rd, #imm8: 00100ddd iiiiiiii
                for j in range(0, len(window) - 1, 2):
                    w = window[j] | (window[j+1] << 8)
                    if (w & 0xF800) == 0x2000:
                        imm = w & 0xFF
                bl_to_set.append((base + o, imm))
        print(f"BL→SET_APP count={len(bl_to_set)}")
        for site, imm in bl_to_set:
            print(f"  BL {hex(site)} last_MOVS_imm8={imm}")

    # STATUS cluster: search "Present:" or known offset region
    # Look for LDRB.W [Rn,#0xBF6] then CMP #2 then BL SET with imm5
    ready_gate = []
    for o in range(0, len(main) - 8, 2):
        hw = main[o] | (main[o+1] << 8)
        hw2 = main[o+2] | (main[o+3] << 8)
        if (hw & 0xFFF0) == 0xF890 and (hw2 & 0x0FFF) == 0xBF6:
            # scan forward 64B for CMP #2 and BL SET_APP
            has_cmp2 = False
            has_cmp3_pin = False
            bl_imm5 = False
            for k in range(o, min(o + 80, len(main) - 4), 2):
                w = main[k] | (main[k+1] << 8)
                w2 = main[k+2] | (main[k+3] << 8)
                # CMP.W Rn,#2 : F1Bx 0F02
                if (w & 0xFFF0) == 0xF1B0 and (w2 & 0x0FFF) == 0x0F02:
                    has_cmp2 = True
                if (w & 0xFFF0) == 0xF1B0 and (w2 & 0x0FFF) == 0x0F03:
                    has_cmp3_pin = True
                # CMP Rn,#2 short: 2x02
                if (w & 0xFF00) == 0x2800 and (w & 0xFF) == 2:
                    has_cmp2 = True
                if (w & 0xFF00) == 0x2800 and (w & 0xFF) == 3:
                    has_cmp3_pin = True
                t = bl_target(base + k, w, w2)
                if set_app is not None and t == set_app:
                    # MOVS before
                    for j in range(max(0, k - 24), k, 2):
                        ww = main[j] | (main[j+1] << 8)
                        if (ww & 0xF800) == 0x2000 and (ww & 0xFF) == 5:
                            bl_imm5 = True
            if has_cmp2 or bl_imm5:
                ready_gate.append((base + o, has_cmp2, has_cmp3_pin, bl_imm5))
    print(f"LDRB +0xBF6 near READY markers: {len(ready_gate)}")
    for site, c2, c3, i5 in ready_gate[:12]:
        print(f"  {hex(site)} PresentCMP2={c2} nearbyCMP3={c3} SET#5={i5}")

    # Critical: CMP pin1 field #3 then branch to SET#5 — search in STATUS 0x14fb3xx region for B
    # For any image: find SET#5 BL sites and dump preceding 96 bytes for CMP #3
    print("SET#5 sites with preceding CMP#3 (pin1 DISABLED candidate):")
    found_disabled_ready = False
    for site, imm in bl_to_set:
        if imm != 5:
            continue
        rel = site - base
        win_start = max(0, rel - 96)
        window = main[win_start:rel]
        cmps = []
        for j in range(0, len(window) - 3, 2):
            w = window[j] | (window[j+1] << 8)
            w2 = window[j+2] | (window[j+3] << 8)
            if (w & 0xFFF0) == 0xF1B0 and (w2 & 0xFF00) == 0x0F00:
                cmps.append((base + win_start + j, w2 & 0xFF))
            if (w & 0xFF00) == 0x2800:
                cmps.append((base + win_start + j, w & 0xFF))
        print(f"  SET#5 BL {hex(site)} preceding CMPs: {[(hex(a), v) for a,v in cmps]}")
        # DISABLED→READY would be CMP #3 then (without Present==2) BL #5
        if any(v == 3 for _, v in cmps):
            # check if Present CMP#2 also in window
            if not any(v == 2 for _, v in cmps):
                print(f"  *** CANDIDATE DISABLED→READY (CMP#3 without CMP#2) at {hex(site)}")
                found_disabled_ready = True
            else:
                print(f"  (CMP#3 with CMP#2 also present — likely Present/PERSO cluster, not pin1 DISABLED skip)")
    if not found_disabled_ready:
        print("  NO pin1=#3 → SET_APP#5 without Present==2 gate found")

    # Hash MAIN for identity
    import hashlib
    h = hashlib.sha256(main[: min(len(main), 0x100000)]).hexdigest()[:16]
    print(f"MAIN first-1MiB SHA256 prefix={h}")

if __name__ == "__main__":
    root = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw")
    if not root.exists():
        root = Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw")
    for label, name in [("A", "saaios-probe-a-modem.bin"), ("B", "saaios-probe-b-modem.bin")]:
        p = root / name
        if p.exists():
            scan_image(p, label)
        else:
            print("missing", p)
