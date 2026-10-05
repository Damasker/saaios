#!/usr/bin/env python3
"""Find a SitMsgInfo-style table: an array of fixed-stride records containing
the known wire opcodes. Search both RIL carve and CP image. Report stride and
the field offset holding the opcode, then dump records near the anchors."""
import struct
FILES={
 "RIL":"/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-sitril-builder-0x25f54000.so",
 "CP":"/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin",
}
anchors=[0x0800,0x0801,0x0600,0x0208,0x0704,0x0710,0x070a]
for name,path in FILES.items():
    data=open(path,"rb").read()
    print("######",name,len(data))
    # find all u16 positions of each anchor
    pos={a:[m for m in range(0,len(data)-1,2) if struct.unpack_from('<H',data,m)[0]==a] for a in [0x0800]}
    # too many; instead look for 0x0800 and 0x0801 appearing within 64 bytes (adjacent records)
    locs800=[o for o in range(0,len(data)-1) if data[o]==0x00 and data[o+1]==0x08]
    hits=[]
    for o in locs800:
        # look for 0x0801 within +/-128 at same low-byte alignment
        for d in range(4,129,2):
            for s in (o+d,o-d):
                if 0<=s<len(data)-1 and struct.unpack_from('<H',data,s)[0]==0x0801:
                    hits.append((o,s,abs(d)))
                    break
    # pick the smallest-stride candidate
    hits.sort(key=lambda x:x[2])
    print("  0x0800/0x0801 proximate pairs (closest 6):",[(hex(a),hex(b),st) for a,b,st in hits[:6]])
