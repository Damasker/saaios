#!/usr/bin/env python3
"""Deep 0x2f50 / 12080 constant-build scan on carved factory-td1a vendor ELFs/SOs.

Prior hunts used bare MOVZ #0x2f50 only. This pass also looks for:
  - MOVZ + MOVK / MOVN / ORR immediate builds of 0x2f50 / 12080
  - little-endian 50 2f / 2f 50 in likely rodata near IPC/oem/SIM strings
  - open/write of /dev/oem_ipc* with msgid args from tables

Offline only. No invent. No live inject.
"""
from __future__ import annotations

import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CARVE = ROOT / "fw" / "cdma-hunt" / "factory-td1a-vendor"
OUT = ROOT / "tmp-vendor-deep-2f50-constbuild.out"

TARGET_IMM = 0x2F50  # 12080
TARGET_BE = bytes([0x2F, 0x50])
TARGET_LE = bytes([0x50, 0x2F])

KEYWORDS = (
    b"oem_ipc",
    b"/dev/oem_ipc",
    b"SIM_INIT",
    b"SIM_INIT_REQ",
    b"IpcTxSim",
    b"msgid",
    b"message_id",
    b"MessageId",
    b"oem_ipc_message",
    b"sit_ipc",
    b"SitOem",
    b"USIM",
    b"VerifyPin",
    b"WRITE",
    b"open(",
)

# AArch64 encodings (LE words)
# MOVZ Wd,#imm16,LSL#0 : 0x52800000 | (imm<<5) | rd
# MOVZ Xd,#imm16,LSL#0 : 0xD2800000 | (imm<<5) | rd
# MOVK Wd,#imm16,LSL#0 : 0x72800000 | (imm<<5) | rd
# MOVK Xd,#imm16,LSL#0 : 0xF2800000 | (imm<<5) | rd
# MOVN Wd,#imm16,LSL#0 : 0x12800000 | (imm<<5) | rd  (result = ~imm)
# ORR  Wd, WZR, #imm   : complex — scan ORR-immediate forms separately


def u32s(data: bytes):
    n = len(data) // 4
    return memoryview(data).cast("I")[:n]


def find_all(data: bytes, needle: bytes, align: int = 1, limit: int = 200) -> list[int]:
    out = []
    start = 0
    while len(out) < limit:
        i = data.find(needle, start)
        if i < 0:
            break
        if align == 1 or (i % align) == 0:
            out.append(i)
        start = i + 1
    return out


def mov_imm16_hits(data: bytes, opcode_base: int, imm: int) -> list[tuple[int, int]]:
    """Return (offset, rd) for MOVZ/MOVK/MOVN with given imm16 hw=0."""
    hits = []
    base = opcode_base | ((imm & 0xFFFF) << 5)
    for rd in range(32):
        w = struct.pack("<I", base | rd)
        for off in find_all(data, w, align=4, limit=500):
            hits.append((off, rd))
    return sorted(hits)


def decode_orr_imm(insn: int) -> int | None:
    """Decode AArch64 ORR (immediate) Wn/Xn result when Rn=31 (ZR).

    ORR Wd, Wn, #imm : sf=0 opc=01 N=0 : 0x32000000 family
    bits: sf N immr imms Rn Rd
    Returns 32-bit immediate or None if not ORR-imm with Rn=ZR.
    """
    # Mask: check top bits for ORR immediate
    # 32-bit ORR imm: 0011001Niiiiiiiissssssnnnnnddddd  -> 0x32000000 base with N in bit 22
    # 64-bit ORR imm: 1011001N...
    sf = (insn >> 31) & 1
    opc = (insn >> 29) & 3
    top = (insn >> 23) & 0x3F
    # ORR imm has opc=01 and bits[28:23]=100100 for logical immediate
    if opc != 0b01:
        return None
    if ((insn >> 24) & 0x1F) != 0b10010:  # bits 28:24 = 10010 for logical imm
        # Actually logical immediate: bits[28:23] = 100100
        pass
    if ((insn >> 23) & 0x3F) != 0b100100:
        return None
    N = (insn >> 22) & 1
    immr = (insn >> 16) & 0x3F
    imms = (insn >> 10) & 0x3F
    rn = (insn >> 5) & 0x1F
    if rn != 31:  # only WZR/XZR forms are "build constant"
        return None
    try:
        val = decode_bitmask(N, immr, imms, 64 if sf else 32)
    except ValueError:
        return None
    if sf == 0:
        val &= 0xFFFFFFFF
    return val


