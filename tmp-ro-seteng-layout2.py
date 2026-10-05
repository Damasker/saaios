#!/usr/bin/env python3
"""Find SetEngMode builder layout (SIT id + length) via mangled names + ARM64 immediates."""
import re
import struct
import subprocess
from pathlib import Path

STREAM = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
)
LIB = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
)
BASE = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-base.so"
)
IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()


def readelf_syms(path):
    out = subprocess.check_output(
        ["aarch64-linux-gnu-readelf", "-sW", str(path)], text=True, errors="replace"
    )
    keys = (
        "Eng",
        "Virtual",
        "Oem",
        "Fake",
        "Desense",
        "RadioNode",
        "ImsTest",
        "Hook",
        "Camp",
        "Lab",
        "TestMode",
    )
    for line in out.splitlines():
        if any(k in line for k in keys):
            print(line)


print("=== STREAM syms ===")
readelf_syms(STREAM)
print("=== LIB syms ===")
readelf_syms(LIB)
print("=== BASE syms ===")
readelf_syms(BASE)


def find_mov_imm_near_string(data: bytes, needle: bytes, window=0x200):
    """Find needle string, then look for ADRP/ADD refs is hard; instead scan
    for MOVZ/MOVK encoding SIT ids 0x0700-0x0fff in whole .text near string
    only if we can find code that loads pointer to string.

    Simpler approach used previously in project: builders store
      req[2]=lo, req[3]=hi of id, req[4]=length
    as immediate stores. Search functions by mangled name offset.
    """
    # Find all occurrences of mangled-ish
    offs = []
    start = 0
    while True:
        i = data.find(needle, start)
        if i < 0:
            break
        offs.append(i)
        start = i + 1
    return offs


stream = STREAM.read_bytes()
# ELF sections
# parse ELF64 to get .text VA and file offset
assert stream[:4] == b"\x7fELF"
e_phoff = struct.unpack_from("<Q", stream, 32)[0]
e_phentsize = struct.unpack_from("<H", stream, 54)[0]
e_phnum = struct.unpack_from("<H", stream, 56)[0]
load_maps = []  # (vaddr, memsz, offset, filesz)
for i in range(e_phnum):
    o = e_phoff + i * e_phentsize
    p_type, p_flags, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_align = struct.unpack_from(
        "<IIQQQQQQ", stream, o
    )
    if p_type == 1:  # PT_LOAD
        load_maps.append((p_vaddr, p_memsz, p_offset, p_filesz))


def va_to_off(va):
    for vaddr, memsz, offset, filesz in load_maps:
        if vaddr <= va < vaddr + filesz:
            return offset + (va - vaddr)
    return None


def off_to_va(off):
    for vaddr, memsz, offset, filesz in load_maps:
        if offset <= off < offset + filesz:
            return vaddr + (off - offset)
    return None


# Find mangled SetEngMode builder string and its code via relative refs is hard.
# Instead: search for ASCII "SIT_SET_ENG_MODE" in stream/lib — unlikely.
# Decode Protocol builder pattern from prior known BuildRadioPower @ comments.

# Known from sit-sim-status: builders encode id as little halfword.
# Search stream for unique sequence: store of eng-mode related.
# Look at demangled RTTI: SetEngModeEh → ProtocolMiscBuilder::SetEngMode(unsigned char)

for needle in [
    b"SetEngModeEh",
    b"SetEngModeEhh",
    b"BuildSetEngMode",
    b"FakeSimPresence",
    b"BuildOemSimRequest",
    b"SetImsTestMode",
    b"SetRfDesenseMode",
]:
    idxs = find_mov_imm_near_string(stream, needle)
    print(f"stream {needle!r} at {[hex(x) for x in idxs]}")

# In Itanium ABI, typeinfo name is followed; the actual function may be nearby in .text
# Better: use dynsym via readelf -sW already printed; parse addresses

