#!/usr/bin/env python3
# Dump readable ASCII context around key offsets to understand the RFS read op
# and the operational-mode data source. Strings only.
import re

IMG = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
data = open(IMG, "rb").read()

def strings_around(off, back=160, fwd=200):
    lo = max(0, off-back); hi = min(len(data), off+fwd)
    chunk = data[lo:hi]
    out = []
    cur = bytearray()
    for b in chunk:
        if 32 <= b < 127:
            cur.append(b)
        else:
            if len(cur) >= 4:
                out.append(cur.decode())
            cur = bytearray()
    if len(cur) >= 4:
        out.append(cur.decode())
    return out

for label, off in [
    ("RfsRead#1", 0x4d32e4d), ("RfsRead#2", 0x4d32eb0),
    ("NvRead", 0x29a1b78), ("NvWrite", 0x29a2120),
    ("OperationMode", 0xfca68c), ("OPERATION_MODE", 0xcc3bf0),
    ("OperatingMode", 0x10598a8),
]:
    print("====", label, hex(off))
    for s in strings_around(off):
        print("   ", s)

# Hunt for RFS command verb strings clustered together (open/read/write/close)
print("==== RFS verb cluster search ====")
for verb in [b"RfsOpen", b"RfsClose", b"RfsWrite", b"RfsRead", b"RfsStat",
             b"RfsGetSize", b"RfsCreate", b"RfsUnlink", b"RfsPut", b"RfsGet"]:
    idxs = [m.start() for m in re.finditer(re.escape(verb), data)]
    print(verb.decode(), len(idxs), [hex(i) for i in idxs[:3]])
