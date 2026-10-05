#!/usr/bin/env python3
"""RE SitOem Ping protobuf wire shape from carved oem_ipc0 ELF.

Recover: IpcMessageType REQUEST value, PayloadCase for ping, field tags
for PingRequest, and whether userspace write is raw protobuf (no SIT-12).
No invent 0x2f50. No secrets (ping string length only).
"""
from __future__ import annotations

import struct
from pathlib import Path

def _so() -> Path:
    cands = [
        Path(
            "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
            "fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-25e7b000.so"
        ),
        Path(
            r"C:\Users\Admin\Projects\saaios-som\os\targets\panther\diagnostics"
            r"\fw\cdma-hunt\factory-td1a-vendor\carved-oemipc-25e7b000.so"
        ),
    ]
    for p in cands:
        if p.exists():
            return p
    raise SystemExit("carved-oemipc-25e7b000.so not found")


SO = _so().read_bytes()


def u16(o):
    return struct.unpack_from("<H", SO, o)[0]


def u32(o):
    return struct.unpack_from("<I", SO, o)[0]


def u64(o):
    return struct.unpack_from("<Q", SO, o)[0]


print(f"size={len(SO)}")

# Parse ELF sections / dynsym
e_phoff = u64(0x20)
e_shoff = u64(0x28)
e_phentsize = u16(0x36)
e_phnum = u16(0x38)
e_shentsize = u16(0x3A)
e_shnum = u16(0x3C)
e_shstrndx = u16(0x3E)


def sh(i):
    o = e_shoff + i * e_shentsize
    return {
        "name": u32(o),
        "type": u32(o + 4),
        "flags": u64(o + 8),
        "addr": u64(o + 16),
        "off": u64(o + 24),
        "size": u64(o + 32),
        "link": u32(o + 40),
        "entsize": u64(o + 56),
    }


shstr = sh(e_shstrndx)
names = SO[shstr["off"] : shstr["off"] + shstr["size"]]


def sn(off):
    end = names.find(b"\x00", off)
    return names[off:end].decode(errors="replace")


secs = {sn(s["name"]): s for s in (sh(i) for i in range(e_shnum))}
for n in (".text", ".rodata", ".data", ".data.rel.ro", ".dynsym", ".dynstr"):
    if n in secs:
        s = secs[n]
        print(f"  {n}: off={hex(s['off'])} size={hex(s['size'])} va={hex(s['addr'])}")


def va_to_off(va: int) -> int | None:
    for i in range(e_phnum):
        o = e_phoff + i * e_phentsize
        if u32(o) != 1:
            continue
        p_offset, p_vaddr, p_filesz = u64(o + 8), u64(o + 16), u64(o + 32)
        if p_vaddr <= va < p_vaddr + p_filesz:
            return p_offset + (va - p_vaddr)
    return None


def off_to_va(off: int) -> int | None:
    for i in range(e_phnum):
        o = e_phoff + i * e_phentsize
        if u32(o) != 1:
            continue
        p_offset, p_vaddr, p_filesz = u64(o + 8), u64(o + 16), u64(o + 32)
        if p_offset <= off < p_offset + p_filesz:
            return p_vaddr + (off - p_offset)
    return None