out = subprocess.check_output(
    ["aarch64-linux-gnu-readelf", "-sW", str(STREAM)], text=True, errors="replace"
)
targets = []
for line in out.splitlines():
    if "SetEngMode" in line or "FakeSim" in line or "OemSimRequest" in line or "ImsTest" in line:
        parts = line.split()
        # Num Value Size Type Bind Vis Ndx Name
        if len(parts) >= 8:
            try:
                val = int(parts[1], 16)
                name = parts[-1]
                targets.append((name, val))
            except ValueError:
                pass

print("\nParsed targets:", targets)

# Also try all Build* with Eng
for line in out.splitlines():
    if "Eng" in line and ("Build" in line or "SetEng" in line or "Protocol" in line):
        print("ENGLINE", line)


def decode_movz_w(insn):
    # MOVZ Wd, #imm16, LSL #shift ; sf=0 opc=10 101 00 hw imm16 Rd
    if (insn & 0xFF800000) != 0x52800000:
        return None
    rd = insn & 0x1F
    imm16 = (insn >> 5) & 0xFFFF
    hw = (insn >> 21) & 0x3
    return rd, imm16 << (hw * 16)


def decode_movk_w(insn):
    if (insn & 0xFF800000) != 0x72800000:
        return None
    rd = insn & 0x1F
    imm16 = (insn >> 5) & 0xFFFF
    hw = (insn >> 21) & 0x3
    return rd, imm16, hw


def scan_fn(va, size=0x180):
    off = va_to_off(va)
    if off is None:
        print(f"  no mapping for VA {hex(va)}")
        return
    print(f"\n## disasm scan VA {hex(va)} file@{hex(off)}")
    imms = []
    for i in range(0, size, 4):
        insn = struct.unpack_from("<I", stream, off + i)[0]
        mz = decode_movz_w(insn)
        mk = decode_movk_w(insn)
        if mz:
            rd, val = mz
            if 0x100 <= val <= 0xFFF or val in (12, 13, 16, 18, 38, 72):
                imms.append((i, f"MOVZ w{rd},#{hex(val)}"))
        if mk and mk[2] == 0:
            rd, imm16, hw = mk
            if 0x100 <= imm16 <= 0xFFF:
                imms.append((i, f"MOVK w{rd},#{hex(imm16)}"))
        # MOV Wd, #imm via ORR immediate is rarer for these
    for i, s in imms:
        print(f"  +{hex(i)}: {s}")


for name, val in targets:
    scan_fn(val)

# Broader: any dynsym containing EngMode / SetEng
out_all = out
for line in out_all.splitlines():
    if re.search(r"EngMode|SetEng|FakeSim|OemHook|Desense|RadioNode", line):
        parts = line.split()
        if len(parts) >= 8:
            try:
                val = int(parts[1], 16)
                print("SCAN", parts[-1], hex(val))
                scan_fn(val)
            except ValueError:
                pass

# CP: does START_NETWORK have alternate entry that accepts other app states?
# Already known sole gate. Confirm no second "SIM is not ready" bypass via ENG.
print("\n=== CP: ENG_MODE vs START_NETWORK cross ===")
# Search handler log for SET_ENG_MODE
for n in [b"SIT_SET_ENG_MODE", b"SET_ENG_MODE REQ", b"SET_ENG_MODE isn't", b"Eng mode"]:
    j = IMG.find(n)
    print(n, hex(j) if j >= 0 else None)
    if j and j > 0:
        # surrounding strings
        chunk = IMG[j : j + 120].split(b"\x00")[0]
        print(" ", chunk)

# FakeSimPresence — is it RIL property only?
lib = LIB.read_bytes()
for n in [b"FakeSimPresence", b"fake_sim", b"persist.radio.fake", b"vendor.fake"]:
    j = lib.find(n)
    print("lib", n, hex(j) if j >= 0 else None)
    if j is not None and j >= 0:
        print(" ", lib[max(0, j - 32) : j + 64])
