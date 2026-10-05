#!/usr/bin/env python3
"""Recover SIT opcode ids from the aarch64 vendor RIL builder carve.
Strategy: in each BuildXxx function the opcode is a movz/mov immediate and the
name string is referenced via adrp/add for logging; both live in the same code
window. Parse objdump output: locate adrp/add that compute a SIT_* name's file
offset (ELF is PIE, vaddr base 0 => vaddr==file off), then take the nearest
movz w?,#imm (imm in SIT range) within +/-200 bytes. Validate on anchors."""
import re, subprocess
SO="/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-sitril-builder-0x25f54000.so"
data=open(SO,"rb").read()

def nameoff(name):
    m=re.search(re.escape(name.encode())+rb"\x00",data); return m.start() if m else None

# disassemble raw (vma 0)
dis=subprocess.run(["aarch64-linux-gnu-objdump","-D","-b","binary","-maarch64",
                    "--adjust-vma=0",SO],capture_output=True,text=True).stdout
lines=dis.splitlines()
# index: addr -> (text)
rows=[]
for ln in lines:
    m=re.match(r"\s*([0-9a-f]+):\s+([0-9a-f ]+)\t(.*)",ln)
    if m:
        rows.append((int(m.group(1),16),m.group(3).strip()))
addr2i={a:i for i,(a,_) in enumerate(rows)}
print("disasm rows:",len(rows))

# Precompute adrp/add page resolution per instruction index to a target address.
# We instead: for a given name file-offset (==vaddr), find adrp whose page matches
# and a subsequent add with :lo12: matching, in nearby rows. objdump prints the
# resolved address as a trailing comment "// <hex>" for adrp+add? Not always.
# Simpler: scan for 'adrp xN, 0xPAGE' then 'add xN, xN, #0xLO' producing target.
adrps=[]
for i,(a,t) in enumerate(rows):
    m=re.match(r"adrp\s+(x\d+),\s+0x([0-9a-f]+)",t)
    if m: adrps.append((i,a,m.group(1),int(m.group(2),16)))

def find_name_refs(target):
    page=target & ~0xfff
    refs=[]
    for (i,a,reg,pg) in adrps:
        if pg!=page: continue
        for j in range(i+1,min(i+8,len(rows))):
            aj,tj=rows[j]
            m=re.match(r"add\s+"+reg+r",\s+"+reg+r",\s+#0x([0-9a-f]+)",tj)
            if m and (page+int(m.group(1),16))==target:
                refs.append(a)
    return refs

def nearest_opcode(addr):
    i=addr2i.get(addr)
    if i is None:
        # find closest row
        i=min(range(len(rows)),key=lambda k:abs(rows[k][0]-addr))
    best=None
    for j in range(max(0,i-60),min(len(rows),i+60)):
        t=rows[j][1]
        m=re.match(r"mov[z]?\s+w\d+,\s+#0x([0-9a-f]+)",t)
        if m:
            v=int(m.group(1),16)
            if 0x180<=v<0x4800:
                d=abs(rows[j][0]-addr)
                if best is None or d<best[1]: best=(v,d)
    return best

for name in ["SIT_SET_RADIO_POWER","SIT_GET_RADIO_STATE","SIT_SETUP_DATA_CALL",
             "SIT_SET_PS_SERVICE_DOMAIN","SIT_GET_PS_SERVICE_DOMAIN",
             "SIT_SET_INTPS_SERVICE","SIT_SET_DEVICE_SERVICE","SIT_GET_DEVICE_SERVICE",
             "SIT_SET_VOICE_OPERATION","SIT_GET_VOICE_OPERATION","SIT_SET_MODEM_CONFIG"]:
    o=nameoff(name)
    if o is None: print(f"{name}: not in RIL"); continue
    refs=find_name_refs(o)
    ops=[nearest_opcode(r) for r in refs]
    ops=[x for x in ops if x]
    print(f"{name}: off={o:#x} refs={[hex(r) for r in refs]} opcodes={[hex(v) for v,_ in ops]}")
