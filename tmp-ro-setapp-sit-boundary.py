#!/usr/bin/env python3
"""Challenge: is SET_APP / GET_APP an AP-facing SIT opcode, or CP-internal only?

Evidence sources:
  - sit-stream.so / libsitril.so / sit-base.so (AP HAL builders)
  - MAIN probe-b (CP): SET_APP callers, SIT dispatch near SET_APP, Present writers
Never invent opcodes; report only proven strings/ids/BL sites.
"""
from __future__ import annotations

import re
import struct
from pathlib import Path

DIAG_WIN = Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics")
DIAG_WSL = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics")
DIAG = DIAG_WSL if DIAG_WSL.exists() else DIAG_WIN
STREAM = DIAG / "sit-stream.so"
LIBSIT = DIAG / "libsitril.so"
BASE = DIAG / "sit-base.so"
MAIN = DIAG / "fw" / "saaios-probe-b-modem.bin"

SET_APP = 0x19916D2
GET_APP = 0x18EC8C0
SET_APP_STRB = 0x1991734
STATUS_BF6 = 0x14FB380  # sole +0xBF6 store (STATUS Present copy)
FN_A = 0x14F692C
READY_SITE = 0x14FB5C6


def u16(b: bytes, off: int) -> int:
    return struct.unpack_from("<H", b, off)[0]


def u32(b: bytes, off: int) -> int:
    return struct.unpack_from("<I", b, off)[0]


def bl_target(img: bytes, off: int) -> int | None:
    """Thumb BL target from instruction at off (must be even)."""
    if off + 4 > len(img) or (off & 1):
        return None
    hi, lo = u16(img, off), u16(img, off + 2)
    if (hi & 0xF800) != 0xF000 or (lo & 0xD000) != 0xD000:
        return None
    s = (hi >> 10) & 1
    imm10 = hi & 0x3FF
    j1 = (lo >> 13) & 1
    j2 = (lo >> 11) & 1
    imm11 = lo & 0x7FF
    i1 = 1 - (j1 ^ s)
    i2 = 1 - (j2 ^ s)
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        imm32 = imm32 - (1 << 32) if imm32 >= (1 << 31) else imm32
    return (off + 4 + imm32) & 0xFFFFFFFF


def find_asciis(data: bytes, needles: list[bytes]) -> list[tuple[bytes, int, bytes]]:
    out = []
    for n in needles:
        start = 0
        while True:
            j = data.find(n, start)
            if j < 0:
                break
            end = data.find(b"\0", j)
            if end < 0 or end - j > 160:
                end = j + min(120, len(data) - j)
            out.append((n, j, data[j:end]))
            start = j + 1
    return out


def elf_dynsym_names(path: Path) -> list[str]:
    """Best-effort: extract printable C++/C symbol-ish strings containing Build/SIT/Sim."""
    data = path.read_bytes()
    # Prefer demangled-looking substrings from .dynstr / rodata via regex on whole file
    names = set()
    for m in re.finditer(rb"[A-Za-z0-9_:]+Build[A-Za-z0-9_]+", data):
        names.add(m.group().decode("ascii", "ignore"))
    for m in re.finditer(rb"ProtocolSim[A-Za-z0-9_:]+", data):
        names.add(m.group().decode("ascii", "ignore"))
    for m in re.finditer(rb"SIT_[A-Z0-9_]+", data):
        names.add(m.group().decode("ascii", "ignore"))
    return sorted(names)


