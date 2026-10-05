#!/usr/bin/env python3
import re, subprocess
SO="/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-sitril-builder-0x25f54000.so"
data=open(SO,"rb").read()
def nameoff(name):
    m=re.search(re.escape(name.encode())+rb"\x00",data); return m.start() if m else None
dis=subprocess.run(["aarch64-linux-gnu-objdump","-D","-b","binary","-maarch64","--start-address=0x218400","--stop-address=0x219100",SO],capture_output=True,text=True).stdout
# annotate adrp/add resolving to a SIT_ name
rows=[]
for ln in dis.splitlines():
    m=re.match(r"\s*([0-9a-f]+):\s+[0-9a-f ]+\t(.*)",ln)
    if m: rows.append((int(m.group(1),16),m.group(2).strip()))
# resolve adrp+add
pending={}
def nm_at(off):
    # name string starting at off
    e=data.find(b"\x00",off,off+64)
    try: return data[off:e].decode("ascii")
    except: return None
out=[]
for i,(a,t) in enumerate(rows):
    ann=""
    m=re.match(r"adrp\s+(x\d+),\s+0x([0-9a-f]+)",t)
    if m: pending[m.group(1)]=int(m.group(2),16)
    m=re.match(r"add\s+(x\d+),\s+(x\d+),\s+#0x([0-9a-f]+)",t)
    if m and m.group(2) in pending:
        tgt=pending[m.group(2)]+int(m.group(3),16)
        nm=nm_at(tgt)
        if nm and nm.startswith("SIT_"): ann=f"   ; -> {nm}"
    out.append(f"{a:#x}: {t}{ann}")
print("\n".join(out))
