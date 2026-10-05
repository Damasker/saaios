#!/usr/bin/env python3
"""Find ADR/ADRP refs to ipc1; check SlotMapping from SlotStatus."""
import struct, subprocess
from pathlib import Path

libp = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
data = libp.read_bytes()
text_off = text_vma = 0xf7000
hdr = subprocess.check_output(["aarch64-linux-gnu-objdump","-h",str(libp)], text=True)
for line in hdr.splitlines():
    parts=line.split()
    if len(parts)>=6 and parts[1]=='.text':
        text_size=int(parts[2],16)
text = data[text_off:text_off+text_size]

# ADR (literal): op=0 immlo?
# ADR: 0xx10000 immlo(2) immhi(19) Rd — same as ADRP but page bit differs
# ADR encoding: 0bb10000 where bb=immlo, bit 31=0 for ADR vs ADRP bit31 related
# Actually ADR: 0 immlo[1:0] 10000 immhi Rd  (bit31=0)
# ADRP: 1 immlo 10000 immhi Rd (bit31=1)
want = {0xb4531, 0xb4536, 0xb44ef}  # ipc1 paths

def decode_adr(word, pc, is_adrp):
    if is_adrp:
        if (word & 0x9F000000) != 0x90000000: return None, None
    else:
        if (word & 0x9F000000) != 0x10000000: return None, None
    rd = word & 0x1F
    immlo = (word >> 29) & 3
    immhi = (word >> 5) & 0x7FFFF
    imm = (immhi << 2) | immlo
    if imm & (1<<20): imm -= 1<<21
    if is_adrp:
        return rd, (pc & ~0xFFF) + (imm << 12)
    return rd, pc + imm

hits=[]
for i in range(0,len(text)-4,4):
    w=struct.unpack_from('<I',text,i)[0]
    pc=text_vma+i
    for is_adrp in (False, True):
        rd, base = decode_adr(w, pc, is_adrp)
        if rd is None: continue
        if not is_adrp:
            if base in want:
                hits.append((pc,'ADR',base,rd))
        else:
            for j in range(1,16):
                if i+4*j+4>len(text): break
                w2=struct.unpack_from('<I',text,i+4*j)[0]
                if (w2 & 0xFF000000)!=0x91000000: continue
                rd2=(w2)&0x1F; rn=(w2>>5)&0x1F; imm12=(w2>>10)&0xFFF; sh=(w2>>22)&1
                if sh: imm12<<=12
                if rn==rd:
                    addr=base+imm12
                    if addr in want:
                        hits.append((pc,'ADRP+ADD',addr,rd))
                    break
print('ipc1 hits', hits)

# Search SlotMappingHandler / SetSlotMapping for GetLogicalSlotId / SlotStatus calls
out=subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",str(libp)], text=True, errors="replace")
for line in out.splitlines():
    if "GetLogicalSlot" in line or "GetSlotStatus" in line or "FillSlotStatus" in line or "SetSlotMapping" in line:
        print(line)

# Check if SetSlotMapping or SlotMappingHandler references GetSlotStatus response fields
# Disassemble end of SetSlotMapping where it sends request 0x200000c9 - already know property path
# Look for code that iterates ports and calls BuildSimSetLogicalSlotMapping
stream="/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
# strings
for n in (b"from slot status", b"logical slot mapping", b"port mapping", b"setSimSlotsMapping", b"SlotPortMapping"):
    j=data.find(n)
    print(n, hex(j) if j>=0 else None)
