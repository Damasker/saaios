#!/usr/bin/env python3
"""Recover SIT opcode ids via handler-pointer dispatch table.
For each name: nameVA via MAIN map -> find MOVW/MOVT site building nameVA (inside
handler) -> walk back to Thumb prologue (push) = handler entry F -> search image
for u32==F|1 (dispatch table entry) -> read adjacent u16 for the opcode id.
Validate on anchors with known ids before trusting target results."""
import struct, re
IMG=open("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin","rb").read()
LOAD=0x40010000; FOFF=0x16c10
def off2va(o): return o-FOFF+LOAD
def va2off(v): return v-LOAD+FOFF
def u16(o): return struct.unpack_from("<H",IMG,o)[0]
def u32(o): return struct.unpack_from("<I",IMG,o)[0]
def movw(o):
    hw,hw2=u16(o),u16(o+2)
    if (hw&0xFBF0)!=0xF240 or (hw2&0x8000): return None
    i=(hw>>10)&1; return ((i<<11)|((hw&0xF)<<12)|(((hw2>>12)&7)<<8)|(hw2&0xFF),(hw2>>8)&0xF)
def movt(o):
    hw,hw2=u16(o),u16(o+2)
    if (hw&0xFBF0)!=0xF2C0 or (hw2&0x8000): return None
    i=(hw>>10)&1; return ((i<<11)|((hw&0xF)<<12)|(((hw2>>12)&7)<<8)|(hw2&0xFF),(hw2>>8)&0xF)

CODE_LO, CODE_HI = FOFF, 0x2200000

def nameoff(name):
    m=re.search(re.escape(name.encode())+rb"\x00",IMG); return m.start() if m else None

def movw_movt_sites(target_va):
    """find code offsets where movw+movt (any order, same reg, within 16B) build target_va"""
    lo16=target_va&0xFFFF; hi16=(target_va>>16)&0xFFFF
    hits=[]
    o=CODE_LO
    while o+4<CODE_HI:
        mw=movw(o)
        if mw and mw[0]==lo16:
            for k in range(o+4,o+20,2):
                mt=movt(k)
                if mt and mt[1]==mw[1] and mt[0]==hi16:
                    hits.append(o); break
        mt=movt(o)
        if mt and mt[0]==hi16:
            for k in range(o+4,o+20,2):
                mw2=movw(k)
                if mw2 and mw2[1]==mt[1] and mw2[0]==lo16:
                    hits.append(o); break
        o+=2
    return hits

def prologue_before(o):
    """nearest preceding Thumb push (B5xx) or push.w (E92D) -> handler entry file off"""
    for j in range(o, max(CODE_LO,o-0x400), -2):
        hw=u16(j)
        if (hw&0xFF00)==0xB500: return j      # push {..,lr}
        if hw==0xE92D: return j               # stmdb sp!,{...}
    return None

def dispatch_ids_for_entry(F):
    """search for u32 == (va(F)|1) and read nearby u16 candidates as ids"""
    ptr=off2va(F)|1
    pb=struct.pack('<I',ptr)
    ids=set(); locs=[m.start() for m in re.finditer(re.escape(pb),IMG)]
    for loc in locs:
        for d in (-8,-6,-4,-2,2,4,6,8):
            try:
                v=struct.unpack_from('<H',IMG,loc+d)[0]
                if 0x100<=v<0x4800: ids.add((v,d))
            except: pass
    return locs,ids

def resolve(name):
    no=nameoff(name)
    if no is None: return name,None,"name-missing"
    va=off2va(no)
    sites=movw_movt_sites(va)
    allids={}
    entries=[]
    for s in sites:
        F=prologue_before(s)
        if F is None: continue
        entries.append(F)
        locs,ids=dispatch_ids_for_entry(F)
        for (v,d) in ids:
            allids.setdefault(v,0); allids[v]+=1
    return name,allids,f"xref_sites={len(sites)} entries={len(set(entries))}"

for anchor,eid in [("SIT_GET_RADIO_STATE",0x801),("SIT_SET_RADIO_POWER",0x800),("SIT_SIM_IO",0x208)]:
    n,ids,info=resolve(anchor)
    print(f"ANCHOR {anchor} expect={eid:#x} {info} ids={ {hex(k):v for k,v in (ids or {}).items()} }")
