#!/usr/bin/env python3
import os, struct, re, subprocess
from pathlib import Path

KOS = Path("/home/mike/kernel-work/phone-kos")
K = Path("/home/mike/kernel-work/common-bd23337")
BUILD = Path("/home/mike/kernel-work/saaios_cp_poke_build")

print("=== build dir listing ===")
for p in sorted(BUILD.iterdir()):
    print(p.name, p.stat().st_size)

print("=== .mod ===")
print((BUILD / "saaios_cp_poke.mod").read_text())

print("=== modules.order ===")
print((BUILD / "modules.order").read_text())

# find modpost source message
for p in (K / "scripts/mod").glob("*.c"):
    t = p.read_text(errors="ignore")
    if "parse error in symbol dump" in t:
        print("found in", p)
        idx = t.index("parse error in symbol dump")
        print(t[max(0, idx - 400) : idx + 200])

# extract versions with aarch64 objcopy
syms = {}
for ko in sorted(KOS.glob("*.ko")):
    raw = Path("/tmp") / (ko.name + ".ver")
    r = subprocess.run(
        ["aarch64-linux-gnu-objcopy", "-O", "binary", "-j", "__versions", str(ko), str(raw)],
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print("objcopy fail", ko.name, r.stderr[:200])
        continue
    data = raw.read_bytes()
    n0 = len(syms)
    for off in range(0, len(data) // 64 * 64, 64):
        crc = struct.unpack_from("<Q", data, off)[0] & 0xFFFFFFFF
        name_b = data[off + 8 : off + 64].split(b"\x00", 1)[0]
        if name_b and re.fullmatch(rb"[A-Za-z0-9_]+", name_b):
            syms[name_b.decode()] = crc
    print(ko.name, "added", len(syms) - n0, "total", len(syms))

for n in ["_printk", "ioremap_prot", "iounmap", "param_ops_int", "arm64_use_ng_mappings", "log_write_mmio", "log_post_write_mmio", "module_layout"]:
    print("CRC", n, hex(syms[n]) if n in syms else "MISSING")
