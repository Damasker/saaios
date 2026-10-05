#!/usr/bin/env python3
"""Focused OEM host encoder hunt for 0x2f50 / 0x2f52. Evidence-only."""
from __future__ import annotations

import gzip
import struct
from pathlib import Path

ROOT = Path("/mnt/c/Users/Admin/Projects")
DIAG = ROOT / "saaios-som/os/targets/panther/diagnostics"
MR = ROOT / "saaios-modem-research/os/targets/panther/diagnostics"
LIBS = {
    "libsitril": MR / "libsitril.so",
    "sit-stream": MR / "sit-stream.so",
    "sit-base": MR / "sit-base.so",
    "libsec-ril-e4": ROOT / "saaios/os/build/e4-www/libsec-ril.so",
    "libsec-ril": ROOT / "saaios/os/build/libsec-ril.so",
    "raw-cbd": ROOT / "saaios/dist/panther/cbd-extract/raw-cbd",
}
MAIN = MR / "fw/saaios-probe-b-modem.bin"
VA0, MAIN_OFF = 0x40010000, 0x16C10


def log(*a):
    print(*a, flush=True)


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def cstr(b, o, n=100):
    s = bytearray()
    for i in range(o, min(len(b), o + n)):
        c = b[i]
        if c == 0:
            break
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def find_all(hay, needle, limit=50):
    out, pos = [], 0
    while len(out) < limit:
        i = hay.find(needle, pos)
        if i < 0:
            break
        out.append(i)
        pos = i + 1
    return out


def expand(blob, h, maxlen=160):
    a = h
    while a > 0 and 32 <= blob[a - 1] < 127:
        a -= 1
    b = h
    while b < len(blob) and blob[b] and b - a < maxlen:
        b += 1
    return blob[a:b].decode("ascii", "ignore")


def dump(blob, off, before=16, after=32):
    lo, hi = max(0, off - before), min(len(blob), off + after)
    return " ".join(f"{x:02x}" for x in blob[lo:hi])


def movz_hits(blob, imm, limit=10):
    hits = []
    for i in range(0, len(blob) - 3, 4):
        w = struct.unpack_from("<I", blob, i)[0]
        if (w & 0xFF800000) != 0x52800000:
            continue
        imm16 = (w >> 5) & 0xFFFF
        hw = (w >> 21) & 3
        val = imm16 << (hw * 16)
        if val == imm:
            hits.append((i, w & 0x1F))
            if len(hits) >= limit:
                break
    return hits


