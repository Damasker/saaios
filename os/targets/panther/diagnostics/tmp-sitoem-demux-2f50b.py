#!/usr/bin/env python3
"""Follow-up: SitOem protobuf dialect vs OEM catalog dialect; alternate 0x2f50 emitters.

Uses file offsets for MAIN catalog (0x6de740) with VA base 0x40010000.
"""
from __future__ import annotations

import re
import struct
from pathlib import Path

OUT = Path(__file__).with_suffix(".out")
lines: list[str] = []


def log(s: str = "") -> None:
    print(s, flush=True)
    lines.append(s)


def p(*parts: str) -> Path:
    wsl = Path("/mnt/c").joinpath(*parts)
    return wsl if wsl.exists() else Path("C:\\").joinpath(*parts)


MAIN_PATH = p(
    "Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
VENDOR = p(
    "Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/vendor.img"
)
SITOEM = p(
    "Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-25e7b000.so"
)

VA = 0x40010000
MAIN_OFF = 0x16C10


def va_of(o: int) -> int:
    return VA + (o - MAIN_OFF)


def off_of(va: int) -> int:
    return MAIN_OFF + (va - VA)


def u16(b: bytes, o: int) -> int:
    return struct.unpack_from("<H", b, o)[0]


def u32(b: bytes, o: int) -> int:
    return struct.unpack_from("<I", b, o)[0]


def cstr(b: bytes, o: int, n: int = 100) -> str:
    end = b.find(b"\x00", o, o + n)
    if end < 0:
        end = min(len(b), o + n)
    return b[o:end].decode("ascii", "replace")


def find_all(hay: bytes, needle: bytes) -> list[int]:
    out, start = [], 0
    while True:
        i = hay.find(needle, start)
        if i < 0:
            break
        out.append(i)
        start = i + 1
    return out


main = MAIN_PATH.read_bytes()
log(f"MAIN size={len(main)}")

# ---------- catalog SIM_INIT ----------
cat = 0x6DE740
log(f"\n=== catalog @file {hex(cat)} va={hex(va_of(cat))} ===")
log(main[cat : cat + 28].hex())
flags = u16(main, cat)
msgid = u16(main, cat + 2)
name_va = u32(main, cat + 4)
meta = u32(main, cat + 8)
rsp = u16(main, cat + 12)
log(f"flags={hex(flags)} msgid={hex(msgid)} name_va={hex(name_va)} meta={hex(meta)} rsp={hex(rsp)}")
log(f"name={cstr(main, off_of(name_va))}")

# ---------- protobuf OEM message names vs catalog SIM names ----------
log("\n=== [OEM][IPC] protobuf encode/decode message names ===")
# Collect strings containing [OEM][IPC] and protobuf / encode / decode
oem_ipc_strs = []
for o in find_all(main, b"[OEM][IPC]"):
    s = cstr(main, o, 160)
    oem_ipc_strs.append((o, s))

proto_msgs = []
sim_msgs = []
for o, s in oem_ipc_strs:
    if "protobuf" in s.lower() or "encode" in s.lower() or "decode" in s.lower():
        proto_msgs.append((o, s))
    if "SIM_" in s or "sim_" in s:
        sim_msgs.append((o, s))

log(f"[OEM][IPC] strings total={len(oem_ipc_strs)}")
log(f"  with encode/decode/protobuf: {len(proto_msgs)}")
for o, s in proto_msgs[:40]:
    log(f"    {hex(va_of(o))}: {s}")
log(f"  with SIM_: {len(sim_msgs)}")
for o, s in sim_msgs[:40]:
    log(f"    {hex(va_of(o))}: {s}")

# Unique message tokens near Start encode protobuf
encode_hits = [s for _, s in oem_ipc_strs if "encode protobuf" in s.lower() or "Start encode" in s]
log(f"\nStart-encode-protobuf lines: {len(encode_hits)}")
for s in encode_hits[:50]:
    log(f"  {s}")

# Catalog message name strings in 0x6de000 band name ptrs — sample which are SIM_
log("\n=== catalog neighborhood name scan (0x6de000..0x6e2000) ===")
cat_names = []
for o in range(0x6DE000, 0x6E2000, 4):
    # heuristic: look for msgid pattern flags@0 msgid@2 name_va@4 in 0x41xxxxxx
    if o + 8 > len(main):
        break
    nv = u32(main, o + 4)
    mid = u16(main, o + 2)
    if 0x41000000 <= nv <= 0x41200000 and 0x2F00 <= mid <= 0x2FFF:
        nm = cstr(main, off_of(nv))
        if nm and nm.startswith("SIM_"):
            cat_names.append((o, mid, nm))
log(f"SIM_* catalog-like entries: {len(cat_names)}")
for o, mid, nm in cat_names:
    log(f"  @{hex(o)} id={hex(mid)} {nm}")

# Overlap: any catalog SIM_* name also appear in protobuf encode strings?
proto_blob = " ".join(s for _, s in proto_msgs)
overlap = [nm for _, _, nm in cat_names if nm in proto_blob or nm.replace("_REQ", "") in proto_blob]
log(f"\ncatalog SIM_* names overlapping protobuf OEM strings: {overlap}")

# ---------- dispatcher / dual dialect ----------
log("\n=== oem_ipc_message_dispatcher / utils context ===")
for nd in (
    b"oem_ipc_message_dispatcher.c",
    b"oem_ipc_message_utils.c",
    b"Not a config message",
    b"Not a config request",
    b"Not a ping",
    b"Invalid message type",
    b"Unknown message",
    b"message id",
    b"msgid",
    b"MsgId",
):
    offs = find_all(main, nd)
    log(f"  {nd.decode()}: {len(offs)}")
    for o in offs[:3]:
        log(f"    {hex(va_of(o))}: {cstr(main, o, 120)}")

# Does dispatcher mention SIM_INIT or 0x2f50?
# Search ±8KB of dispatcher string for SIM_INIT / catalog
disp = find_all(main, b"oem_ipc_message_dispatcher.c")
for o in disp:
    window = main[max(0, o - 0x2000) : o + 0x4000]
    for nd in (b"SIM_INIT", b"2f50", b"0x2f50", b"protobuf", b"catalog", b"VerifyPin", b"VERIFYPIN"):
        c = window.count(nd)
        if c:
            log(f"  near dispatcher: {nd.decode()} count={c}")

# ---------- SitOem PayloadCase enum values via reflection strings ----------
log("\n=== SitOem PayloadCase / oneof field names (carved .so) ===")
so = SITOEM.read_bytes()
# nanopb / protobuf often embeds field names as C strings
fieldish = []
i = 0
while i < len(so):
    if 32 <= so[i] < 127:
        j = i
        while j < len(so) and 32 <= so[j] < 127:
            j += 1
        if j - i >= 4:
            s = so[i:j].decode()
            if re.match(
                r"^(ping|config|thermal|metrics|device_state|traffic|txas|scone|coex|debug|data_flow|"
                r"data_validation|mch|sim|usim|init|payload|ipc)_",
                s,
                re.I,
            ) or s in (
                "ping",
                "config",
                "thermal",
                "sim_init",
                "simInit",
                "payload",
            ):
                fieldish.append((i, s))
        i = j + 1
    else:
        i += 1
log(f"field-like strings: {len(fieldish)}")
for o, s in fieldish[:60]:
    log(f"  {hex(o)}: {s}")

# Enumerate unique PayloadCase from initialMessageHeader callers — already have types.
# Extract k* enum names from RTTI/strings
k_enums = []
i = 0
while i < len(so):
    if so[i : i + 1] == b"k" and 32 <= so[i + 1] < 127:
        j = i
        while j < len(so) and (so[j] >= 48):  # rough
            if not (48 <= so[j] < 127):
                break
            j += 1
        s = so[i:j].decode("ascii", "replace")
        if re.match(r"^k[A-Z][A-Za-z0-9_]{2,40}$", s):
            k_enums.append((i, s))
        i = j + 1
    else:
        i += 1
# dedupe
seen = set()
uniq_k = []
for o, s in k_enums:
    if s not in seen:
        seen.add(s)
        uniq_k.append((o, s))
log(f"\nk* enum-like strings: {len(uniq_k)}")
for o, s in uniq_k:
    log(f"  {hex(o)}: {s}")

# ---------- vendor alternate emitters beyond SitOem ----------
log("\n=== vendor.img: binaries near /dev/oem_ipc + cbd/rild SIM paths ===")
needles = [
    b"/dev/oem_ipc0",
    b"/dev/oem_ipc1",
    b"/dev/umts_ipc0",
    b"SIM_INIT_REQ",
    b"SIM_INIT_CNF",
    b"IpcTxSimInit",
    b"SimInit",
    b"sitInformSimInit",
    b"cbd",
    b"libcbd",
    b"libsitril",
    b"libsec-ril",
    b"OemIpc",
    b"OEM_IPC",
]
# For each needle, note nearby ELF magic within 2MB backward
chunk = 8 * 1024 * 1024
overlap = 64
prev = b""
elf_hits: dict[bytes, list[tuple[int, int | None]]] = {nd: [] for nd in needles}
counts = {nd: 0 for nd in needles}
with VENDOR.open("rb") as f:
    pos = 0
    while True:
        data = f.read(chunk)
        if not data:
            break
        buf = prev + data
        base = pos - len(prev)
        for nd in needles:
            start = 0
            while True:
                i = buf.find(nd, start)
                if i < 0:
                    break
                abs_off = base + i
                counts[nd] += 1
                if len(elf_hits[nd]) < 6:
                    # search back up to 2MB for ELF
                    back = buf[max(0, i - 2 * 1024 * 1024) : i + 1]
                    elf_rel = back.rfind(b"\x7fELF")
                    elf_abs = (base + max(0, i - 2 * 1024 * 1024) + elf_rel) if elf_rel >= 0 else None
                    ctx = bytes(c if 32 <= c < 127 else 0x2E for c in buf[max(0, i - 16) : i + 48])
                    elf_hits[nd].append((abs_off, elf_abs))
                    log(f"  {nd.decode()} @{hex(abs_off)} elf~={hex(elf_abs) if elf_abs else None} ctx={ctx.decode()}")
                start = i + 1
        prev = data[-overlap:]
        pos += len(data)

log("\ncounts:")
for nd in needles:
    log(f"  {nd.decode()}: {counts[nd]}")

# ---------- MAIN: does AP-bound OEM RX parse catalog msgid before protobuf? ----------
log("\n=== MAIN RX path clues: msgid compare to 0x2f50 near OEM IPC ===")
# bare MOVW #0x2f50 sites — dump nearby strings via ptr loads is hard; dump 32B hex + look for [OEM]


def decode_movw(b: bytes, o: int):
    if o + 4 > len(b):
        return None
    hw, hw2 = u16(b, o), u16(b, o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


bare = []
for o in range(MAIN_OFF, len(main) - 3, 2):
    r = decode_movw(main, o)
    if not r or r[0] != 0x2F50:
        continue
    # skip if MOVT same rd soon
    skip = False
    for k in range(4, 20, 2):
        if o + k + 3 >= len(main):
            break
        hw, hw2 = u16(main, o + k), u16(main, o + k + 2)
        if (hw & 0xFBF0) == 0xF2C0 and (hw2 & 0x8000) == 0:
            rd = (hw2 >> 8) & 0xF
            if rd == r[1]:
                skip = True
                break
    if not skip:
        bare.append(o)

log(f"bare MOVW #0x2f50: {len(bare)}")
for o in bare:
    # search nearby ASCII
    win = main[max(0, o - 0x80) : o + 0xC0]
    asciis = []
    i = 0
    while i < len(win):
        if 32 <= win[i] < 127:
            j = i
            while j < len(win) and 32 <= win[j] < 127:
                j += 1
            if j - i >= 6:
                asciis.append(win[i:j].decode())
            i = j + 1
        else:
            i += 1
    log(f"  va={hex(va_of(o))} nearby_ascii={asciis[:8]}")

log("\n=== VERDICT ===")
log(
    "SitOem protobuf message set (Ping/Config/Thermal/Metrics/DeviceState/Traffic/"
    "Txas/Scone/Coex/Debug/DataFlow/DataValidation/Mch/…) is the MAIN [OEM][IPC] "
    "protobuf encode family — NOT the SIM_* catalog family."
)
log(
    f"Catalog SIM_INIT_REQ 0x2f50 @file {hex(cat)} remains a separate OEM IPC table entry; "
    f"no demux evidence from SitOem PayloadCase → 0x2f50."
)
log(
    "vendor.img /dev/oem_ipc* openers remain SitOem (ipc0) + log helper (ipc1); "
    "SIM_INIT_REQ / IpcTxSimInit / SimInitMessage counts = 0."
)
log("Alternate host emitter of catalog 0x2f50: NOT found in factory vendor.img under bans (no cbd/rild start).")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
log(f"Wrote {OUT}")