def decode_bitmask(N: int, immr: int, imms: int, regsize: int) -> int:
    """ARMv8 logical immediate decode (simplified)."""
    length = 0
    if N == 1:
        length = 6
    else:
        # highest set bit in (~imms) among low bits determines element size
        t = (~imms) & 0x3F
        if t == 0:
            raise ValueError("invalid")
        length = t.bit_length() - 1
        if length < 1:
            raise ValueError("invalid")
    esize = 1 << length
    if esize > regsize:
        raise ValueError("esize")
    levels = esize - 1
    S = imms & levels
    R = immr & levels
    if S == levels:
        raise ValueError("all ones")
    welem = ((1 << (S + 1)) - 1)
    # ROR welem by R within esize
    welem = ((welem >> R) | (welem << (esize - R))) & ((1 << esize) - 1)
    out = 0
    for i in range(0, regsize, esize):
        out |= welem << i
    return out


def scan_orr_builds(data: bytes, target: int) -> list[tuple[int, int]]:
    hits = []
    mv = u32s(data)
    for i, insn in enumerate(mv):
        val = decode_orr_imm(int(insn))
        if val is None:
            continue
        if (val & 0xFFFFFFFF) == target or (val & 0xFFFF) == target:
            hits.append((i * 4, val))
    return hits


def scan_movz_movk_pairs(data: bytes, lo16: int, hi16: int = 0) -> list[dict]:
    """Find MOVZ Rd,#lo then nearby MOVK Rd,#hi (same rd) within +/- 32 insn."""
    movz_w = mov_imm16_hits(data, 0x52800000, lo16)
    movz_x = mov_imm16_hits(data, 0xD2800000, lo16)
    results = []
    for off, rd in movz_w + movz_x:
        window = data[max(0, off - 128) : off + 128]
        base = max(0, off - 128)
        # scan window for MOVK same rd with hi16 (often 0 for 0x2f50)
        for hw, opc_w, opc_x in (
            (0, 0x72800000, 0xF2800000),
            (1, 0x72A00000, 0xF2A00000),
            (2, 0x72C00000, 0xF2C00000),
            (3, 0x72E00000, 0xF2E00000),
        ):
            for opc in (opc_w, opc_x):
                w = struct.pack("<I", opc | ((hi16 & 0xFFFF) << 5) | rd)
                for rel in find_all(window, w, align=4, limit=20):
                    abs_off = base + rel
                    if abs_off == off:
                        continue
                    results.append(
                        {
                            "movz": off,
                            "movk": abs_off,
                            "rd": rd,
                            "hw": hw,
                            "hi": hi16,
                            "built": lo16 | (hi16 << (16 * hw)) if hw else lo16,
                        }
                    )
        # also MOVK with ANY imm16 on same rd within window — report if final &0xffff==lo
        # (already have movz lo); look for movk that completes pointer-like values
        for rel in range(0, len(window) - 3, 4):
            insn = struct.unpack_from("<I", window, rel)[0]
            abs_off = base + rel
            if abs_off == off:
                continue
            # MOVK W/X any hw
            if (insn & 0xFF800000) not in (
                0x72800000,
                0x72A00000,
                0x72C00000,
                0x72E00000,
                0xF2800000,
                0xF2A00000,
                0xF2C00000,
                0xF2E00000,
            ):
                continue
            if (insn & 0x1F) != rd:
                continue
            imm = (insn >> 5) & 0xFFFF
            hw = (insn >> 21) & 0x3
            built = lo16 | (imm << (16 * hw))
            # skip if already recorded
            if any(r["movk"] == abs_off and r["movz"] == off for r in results):
                continue
            results.append(
                {
                    "movz": off,
                    "movk": abs_off,
                    "rd": rd,
                    "hw": hw,
                    "hi": imm,
                    "built": built,
                    "note": "any-movk",
                }
            )
    return results


def scan_movn_not(data: bytes, target: int) -> list[tuple[int, int, int]]:
    """MOVN Rd,#imm where (~imm)&0xffff == target or (~imm)==target."""
    hits = []
    # For target 0x2f50, imm would be (~0x2f50)&0xffff = 0xd0af
    inv16 = (~target) & 0xFFFF
    for opc in (0x12800000, 0x92800000):  # MOVN W / MOVN X hw=0
        for off, rd in mov_imm16_hits(data, opc, inv16):
            hits.append((off, rd, inv16))
    return hits


def ascii_near(data: bytes, off: int, win: int = 0x200) -> list[str]:
    chunk = data[max(0, off - win) : off + win]
    out = []
    cur = bytearray()
    for b in chunk:
        if 32 <= b < 127:
            cur.append(b)
        else:
            if len(cur) >= 6:
                t = cur.decode("ascii", "replace")
                low = t.lower()
                if any(
                    k in low
                    for k in (
                        "sim",
                        "oem",
                        "ipc",
                        "ril",
                        "init",
                        "sit",
                        "modem",
                        "msgid",
                        "message",
                        "uicc",
                        "usim",
                        "catalog",
                        "write",
                        "open",
                    )
                ):
                    out.append(t[:100])
            cur = bytearray()
    seen = set()
    uniq = []
    for t in out:
        if t not in seen:
            seen.add(t)
            uniq.append(t)
    return uniq[:12]