def main():
    log("=== presence ===")
    for k, p in LIBS.items():
        log(f"  {k}: {p.exists()} {p.stat().st_size if p.exists() else 0}")
    log(f"  MAIN: {MAIN.exists()} {MAIN.stat().st_size if MAIN.exists() else 0}")

    # sit-stream 0x2f52 contexts (prior said count=7)
    ss = LIBS["sit-stream"].read_bytes()
    log("\n=== sit-stream msgid halfwords ===")
    for mid in (0x2F50, 0x2F52, 0x2F57, 0x2F58, 0x2FA1, 0x0201):
        hits = find_all(ss, struct.pack("<H", mid), 40)
        log(f"  {hex(mid)} count={len(hits)}")
        for h in hits[:8]:
            # look for surrounding table of ids
            neigh = []
            for d in range(-32, 33, 2):
                o = h + d
                if 0 <= o < len(ss) - 1:
                    v = u16(ss, o)
                    if 0x2F00 <= v <= 0x2FFF or v in (0x200, 0x201, 0x202):
                        neigh.append((hex(o), hex(v)))
            log(f"    @{hex(h)} {dump(ss, h, 24, 24)}")
            if neigh:
                log(f"      neigh={neigh[:12]}")
            # nearest C string before
            for back in range(1, 96):
                o = h - back
                if o > 0 and ss[o - 1] == 0 and 65 <= ss[o] <= 122:
                    s = cstr(ss, o, 80)
                    if s and len(s) > 5:
                        log(f"      str@{hex(o)}: {s}")
                        break

    # libsitril: registry + builders
    ls = LIBS["libsitril"].read_bytes()
    log("\n=== libsitril registry + builders ===")
    for mid in (0x2F50, 0x2F52, 0x2F58):
        pat = struct.pack("<H", mid) + b"\x23\x00"
        hits = find_all(ls, pat, 5)
        log(f"  tab {hex(mid)}|0x23 @{[hex(h) for h in hits]}")
        for h in hits:
            log(f"    {ls[h:h+24].hex()}")
    for nb in [
        b"/dev/oem_ipc",
        b"oem_ipc0",
        b"SIM_INIT_REQ",
        b"SIM_VERIFYPIN_REQ",
        b"BuildSimInit",
        b"SendSimInit",
        b"OemIpc",
        b"IpcMessage",
        b"message_id",
        b"DoOem",
        b"OnOem",
        b"IoChannel",
        b"umts_ipc0",
    ]:
        hits = find_all(ls, nb, 15)
        if hits:
            log(f"  str {nb!r} x{len(hits)}")
            for h in hits[:4]:
                log(f"    @{hex(h)} {expand(ls, h)[:140]}")
    log(f"  MOVZ #0x2f50: {[(hex(i), r) for i, r in movz_hits(ls, 0x2F50)]}")
    log(f"  MOVZ #0x2f52: {[(hex(i), r) for i, r in movz_hits(ls, 0x2F52)]}")

    sb = LIBS["sit-base"].read_bytes()
    log("\n=== sit-base ===")
    for nb in [b"/dev/oem_ipc", b"oem_ipc", b"SIM_INIT", b"2f50", b"OemIpc", b"IoChannel"]:
        hits = find_all(sb, nb, 10)
        if hits:
            log(f"  {nb!r} x{len(hits)} eg {expand(sb, hits[0])[:120]}")
    log(f"  MOVZ #0x2f50: {movz_hits(sb, 0x2F50)}")
    log(f"  MOVZ #0x2f52: {movz_hits(sb, 0x2F52)}")

    # libsec-ril: is OemIpc Shannon or classic?
    log("\n=== libsec-ril OemIpc path ===")
    for name in ("libsec-ril-e4", "libsec-ril"):
        p = LIBS[name]
        if not p.exists():
            continue
        blob = p.read_bytes()
        log(f"-- {name} --")
        for nb in [
            b"/dev/oem_ipc",
            b"oem_ipc0",
            b"SIM_INIT_REQ",
            b"IPC_SIM",
            b"OemIpcRecord",
            b"EnableOemIpcForwarding",
            b"IpcTxSim",
            b"main_cmd",
            b"sub_cmd",
            b"msg_seq",
            b"sipc",
            b"Shannon",
            b"SIT_",
            b"umts_ipc",
        ]:
            hits = find_all(blob, nb, 8)
            if hits:
                log(f"  {nb!r} x{len(hits)} eg @{hex(hits[0])} {expand(blob, hits[0])[:130]}")
        log(f"  MOVZ #0x2f50: {[(hex(i), r) for i, r in movz_hits(blob, 0x2F50, 5)]}")
        log(f"  MOVZ #0x2f52: {[(hex(i), r) for i, r in movz_hits(blob, 0x2F52, 5)]}")

    cbd = LIBS["raw-cbd"].read_bytes()
    log("\n=== raw-cbd ===")
    for nb in [b"/dev/oem_ipc", b"oem_ipc", b"SIM_INIT", b"/dev/umts_boot", b"/dev/umts_ramdump"]:
        hits = find_all(cbd, nb, 8)
        if hits:
            log(f"  {nb!r} x{len(hits)} eg {expand(cbd, hits[0])[:120]}")
    log(f"  MOVZ #0x2f50: {movz_hits(cbd, 0x2F50)}")

    # Known extra candidates only (no rglob)
    extras = [
        ROOT / "saaios/os/build/libril_sem.so",
        ROOT
        / "saaios/os/build/e4-www/BaseMirror/RIL_Analyzer-bw/ril_binaries/samsung/ril/A536EXXS4AVJ3_A536EOWO4AVI2_ARO/libsec-ril.so",
        ROOT / "saaios/dist/panther/cbd-extract/raw-cbd",
        ROOT / "saaios/dist/panther/bin/saaios-modem-probe-arm64",
        ROOT / "saaios/dist/panther/bin/saaios-cp-boot-arm64",
    ]
    log("\n=== extras ===")
    for p in extras:
        if not p.exists():
            log(f"  missing {p}")
            continue
        blob = p.read_bytes()
        oem = blob.find(b"/dev/oem_ipc")
        init = blob.find(b"SIM_INIT_REQ")
        m250 = movz_hits(blob, 0x2F50, 3)
        log(
            f"  {p.name}: size={len(blob)} oem_ipc={oem} SIM_INIT_REQ={init} "
            f"movz2f50={[(hex(i), r) for i, r in m250]}"
        )
        if oem >= 0:
            log(f"    {expand(blob, oem)[:120]}")
        if init >= 0:
            log(f"    {expand(blob, init)[:120]}")

    # MAIN catalog + not-REQUEST field offsets
    img = MAIN.read_bytes()
    log("\n=== MAIN catalog SIM OEM ===")
    for o in range(0x6DE500, 0x6DE900, 28):
        mid = u16(img, o + 2)
        if mid in (0x2F48, 0x2F50, 0x2F52, 0x2F57, 0x2F58, 0x2FA1, 0x2FA8):
            nva = u32(img, o + 4)
            noff = nva - VA0 + MAIN_OFF
            name = cstr(img, noff, 48) if 0 <= noff < len(img) else "?"
            log(
                f"  @{hex(o)} flags={u16(img, o)} id={hex(mid)} meta={hex(u32(img, o + 8))} "
                f"rsp={hex(u16(img, o + 0xC))} u32@+0x10={hex(u32(img, o + 0x10))} "
                f"u32@+0x14={hex(u32(img, o + 0x14))} +18={u32(img, o + 0x18)} name={name}"
            )

    # Decode "not a REQUEST": find type byte compare near string use
    nr = img.find(b"[OEM][IPC] Message is not a REQUEST")
    nr_va = VA0 + (nr - MAIN_OFF)
    log(f"\nnot_request off={hex(nr)} va={hex(nr_va)}")
    lits = find_all(img, struct.pack("<I", nr_va), 8)
    log(f"lits={[hex(x) for x in lits]}")
    for lit in lits[:2]:
        # scan back 0x300 for LDRB imm offsets 0..31 and CMP #imm
        start = max(0, lit - 0x300) & ~1
        j = start
        while j < lit:
            w = u16(img, j)
            w2 = u16(img, j + 2)
            if (w & 0xF800) >= 0xE800:
                if (w & 0xFFF0) == 0xF890:
                    rn = w & 0xF
                    rt = (w2 >> 12) & 0xF
                    imm12 = w2 & 0xFFF
                    if imm12 <= 0x40:
                        log(f"  {hex(j)} LDRB.W r{rt},[r{rn},#{imm12}]")
                elif (w & 0xFFF0) == 0xF8B0:
                    rn = w & 0xF
                    rt = (w2 >> 12) & 0xF
                    imm12 = w2 & 0xFFF
                    if imm12 <= 0x40:
                        log(f"  {hex(j)} LDRH.W r{rt},[r{rn},#{imm12}]")
                elif (w & 0xFF70) == 0xF010:  # TST/AND? skip
                    pass
                # CMP Rn, #imm12 T2: F1B0 | ...
                elif (w & 0xFBF0) == 0xF1B0:
                    rn = w & 0xF
                    imm8 = w2 & 0xFF
                    log(f"  {hex(j)} CMP.W r{rn},#{hex(imm8)} (approx)")
                j += 4
            else:
                if (w & 0xF800) == 0x7800:
                    imm5 = (w >> 6) & 0x1F
                    rn = (w >> 3) & 7
                    rt = w & 7
                    log(f"  {hex(j)} LDRB r{rt},[r{rn},#{imm5}]")
                elif (w & 0xF800) == 0x2800:  # CMP Rn,#imm8
                    rn = (w >> 8) & 7
                    imm8 = w & 0xFF
                    log(f"  {hex(j)} CMP r{rn},#{hex(imm8)}")
                j += 2

    # phone-config oem_ipc
    log("\n=== phone-config ===")
    for p in list(DIAG.glob("phone-config*")) + list(MR.glob("phone-config*")):
        data = p.read_bytes()
        if data[:2] == b"\x1f\x8b":
            data = gzip.decompress(data)
        log(f"  {p.name} decompressed={len(data)}")
        for nb in [b"oem_ipc", b"umts_ipc", b"IPC_OEM", b"IPC_FMT", b"link_header"]:
            hs = find_all(data, nb, 15)
            if hs:
                log(f"    {nb!r} @{[hex(h) for h in hs[:10]]}")
                for h in hs[:2]:
                    chunk = bytes(
                        c if 32 <= c < 127 else 0x2E for c in data[max(0, h - 20) : h + 40]
                    )
                    log(f"      {chunk}")

    # Live soft SIT VerifyPin as contrast (already known) — document only
    log("\n=== evidenced vs missing ===")
    log("Evidenced kernel: userspace writes APP payload; EXYNOS 12B prepended if link_header")
    log("Evidenced soft SIT FMT (umts_ipc0): type:u8 pad:u8 id:u16le len:u16le token:u32le + body")
    log("Evidenced catalog 0x2f50: flags/body_hint=2 meta=0x10104 rsp=0")
    log("Evidenced catalog 0x2f52: see dump above")
    log("MISSING A: OEM app header field order/size (SIT-12B reuse UNPROVEN)")
    log("MISSING B: 2-byte body contents for flags=2")
    log("MISSING C: token/seq/transaction rules")
    log("MISSING D: which oem_ipcN stock uses")
    log("Host encoder with /dev/oem_ipc write + 0x2f50 builder: NOT FOUND")
    log("DONE")


if __name__ == "__main__":
    main()