def scan_hal():
    print("=== AP HAL string hits (SET_APP / APP_STATUS / FORCE_READY family) ===")
    needles = [
        b"SET_APP",
        b"GET_APP",
        b"SetApp",
        b"GetApp",
        b"sitSetApp",
        b"SetSimApp",
        b"GetSimApp",
        b"APP_STATUS",
        b"AppStatus",
        b"FORCE_READY",
        b"ForceReady",
        b"ForceSimReady",
        b"SET_SIM_APP",
        b"SET_SIM_STATUS",
        b"SetSimStatus",
        b"SetApplicationStatus",
        b"APPSTATE",
        b"AppState",
        b"SIM_APP_STATE",
        b"SIT_SIM_SET",
        b"SIT_SET_SIM",
    ]
    for label, path in (("sit-stream", STREAM), ("libsitril", LIBSIT), ("sit-base", BASE)):
        data = path.read_bytes()
        hits = find_asciis(data, needles)
        print(f"\n-- {label} ({path.name}) size={len(data)} hits={len(hits)} --")
        for n, off, s in hits[:40]:
            print(f"  @{off:#x} needle={n!r} -> {s[:100]!r}")
        if not hits:
            print("  (none)")

    print("\n=== ProtocolSimBuilder* / BuildSim* / SIT_SIM_* symbol-ish strings ===")
    for label, path in (("sit-stream", STREAM), ("libsitril", LIBSIT), ("sit-base", BASE)):
        names = elf_dynsym_names(path)
        sim_builds = [n for n in names if "BuildSim" in n or "BuildSetSim" in n or "BuildSetUicc" in n]
        sit_sim = [n for n in names if n.startswith("SIT_SIM") or "SIT_SIM" in n]
        setish = [n for n in names if re.search(r"(SetApp|SET_APP|ForceReady|AppStatus|SimApp)", n, re.I)]
        print(f"\n-- {label} BuildSim* count={len(sim_builds)} --")
        for n in sim_builds:
            print(f"  {n}")
        print(f"  SIT_SIM* count={len(sit_sim)}")
        for n in sit_sim[:60]:
            print(f"  {n}")
        print(f"  SetApp/ForceReady-ish count={len(setish)}")
        for n in setish:
            print(f"  {n}")


def scan_main_setapp_vs_sit():
    img = MAIN.read_bytes()
    print(f"\n=== MAIN size={len(img)} SET_APP={SET_APP:#x} GET_APP={GET_APP:#x} ===")

    # Confirm sole +0xBF4 STRB and count BL→SET_APP
    # STRB.W Rt,[Rn,#imm12] encoding: F8C0|imm12 with op=STRB
    bf4_stores = []
    for o in range(0, len(img) - 4, 2):
        hi, lo = u16(img, o), u16(img, o + 2)
        # T3 STRB.W: 1111 1000 1000 Rn | Rt imm12  => hi = 0xF880 | Rn
        if (hi & 0xFFF0) == 0xF880:
            imm12 = lo & 0xFFF
            if imm12 == 0xBF4:
                bf4_stores.append(o)
    print(f"STRB.W #+0xBF4 sites: {[hex(x) for x in bf4_stores]}")

    bls = []
    for o in range(0, len(img) - 4, 2):
        t = bl_target(img, o)
        if t == SET_APP:
            # lookback for MOVS Rd,#imm
            imm = None
            for b in range(o - 2, max(0, o - 24), -2):
                w = u16(img, b)
                if (w & 0xFF00) == 0x2000:  # MOVS Rd,#imm8
                    imm = w & 0xFF
                    break
                if (w & 0xF800) == 0xF000:
                    break  # hit another wide
            bls.append((o, imm))
    print(f"BL→SET_APP count={len(bls)}")
    for o, imm in bls:
        print(f"  @{o:#x} imm={imm}")

    # Strings around SET_APP / GET_APP names in MAIN
    print("\n=== MAIN ASCII near SET_APP/GET_APP name fragments ===")
    for n in [
        b"SET_APP",
        b"GET_APP",
        b"SetAppStatus",
        b"APP_STATUS",
        b"FORCE_READY",
        b"SIT_SIM",
        b"SIM_STATUS",
        b"STATUS",
    ]:
        hits = []
        start = 0
        while len(hits) < 8:
            j = img.find(n, start)
            if j < 0:
                break
            end = img.find(b"\0", j)
            frag = img[j : end if 0 < end - j < 120 else j + 80]
            hits.append((j, frag))
            start = j + 1
        print(f"  needle={n!r} n={len(hits)}")
        for j, frag in hits[:5]:
            print(f"    @{j:#x} {frag[:90]!r}")

    # Does any SIT dispatch (MOVW id 0x02xx then BL SET_APP) exist?
    # Scan for MOVW Rd,#0x02xx within ±0x80 of each BL SET_APP
    print("\n=== MOVW #0x02xx near BL→SET_APP (±0x100) ===")
    for o, imm in bls:
        movs = []
        for b in range(max(0, o - 0x100), min(len(img) - 4, o + 0x100), 2):
            hi, lo = u16(img, b), u16(img, b + 2)
            if (hi & 0xFBF0) == 0xF240:  # MOVW
                imm16 = ((hi & 0xF) << 12) | ((lo & 0x7000) >> 4) | ((hi & 0x0400) << 1) | (lo & 0xFF)
                # simpler extract:
                i = (hi >> 10) & 1
                imm4 = hi & 0xF
                imm3 = (lo >> 12) & 7
                imm8 = lo & 0xFF
                imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8
                if 0x0200 <= imm16 <= 0x02FF:
                    movs.append((b, imm16))
        if movs:
            print(f"  SET_APP BL @{o:#x} imm={imm}: {[(hex(a), hex(v)) for a, v in movs[:12]]}")
        else:
            print(f"  SET_APP BL @{o:#x} imm={imm}: no MOVW 0x02xx nearby")

    # Who calls SET_APP from functions that also handle SIT opcode tables?
    # Search STRB +0xBF6 writers (Present mirror)
    print("\n=== STRB.W #+0xBF6 sites (Present mirror) ===")
    bf6 = []
    for o in range(0, len(img) - 4, 2):
        hi, lo = u16(img, o), u16(img, o + 2)
        if (hi & 0xFFF0) == 0xF880 and (lo & 0xFFF) == 0xBF6:
            bf6.append(o)
    print(f"  sites={[hex(x) for x in bf6]}")

    # Alternate PresentObj[0] writers: STRB to offset 0 of objects used with +0xBF6
    # Prior claim: FN_A @0x14f6a16 stores Present=2. Search other STRB imm #2 near Present getobj.
    print("\n=== READY gate re-check: LDRB +0xBF6 then CMP #2 then BL SET_APP near READY_SITE ===")
    dump_range(img, READY_SITE - 0x40, READY_SITE + 0x20, "READY cluster")


