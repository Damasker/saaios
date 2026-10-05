#!/usr/bin/env python3
"""No-CDMA USIM READY path RE on MAIN B (+ optional Verizon AP1A).

Goal: given Verizon AP1A also embeds No CDMA / EnableCdmaRat=0, FN_A/CDMA
cannot be the stock USIM READY path. Remap all SET_APP/GET_APP/+0xBF4
writers and PresentObj[0]=2 stores; classify USIM vs CDMA.

No live I/O. No secrets.
"""
from __future__ import annotations

import struct
import sys
from pathlib import Path

VA = 0x40010000
MAIN = 0x16C10
SET_APP = 0x19916D2
GET_APP = 0x18EC8C0
STATUS = 0x14FB322
FN_A = 0x14F692C
PRESENT_GETOBJ = 0x636C

APP = {
    0: "UNKNOWN",
    1: "DETECTED",
    2: "PIN",
    3: "PUK",
    4: "PERSO",
    5: "READY",
    6: "SET6",
    7: "SET7",
}


def load(path: Path) -> bytes:
    return path.read_bytes()


def u16(img, o):
    return struct.unpack_from("<H", img, o)[0]


def u32(img, o):
    return struct.unpack_from("<I", img, o)[0]


def bl(img, o):
    if o + 4 > len(img):
        return None
    hw, hw2 = u16(img, o), u16(img, o + 2)
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


