#!/usr/bin/env python3
"""Enumerate SIT_REG(opcode_id, descriptor_ptr) registrations and resolve each
descriptor's name pointer to build an id->name map. Thumb-2, file-offset space
for branch targets; MOVW/MOVT build absolute VAs in MAIN (load 0x40010000,
foff 0x16c10). Output only symbol strings + ids."""
import struct, re
IMG = open("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin","rb").read()
SIT_REG = 0x20D1AFA
MAIN_LOAD=0x40010000; MAIN_FOFF=0x16c10
def va2off(va): return va - MAIN_LOAD + MAIN_FOFF
def off_ok(o): return 0 <= o < len(IMG)
def u16(o): return struct.unpack_from("<H",IMG,o)[0]
def u32(o): return struct.unpack_from("<I",IMG,o)[0]

def bl_target(o):
    hw,hw2=u16(o),u16(o+2)
    if (hw&0xF800)!=0xF000 or (hw2&0xD000)!=0xD000: return None
    s=(hw>>10)&1; imm10=hw&0x3FF; j1=(hw2>>13)&1; j2=(hw2>>11)&1; imm11=hw2&0x7FF
    i1=~(j1^s)&1; i2=~(j2^s)&1
    imm32=(s<<24)|(i1<<23)|(i2<<22)|(imm10<<12)|(imm11<<1)
    if s:
        imm32|=~((1<<25)-1)&0xFFFFFFFF
        if imm32>=0x80000000: imm32-=0x100000000
    return o+4+imm32

def movw(o):
    hw,hw2=u16(o),u16(o+2)
    if (hw&0xFBF0)!=0xF240 or (hw2&0x8000): return None
    i=(hw>>10)&1; imm=(i<<11)|((hw&0xF)<<12)|(((hw2>>12)&7)<<8)|(hw2&0xFF)
    return imm,(hw2>>8)&0xF
def movt(o):
    hw,hw2=u16(o),u16(o+2)
    if (hw&0xFBF0)!=0xF2C0 or (hw2&0x8000): return None
    i=(hw>>10)&1; imm=(i<<11)|((hw&0xF)<<12)|(((hw2>>12)&7)<<8)|(hw2&0xFF)
    return imm,(hw2>>8)&0xF

def cstr(off, maxlen=64):
    e=IMG.find(b"\x00",off,off+maxlen)
    if e<0: return None
    try: return IMG[off:e].decode("ascii")
    except: return None

# Find all BL->SIT_REG in MAIN code
sites=[]
p=MAIN_FOFF
end=0x2200000  # code region upper bound (well past dispatch area)
while p+4<end:
    if bl_target(p)==SIT_REG:
        sites.append(p)
    p+=2
print("BL->SIT_REG sites:", len(sites))

# For each site, recover reg state by a tiny backward scan building regs via MOVW/MOVT
idmap={}
for s in sites:
    regs={}
    for j in range(s-0x40, s, 2):
        mw=movw(j)
        if mw: regs[mw[1]]=(regs.get(mw[1],0)&0xFFFF0000)|mw[0]
        mt=movt(j)
        if mt: regs[mt[1]]=(regs.get(mt[1],0)&0xFFFF)|(mt[0]<<16)
    rid=regs.get(0); desc=regs.get(1)
    name=None
    if desc and 0x40010000<=desc<0x46000000:
        do=va2off(desc)
        if off_ok(do):
            # scan descriptor words for a pointer into names region
            for w in range(0,0x28,4):
                if off_ok(do+w+4):
                    ptr=u32(do+w)
                    if 0x40010000<=ptr<0x46000000:
                        no=va2off(ptr)
                        if off_ok(no):
                            cs=cstr(no)
                            if cs and cs.startswith("SIT_"):
                                name=cs; break
    if rid is not None and rid<0x10000:
        idmap.setdefault(rid,set())
        if name: idmap[rid].add(name)

print("distinct ids:", len(idmap))
WANT=("PS_SERVICE_DOMAIN","DEVICE_SERVICE","VOICE_OPERATION","INTPS","MODEM_CONFIG",
      "DUAL_NTW","PS_REG","MOBILE_DATA","NORMAL_START","RADIO_POWER","RADIO_STATE",
      "SIM_IO","SETUP_DATA","PS_SERVICE","ATTACH","DEVICE_SERVICE")
for rid in sorted(idmap):
    names=idmap[rid]
    if any(any(w in n for w in WANT) for n in names):
        print(f"  {rid:#06x}: {sorted(names)}")
print("--- anchors check ---")
for rid in sorted(idmap):
    if rid in (0x208,0x800,0x801,0x600):
        print(f"  anchor {rid:#06x}: {sorted(idmap[rid])}")
