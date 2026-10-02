import sys, math, collections

p = "/mnt/c/Users/Admin/AppData/Local/Temp/candidate-quar.bin"
d = open(p,"rb").read()
print("size", len(d))
print("head64", d[:64].hex())
# interpret header words
import struct
words = struct.unpack("<16I", d[:64])
print("head words LE:", [hex(w) for w in words])

def ent(b):
    if not b: return 0.0
    c = collections.Counter(b); n=len(b); e=0.0
    for v in c.values():
        pr=v/n; e-=pr*math.log2(pr)
    return e

# per 4KB block entropy summary
blk=4096
ents=[ent(d[i:i+blk]) for i in range(0,len(d),blk)]
hi=sum(1 for e in ents if e>7.5)
mid=sum(1 for e in ents if 4.0<=e<=7.5)
lo=sum(1 for e in ents if e<4.0)
zero=sum(1 for i in range(0,len(d),blk) if d[i:i+blk]==b"\x00"*len(d[i:i+blk]))
ff=sum(1 for i in range(0,len(d),blk) if d[i:i+blk]==b"\xff"*len(d[i:i+blk]))
print(f"blocks={len(ents)} entropy hi(>7.5)={hi} mid={mid} lo(<4)={lo} zeroblk={zero} ffblk={ff}")
print("overall entropy", round(ent(d),3))
# where does non-high-entropy content sit? first 16 block entropies
print("first20 blk ent:", [round(e,2) for e in ents[:20]])
# count printable-ascii runs >=8
import re
runs = re.findall(rb"[ -~]{8,}", d)
print("ascii runs>=8:", len(runs), " total ascii bytes", sum(len(r) for r in runs))
# filesystem/efs magics
for mg,nm in [(b"\x53\xEF","ext2/3/4@0x438"),(b"EFSP","EFSP"),(b"\x85\x19","jffs2"),(b"-rom1fs-","romfs"),(b"SHNN","shannon"),(b"NVNV","nvnv")]:
    idx=d.find(mg)
    if idx>=0: print("magic",nm,"@",hex(idx))
# nonzero tail region boundary (where CP data 189446 ends)
nz_last=max((i for i in range(len(d)) if d[i]!=0), default=-1)
print("last nonzero byte offset", hex(nz_last), nz_last)
