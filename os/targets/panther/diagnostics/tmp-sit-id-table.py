#!/usr/bin/env python3
# Derive SIT opcode IDs by locating the CP {id, name_ptr} table entries.
# For each target name: find its NUL-terminated string offset, try candidate
# VA bases to form the pointer, search the image for that 8-byte LE pointer,
# and dump nearby bytes to reveal the adjacent opcode id. Validate on anchors.
import re, struct
CP = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
data = open(CP, "rb").read()

def name_off(name):
    m = re.search(re.escape(name.encode()) + rb"\x00", data)
    return m.start() if m else None

# Known anchors (name -> expected opcode id)
anchors = {"SIT_SIM_IO":0x0208, "SIT_SET_RADIO_POWER":0x0800, "SIT_GET_RADIO_STATE":0x0801}
targets = list(anchors) + [
 "SIT_GET_PS_SERVICE_DOMAIN","SIT_SET_PS_SERVICE_DOMAIN","SIT_GET_VOICE_OPERATION",
 "SIT_SET_VOICE_OPERATION","SIT_SET_INTPS_SERVICE","SIT_SET_PS_SERVICE",
 "SIT_GET_PS_SERVICE","SIT_SET_DEVICE_SERVICE","SIT_GET_DEVICE_SERVICE",
 "SIT_SET_MODEM_CONFIG","SIT_SET_DUAL_NTW_AND_PS_TYPE","SIT_GET_PS_REG_STATE",
 "SIT_SET_MOBILE_DATA_STATE","SIT_DS_UPDATE_MODEM_NETWORK_STATUS",
]

offs = {t: name_off(t) for t in targets}
for t in targets:
    print("name", t, hex(offs[t]) if offs[t] is not None else "MISSING")

# Determine the VA base by brute-forcing: for an anchor, its pointer appears in
# the table immediately followed/preceded by its id. Try a set of plausible
# bases; pick the base for which the anchor pointer exists AND a 16/32-bit field
# within +/-16 bytes equals the expected id for ALL anchors.
cand_bases = [0x40010000 - 0x16c10, 0x40000000, 0x40010000, 0x0, 0x40010000-0x1000]
# Also derive base generically: search for any 8-byte LE value that could be a
# pointer to SIT_SIM_IO's string across a wide base range by scanning the image
# for the id 0x0208 near a pointer into the names region.

def find_ptr(ptr):
    pb = struct.pack('<Q', ptr)
    return [m.start() for m in re.finditer(re.escape(pb), data)]

for base in cand_bases:
    sim_va = base + offs["SIT_SIM_IO"]
    locs = find_ptr(sim_va)
    if locs:
        print(f"BASE {hex(base)} -> SIT_SIM_IO ptr {hex(sim_va)} found at {[hex(x) for x in locs]}")
        for loc in locs[:3]:
            ctx = data[loc-16:loc+24]
            print("   ctx:", ctx.hex())
