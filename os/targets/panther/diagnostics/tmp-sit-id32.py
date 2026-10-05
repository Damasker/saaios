#!/usr/bin/env python3
# Find the SIT dispatch/logging table that references names by 32-bit VA, and
# read the adjacent opcode id. Validate on anchors, then emit ids for targets.
import re, struct
CP = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
data = open(CP, "rb").read()
BASE = 0x40010000 - 0x16c10  # file->VA for MAIN

def noff(name):
    m = re.search(re.escape(name.encode()) + rb"\x00", data)
    return m.start() if m else None
def va(off): return BASE + off

anchors = {"SIT_SIM_IO":0x0208,"SIT_SET_RADIO_POWER":0x0800,
           "SIT_GET_RADIO_STATE":0x0801,"SIT_SETUP_DATA_CALL":0x0600}
targets = [
 "SIT_GET_PS_SERVICE_DOMAIN","SIT_SET_PS_SERVICE_DOMAIN","SIT_GET_VOICE_OPERATION",
 "SIT_SET_VOICE_OPERATION","SIT_SET_INTPS_SERVICE","SIT_SET_DEVICE_SERVICE",
 "SIT_GET_DEVICE_SERVICE","SIT_SET_MODEM_CONFIG","SIT_SET_DUAL_NTW_AND_PS_TYPE",
 "SIT_GET_PS_REG_STATE","SIT_SET_MOBILE_DATA_STATE","SIT_SET_INITIAL_ATTACH_APN",
]

def find32(ptr):
    pb = struct.pack('<I', ptr & 0xffffffff)
    return [m.start() for m in re.finditer(re.escape(pb), data)]

# First, learn the table stride/id position from anchors.
print("=== anchor table probing (32-bit name ptr) ===")
layout=None
for name,eid in anchors.items():
    o=noff(name)
    if o is None: print(name,"MISSING"); continue
    locs=find32(va(o))
    for loc in locs:
        row=data[loc-16:loc+16]
        # look for eid as u16/u32 within +/-16
        found=[]
        for d in range(-16,16,2):
            if struct.unpack_from('<H',data,loc+d)[0]==eid: found.append(('u16',d))
        for d in range(-16,16,4):
            if struct.unpack_from('<I',data,loc+d)[0]==eid: found.append(('u32',d))
        if found:
            print(f"{name} eid={eid:#06x} ptrloc={loc:#x} row={row.hex()} idpos={found}")
            if layout is None and found: layout=found[0]
print("learned layout:", layout)

if layout:
    kind,delta=layout
    print("\n=== target ids ===")
    for name in targets:
        o=noff(name)
        if o is None: print(f"{name}: MISSING"); continue
        locs=find32(va(o))
        ids=set()
        for loc in locs:
            try:
                if kind=='u16': ids.add(struct.unpack_from('<H',data,loc+delta)[0])
                else: ids.add(struct.unpack_from('<I',data,loc+delta)[0])
            except Exception: pass
        print(f"{name}: ptrlocs={len(locs)} id(s)={[hex(x) for x in sorted(ids)]}")
