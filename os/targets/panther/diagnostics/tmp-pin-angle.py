#!/usr/bin/env python3
"""Factory: (a) MEP/port assign from SlotStatus; (c) post-GetSimStatus PIN+DISABLED path."""
import subprocess
from pathlib import Path

lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
stream = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so")
ld = lib.read_bytes()
sd = stream.read_bytes()

print("== (a) MEP / port / mapping from slot status ==")
for n in (b"MEP", b"mep", b"PortAssign", b"assignPort", b"unassigned",
          b"LOGICAL_SLOT", b"Inactive", b"PORT_ACTIVE", b"SetPort",
          b"from SlotStatus", b"GetLogicalSlotId", b"FillSlotStatus",
          b"BuildSimSetLogical", b"slotmap", b"multipleEnabled"):
    for name, data in (("lib", ld), ("stream", sd)):
        j = data.find(n)
        if j >= 0:
            end = data.find(b"\0", j)
            s = "".join(chr(c) if 32 <= c < 127 else "." for c in data[j:min(end if end>0 else j+60, j+80)])
            print(f"  {name} {j:#x} {s}")

print("== (c) after GetSimStatus / PIN DISABLED ==")
for n in (b"PIN_REQUIRED", b"pin1 disabled", b"PIN_DISABLED", b"app_state",
          b"GetSimStatusDone", b"OnGetSimStatus", b"BuildSimStatus",
          b"DISABLED", b"after GetSimStatus", b"SimStatusChanged",
          b"RequestNotSupported", b"pin state"):
    j = 0
    hits = 0
    while hits < 3:
        j = ld.find(n, j)
        if j < 0:
            break
        end = ld.find(b"\0", j)
        s = "".join(chr(c) if 32 <= c < 127 else "." for c in ld[j:min(end if end>0 else j+70, j+90)])
        print(f"  lib {j:#x} {s}")
        j += 1
        hits += 1

# Symbols around BuildSimStatus / OnGetSimStatusDone
out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",str(lib)], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("GetSimStatusDone", "OnGetSimStatus", "BuildSimStatus",
                               "IsEarlySim", "PinState", "DoGetSimStatus")):
        print("sym", line)
