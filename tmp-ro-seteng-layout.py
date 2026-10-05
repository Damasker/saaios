#!/usr/bin/env python3
"""Extract SetEngMode / FakeSimPresence opcode+length from sit-stream / sitril."""
import struct
import subprocess
from pathlib import Path

STREAM = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
)
LIB = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
)
IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()


def disasm_arm64_fn(path, symbol_substr):
    """Use objdump -d and grep for symbol; return first 80 lines of matching function."""
    try:
        # find symbol address via nm
        nm = subprocess.check_output(
            ["aarch64-linux-gnu-nm", "-C", str(path)], text=True, errors="replace"
        )
    except Exception as e:
        print("nm fail", e)
        return
    hits = [ln for ln in nm.splitlines() if symbol_substr in ln and " T " in ln or (" t " in ln and symbol_substr in ln)]
    # broader
    hits = [ln for ln in nm.splitlines() if symbol_substr in ln]
    print(f"\n## symbols matching {symbol_substr!r} in {path.name}")
    for h in hits[:20]:
        print(" ", h)
    addrs = []
    for h in hits:
        parts = h.split()
        if len(parts) >= 3 and parts[1] in ("T", "t", "W", "w"):
            try:
                addrs.append(int(parts[0], 16))
            except ValueError:
                pass
    if not addrs:
        return
    addr = addrs[0]
    # objdump around address
    try:
        od = subprocess.check_output(
            [
                "aarch64-linux-gnu-objdump",
                "-d",
                f"--start-address=0x{addr:x}",
                f"--stop-address=0x{addr+0x120:x}",
                str(path),
            ],
            text=True,
            errors="replace",
        )
        print(od)
    except Exception as e:
        print("objdump fail", e)


for sym in [
    "SetEngMode",
    "BuildSetEng",
    "EngMode",
    "FakeSimPresence",
    "BuildOemSimRequest",
    "OemHook",
    "SetImsTest",
    "ImsTestMode",
    "RfDesense",
    "RadioNode",
]:
    disasm_arm64_fn(STREAM, sym)

# Heuristic: in Protocol builders, often mov wN, #imm16 for SIT id then store
# Search stream for immediate 0x09xx patterns near EngMode string
data = STREAM.read_bytes()
for needle in [b"SetEngMode", b"EngMode", b"FakeSimPresence", b"IMS_TEST_MODE", b"RF_DESENSE"]:
    i = data.find(needle)
    print(f"\nstream find {needle!r} @{hex(i) if i>=0 else None}")

# sit-base ENG_MODE constant neighborhood — often enum values
base = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-base.so"
).read_bytes()
for needle in [b"ENG_MODE", b"IMS_TEST_MODE", b"RF_DESENSE_MODE", b"VIRTUAL_SIM"]:
    i = base.find(needle)
    print(f"\nbase {needle!r} @{hex(i) if i>=0 else None}")
    if i is not None and i >= 0:
        # print surrounding as hex+ascii
        chunk = base[max(0, i - 32) : i + 64]
        print(chunk)

# CP: does SIT_SET_ENG_MODE handler call START_NETWORK or skip GET_APP?
# Find xref via string in log "SIT_SET_ENG_MODE"
print("\n=== CP logs for SET_ENG_MODE ===")
for n in [b"SIT_SET_ENG_MODE", b"SET_ENG_MODE", b"EngMode"]:
    off = 0
    c = 0
    while c < 10:
        j = IMG.find(n, off)
        if j < 0:
            break
        s = IMG[j : j + 100].split(b"\x00")[0]
        print(hex(j), s)
        off = j + 1
        c += 1

# FakeSimPresence in sitril — property or SIT?
lib = LIB.read_bytes()
i = lib.find(b"FakeSimPresence")
print("\nFakeSimPresence context in sitril:")
if i >= 0:
    print(lib[i - 40 : i + 80])
