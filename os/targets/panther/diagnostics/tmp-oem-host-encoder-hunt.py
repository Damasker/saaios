#!/usr/bin/env python3
"""Hunt host OEM IPC encoder for SIM_INIT 0x2f50 / VerifyPin twin 0x2f52.

Evidence-only; no invented frame bytes.
"""
from __future__ import annotations

import gzip
import struct
from pathlib import Path

ROOT = Path("/mnt/c/Users/Admin/Projects")
LIBS = {
    "libsitril": ROOT
    / "saaios-modem-research/os/targets/panther/diagnostics/libsitril.so",
    "sit-stream": ROOT
    / "saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so",
    "sit-base": ROOT
    / "saaios-modem-research/os/targets/panther/diagnostics/sit-base.so",
    "libsec-ril-e4": ROOT / "saaios/os/build/e4-www/libsec-ril.so",
    "libsec-ril": ROOT / "saaios/os/build/libsec-ril.so",
    "raw-cbd": ROOT / "saaios/dist/panther/cbd-extract/raw-cbd",
}
MAIN = (
    ROOT
    / "saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
VA0, MAIN_OFF = 0x40010000, 0x16C10
DIAG = ROOT / "saaios-som/os/targets/panther/diagnostics"


def log(*a):
    print(*a, flush=True)


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def cstr(b, o, n=120):
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


def find_all(hay, needle, limit=80):
    out = []
    pos = 0
    while len(out) < limit:
        i = hay.find(needle, pos)
        if i < 0:
            break
        out.append(i)
        pos = i + 1
    return out


def aarch64_movz_imm16(w):
    if (w & 0xFF800000) == 0x52800000:
        imm16 = (w >> 5) & 0xFFFF
        hw = (w >> 21) & 3
        rd = w & 0x1F
        return rd, imm16 << (hw * 16)
    return None


def scan_movz(blob, targets, limit_per=20):
    hits = {t: [] for t in targets}
    for i in range(0, len(blob) - 3, 4):
        w = struct.unpack_from("<I", blob, i)[0]
        m = aarch64_movz_imm16(w)
        if not m:
            continue
        rd, imm = m
        if imm in targets and len(hits[imm]) < limit_per:
            hits[imm].append((i, rd))
    return hits


def dump_near(blob, off, before=64, after=96):
    lo = max(0, off - before)
    hi = min(len(blob), off + after)
    return " ".join(f"{x:02x}" for x in blob[lo:hi])


def expand_str(blob, h, maxlen=180):
    a = h
    while a > 0 and 32 <= blob[a - 1] < 127:
        a -= 1
    b = h
    while b < len(blob) and blob[b] and b - a < maxlen:
        b += 1
    return blob[a:b].decode("ascii", "ignore")


def strings_with(blob, needles):
    res = {}
    for n in needles:
        nb = n if isinstance(n, bytes) else n.encode()
        hits = find_all(blob, nb, 30)
        res[n] = [(h, expand_str(blob, h)) for h in hits]
    return res


def thumb_movw_hits(blob, imm, band=None, limit=30):
    lo, hi = band if band else (0, len(blob) - 4)
    hits = []
    i = lo & ~1
    while i < hi and len(hits) < limit:
        w = u16(blob, i)
        w2 = u16(blob, i + 2)
        if (w & 0xFBF0) == 0xF240:
            i_bit = (w >> 10) & 1
            imm4 = w & 0xF
            imm3 = (w2 >> 12) & 7
            rd = (w2 >> 8) & 0xF
            imm8 = w2 & 0xFF
            val = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
            if val == imm:
                hits.append((i, rd))
            i += 4
            continue
        i += 2
    return hits


def main():
    log("=== binary presence ===")
    for k, p in LIBS.items():
        log(f"  {k}: exists={p.exists()} size={p.stat().st_size if p.exists() else 0}")
    log(f"  MAIN: exists={MAIN.exists()} size={MAIN.stat().st_size if MAIN.exists() else 0}")

    ss = LIBS["sit-stream"].read_bytes()
    log("\n=== sit-stream u16le 0x2f50/52/57/58 contexts ===")
    for mid in (0x2F50, 0x2F52, 0x2F57, 0x2F58, 0x2FA1):
        pat = struct.pack("<H", mid)
        hits = find_all(ss, pat, 40)
        log(f"  u16 {hex(mid)} count={len(hits)}")
        for h in hits[:12]:
            win = ss[max(0, h - 16) : h + 24]
            asciiish = sum(1 for c in win if 32 <= c < 127)
            log(f"    @{hex(h)} asciiish={asciiish} hex={dump_near(ss, h, 16, 24)}")
            for back in range(0, 80):
                o = h - back
                if o <= 0:
                    break
                if ss[o - 1] == 0 and 65 <= ss[o] <= 90:
                    s = cstr(ss, o, 80)
                    if s and len(s) > 4:
                        log(f"      near-str@{hex(o)}: {s[:80]}")
                        break

    log("\n=== sit-stream / sit-base / libsitril OEM/IPC strings ===")
    needles = [
        b"oem_ipc",
        b"/dev/oem",
        b"OEM_IPC",
        b"OemIpc",
        b"SIM_INIT",
        b"SIM_VERIFYPIN",
        b"2f50",
        b"0x2f50",
        b"IPC message",
        b"EncodeIpc",
        b"IpcMessage",
        b"message_id",
        b"MessageId",
        b"SIT_OEM",
        b"ProtocolOem",
        b"oem channel",
        b"exynos_ipc",
        b"sipc_fmt",
        b"FmtHeader",
        b"SitHeader",
        b"BuildOem",
        b"SendOem",
        b"WriteOem",
        b"oem_fmt",
        b"IPC_OEM",
        b"SimInit",
        b"RequestSimInit",
        b"OnOem",
        b"DoOem",
    ]
    for name in ("sit-stream", "sit-base", "libsitril"):
        blob = LIBS[name].read_bytes()
        found = strings_with(blob, needles)
        log(f"-- {name} --")
        for n, hits in found.items():
            if hits:
                log(f"  {n!r} x{len(hits)}")
                for h, s in hits[:4]:
                    log(f"    @{hex(h)} {s[:140]}")

    log("\n=== libsitril table entry 0x2f50 / 0x2f52 ===")
    ls = LIBS["libsitril"].read_bytes()
    for mid in (0x2F50, 0x2F52, 0x2F58, 0x0201):
        pat = struct.pack("<H", mid) + b"\x23\x00"
        hits = find_all(ls, pat, 10)
        log(f"  table-ish {hex(mid)}|0x23 hits={ [hex(h) for h in hits] }")
        for h in hits[:2]:
            log(f"    raw@{hex(h)}: {ls[h:h+24].hex()}")

    # demangle-ish
    log("\n=== libsitril symbol-like ===")
    for nb in [
        b"SimInit",
        b"SIM_INIT",
        b"OemHook",
        b"OemIpc",
        b"IpcTx",
        b"SendIpc",
        b"BuildSim",
        b"SitOem",
        b"umts_ipc",
        b"Encode",
        b"message_id",
        b"kMessageId",
        b"Channel",
        b"IoChannel",
    ]:
        hits = find_all(ls, nb, 20)
        if not hits:
            continue
        log(f"  {nb!r} x{len(hits)}")
        for h in hits[:5]:
            log(f"    @{hex(h)} {expand_str(ls, h)[:150]}")

    log("\n=== libsec-ril OemIpc / SIM_INIT ===")
    for name in ("libsec-ril-e4", "libsec-ril"):
        p = LIBS[name]
        if not p.exists():
            log(f"  {name}: missing")
            continue
        blob = p.read_bytes()
        found = strings_with(
            blob,
            [
                b"OemIpc",
                b"SIM_INIT",
                b"IpcTxSim",
                b"sipc_fmt",
                b"/dev/oem_ipc",
                b"oem_ipc0",
                b"SIM_INIT_REQ",
                b"IPC_SIM_INIT",
                b"MakeRawIpc",
                b"IpcHeader",
                b"main_cmd",
                b"sub_cmd",
            ],
        )
        log(f"-- {name} size={len(blob)} --")
        for n, hits in found.items():
            if hits:
                log(f"  {n!r} x{len(hits)}")
                for h, s in hits[:4]:
                    log(f"    @{hex(h)} {s[:150]}")
        mz = scan_movz(blob, {0x2F50, 0x2F52, 0x2F58, 0x0200, 0x0201})
        for t, hs in mz.items():
            if hs:
                log(f"  MOVZ #{hex(t)}: {[(hex(i), rd) for i, rd in hs[:8]]}")

    log("\n=== raw-cbd oem/sim strings ===")
    cbd = LIBS["raw-cbd"].read_bytes()
    found = strings_with(
        cbd,
        [b"oem_ipc", b"/dev/oem", b"SIM_INIT", b"2f50", b"umts_boot", b"ramdump"],
    )
    for n, hits in found.items():
        if hits:
            log(f"  {n!r} x{len(hits)}")
            for h, s in hits[:3]:
                log(f"    @{hex(h)} {s[:120]}")
    mz = scan_movz(cbd, {0x2F50, 0x2F52})
    for t, hs in mz.items():
        log(f"  MOVZ #{hex(t)}: {[(hex(i), rd) for i, rd in hs]}")

    log("\n=== MAIN OEM catalog SIM bank + encode ===")
    img = MAIN.read_bytes()
    for o in range(0x6DE500, 0x6DE900, 28):
        mid = u16(img, o + 2)
        if mid in (0x2F50, 0x2F52, 0x2F57, 0x2F58, 0x2FA1, 0x2FA8, 0x2F48):
            nva = u32(img, o + 4)
            noff = nva - VA0 + MAIN_OFF
            name = cstr(img, noff, 48) if 0 <= noff < len(img) else f"va={hex(nva)}"
            log(
                f"  @{hex(o)} flags={u16(img, o)} id={hex(mid)} meta={hex(u32(img, o + 8))} "
                f"rsp={hex(u16(img, o + 0xC))} +18={u32(img, o + 0x18)} name={name}"
            )

    for s in [
        b"GetHeader()->message_id",
        b"kMessageId<",
        b"Unable to encode IPC message",
        b"Message is not a REQUEST",
        b"Sending data length %u to channel %u",
        b"oem_ipc_message_utils.c",
        b"message_id ==",
        b"encoded size for",
        b"Host interface is not ready",
    ]:
        hits = find_all(img, s, 5)
        log(f"str {s!r}: {[hex(h) for h in hits]}")

    # not-REQUEST decode: find LDR to litpool
    nr = img.find(b"[OEM][IPC] Message is not a REQUEST")
    nr_va = VA0 + (nr - MAIN_OFF)
    log(f"not_request off={hex(nr)} va={hex(nr_va)}")
    lits = find_all(img, struct.pack("<I", nr_va), 10)
    log(f"litpool refs: {[hex(x) for x in lits]}")
    for lit in lits[:3]:
        for i in range(max(0, lit - 0x400), lit, 2):
            w = u16(img, i)
            if (w & 0xF800) == 0x4800:
                imm8 = w & 0xFF
                pc = (i + 4) & ~2
                target = pc + imm8 * 4
                if abs(target - lit) <= 3:
                    log(f"  LDR @{hex(i)} -> {hex(target)} lit={hex(lit)}")
                    start = i
                    for b in range(i, max(0, i - 0xA0), -2):
                        ww = u16(img, b)
                        if ww in (0xB570, 0xB5F0, 0xB5F8) or (ww & 0xFF00) == 0xB500:
                            start = b
                            break
                        if ww == 0xE92D or (u16(img, b) == 0xE92D):
                            start = b
                            break
                    j = start
                    end = start + 0x140
                    while j < end:
                        w = u16(img, j)
                        w2 = u16(img, j + 2)
                        if (w & 0xF800) >= 0xE800:
                            if (w & 0xFBF0) == 0xF240:
                                i_bit = (w >> 10) & 1
                                imm4 = w & 0xF
                                imm3 = (w2 >> 12) & 7
                                rd = (w2 >> 8) & 0xF
                                imm8 = w2 & 0xFF
                                imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                                log(f"    {hex(j)} MOVW r{rd},#{hex(imm)}")
                            elif (w & 0xFBF0) == 0xF2C0:
                                i_bit = (w >> 10) & 1
                                imm4 = w & 0xF
                                imm3 = (w2 >> 12) & 7
                                rd = (w2 >> 8) & 0xF
                                imm8 = w2 & 0xFF
                                imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                                log(f"    {hex(j)} MOVT r{rd},#{hex(imm)}")
                            # LDRB.W / LDRH.W T3
                            elif (w & 0xFFF0) == 0xF890:
                                rn = w & 0xF
                                rt = (w2 >> 12) & 0xF
                                imm12 = w2 & 0xFFF
                                log(f"    {hex(j)} LDRB.W r{rt},[r{rn},#{imm12}]")
                            elif (w & 0xFFF0) == 0xF8B0:
                                rn = w & 0xF
                                rt = (w2 >> 12) & 0xF
                                imm12 = w2 & 0xFFF
                                log(f"    {hex(j)} LDRH.W r{rt},[r{rn},#{imm12}]")
                            j += 4
                        else:
                            if (w & 0xF800) == 0x7800:
                                imm5 = (w >> 6) & 0x1F
                                rn = (w >> 3) & 7
                                rt = w & 7
                                log(f"    {hex(j)} LDRB r{rt},[r{rn},#{imm5}]")
                            elif (w & 0xF800) == 0x8800:
                                imm5 = (w >> 6) & 0x1F
                                rn = (w >> 3) & 7
                                rt = w & 7
                                log(f"    {hex(j)} LDRH r{rt},[r{rn},#{imm5 * 2}]")
                            j += 2

    for mid in (0x2F50, 0x2F52):
        hs = thumb_movw_hits(img, mid, (0x6D0000, 0x6F0000), 20)
        log(f"MOVW #{hex(mid)} OEM band: {[(hex(i), rd) for i, rd in hs]}")

    # phone-config
    log("\n=== phone-config / iod OEM attrs ===")
    for p in list(DIAG.glob("phone-config*")) + list(
        (ROOT / "saaios-modem-research/os/targets/panther/diagnostics").glob(
            "phone-config*"
        )
    ):
        data = p.read_bytes()
        if data[:2] == b"\x1f\x8b":
            try:
                data = gzip.decompress(data)
            except Exception as e:
                log(f"  {p.name}: gzip fail {e}")
                continue
        log(f"  {p} size={len(data)}")
        for nb in [
            b"oem_ipc",
            b"umts_ipc",
            b"link_header",
            b"IPC_FMT",
            b"IPC_OEM",
            b"format",
            b"SIT",
        ]:
            hs = find_all(data, nb, 20)
            if hs:
                log(f"    {nb!r} x{len(hs)} @ {[hex(h) for h in hs[:8]]}")
                for h in hs[:2]:
                    chunk = bytes(
                        c if 32 <= c < 127 else 0x2E for c in data[max(0, h - 16) : h + 48]
                    )
                    log(f"      {chunk}")

    # Search vendor radio HAL dumps under saaios build
    log("\n=== extra vendor blobs with oem_ipc / SIM_INIT_REQ ===")
    search_roots = [
        ROOT / "saaios/os/build",
        ROOT / "saaios/dist/panther",
        ROOT / "saaios-modem-research/os/targets/panther/diagnostics",
    ]
    interesting = []
    for root in search_roots:
        if not root.exists():
            continue
        for p in root.rglob("*"):
            if not p.is_file():
                continue
            if p.stat().st_size > 80_000_000:
                continue
            name = p.name.lower()
            if not any(
                x in name
                for x in (
                    "ril",
                    "sit",
                    "oem",
                    "ipc",
                    "radio",
                    "shannon",
                    "modem",
                    "cbd",
                    "sec-",
                )
            ):
                continue
            if p.suffix.lower() not in (
                ".so",
                ".bin",
                "",
                ".img",
                ".elf",
                ".o",
            ) and "raw-" not in name:
                continue
            interesting.append(p)
    log(f"candidate files: {len(interesting)}")
    for p in interesting[:80]:
        try:
            blob = p.read_bytes()
        except Exception:
            continue
        hits_oem = blob.find(b"/dev/oem_ipc")
        hits_init = blob.find(b"SIM_INIT_REQ")
        hits_2f50 = blob.find(b"\x50\x2f\x23\x00")  # table pattern
        movz = scan_movz(blob, {0x2F50}, 3)[0x2F50] if len(blob) < 20_000_000 else []
        if hits_oem >= 0 or hits_init >= 0 or (hits_2f50 >= 0 and p.suffix == ".so") or movz:
            log(
                f"  HIT {p} oem_ipc={hits_oem} SIM_INIT_REQ={hits_init} "
                f"tab2f50={hits_2f50} movz2f50={[(hex(i), r) for i, r in movz[:4]]}"
            )
            if hits_oem >= 0:
                log(f"    oem_str: {expand_str(blob, hits_oem)[:120]}")
            if hits_init >= 0:
                log(f"    init_str: {expand_str(blob, hits_init)[:120]}")

    log("\n=== MISSING SUMMARY ===")
    log("A) OEM app-layer header layout: NOT recovered")
    log("B) Body 2 bytes for flags=2: NOT recovered")
    log("C) Token/seq rules: NOT recovered")
    log("D) oem_ipcN channel for SIM_INIT: NOT recovered")
    log("Host builder for 0x2f50: NOT FOUND in scanned libs")
    log("DONE")


if __name__ == "__main__":
    main()
