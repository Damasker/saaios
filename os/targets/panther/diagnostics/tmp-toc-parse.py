#!/usr/bin/env python3
# Parse the Samsung modem.bin TOC and report each segment's file offset, load
# address and size. Then show which segment contains the SIT names region
# (file ~0x102a000) so we can compute its runtime VA.
import struct
CP = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
data = open(CP, "rb").read()

# TOC entry classic layout: char name[12]; u32 file_off; u32 load_addr; u32 size;
# u32 crc; u32 entry_id  => 32 bytes. Parse first ~32 entries.
print("=== TOC (32-byte entries) ===")
segs=[]
for i in range(40):
    e = data[i*32:(i+1)*32]
    if len(e) < 32: break
    name = e[0:12].split(b"\x00")[0]
    foff, laddr, size, crc, eid = struct.unpack("<5I", e[12:32])
    # Heuristic: valid entries have printable name or zero
    pname = name.decode("latin1","replace")
    if foff > len(data)*2 and laddr==0 and size==0: 
        pass
    print(f"{i:2d} name={pname:12s} foff={foff:#010x} load={laddr:#010x} size={size:#010x} crc={crc:#010x} id={eid}")
    segs.append((pname,foff,laddr,size))

NAMES=0x102a3ba
print("\n=== segment containing file %#x ===" % NAMES)
for pname,foff,laddr,size in segs:
    if size and foff <= NAMES < foff+size:
        va = laddr + (NAMES - foff)
        print(f"  seg {pname}: foff={foff:#x} load={laddr:#x} size={size:#x} -> names VA={va:#x}")
