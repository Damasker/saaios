#!/usr/bin/env python3
from pathlib import Path
import zlib

p = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin")
d = p.read_bytes()
b_off, sz, want = 0x410, 0x16800, 0x101B4A2C
boot = d[b_off : b_off + sz]
print("BOOT zlib", hex(zlib.crc32(boot) & 0xFFFFFFFF), "want", hex(want))
print("jamcrc", hex((zlib.crc32(boot) ^ 0xFFFFFFFF) & 0xFFFFFFFF))

try:
    import crcmod
except ImportError:
    import subprocess, sys
    subprocess.check_call([sys.executable, "-m", "pip", "install", "crcmod", "-q"])
    import crcmod

variants = [
    ("crc32", 0x104C11DB7, 0xFFFFFFFF, 0xFFFFFFFF, True),
    ("crc32_norev", 0x104C11DB7, 0xFFFFFFFF, 0xFFFFFFFF, False),
    ("crc32_init0", 0x104C11DB7, 0, 0xFFFFFFFF, True),
    ("crc32c", 0x11EDC6F41, 0xFFFFFFFF, 0xFFFFFFFF, True),
    ("crc32k", 0x1741B8CD7, 0xFFFFFFFF, 0xFFFFFFFF, True),
    ("posix", 0x104C11DB7, 0, 0xFFFFFFFF, False),
]
for name, poly, init, xor, rev in variants:
    fn = crcmod.mkCrcFun(poly, initCrc=init, rev=rev, xorOut=xor)
    v = fn(boot) & 0xFFFFFFFF
    print(f"{name}: {v:#010x} {'MATCH' if v == want else ''}")

# INFO matched zlib — confirm
info = d[0x5DB6780 : 0x5DB6780 + 0xD0]
print("INFO zlib", hex(zlib.crc32(info) & 0xFFFFFFFF), "toc", hex(0x2C231591))

# Maybe BOOT CRC is zlib of size-4 excluding trailing crc field? or size aligned
for n in [sz, sz - 4, sz - 8, 0x16000, 0x16800 - 0x10]:
    if n <= 0:
        continue
    v = zlib.crc32(boot[:n]) & 0xFFFFFFFF
    if v == want:
        print("MATCH zlib len", hex(n))
