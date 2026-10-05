#!/usr/bin/env python3
"""Prove whether SitOem protobuf / oem_ipc demuxes to catalog msgid 0x2f50.

Evidence axes:
  A) SitOem carved .so — PayloadCase / message names / msgid immediates
  B) MAIN — oem_ipc / protobuf / sit_ipc strings vs catalog 0x2f50 consumers
  C) vendor.img strings — other openers of /dev/oem_ipc* + SIM_INIT / 0x2f50
"""
from __future__ import annotations

import re
import struct
from pathlib import Path

OUT = Path(__file__).with_suffix(".out")
lines: list[str] = []


def log(s: str = "") -> None:
    print(s)
    lines.append(s)


def u16(b: bytes, o: int) -> int:
    return struct.unpack_from("<H", b, o)[0]


def u32(b: bytes, o: int) -> int:
    return struct.unpack_from("<I", b, o)[0]


def u64(b: bytes, o: int) -> int:
    return struct.unpack_from("<Q", b, o)[0]


def cstr(b: bytes, o: int, n: int = 120) -> str:
    end = b.find(b"\x00", o, o + n)
    if end < 0:
        end = o + n
    return b[o:end].decode("ascii", "replace")


def find_all(hay: bytes, needle: bytes) -> list[int]:
    out = []
    start = 0
    while True:
        i = hay.find(needle, start)
        if i < 0:
            break
        out.append(i)
        start = i + 1
    return out


# ---------- paths ----------
def p(*parts: str) -> Path:
    """Prefer /mnt/c under WSL; fall back to Windows drive path."""
    wsl = Path("/mnt/c").joinpath(*parts)
    if wsl.exists():
        return wsl
    return Path("C:\\").joinpath(*parts)


