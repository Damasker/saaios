#!/usr/bin/env python3
import re
IMG = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
data = open(IMG, "rb").read()

def ctx(off, back=120, fwd=180):
    lo=max(0,off-back); hi=min(len(data),off+fwd)
    out=[]; cur=bytearray()
    for b in data[lo:hi]:
        if 32<=b<127: cur.append(b)
        else:
            if len(cur)>=4: out.append(cur.decode())
            cur=bytearray()
    if len(cur)>=4: out.append(cur.decode())
    return out

for t in [b"GetRfCalDate", b"RfCalDate", b"RF_CAL", b"RfCalValid", b"CalDone",
          b"FactoryTestMode", b"FTM", b"LimitedService", b"NO_SERVICE",
          b"NoService", b"SAE_UE_OPERATION_MODE", b"UE_OPERATION_MODE",
          b"NORMAL_MODE", b"PsOperationMode", b"MM_STATE", b"RegDenied",
          b"AttachReject", b"attach_reject", b"local", b"GmmState"]:
    idxs=[m.start() for m in re.finditer(re.escape(t),data)]
    print(t.decode(), len(idxs), [hex(i) for i in idxs[:3]])

print("==== SAE_UE_OPERATION_MODE ctx ====")
m=re.search(re.escape(b"SAE_UE_OPERATION_MODE"),data)
if m:
    for s in ctx(m.start(),200,260): print("  ",s)
print("==== RfCalDate ctx ====")
m=re.search(re.escape(b"RfCalDate"),data)
if m:
    for s in ctx(m.start(),200,260): print("  ",s)
