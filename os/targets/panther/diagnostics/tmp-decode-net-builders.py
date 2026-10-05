#!/usr/bin/env python3
"""Decode opcode/length of candidate empty NET builders from sit-stream.so."""
import struct
from pathlib import Path

p = Path("sit-stream.so")
b = p.read_bytes()

# ARM64: look for MOVZ/MOVK patterns writing halfword id near function starts.
# Simpler: disassemble known function starts with objdump via subprocess.
import subprocess

syms = {
    "BuildAllowData": 0x74DF0,
    "BuildGetDataCallList": 0x7AD10,
    "BuildGetDeviceService": 0x76C50,
    "BuildGetNitzTime": 0x76B50,
    "BuildGetEndcMode": 0x76410,
    "BuildGetCellInfoList": 0x74CE0,
    "BuildGetVonrCapa": 0x76AD0,
    "BuildGetBarringInfo": 0x75FE0,
    "BuildGetFrequencyInfo": 0x76490,
    "BuildGetDuplexMode": 0x75320,
    "BuildGetManualRatMode": 0x760F0,
    "BuildGetNrMode": 0x765A0,
    "BuildGetAllowedNetworkTypeBitmap": 0x766F0,
    "BuildSetMobileDataState": 0x76CD0,
    "BuildSetVoiceOperation": 0x717E0,
    "GetIMEI": 0x70B50,
    "GetDevID": 0x73BC0,
    "BuildQueryAvailableBandMode": 0x74C60,
    "BuildGetRCNetworkType": 0x75140,
}

out = subprocess.check_output(
    ["aarch64-linux-gnu-objdump", "-d", "--start-address=0x74df0", "--stop-address=0x74e80", "sit-stream.so"],
    text=True,
    errors="replace",
)
print("=== BuildAllowData ===")
print(out)

for name, addr in [
    ("BuildGetDataCallList", 0x7AD10),
    ("BuildGetDeviceService", 0x76C50),
    ("BuildGetNitzTime", 0x76B50),
    ("BuildGetVonrCapa", 0x76AD0),
    ("BuildSetMobileDataState", 0x76CD0),
    ("BuildSetVoiceOperation", 0x717E0),
    ("BuildQueryAvailableBandMode", 0x74C60),
]:
    end = addr + 0xA0
    out = subprocess.check_output(
        [
            "aarch64-linux-gnu-objdump",
            "-d",
            f"--start-address={hex(addr)}",
            f"--stop-address={hex(end)}",
            "sit-stream.so",
        ],
        text=True,
        errors="replace",
    )
    print(f"=== {name} @ {hex(addr)} ===")
    # keep lines with mov / strh / bl
    for line in out.splitlines():
        if any(k in line for k in ("mov ", "movz", "movk", "strb", "strh", "str ", "bl\t", "ret")):
            print(line)
    print()