def movw(img, o):
    hw, hw2 = u16(img, o), u16(img, o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def movt(img, o):
    hw, hw2 = u16(img, o), u16(img, o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def scan_bl(img, target, start=0x1000000, end=0x3C00000):
    hits = []
    o = start
    while o < end - 4:
        if bl(img, o) == target:
            hits.append(o)
        o += 2
    return hits


def infer_r0(img, site, back=0x80):
    imm = None
    notes = []
    for p in range(site - 2, max(0, site - back), -2):
        hw = u16(img, p)
        if (hw & 0xFF00) == 0x2000:  # MOVS r0,#imm
            imm = hw & 0xFF
            notes.append(f"MOVS r0,#{imm}@{hex(p)}")
            break
        r = movw(img, p)
        if r and r[1] == 0:
            imm = r[0]
            notes.append(f"MOVW r0,#{hex(imm)}@{hex(p)}")
            break
        if (hw & 0xFFC7) == 0x4600:  # MOV r0,rm
            rm = (hw >> 3) & 7
            notes.append(f"MOV r0,r{rm}@{hex(p)}")
            for q in range(p - 2, max(0, p - 0x50), -2):
                h2 = u16(img, q)
                if (h2 & 0xFF00) == (0x2000 | (rm << 8)):
                    imm = h2 & 0xFF
                    notes.append(f"MOVS r{rm},#{imm}@{hex(q)}")
                    break
            break
    return imm, notes


def nearby_gate(img, site, back=0xA0):
    has_bf6 = False
    cmp2 = False
    cmp_vals = []
    for p in range(max(0, site - back), site, 2):
        hw, hw2 = u16(img, p), u16(img, p + 2) if p + 2 < len(img) else 0
        if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF6:
            has_bf6 = True
        if (hw & 0xFF00) == 0x2800:
            cmp_vals.append(hw & 0xFF)
            if (hw & 0xFF) == 2:
                cmp2 = True
    return has_bf6, cmp2, cmp_vals


def find_ascii(img, needles):
    out = {}
    for n in needles:
        b = n.encode() if isinstance(n, str) else n
        j = img.find(b)
        out[n if isinstance(n, str) else n.decode("latin1", "replace")] = j
    return out


def classify_window(img, site, win=0x200):
    """Look for USIM/CDMA/SIM keywords in nearby literal pools / log movw pairs."""
    tags = set()
    # scan backwards for MOVW/MOVT building string VAs, resolve if printable
    for p in range(max(0, site - win), site + 0x40, 2):
        r = movw(img, p)
        t = movt(img, p + 4) if p + 4 < len(img) else None
        if not r or not t or r[1] != t[1]:
            continue
        va = (t[0] << 16) | r[0]
        off = MAIN + (va - VA)
        if off < 0 or off + 8 >= len(img):
            continue
        s = bytearray()
        for i in range(off, min(len(img), off + 80)):
            c = img[i]
            if 32 <= c < 127:
                s.append(c)
            else:
                break
        text = s.decode("ascii", "replace")
        low = text.lower()
        if "cdma" in low:
            tags.add("CDMA")
        if "usim" in low or "uicc" in low:
            tags.add("USIM")
        if "sim status" in low or "present" in low:
            tags.add("STATUS")
        if "pin" in low:
            tags.add("PIN")
        if "detect" in low:
            tags.add("DETECT")
    # also raw nearby string hits by walking img for known markers near site
    chunk = img[max(0, site - 0x400) : site + 0x100]
    for k, tag in (
        (b"CDMA", "CDMA"),
        (b"USIM", "USIM"),
        (b"SIM STATUS", "STATUS"),
        (b"Present", "STATUS"),
        (b"DETECTED", "DETECT"),
        (b"Pin1Verified", "PIN"),
        (b"No CDMA", "NO_CDMA"),
    ):
        if k in chunk:
            tags.add(tag)
    return tags


def strb_bf4(img):
    hits = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        hw, hw2 = u16(img, o), u16(img, o + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF4:
            hits.append(o)
        o += 2
    return hits


def strb_bf6(img):
    hits = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        hw, hw2 = u16(img, o), u16(img, o + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
            hits.append(o)
        o += 2
    return hits


def present2_via_636c(img):
    """Find MOVW #0x636c windows that store #2 to [reg,#0]."""
    hits = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        r = movw(img, o)
        if r and r[0] == PRESENT_GETOBJ:
            # look ahead 0x80 for MOVS #2 + STRB [rx,#0]
            window = []
            for p in range(o, min(o + 0xA0, len(img) - 4), 2):
                hw = u16(img, p)
                hw2 = u16(img, p + 2)
                if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
                    window.append(("MOVS2", p, (hw >> 8) & 7))
                # STRB rt,[rn,#0] T1: 0x7000 | imm5=0
                if (hw & 0xF800) == 0x7000 and ((hw >> 6) & 0x1F) == 0:
                    window.append(("STRB0", p, hw & 7, (hw >> 3) & 7))
                # STRB.W rt,[rn,#0]
                if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0:
                    window.append(("STRBW0", p, (hw2 >> 12) & 0xF, hw & 0xF))
            # correlate MOVS2 then STRB0 of same rt
            for i, w in enumerate(window):
                if w[0] != "MOVS2":
                    continue
                rt = w[2]
                for w2 in window[i + 1 :]:
                    if w2[0] in ("STRB0", "STRBW0") and w2[2] == rt:
                        hits.append((o, w[1], w2[1], r[1]))
                        break
        o += 2
    return hits


def find_set5_gate(img, set5_site):
    """Disasm key CMP/LDRB around SET#5."""
    lines = []
    for p in range(set5_site - 0x40, set5_site + 8, 2):
        hw = u16(img, p)
        hw2 = u16(img, p + 2) if p + 2 < len(img) else 0
        extra = ""
        if (hw & 0xFFF0) == 0xF890:
            extra = f" LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2000:
            extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif bl(img, p) is not None:
            extra = f" BL->{hex(bl(img, p))}"
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            lines.append(f"  {hex(p)}: {hw:04x} {hw2:04x}{extra}")
        else:
            lines.append(f"  {hex(p)}: {hw:04x}{extra}")
    return "\n".join(lines)


def analyze(label: str, path: Path, set_app=SET_APP, get_app=GET_APP):
    print(f"\n######## {label}: {path.name} size={path.stat().st_size} ########")
    img = load(path)
    needles = find_ascii(
        img,
        [
            "No CDMA in SupportedRatMap",
            "EnableCdmaRat",
            "No CDMA in InitRapMap",
            "SIM STATUS update: Present",
            "START_NETWORK Ignored: SIM is not ready",
            "MMC_LTEL1_CDMA_MEAS_RESULT_IND",
            "MMC_LTEL1_CDMA_TIMING_LATCH_CNF",
            "USIM ==> SIM_START_IND",
            "USIM <== SIM_INIT_REQ",
            "Waiting for SIM_INIT_REQ",
        ],
    )
    for k, v in needles.items():
        print(f"  str[{k!r}] off={hex(v) if v >= 0 else None}")

    # If SET_APP address differs on Verizon image, rediscover via STRB #BF4 sole
    bf4 = strb_bf4(img)
    print(f"  STRB.W #BF4 count={len(bf4)} sites={[hex(x) for x in bf4]}")
    bf6 = strb_bf6(img)
    print(f"  STRB.W #BF6 count={len(bf6)} sites={[hex(x) for x in bf6]}")

    # Prefer known SET_APP if BL density looks right; else infer from BF4 writer-32
    sa = set_app
    callers = scan_bl(img, sa)
    if len(callers) < 5 and bf4:
        # SET_APP body contains STRB; entry often ~0x60 before STRB
        cand = bf4[0] - 0x62
        # snap to even
        cand &= ~1
        sa = cand
        callers = scan_bl(img, sa)
        print(f"  rediscovered SET_APP~{hex(sa)} callers={len(callers)}")
    else:
        print(f"  SET_APP={hex(sa)} callers={len(callers)}")

    rows = []
    for c in callers:
        imm, notes = infer_r0(img, c)
        bf6g, cmp2, cmps = nearby_gate(img, c)
        tags = classify_window(img, c)
        rows.append((c, imm, bf6g, cmp2, cmps, tags, notes))
        print(
            f"  BL {hex(c)} imm={imm}({APP.get(imm, '?')}) "
            f"bf6={bf6g} cmp2={cmp2} cmps={cmps[-4:]} tags={sorted(tags)} "
            f"| {'; '.join(notes[:2])}"
        )

    for want in (1, 4, 5):
        xs = [r for r in rows if r[1] == want]
        print(f"  --- app_state#{want} ({APP[want]}) n={len(xs)} ---")
        for r in xs:
            print(f"     {hex(r[0])} tags={sorted(r[5])} bf6={r[2]} cmp2={r[3]}")
            if want == 5:
                print(find_set5_gate(img, r[0]))

    p2 = present2_via_636c(img)
    print(f"  PresentObj#636c + MOVS#2 + STRB[*,#0] n={len(p2)}")
    for h in p2:
        tags = classify_window(img, h[0], win=0x300)
        in_fn_a = FN_A <= h[0] <= FN_A + 0x200 or FN_A <= h[1] <= FN_A + 0x200
        print(
            f"    MOVW@ {hex(h[0])} MOVS2@{hex(h[1])} STRB@{hex(h[2])} "
            f"in_FN_A_window={in_fn_a} tags={sorted(tags)}"
        )

    # GET_APP LDRB #BF4 sole?
    ga_hits = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        hw, hw2 = u16(img, o), u16(img, o + 2)
        if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF4:
            # count as GET if followed soon by BX LR / MOV PC pattern — just list
            ga_hits.append(o)
        o += 2
    print(f"  LDRB.W #BF4 count={len(ga_hits)} (first10={[hex(x) for x in ga_hits[:10]]})")

    # START_NETWORK gate pattern: CMP #1/#4/#5 near GET_APP BL
    sn_pat = img.find(b"START_NETWORK Ignored: SIM is not ready")
    print(f"  START_NETWORK ignore str off={hex(sn_pat) if sn_pat>=0 else None}")

    return rows


def main():
    # Prefer WSL /mnt paths; fall back to Windows.
    candidates_b = [
        Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
        Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw\saaios-probe-b-modem.bin"),
    ]
    candidates_v = [
        Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/EXTRACT-ap1a-verizon-modem.bin"),
        Path(r"C:\Users\Admin\Projects\saaios-som\os\targets\panther\diagnostics\fw\cdma-hunt\EXTRACT-ap1a-verizon-modem.bin"),
    ]
    b = next(p for p in candidates_b if p.exists())
    v = next((p for p in candidates_v if p.exists()), candidates_v[0])
    rows_b = analyze("LIVE_MAIN_B", b)
    if v.exists():
        # Verizon image may be full modem.bin (TOC), not raw MAIN — detect
        head = v.read_bytes()[:0x100]
        print(f"\nVerizon head16={head[:16].hex()}")
        if b"TOC" in head or head[:4] == b"\x7fELF" or True:
            # try same MAIN offset; if SET_APP density low, report
            analyze("VERIZON_AP1A_EXTRACT", v)
    print("\n=== MODEL NOTE ===")
    print(
        "If Verizon also has sole READY behind Present==2 and Present=2 only in FN_A,"
        " then stock USIM READY must either (a) hit FN_A despite No-CDMA string"
        " (falsified by string+live), (b) use a Present=2 writer we still miss,"
        " or (c) never take Present0->PIN (keep DETECTED/#1) until camp/SET6/7."
    )


if __name__ == "__main__":
    main()