def find_strings(data: bytes, needles: tuple[bytes, ...]) -> dict[bytes, list[int]]:
    res = {}
    for n in needles:
        res[n] = find_all(data, n, limit=40)
    return res


def rodata_le_near_keywords(data: bytes) -> list[dict]:
    """Find LE 50 2f occurrences near keyword strings (within 0x400)."""
    keyword_offs: list[int] = []
    for n in KEYWORDS:
        keyword_offs.extend(find_all(data, n, limit=80))
    keyword_offs = sorted(set(keyword_offs))
    le_hits = find_all(data, TARGET_LE, limit=2000)
    be_hits = find_all(data, TARGET_BE, limit=2000)
    near = []
    for kind, hits in (("le50_2f", le_hits), ("be2f_50", be_hits)):
        for h in hits:
            # skip if looks like code MOVZ (already covered) — still report if near kw
            close = [k for k in keyword_offs if abs(k - h) <= 0x400]
            if not close:
                continue
            # prefer non-code-aligned or in stringy regions
            near.append(
                {
                    "kind": kind,
                    "off": h,
                    "kw_near": close[:5],
                    "strings": ascii_near(data, h, 0x180),
                }
            )
            if len(near) >= 80:
                return near
    return near


def scan_oem_ipc_open_write(data: bytes) -> dict:
    """Locate /dev/oem_ipc* strings and nearby open/write PLT-ish patterns + tables."""
    paths = {}
    for p in (b"/dev/oem_ipc0", b"/dev/oem_ipc1", b"/dev/oem_ipc"):
        paths[p.decode()] = find_all(data, p, limit=20)
    # look for msgid table patterns: repeating u16 including 0x2f50
    tables = []
    le = TARGET_LE
    start = 0
    while len(tables) < 40:
        i = data.find(le, start)
        if i < 0:
            break
        if i % 2 == 0:
            # check neighbors for other SIM bank ids (0x2f51..0x2f58 etc)
            window = data[max(0, i - 64) : i + 64]
            ids = []
            for j in range(0, len(window) - 1, 2):
                v = window[j] | (window[j + 1] << 8)
                if 0x2F00 <= v <= 0x2FFF:
                    ids.append(v)
            if len(set(ids)) >= 3:
                tables.append({"off": i, "ids_near": sorted(set(ids))[:16]})
        start = i + 2
    return {"paths": paths, "msgid_tables_near_2f50": tables[:20]}


