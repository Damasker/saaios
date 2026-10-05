#!/usr/bin/env python3
from pathlib import Path
import struct
SS=Path('/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so').read_bytes()
def u32(b,o): return struct.unpack_from('<I',b,o)[0]
base=196
print('=== BuildSimIO stores relative to pkt (sp+#196) ===')
for i in range(0x7e0d0, 0x7e0d0+1672, 4):
    w=u32(SS,i)
    if (w & 0xFFC00000)==0x39000000:
        rt=w&0x1F; rn=(w>>5)&0x1F; imm=(w>>10)&0xFFF
        if rn==31:
            print(f'{hex(i)} STRB w{rt},[sp,#{imm}] pkt+{imm-base if imm>=base else "tmp"+str(imm)}')
    elif (w & 0xFFC00000)==0x79000000:
        rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*2
        if rn==31:
            print(f'{hex(i)} STRH w{rt},[sp,#{imm}] pkt+{imm-base if imm>=base else "tmp"+str(imm)}')
    elif (w & 0xFFC00000)==0xB9000000:
        rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*4
        if rn==31:
            print(f'{hex(i)} STR w{rt},[sp,#{imm}] pkt+{imm-base if imm>=base else "tmp"+str(imm)}')

# Also dump early arg save and the length/id immediates neighborhood around fill
print('\n=== context around fill 0x7e448..0x7e720 ===')
def decode_one(i):
    w=u32(SS,i)
    if (w & 0xFF800000)==0x52800000:
        imm16=(w>>5)&0xFFFF; hw=(w>>21)&3; rd=w&0x1F
        return f'MOVZ w{rd},#{hex(imm16<<(hw*16))}'
    if (w & 0x7FE0FFE0)==0x2A0003E0:
        rd=w&0x1F; rm=(w>>16)&0x1F; sf=(w>>31)&1; r='x' if sf else 'w'
        return f'MOV {r}{rd},{r}{rm}'
    if (w & 0x7F000000)==0x11000000:
        rd=w&0x1F; rn=(w>>5)&0x1F; imm12=(w>>10)&0xFFF; sh=(w>>22)&1; sf=(w>>31)&1
        op='ADD' if ((w>>30)&1)==0 else 'SUB'; r='x' if sf else 'w'
        return f'{op} {r}{rd},{r}{rn},#{imm12<<(12*sh)}'
    if (w & 0xFFC00000)==0xB9400000:
        rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*4
        return f'LDR w{rt},[x{rn},#{imm}]'
    if (w & 0xFFC00000)==0xF9400000:
        rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*8
        return f'LDR x{rt},[x{rn},#{imm}]'
    if (w & 0xFC000000)==0x94000000:
        imm26=w&0x3FFFFFF
        if imm26&(1<<25): imm26-=1<<26
        return f'BL {hex(i+imm26*4)}'
    if (w & 0xFFC00000)==0x39000000:
        rt=w&0x1F; rn=(w>>5)&0x1F; imm=(w>>10)&0xFFF
        return f'STRB w{rt},[x{rn},#{imm}]'
    if (w & 0xFFC00000)==0x79000000:
        rt=w&0x1F; rn=(w>>5)&0x1F; imm=((w>>10)&0xFFF)*2
        return f'STRH w{rt},[x{rn},#{imm}]'
    if (w & 0x7E000000)==0x34000000:
        rt=w&0x1F; imm19=(w>>5)&0x7FFFF; op=(w>>24)&1
        if imm19&(1<<18): imm19-=1<<19
        return f'{"CBNZ" if op else "CBZ"} w{rt},{hex(i+imm19*4)}'
    return None
for i in range(0x7e448, 0x7e720, 4):
    s=decode_one(i)
    if s: print(f'  {hex(i)} {s}')

# mangled demangle reminder
print('\nMangled: BuildSimIOEiiiPKciiiiS1_S1_S1_')
print('= (int,int,int,char const*,int,int,int,int,char const*,char const*,char const*)')
print('RIL_REQUEST_SIM_IO=28 -> BuildOemSimRequest -> SIT 0x208')
print('RIL_REQUEST_SIM_TRANSMIT_APDU_BASIC=114 -> 0x20c')
print('RIL_REQUEST_SIM_OPEN_CHANNEL=115 -> 0x247 (WithP2!)')
print('RIL_REQUEST_SIM_TRANSMIT_APDU_CHANNEL=117 -> 0x20f')
