#!/usr/bin/env python3
"""Scan uncompressed nested factory image zip for /dev/oem_ipc; deep-disasm catalog xref."""
from __future__ import annotations

import struct
import zipfile
from pathlib import Path

# ---------- MAIN deep disasm ----------
img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()
VA0 = 0x40010000
MAIN = 0x16C10


def va_of(o):
    return VA0 + (o - MAIN)


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def cstr(o, n=60):
    if o < 0 or o >= len(img):
        return None
    s = bytearray()
    for i in range(o, min(len(img), o + n)):
        c = img[i]
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def dec_imm(w, w2):
    i_bit = (w >> 10) & 1
    imm4 = w & 0xF
    imm3 = (w2 >> 12) & 7
    rd = (w2 >> 8) & 0xF
    imm8 = w2 & 0xFF
    return rd, (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8


def full_disasm(start, length=0x120):
    """More complete Thumb dump including BL/B/ADD."""
    out = []
    end = min(len(img) - 4, start + length)
    i = start & ~1
    while i < end:
        w = u16(i)
        w2 = u16(i + 2)
        if (w & 0xF800) >= 0xE800:
            if (w & 0xFBF0) == 0xF240:
                rd, imm = dec_imm(w, w2)
                out.append((i, 4, f"MOVW r{rd},#{hex(imm)}"))
            elif (w & 0xFBF0) == 0xF2C0:
                rd, imm = dec_imm(w, w2)
                out.append((i, 4, f"MOVT r{rd},#{hex(imm)}"))
            elif (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0xD000:
                # BL
                s = (w >> 10) & 1
                imm10 = w & 0x3FF
                j1 = (w2 >> 13) & 1
                j2 = (w2 >> 11) & 1
                imm11 = w2 & 0x7FF
                i1 = ~(j1 ^ s) & 1
                i2 = ~(j2 ^ s) & 1
                imm = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
                if s:
                    imm |= ~((1 << 25) - 1)
                    imm = imm - (1 << 25) if False else (imm | (-(1 << 25)))
                    # sign extend 25-bit
                    if imm & (1 << 24):
                        imm -= 1 << 25
                target = i + 4 + imm
                out.append((i, 4, f"BL {hex(target)}"))
            elif (w & 0xFFF0) in (0xF890, 0xF8B0, 0xF8D0, 0xF880, 0xF8A0, 0xF8C0):
                op = {
                    0xF890: "LDRB.W",
                    0xF8B0: "LDRH.W",
                    0xF8D0: "LDR.W",
                    0xF880: "STRB.W",
                    0xF8A0: "STRH.W",
                    0xF8C0: "STR.W",
                }[w & 0xFFF0]
                rt, rn, imm = (w2 >> 12) & 0xF, w & 0xF, w2 & 0xFFF
                out.append((i, 4, f"{op} r{rt},[r{rn},#{imm}]"))
            else:
                out.append((i, 4, f"W32 {hex(w)} {hex(w2)}"))
            i += 4
        else:
            if (w & 0xF800) == 0x2000:
                out.append((i, 2, f"MOVS r{(w>>8)&7},#{w&0xFF}"))
            elif (w & 0xF800) == 0x2800:
                out.append((i, 2, f"CMP r{(w>>8)&7},#{w&0xFF}"))
            elif (w & 0xF800) == 0x7800:
                out.append((i, 2, f"LDRB r{w&7},[r{(w>>3)&7},#{(w>>6)&0x1F}]"))
            elif (w & 0xF800) == 0x7000:
                out.append((i, 2, f"STRB r{w&7},[r{(w>>3)&7},#{(w>>6)&0x1F}]"))
            elif (w & 0xF800) == 0x8800:
                out.append((i, 2, f"LDRH r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<1}]"))
            elif (w & 0xF800) == 0x8000:
                out.append((i, 2, f"STRH r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<1}]"))
            elif (w & 0xF800) == 0x6800:
                out.append((i, 2, f"LDR r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<2}]"))
            elif (w & 0xF800) == 0x6000:
                out.append((i, 2, f"STR r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<2}]"))
            elif (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400:
                out.append((i, 2, f"PUSH {hex(w)}"))
            elif (w & 0xFF00) == 0xBD00 or (w & 0xFE00) == 0xBC00:
                out.append((i, 2, f"POP {hex(w)}"))
            elif (w & 0xF000) == 0xD000:
                out.append((i, 2, f"Bcond {hex(w)}"))
            elif (w & 0xF800) == 0xE000:
                imm = (w & 0x7FF) << 1
                if imm & 0x800:
                    imm -= 0x1000
                out.append((i, 2, f"B {hex(i+4+imm)}"))
            else:
                out.append((i, 2, f"H {hex(w)}"))
            i += 2
    return out


print("=== FULL disasm catalog_start xref fn @0x331a548 ===")
for off, n, txt in full_disasm(0x331A548, 0x100):
    mark = ""
    if "0x7af8" in txt or "#12" in txt or "0x2f" in txt or "MOVW" in txt or "MOVT" in txt or "STR" in txt or "LDR" in txt or "BL" in txt or "MOVS" in txt:
        mark = " <<<" if ("#12" in txt or "7af8" in txt or "7b30" in txt) else ""
    print(f"  {hex(off)}: {txt}{mark}")

print("\n=== FULL disasm around SIM_INIT catalog xref @0x32e4474 ===")
for off, n, txt in full_disasm(0x32E4474, 0x100):
    print(f"  {hex(off)}: {txt}")

print("\n=== FULL disasm near MOVS #12 sites in that band ===")
for site in (0x32E4648, 0x331A5F2, 0x331A6F8):
    print(f"-- around {hex(site)} --")
    for off, n, txt in full_disasm(site - 0x40, 0xA0):
        print(f"  {hex(off)}: {txt}")

# ---------- factory nested scan ----------
print("\n=== scan nested factory image zip (stored) for oem_ipc ===")
outer = zipfile.ZipFile(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/panther-td1a.221105.001-factory-10a338fe.zip"
)
nested_name = [n for n in outer.namelist() if n.endswith("image-panther-td1a.221105.001.zip")][0]
# Nested is stored: open as ZipFile via ZipExtFile — may not be seekable.
# Workaround: use raw offset into outer file for nested local file payload.
info = outer.getinfo(nested_name)
print("nested", nested_name, "compress", info.compress_type, "size", info.file_size)

# Get payload offset of nested member inside outer zip
with open(outer.filename, "rb") as f:
    # parse local header for nested
    # ZipInfo.header_offset points to local header
    f.seek(info.header_offset)
    lh = f.read(30)
    sig, ver, flags, method, _, _, _, comp, uncomp, nlen, elen = struct.unpack("<IHHHHHIIIHH", lh)
    name = f.read(nlen)
    extra = f.read(elen)
    payload_off = f.tell()
    print(f"nested payload_off={payload_off} method={method} name={name}")

    # Nested zip central directory is at end — read last 256KB to list members without full parse of 2.6G
    f.seek(payload_off + uncomp - 256 * 1024)
    tail = f.read(256 * 1024)
    # find EOCD
    eocd = tail.rfind(b"PK\x05\x06")
    print("EOCD in tail", eocd)
    if eocd >= 0:
        (
            sig,
            disk,
            disk_cd,
            entries_disk,
            entries,
            cd_size,
            cd_off,
            comment_len,
        ) = struct.unpack_from("<IHHHHIIH", tail, eocd)
        print(f"entries={entries} cd_size={cd_size} cd_off={cd_off}")
        # read central directory
        f.seek(payload_off + cd_off)
        cd = f.read(cd_size)
        pos = 0
        vendors = []
        while pos + 46 <= len(cd):
            if cd[pos : pos + 4] != b"PK\x01\x02":
                break
            (
                sig,
                ver_made,
                ver_need,
                flags,
                method,
                _,
                _,
                crc,
                csize,
                usize,
                nlen,
                elen,
                clen,
                _,
                _,
                _,
                _,
                lhoff,
            ) = struct.unpack_from("<IHHHHHHIIIHHHHHII", cd, pos)
            pos += 46
            fname = cd[pos : pos + nlen].decode("utf-8", "replace")
            pos += nlen + elen + clen
            if any(k in fname.lower() for k in ("vendor", "radio", "ril", "oem")):
                vendors.append((fname, usize, csize, method, lhoff))
                print(f"  MEMBER {usize:12d} method={method} {fname}")
        print(f"interesting members={len(vendors)}")

        # For each vendor*.img that is stored, window-scan for /dev/oem_ipc
        needle = b"/dev/oem_ipc"
        for fname, usize, csize, method, lhoff in vendors:
            if "vendor" not in fname.lower() or not fname.endswith(".img"):
                continue
            if method != 0:
                print(f"  skip compressed {fname}")
                continue
            # local header at payload_off + lhoff
            f.seek(payload_off + lhoff)
            lh = f.read(30)
            sig, ver, flags, method, _, _, _, comp, uncomp, nlen, elen = struct.unpack(
                "<IHHHHHIIIHH", lh
            )
            f.read(nlen)
            f.read(elen)
            start = f.tell()
            print(f"  scanning {fname} size={uncomp} at {start}...")
            remaining = uncomp
            buf = b""
            hits = []
            chunk_n = 0
            while remaining > 0 and len(hits) < 20:
                n = min(8 * 1024 * 1024, remaining)
                chunk = f.read(n)
                if not chunk:
                    break
                remaining -= len(chunk)
                data = buf + chunk
                pos = 0
                while True:
                    i = data.find(needle, pos)
                    if i < 0:
                        break
                    abs_off = start + (uncomp - remaining - len(chunk)) - len(buf) + i
                    # context
                    ctx = data[max(0, i - 32) : i + 64]
                    ascii_ctx = bytes(x if 32 <= x < 127 else 0x2E for x in ctx).decode()
                    hits.append((abs_off, ascii_ctx))
                    pos = i + 1
                buf = data[-len(needle) :]
                chunk_n += 1
                if chunk_n % 16 == 0:
                    print(f"    ... {uncomp-remaining}/{uncomp}")
            print(f"  hits={len(hits)}")
            for h in hits[:10]:
                print(f"    off={h[0]} ctx={h[1]!r}")

            # Also search oem_ipc0 and SIM_INIT_REQ in same vendor.img
            for nd in (b"oem_ipc0", b"SIM_INIT_REQ", b"BuildOem", b"OemHook"):
                f.seek(start)
                rem = uncomp
                buf = b""
                found = 0
                while rem > 0 and found < 5:
                    n = min(8 * 1024 * 1024, rem)
                    chunk = f.read(n)
                    rem -= len(chunk)
                    data = buf + chunk
                    if nd in data:
                        found += data.count(nd)
                    buf = data[-len(nd) :]
                print(f"  count {nd!r} ~= {found}")

print("DONE")
