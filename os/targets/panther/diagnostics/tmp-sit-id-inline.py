#!/usr/bin/env python3
# Check for an inline opcode id near each SIT name string, and scan a window
# around each name for any u16 equal to a known-anchor id to learn the layout.
import re, struct
CP = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
data = open(CP, "rb").read()

def noff(name):
    m = re.search(re.escape(name.encode()) + rb"\x00", data)
    return m.start() if m else None

anchors = {"SIT_SIM_IO":0x0208, "SIT_SET_RADIO_POWER":0x0800,
           "SIT_GET_RADIO_STATE":0x0801, "SIT_SETUP_DATA_CALL":0x0600}
for name,eid in anchors.items():
    o=noff(name)
    if o is None:
        print(name,"MISSING"); continue
    before=data[o-16:o]
    print(f"{name} id={eid:#06x} off={o:#x} before16={before.hex()}")
    # search +/-4096 for the u16 id little-endian and report nearest distance
    win=data[o-4096:o+4096]
    pat=struct.pack('<H',eid)
    locs=[m.start()-4096 for m in re.finditer(re.escape(pat),win)]
    near=sorted(locs,key=abs)[:6]
    print("   nearest u16 id rel-offsets:", near)