dynsym = secs.get(".dynsym")
dynstr = secs.get(".dynstr")
strtab = SO[dynstr["off"] : dynstr["off"] + dynstr["size"]]
syms = {}
entsize = dynsym["entsize"] or 24
for i in range(dynsym["size"] // entsize):
    o = dynsym["off"] + i * entsize
    st_name = u32(o)
    st_value = u64(o + 8)
    end = strtab.find(b"\x00", st_name)
    name = strtab[st_name:end].decode(errors="replace")
    if name:
        syms[name] = st_value

PING_KEYS = [
    "sitSendPingReq",
    "PingMessageModemData",
    "fillInput",
    "initialMessageHeader",
    "protobufSerialDataLen",
    "sendMessageData",
    "writeModemData",
    "PingRequest",
    "PingMessage",
    "PayloadCase",
    "IpcMessageType",
]
print("\n=== Ping-related dynsyms ===")
for k, v in sorted(syms.items()):
    if any(x in k for x in ("Ping", "initialMessageHeader", "protobufSerial", "sendMessageData", "writeModemData", "synchronousSend")):
        print(f"  {k} = {hex(v)}")

# Locate protobuf descriptor / schema strings
print("\n=== Schema / field name strings ===")
needles = [
    b"PingRequest",
    b"PingResponse",
    b"PingMessage",
    b"PingIndication",
    b"ping_string",
    b"ping_data",
    b"IpcMessage",
    b"payload",
    b"token",
    b"type",
    b"REQUEST",
    b"RESPONSE",
    b"INDICATION",
]
for n in needles:
    i = 0
    hits = []
    while True:
        j = SO.find(n, i)
        if j < 0:
            break
        # show context
        ctx = SO[max(0, j - 8) : j + len(n) + 24]
        hits.append((j, ctx))
        i = j + 1
        if len(hits) >= 6:
            break
    if hits:
        print(f"  {n.decode()}: n~={len(hits)}+")
        for off, ctx in hits[:3]:
            printable = "".join(chr(b) if 32 <= b < 127 else "." for b in ctx)
            print(f"    @{hex(off)} {printable}")


def disasm(off: int, nins: int = 80):
    hits = []
    base_va = off_to_va(off) or 0
    for i in range(0, nins * 4, 4):
        if off + i + 4 > len(SO):
            break
        w = u32(off + i)
        va = base_va + i
        # MOVZ Wd, #imm
        if (w & 0xFF800000) == 0x52800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            val = imm16 << (hw * 16)
            hits.append((va, f"MOVZ w{rd},#{hex(val)}", val))
        # MOVN
        elif (w & 0xFF800000) == 0x12800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            val = imm16 << (hw * 16)
            hits.append((va, f"MOVN w{rd},#{hex(val)}", val))
        # ORR immediate (sometimes used for small constants) — skip
        # LDRB imm
        elif (w & 0xFFC00000) == 0x39400000:
            imm12 = (w >> 10) & 0xFFF
            hits.append((va, f"LDRB #{imm12}", imm12))
        # STRB imm
        elif (w & 0xFFC00000) == 0x39000000:
            imm12 = (w >> 10) & 0xFFF
            hits.append((va, f"STRB #{imm12}", imm12))
        # ADD imm
        elif (w & 0xFF000000) == 0x91000000:
            sh = (w >> 22) & 1
            imm12 = (w >> 10) & 0xFFF
            val = imm12 << (12 if sh else 0)
            hits.append((va, f"ADD #{val}", val))
        # BL
        elif (w & 0xFC000000) == 0x94000000:
            imm26 = w & 0x03FFFFFF
            if imm26 & 0x2000000:
                imm26 -= 0x4000000
            target = va + imm26 * 4
            hits.append((va, f"BL {hex(target)}", target))
    return hits


def find_func_off(sym_substr: str) -> tuple[str, int, int] | None:
    for k, va in syms.items():
        if sym_substr in k:
            off = va_to_off(va)
            if off is not None:
                return k, va, off
    return None


print("\n=== Disasm PingMessageModemData ctor / fillInput / sitSendPingReq ===")
for key in (
    "PingMessageModemDataC1EN15sit_ipc_message14IpcMessageTypeE",
    "PingMessageModemData9fillInputEPKc",
    "SitOemHandler14sitSendPingReqEPKcPc",
    "ModemData20initialMessageHeader",
    "ModemData21protobufSerialDataLen",
    "ModemData15sendMessageDataEPci",
    "ModemDataTransmitter14writeModemDataEPci",
):
    hit = find_func_off(key)
    if not hit:
        # substring search
        for k, va in syms.items():
            if key in k or k.endswith(key):
                off = va_to_off(va)
                if off is not None:
                    hit = (k, va, off)
                    break
    if not hit:
        print(f"  MISSING {key}")
        continue
    name, va, off = hit
    print(f"\n-- {name}")
    print(f"   va={hex(va)} off={hex(off)}")
    for h in disasm(off, 60):
        # Filter noise: keep small immediates and BLs
        if h[2] < 0x10000 or str(h[1]).startswith("BL"):
            print(f"   @{hex(h[0])} {h[1]}")

# Look for protobuf wire-format builder patterns: set_type / set_token field tags
# Google protobuf typically: field 1 type = tag 0x08, field 2 token = tag 0x10, etc.
print("\n=== Scan for classic protobuf field-tag immediates near Ping symbols ===")
# In generated set_xxx, often MOVZ #tag then encode
# Search .text for consecutive MOVZ of 8, 16, 26 (common tags 1/2/3 varint/len)
ping_ctor = find_func_off("PingMessageModemDataC2EN15sit_ipc_message14IpcMessageTypeE")
if not ping_ctor:
    ping_ctor = find_func_off("PingMessageModemDataC1EN15sit_ipc_message14IpcMessageTypeE")
if ping_ctor:
    name, va, off = ping_ctor
    print(f"ctor {name}")
    # dump raw bytes nearby for MOVZ pattern
    blob = SO[off : off + 0x200]
    movz_vals = []
    for i in range(0, len(blob) - 4, 4):
        w = struct.unpack_from("<I", blob, i)[0]
        if (w & 0xFF800000) == 0x52800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            movz_vals.append((i, imm16 << (16 * hw)))
    print(f"  MOVZ immediates: {[(hex(i), hex(v)) for i, v in movz_vals[:40]]}")

# Find IpcMessageType enum values via switch / cmp near initialMessageHeader
hdr = find_func_off("ModemData20initialMessageHeader")
if hdr:
    name, va, off = hdr
    print(f"\ninitialMessageHeader MOVZ/CMP dump:")
    for h in disasm(off, 100):
        print(f"  @{hex(h[0])} {h[1]}")

# Search for default instance / descriptor tables referencing "ping"
print("\n=== _PingMessage_default_instance_ / descriptor ===")
for k, va in sorted(syms.items()):
    if "Ping" in k and ("default_instance" in k or "descriptor" in k or "_Ping" in k):
        off = va_to_off(va)
        print(f"  {k} va={hex(va)} off={off and hex(off)}")
        if off is not None:
            # dump 64 bytes
            print("   ", SO[off : off + 64].hex())

# Look at writeModemData for header prepend (length? type?)
print("\n=== writeModemData / sendMessageData framing hints ===")
for key in ("writeModemDataEPci", "sendMessageDataEPci", "protobufSerialDataLenEv"):
    hit = find_func_off(key)
    if not hit:
        continue
    name, va, off = hit
    print(f"\n-- {name}")
    for h in disasm(off, 50):
        print(f"  @{hex(h[0])} {h[1]}")

# Extract protobuf reflection field numbers from FileDescriptorProto-ish blobs
# Search for ASCII field names used in ping
print("\n=== Embedded proto field names (ping*) ===")
for n in (b"ping_msg", b"ping_string", b"ping_payload", b"req_data", b"rsp_data", b"data", b"msg"):
    idx = 0
    while True:
        j = SO.find(n + b"\x00", idx)
        if j < 0:
            break
        ctx = SO[max(0, j - 40) : j + 40]
        if b"Ping" in ctx or b"ping" in ctx or b"ipc" in ctx.lower():
            printable = "".join(chr(b) if 32 <= b < 127 else "." for b in ctx)
            print(f"  @{hex(j)} {printable}")
        idx = j + 1
        if idx > j + 1 and idx > 0:
            pass
        if idx - (SO.find(n) if False else idx) > 10:
            break
        # limit
        if idx > j + 500000:
            break
        # only a few
        if SO.find(n + b"\x00", idx) < 0:
            break
        # prevent infinite: break after 5 prints per needle handled above
        break

# Broader: list all null-terminated strings containing Ping
print("\n=== All Ping* C strings ===")
i = 0
count = 0
while i < len(SO) and count < 40:
    if SO[i : i + 4] == b"Ping" or SO[i : i + 4] == b"ping":
        j = i
        while j < len(SO) and 32 <= SO[j] < 127:
            j += 1
        if j - i >= 4:
            s = SO[i:j].decode()
            if "Ping" in s or s.startswith("ping"):
                print(f"  @{hex(i)} {s[:120]}")
                count += 1
        i = j + 1
    else:
        i += 1

print("\nDONE")
