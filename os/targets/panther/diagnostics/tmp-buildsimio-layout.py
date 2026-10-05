#!/usr/bin/env python3
"""Disasm BuildSimIO (0x0208) payload layout — no invent."""
from __future__ import annotations
import struct
from pathlib import Path

SS = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so").read_bytes()
LS = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so").read_bytes()

def u32(b,o): return struct.unpack_from('<I',b,o)[0]
def u16(b,o): return struct.unpack_from('<H',b,o)[0]

def decode(blob, start, nins=400):
    out=[]
    for i in range(start, min(len(blob)-3, start+nins*4), 4):
        w=u32(blob,i)
        if w==0xD65F03C0:
            out.append((i,'RET')); break
        if (w & 0x7F800000)==0x71000000:
            rn=(w>>5)&0x1F; imm=(w>>10)&0xFFF; sf=(w>>31)&1
            out.append((i,f'CMP {"x" if sf else "w"}{rn},#{imm}')); continue
        if (w & 0xFF800000)==0x52800000:
            imm16=(w>>5)&0xFFFF; hw=(w>>21)&3; rd=w&0x1F
            out.append((i,f'MOVZ w{rd},#{hex(imm16<<(hw*16))}')); continue
        if (w & 0xFF800000)==0x72800000:
            imm16=(w>>5)&0xFFFF; hw=(w>>21)&3; rd=w&0x1F
            out.append((i,f'MOVK w{rd},#{hex(imm16)},LSL#{hw*16}')); continue
        if (w & 0xFF000000)==0x54000000:
            imm19=(w>>5)&0x7FFFF
            if imm19&(1<<18): imm19-=1<<19
            conds='EQ NE CS CC MI PL VS VC HI LS GE LT GT LE AL NV'.split()
            out.append((i,f'B.{conds[w&0xF]} {hex(i+imm19*4)}')); continue
        if (w & 0xFC000000)==0x14000000:
            imm26=w&0x3FFFFFF
            if imm26&(1<<25): imm26-=1<<26
            out.append((i,f'B {hex(i+imm26*4)}')); continue
        if (w & 0xFC000000)==0x94000000:
            imm26=w&0x3FFFFFF
            if imm26&(1<<25): imm26-=1<<26
            out.append((i,f'BL {hex(i+imm26*4)}')); continue
        if (w & 0x7FE0FFE0)==0x2A0003E0:
            rd=w&0x1F; rm=(w>>16)&0x1F; sf=(w>>31)&1; r='x' if sf else 'w'
            out.append((i,f'MOV {r}{rd},{r}{rm}')); continue
        if (w & 0x7F000000)==0x11000000:
            rd=w&0x1F; rn=(w>>5)&0x1F; imm12=(w>>10)&0xFFF; sh=(w>>22)&1; sf=(w>>31)&1
            op='ADD' if ((w>>30)&1)==0 else 'SUB'; r='x' if sf else 'w'
            out.append((i,f'{op} {r}{rd},{r}{rn},#{imm12<<(12*sh)}')); continue
        if (w & 0xFFC00000)==0xB9400000:
            rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*4
            out.append((i,f'LDR w{rt},[x{rn},#{imm}]')); continue
        if (w & 0xFFC00000)==0xF9400000:
            rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*8
            out.append((i,f'LDR x{rt},[x{rn},#{imm}]')); continue
        if (w & 0xFFC00000)==0xB9000000:
            rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*4
            out.append((i,f'STR w{rt},[x{rn},#{imm}]')); continue
        if (w & 0xFFC00000)==0x39000000:
            rt=w&0x1F; rn=(w>>5)&0x1F; imm=(w>>10)&0xFFF
            out.append((i,f'STRB w{rt},[x{rn},#{imm}]')); continue
        if (w & 0xFFC00000)==0x39400000:
            rt=w&0x1F; rn=(w>>5)&0x1F; imm=(w>>10)&0xFFF
            out.append((i,f'LDRB w{rt},[x{rn},#{imm}]')); continue
        if (w & 0xFFC00000)==0x79000000:
            rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*2
            out.append((i,f'STRH w{rt},[x{rn},#{imm}]')); continue
        if (w & 0xFFC00000)==0x79400000:
            rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*2
            out.append((i,f'LDRH w{rt},[x{rn},#{imm}]')); continue
        if (w & 0x7E000000)==0x34000000:
            rt=w&0x1F; imm19=(w>>5)&0x7FFFF; is64=(w>>31)&1; op=(w>>24)&1
            if imm19&(1<<18): imm19-=1<<19
            r='x' if is64 else 'w'
            out.append((i,f'{"CBNZ" if op else "CBZ"} {r}{rt},{hex(i+imm19*4)}')); continue
        out.append((i,f'?{hex(w)}'))
    return out

def interesting(s):
    return any(k in s for k in ('MOVZ','MOVK','CMP','B.','B ','BL ','STR','LDR','RET','MOV ','ADD ','SUB ','CBZ','CBNZ'))

print('=== BuildSimIO @0x7e0d0 ===')
for i,s in decode(SS, 0x7e0d0, 420):
    if interesting(s):
        print(f'  {hex(i)} {s}')

# Adapter strings for SimIO fields
print('\n=== SimIO adapter / field strings ===')
for nb in [b'SimIO', b'SimIo', b'GetCommand', b'GetFileId', b'GetPath', b'GetP1', b'GetP2', b'GetP3', b'GetData', b'GetPin2', b'GetAid']:
    pos=0; c=0
    while c<8:
        i=SS.find(nb, pos)
        if i<0: break
        a=i
        while a>0 and 32<=SS[a-1]<127: a-=1
        b=i
        while b<len(SS) and SS[b] and b-a<160: b+=1
        s=SS[a:b].decode('ascii','ignore')
        if 'SimIO' in s or 'SimIo' in s or 'IOAdapter' in s:
            print(f'  @{hex(i)} {s}')
        pos=i+1; c+=1

# libsitril DoSimIo
print('\n=== libsitril DoSimIo / SimIoService ===')
for nb in [b'DoSimIo', b'SimIoService', b'RIL_SIM_IO', b'SIM_IO', b'BuildSimIO']:
    pos=0; c=0
    while c<10:
        i=LS.find(nb, pos)
        if i<0: break
        a=i
        while a>0 and 32<=LS[a-1]<127: a-=1
        b=i
        while b<len(LS) and LS[b] and b-a<180: b+=1
        print(f'  @{hex(i)} {LS[a:b].decode("ascii","ignore")}')
        pos=i+1; c+=1
print('DONE')
