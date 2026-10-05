#!/usr/bin/env python3
"""Find code that references umts_ipc0/1 string addresses via ADRP+ADD."""
import struct
from pathlib import Path
import subprocess

lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
data = lib.read_bytes()

# Get load address of .text and .rodata from readelf
elf = subprocess.check_output(["aarch64-linux-gnu-readelf","-S",str(lib)], text=True)
# For ET_DYN, VMA equals file offset for many sections in this style of so? Check.
# Better: use readelf -s for string - actually strings are in .rodata at VMA.

# Parse section headers for .text and find file offset of text
# Use objdump -h
hdr = subprocess.check_output(["aarch64-linux-gnu-objdump","-h",str(lib)], text=True)
text_vma = text_off = None
ro_vma = ro_off = None
for line in hdr.splitlines():
    parts = line.split()
    if len(parts) >= 6 and parts[1] == ".text":
        text_vma = int(parts[3], 16)
        text_off = int(parts[5], 16)
        text_size = int(parts[2], 16)
    if len(parts) >= 6 and parts[1] == ".rodata":
        ro_vma = int(parts[3], 16)
        ro_off = int(parts[5], 16)

print(f".text vma={text_vma:#x} off={text_off:#x}")
print(f".rodata vma={ro_vma:#x} off={ro_off:#x}")

# String file offsets -> VMA
targets = {}
for name in (b"/dev/umts_ipc0", b"/dev/umts_ipc1", b"umts_ipc0", b"umts_ipc1"):
    fo = data.find(name)
    # which section?
    vma = None
    # assume rodata
    if ro_off is not None and fo >= ro_off:
        vma = ro_vma + (fo - ro_off)
    targets[name.decode()] = (fo, vma)
    print(f"str {name.decode()} file={fo:#x} vma={vma and hex(vma)}")

# Scan .text for ADRP xn, page; ADD xn, xn, #imm patterns that resolve to target VMAs
# ADRP: 1xx10000 immlo(2) immhi(19) Rd(5)  -> page = PC[63:12] + sign_extend(immhi:immlo << 12)
# ADD immediate (64-bit): 10010001 sh(1) imm12(12) Rn(5) Rd(5)

text = data[text_off:text_off+text_size]

def decode_adrp(word, pc):
    if (word & 0x9F000000) != 0x90000000:
        return None, None
    rd = word & 0x1F
    immlo = (word >> 29) & 0x3
    immhi = (word >> 5) & 0x7FFFF
    imm = (immhi << 2) | immlo
    if imm & (1 << 20):
        imm -= (1 << 21)
    page = (pc & ~0xFFF) + (imm << 12)
    return rd, page

def decode_add_imm(word):
    # ADD (immediate) 64-bit: sf=1 op=0 S=0 -> 10010001
    if (word & 0xFF000000) != 0x91000000:
        return None
    rd = word & 0x1F
    rn = (word >> 5) & 0x1F
    imm12 = (word >> 10) & 0xFFF
    sh = (word >> 22) & 1
    if sh:
        imm12 <<= 12
    return rd, rn, imm12

want = {v for (_, v) in targets.values() if v is not None}
hits = []
i = 0
while i + 8 <= len(text):
    w = struct.unpack_from("<I", text, i)[0]
    pc = text_vma + i
    rd, page = decode_adrp(w, pc)
    if rd is not None:
        # look ahead up to 8 instructions for ADD to same rd
        for j in range(1, 12):
            if i + 4*j + 4 > len(text):
                break
            w2 = struct.unpack_from("<I", text, i + 4*j)[0]
            add = decode_add_imm(w2)
            if not add:
                continue
            rd2, rn2, imm = add
            if rn2 == rd and rd2 == rd:
                addr = page + imm
                if addr in want:
                    hits.append((pc, addr, rd))
                break
    i += 4

print(f"hits={len(hits)}")
for pc, addr, rd in hits[:40]:
    name = [k for k,(fo,v) in targets.items() if v==addr]
    print(f"  xref pc={pc:#x} -> {addr:#x} {name} x{rd}")

# For each hit, dump 40 instructions around
for pc, addr, rd in hits:
    start = max(text_vma, pc - 0x40)
    stop = pc + 0x80
    print(subprocess.check_output([
        "aarch64-linux-gnu-objdump","-d",
        f"--start-address={start:#x}", f"--stop-address={stop:#x}", str(lib)
    ], text=True, errors="replace")[:2000])
    print("---")