def dump_range(img: bytes, start: int, end: int, title: str):
    print(f"\n-- {title} {start:#x}..{end:#x} --")
    for o in range(start & ~1, end, 2):
        t = bl_target(img, o)
        w = u16(img, o)
        mark = ""
        if t == SET_APP:
            mark = " BL SET_APP"
        elif t == GET_APP:
            mark = " BL GET_APP"
        elif t == FN_A:
            mark = " BL FN_A"
        if (w & 0xFF00) == 0x2000:
            mark += f" MOVS#{w & 0xFF}"
        if mark or (o >= READY_SITE - 8 and o <= READY_SITE + 8):
            print(f"  {o:#x}: {w:04x}{mark}")


def scan_hal_opcodes_from_builders():
    """Extract MOVW #0x02xx immediates near ProtocolSimBuilder Build* bodies via string xrefs is hard;
    instead list unique 0x02xx constants that appear as halfwords in sit-stream near 'BuildSim'."""
    data = STREAM.read_bytes()
    print("\n=== sit-stream: unique MOVW-like 0x02xx immediates (raw halfword pairs heuristic) ===")
    ids = set()
    for o in range(0, len(data) - 4, 2):
        hi, lo = u16(data, o), u16(data, o + 2)
        if (hi & 0xFBF0) == 0xF240:
            i = (hi >> 10) & 1
            imm4 = hi & 0xF
            imm3 = (lo >> 12) & 7
            imm8 = lo & 0xFF
            imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8
            if 0x0200 <= imm16 <= 0x02FF:
                ids.add(imm16)
    print("  SIM-family ids seen as MOVW imm:", " ".join(f"{x:#06x}" for x in sorted(ids)))


def main():
    for p in (STREAM, LIBSIT, BASE, MAIN):
        if not p.exists():
            raise SystemExit(f"missing {p}")
    scan_hal()
    scan_hal_opcodes_from_builders()
    scan_main_setapp_vs_sit()
    print(
        "\n=== VERDICT HINT ===\n"
        "If HAL has no Build* that SETs app_state and no SET_APP/FORCE_READY string,\n"
        "and MAIN BL→SET_APP sites have no nearby MOVW 0x02xx SIT ids,\n"
        "then SET_APP is CP-internal only; 0x0200 is GET mirror of +0xBF4."
    )


if __name__ == "__main__":
    main()