def analyze_file(path: Path) -> list[str]:
    lines = [f"\n{'=' * 72}", f"FILE {path.name} size={path.stat().st_size}"]
    data = path.read_bytes()
    if not data.startswith(b"\x7fELF"):
        lines.append("NOT_ELF (still scanning)")

    # 1) bare MOVZ
    mz_w = mov_imm16_hits(data, 0x52800000, TARGET_IMM)
    mz_x = mov_imm16_hits(data, 0xD2800000, TARGET_IMM)
    lines.append(f"MOVZ_W #0x2f50 count={len(mz_w)} offs={[hex(o) for o,_ in mz_w[:8]]}")
    lines.append(f"MOVZ_X #0x2f50 count={len(mz_x)} offs={[hex(o) for o,_ in mz_x[:8]]}")

    # 2) MOVZ+MOVK pairs
    pairs = scan_movz_movk_pairs(data, TARGET_IMM, 0)
    # filter: built == 0x2f50 exactly preferred; also report pointer-like
    exact = [p for p in pairs if p["built"] == TARGET_IMM]
    other = [p for p in pairs if p["built"] != TARGET_IMM][:12]
    lines.append(f"MOVZ+MOVK pairs touching #0x2f50 lo: exact_build={len(exact)} other={len(pairs)-len(exact)}")
    for p in exact[:10]:
        lines.append(
            f"  EXACT movz@{p['movz']:#x} movk@{p['movk']:#x} rd={p['rd']} hw={p['hw']} "
            f"built={p['built']:#x} near={ascii_near(data, p['movz'], 0x100)}"
        )
    for p in other[:8]:
        lines.append(
            f"  OTHER movz@{p['movz']:#x} movk@{p['movk']:#x} rd={p['rd']} hw={p['hw']} "
            f"built={p['built']:#x} {p.get('note','')}"
        )

    # 3) MOVN ~0x2f50
    movn = scan_movn_not(data, TARGET_IMM)
    lines.append(f"MOVN #~0x2f50 (=#0xd0af) count={len(movn)}")
    for off, rd, inv in movn[:6]:
        lines.append(f"  movn@{off:#x} rd={rd} near={ascii_near(data, off, 0x100)}")

    # 4) ORR immediate builds
    orr = scan_orr_builds(data, TARGET_IMM)
    lines.append(f"ORR_imm (=0x2f50 or lo16) count={len(orr)}")
    for off, val in orr[:10]:
        lines.append(f"  orr@{off:#x} val={val:#x} near={ascii_near(data, off, 0x100)}")

    # 5) literal pool / rodata near keywords
    near = rodata_le_near_keywords(data)
    lines.append(f"rodata LE/BE 0x2f50 near keywords count={len(near)}")
    for n in near[:15]:
        lines.append(
            f"  {n['kind']}@{n['off']:#x} kw={[hex(x) for x in n['kw_near']]} str={n['strings'][:6]}"
        )

    # 6) string inventory
    strs = find_strings(
        data,
        (
            b"/dev/oem_ipc0",
            b"/dev/oem_ipc1",
            b"/dev/oem_ipc",
            b"/dev/umts_ipc0",
            b"SIM_INIT_REQ",
            b"SIM_INIT",
            b"IpcTxSimInit",
            b"BuildOemSimRequest",
            b"oem_ipc_message",
            b"SitOem",
        ),
    )
    lines.append("strings:")
    for k, v in strs.items():
        lines.append(f"  {k!r}: {len(v)} {[hex(x) for x in v[:4]]}")

    # 7) oem_ipc open/write + tables
    oem = scan_oem_ipc_open_write(data)
    lines.append(f"oem_ipc paths: { {k: [hex(x) for x in v[:4]] for k,v in oem['paths'].items()} }")
    lines.append(f"msgid tables (>=3 of 0x2fxx near LE 50 2f): {len(oem['msgid_tables_near_2f50'])}")
    for t in oem["msgid_tables_near_2f50"][:12]:
        lines.append(f"  table@{t['off']:#x} ids={[hex(x) for x in t['ids_near']]}")

    # 8) heuristic: encoder if has oem_ipc path AND (exact const build OR msgid table w/ 0x2f50)
    has_oem = any(oem["paths"].values())
    has_exact = bool(exact) or bool(orr) or bool(movn)
    has_table = any(TARGET_IMM in t["ids_near"] for t in oem["msgid_tables_near_2f50"])
    has_sim_init = bool(strs[b"SIM_INIT_REQ"]) or bool(strs[b"SIM_INIT"])
    verdict = "MISS"
    if has_oem and (has_exact or has_table) and has_sim_init:
        verdict = "CANDIDATE_ENCODER"
    elif has_oem and has_table:
        verdict = "OEM_PATH_PLUS_TABLE"
    elif has_exact and has_sim_init:
        verdict = "CONST_PLUS_SIM_INIT_NO_OEM_PATH"
    elif mz_w or mz_x:
        verdict = "MOVZ_ONLY_LIKELY_REGISTRY"
    lines.append(f"VERDICT {verdict}")
    return lines


def main() -> int:
    files = sorted(CARVE.rglob("carved-*"))
    files = [p for p in files if p.is_file() and p.suffix in (".so", ".elf", "")]
    # also include explicit known names
    extra = list(CARVE.glob("carved-*.so")) + list((CARVE / "carved-cbd").glob("carved-*.elf"))
    paths = sorted(set(files + extra), key=lambda p: str(p))
    lines = [
        "Deep 0x2f50 constant-build scan (MOVZ/MOVK/MOVN/ORR + rodata + oem_ipc tables)",
        f"carve_dir={CARVE}",
        f"files={len(paths)}",
    ]
    if not paths:
        lines.append("NO_CARVED_FILES")
    for p in paths:
        try:
            lines.extend(analyze_file(p))
        except Exception as e:
            lines.append(f"\nERROR {p}: {e}")
    # summary
    lines.append("\n" + "=" * 72)
    lines.append("SUMMARY")
    for ln in lines:
        if ln.startswith("VERDICT ") or ln.startswith("FILE "):
            lines.append(f"  {ln}")  # will duplicate — fix below
    # rebuild summary cleanly
    summary = ["\n" + "=" * 72, "SUMMARY"]
    cur = None
    for ln in lines:
        if ln.startswith("FILE "):
            cur = ln
        if ln.startswith("VERDICT "):
            summary.append(f"{cur} -> {ln}")
    text = "\n".join(lines + summary) + "\n"
    OUT.write_text(text, encoding="utf-8")
    print(text)
    print(f"wrote {OUT}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