SITOEM = p(
    "Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-25e7b000.so"
)
OEMLOG = p(
    "Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-cf64000.so"
)
MAIN_PATH = p(
    "Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
VENDOR = p(
    "Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/vendor.img"
)

VA0 = 0x40000000
MAIN_OFF = 0x16C10


def va_to_off(va: int) -> int:
    return MAIN_OFF + (va - VA0)


def off_to_va(o: int) -> int:
    return VA0 + (o - MAIN_OFF)


# ============================================================
# A) SitOem protobuf surface
# ============================================================
log("=== A) SitOem carved oem_ipc0 (.so) ===")
so = SITOEM.read_bytes()
log(f"size={len(so)}")

# Collect ASCII strings >= 6
so_strings: list[tuple[int, str]] = []
i = 0
while i < len(so):
    if 32 <= so[i] < 127:
        j = i
        while j < len(so) and 32 <= so[j] < 127:
            j += 1
        if j - i >= 6:
            so_strings.append((i, so[i:j].decode()))
        i = j + 1
    else:
        i += 1

# PayloadCase / protobuf message type names
payload_like = [
    (o, s)
    for o, s in so_strings
    if any(
        k in s
        for k in (
            "PayloadCase",
            "kPing",
            "kConfig",
            "kThermal",
            "kSim",
            "SIM_",
            "SimInit",
            "sim_init",
            "IpcMessage",
            "sit_ipc_message",
            "message_id",
            "msgid",
            "MsgId",
            "0x2f50",
            "2f50",
            "SIM_INIT",
        )
    )
]
log(f"payload/SIM-ish strings: {len(payload_like)}")
for o, s in payload_like[:80]:
    log(f"  {hex(o)}: {s[:160]}")

# Dynsym: list all sit_ipc_message::* and PayloadCase-ish
e_shoff = u64(so, 40)
e_shentsize = u16(so, 58)
e_shnum = u16(so, 60)
e_shstrndx = u16(so, 62)
shstr_off = u64(so, e_shoff + e_shstrndx * e_shentsize + 24)
shstr_size = u64(so, e_shoff + e_shstrndx * e_shentsize + 32)
shstr = so[shstr_off : shstr_off + shstr_size]


def secname(name_off: int) -> str:
    end = shstr.find(b"\x00", name_off)
    return shstr[name_off:end].decode()


sections = {}
for si in range(e_shnum):
    off = e_shoff + si * e_shentsize
    name = secname(u32(so, off))
    sections[name] = dict(
        type=u32(so, off + 4),
        addr=u64(so, off + 16),
        offset=u64(so, off + 24),
        size=u64(so, off + 32),
    )

msg_types: set[str] = set()
if ".dynsym" in sections and ".dynstr" in sections:
    dynstr = so[
        sections[".dynstr"]["offset"] : sections[".dynstr"]["offset"] + sections[".dynstr"]["size"]
    ]
    dynsym = sections[".dynsym"]
    for di in range(dynsym["size"] // 24):
        o = dynsym["offset"] + di * 24
        st_name = u32(so, o)
        end = dynstr.find(b"\x00", st_name)
        name = dynstr[st_name:end].decode()
        if "sit_ipc_message" in name:
            # extract type: N15sit_ipc_messageNNNameE
            m = re.search(r"sit_ipc_message(\d+)([A-Za-z0-9_]+)", name)
            if m:
                msg_types.add(m.group(2))
            else:
                msg_types.add(name[:120])

log(f"\nsit_ipc_message type names from dynsym ({len(msg_types)}):")
for n in sorted(msg_types):
    log(f"  {n}")

# Immediate 0x2f50 / 0x2F50 in SitOem
hits_2f50 = []
for o in range(0, len(so) - 1, 2):
    if u16(so, o) == 0x2F50:
        hits_2f50.append(o)
# ARM64 MOVZ Wd,#0x2f50 = 0x5285EA0?  MOVZ encoding: sf=0 opc=10 100101 hw=00 imm16 Rd
# MOVZ Wd, #imm16 => 0x52800000 | (imm16<<5) | Rd
movz_hits = []
for o in range(0, len(so) - 3, 4):
    w = u32(so, o)
    if (w & 0xFFC00000) == 0x52800000:  # MOVZ W*
        imm = (w >> 5) & 0xFFFF
        if imm == 0x2F50:
            movz_hits.append(o)
    if (w & 0xFFC00000) == 0xD2800000:  # MOVZ X*
        imm = (w >> 5) & 0xFFFF
        if imm == 0x2F50:
            movz_hits.append(o)

log(f"\nu16 LE 0x2f50 raw hits: {len(hits_2f50)} first={list(map(hex, hits_2f50[:12]))}")
log(f"MOVZ #0x2f50 hits: {len(movz_hits)} {list(map(hex, movz_hits[:12]))}")

# SitOem send API surface (already known) — confirm no Sim*
send_apis = [s for _, s in so_strings if "sitSend" in s or "SitOemHandler" in s]
log(f"\nsitSend*/SitOemHandler string count={len(send_apis)}")
simish_api = [s for s in send_apis if re.search(r"Sim|SIM|Init|Pin|Usim|USIM", s)]
log(f"SIM/Init-ish among them: {len(simish_api)}")
for s in simish_api[:40]:
    log(f"  {s[:140]}")

# PayloadCase enum numeric table: look for protobuf reflection DescriptorPool strings
desc_names = [
    (o, s)
    for o, s in so_strings
    if s.startswith("sit_ipc_message.") or "IpcMessage." in s or s.endswith("Request")
]
log(f"\nproto-ish dotted names: {len(desc_names)}")
for o, s in sorted(desc_names, key=lambda x: x[1])[:100]:
    log(f"  {hex(o)}: {s[:140]}")

# ============================================================
# B) MAIN: does oem protobuf path feed catalog 0x2f50?
# ============================================================
log("\n=== B) MAIN oem_ipc / protobuf vs catalog 0x2f50 ===")
main = MAIN_PATH.read_bytes()
log(f"MAIN size={len(main)}")

needles = [
    b"oem_ipc",
    b"sit_ipc_message",
    b"IpcMessage",
    b"protobuf",
    b"Protobuf",
    b"SIM_INIT_REQ",
    b"[OEM][IPC]",
    b"[OEM][SIT]",
    b"oem_ipc_message",
    b"OEM_IPC",
    b"PayloadCase",
    b"google.protobuf",
]
for nd in needles:
    offs = find_all(main, nd)
    log(f"  '{nd.decode()}' hits={len(offs)} offs={[hex(o) for o in offs[:8]]}")
    for o in offs[:3]:
        # show surrounding printable
        lo = max(0, o - 32)
        hi = min(len(main), o + 80)
        ctx = bytes(c if 32 <= c < 127 else 0x2E for c in main[lo:hi])
        log(f"    @{hex(o)} va={hex(off_to_va(o))}: {ctx.decode()}")

# Catalog entry for SIM_INIT already known @0x6de740
cat_va = 0x406DE740
cat_off = va_to_off(cat_va)
# catalog record layout from prior: flags, msgid, ...
# dump 32 bytes
log(f"\ncatalog @VA {hex(cat_va)} file {hex(cat_off)}:")
log("  " + main[cat_off : cat_off + 32].hex())

# Who references catalog VA via MOVW/MOVT to low/high of 0x406de740?
# MOVW rd,#0xe740 ; MOVT rd,#0x406d
lo_imm = 0xE740
hi_imm = 0x406D
movw_sites = []
# Thumb-2 MOVW: 11110 i 100100 imm4 0 imm3 Rd imm8  => F240|i|imm4 , |0|imm3|Rd|imm8
# Simpler: scan for little-endian halfwords encoding MOVW #0xe740 then nearby MOVT #0x406d
# Known approach used in prior scripts: scan MOVW imm16 == 0xe740

# Thumb MOVW encoding helper
def decode_movw_movt(w0: int, w1: int):
    # w0 is first halfword, w1 second
    if (w0 & 0xFBF0) == 0xF240 and (w1 & 0x8000) == 0:  # MOVW
        i = (w0 >> 10) & 1
        imm4 = w0 & 0xF
        imm3 = (w1 >> 12) & 7
        rd = (w1 >> 8) & 0xF
        imm8 = w1 & 0xFF
        imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8
        return ("MOVW", rd, imm16)
    if (w0 & 0xFBF0) == 0xF2C0 and (w1 & 0x8000) == 0:  # MOVT
        i = (w0 >> 10) & 1
        imm4 = w0 & 0xF
        imm3 = (w1 >> 12) & 7
        rd = (w1 >> 8) & 0xF
        imm8 = w1 & 0xFF
        imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8
        return ("MOVT", rd, imm16)
    return None


# Scan code region for MOVW #0x2f50 (bare msgid) — already done prior; recount near oem protobuf strings
bare_2f50 = []
for o in range(MAIN_OFF, len(main) - 3, 2):
    w0 = u16(main, o)
    w1 = u16(main, o + 2)
    d = decode_movw_movt(w0, w1)
    if d and d[0] == "MOVW" and d[2] == 0x2F50:
        # check if next MOVT for same rd within 16 bytes
        has_movt = False
        for k in range(4, 20, 2):
            if o + k + 3 >= len(main):
                break
            d2 = decode_movw_movt(u16(main, o + k), u16(main, o + k + 2))
            if d2 and d2[0] == "MOVT" and d2[1] == d[1]:
                has_movt = True
                break
        if not has_movt:
            bare_2f50.append(o)

log(f"\nbare MOVW #0x2f50 (no same-rd MOVT in +16B): {len(bare_2f50)}")
for o in bare_2f50[:24]:
    log(f"  off={hex(o)} va={hex(off_to_va(o))}")

# Cross: any function that mentions both sit_ipc/protobuf AND 0x2f50 nearby (window 4KB)?
proto_offs = []
for nd in (b"sit_ipc_message", b"IpcMessage", b"google.protobuf", b"protobuf"):
    proto_offs.extend(find_all(main, nd))
proto_vas = [off_to_va(o) for o in proto_offs if MAIN_OFF <= o < len(main)]

log(f"\nprotobuf-ish string VAs: {len(proto_vas)}")
for va in proto_vas[:20]:
    log(f"  {hex(va)}")

# oem_ipc_message_* encode strings region near catalog
oem_enc = find_all(main, b"oem_ipc_message")
log(f"oem_ipc_message* string hits={len(oem_enc)}")
for o in oem_enc[:15]:
    log(f"  {hex(o)} va={hex(off_to_va(o))}: {cstr(main, o, 80)}")

# Look for decode path strings that would map protobuf payload case -> OEM msgid
map_needles = [
    b"PayloadCase",
    b"payload_case",
    b"kSimInit",
    b"SIM_INIT",
    b"ConvertOem",
    b"oem_to_sit",
    b"sit_to_oem",
    b"Demux",
    b"demux",
    b"protobufSerial",
    b"ParseFromArray",
    b"SerializeToArray",
]
log("\nmap/demux needles in MAIN:")
for nd in map_needles:
    offs = find_all(main, nd)
    log(f"  {nd.decode()}: {len(offs)}")
    for o in offs[:3]:
        log(f"    {hex(off_to_va(o))}: {cstr(main, o, 100)}")

# ============================================================
# C) vendor.img alternate emitters
# ============================================================
log("\n=== C) vendor.img alternate /dev/oem_ipc* / 0x2f50 emitters ===")
if VENDOR.exists():
    # Stream-scan large file for needles (chunked)
    needles_v = [
        b"/dev/oem_ipc",
        b"SIM_INIT_REQ",
        b"SIM_INIT",
        b"SitOemHandler",
        b"sit_ipc_message",
        b"oem_ipc0",
        b"IpcTxSimInit",
        b"SimInitMessage",
        b"sendSimInit",
    ]
    counts = {nd: 0 for nd in needles_v}
    contexts: dict[bytes, list[str]] = {nd: [] for nd in needles_v}
    chunk = 8 * 1024 * 1024
    overlap = 256
    prev = b""
    with VENDOR.open("rb") as f:
        pos = 0
        while True:
            data = f.read(chunk)
            if not data:
                break
            buf = prev + data
            base = pos - len(prev)
            for nd in needles_v:
                start = 0
                while True:
                    i = buf.find(nd, start)
                    if i < 0:
                        break
                    abs_off = base + i
                    counts[nd] += 1
                    if len(contexts[nd]) < 8:
                        lo = max(0, i - 20)
                        hi = min(len(buf), i + 80)
                        ctx = bytes(c if 32 <= c < 127 else 0x2E for c in buf[lo:hi])
                        contexts[nd].append(f"@{hex(abs_off)} {ctx.decode()}")
                    start = i + 1
            prev = data[-overlap:]
            pos += len(data)
    for nd in needles_v:
        log(f"  {nd.decode()}: count={counts[nd]}")
        for c in contexts[nd]:
            log(f"    {c}")

    # Also search LE bytes 50 2f as isolated? too noisy — skip
else:
    log("vendor.img MISSING")

# oem_ipc1 carved log helper quick check
log("\n=== oem_ipc1 carved helper ===")
if OEMLOG.exists():
    lg = OEMLOG.read_bytes()
    log(f"size={len(lg)}")
    for nd in (b"SIM_INIT", b"0x2f50", b"sit_ipc", b"protobuf", b"/dev/oem_ipc"):
        offs = find_all(lg, nd)
        log(f"  {nd.decode()}: {len(offs)}")
else:
    log("missing")

# Verdict block
log("\n=== VERDICT ===")
has_sim_in_proto = any(
    re.search(r"SimInit|SIM_INIT|kSim", n, re.I) for n in msg_types
) or any(re.search(r"SimInit|SIM_INIT|kSim", s) for _, s in payload_like)
has_movz = len(movz_hits) > 0
log(f"SitOem has SIM_INIT/SimInit in protobuf types? {has_sim_in_proto}")
log(f"SitOem MOVZ #0x2f50? {has_movz}")
log(f"MAIN has sit_ipc_message/protobuf strings? {bool(proto_vas)}")
log(f"MAIN bare MOVW #0x2f50 count={len(bare_2f50)} (name-reg/false friends expected)")
log(
    "SitOem→catalog 0x2f50 demux: "
    + (
        "POSSIBLE — see mapping evidence above"
        if has_sim_in_proto or has_movz
        else "NO — SitOem protobuf surface has no SIM_INIT / 0x2f50; separate catalog path"
    )
)

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
log(f"\nWrote {OUT}")
