#!/usr/bin/env python3
# Read-only multi-binary string hunt for an operational-mode / attach-enable SET
# SIT opcode. Prints matches + short ASCII context. No secrets (these are code
# symbol/debug strings, not NV/SIM data).
import re, os

FILES = {
 "CP":  "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin",
 "sitril-builder": "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-sitril-builder-0x25f54000.so",
 "libsitril": "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-libsitril-0xd135000.so",
 "rild-hint": "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-rild-hint-0xa4cb000.so",
 "oemipc1": "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-25e7b000.so",
}

TOKENS = [b"SetOperationMode", b"OperationMode", b"OperatingMode", b"OPERATION_MODE",
 b"SET_OPERATION", b"OperMode", b"SetOperMode", b"AttachMode", b"SetAttach",
 b"ServiceDomain", b"SetServiceDomain", b"SetNetworkMode", b"OnlineMode",
 b"OfflineMode", b"SetModem", b"Cfun", b"CFUN", b"SetPsService", b"EnableData",
 b"DetachMode", b"FactoryTest", b"SetGcf", b"GCF", b"SET_MODE", b"SetMode",
 b"SIT_SET", b"SIT_OEM", b"OEM_", b"operation_mode", b"attach_enable",
 b"SetScreenState", b"Nvm", b"WriteNv", b"SetNv", b"SET_UE", b"UE_OPERATION"]

def ascii_ctx(data, off, back=48, fwd=80):
    lo=max(0,off-back); hi=min(len(data),off+fwd)
    cur=bytearray(); out=[]
    for b in data[lo:hi]:
        if 32<=b<127: cur.append(b)
        else:
            if len(cur)>=4: out.append(cur.decode()); 
            cur=bytearray()
    if len(cur)>=4: out.append(cur.decode())
    return " | ".join(out)

for name,path in FILES.items():
    if not os.path.exists(path):
        print("MISSING", name); continue
    data=open(path,"rb").read()
    print("######", name, len(data))
    for t in TOKENS:
        idxs=[m.start() for m in re.finditer(re.escape(t),data)]
        if idxs:
            print(" ", t.decode(), "x", len(idxs))
            for i in idxs[:2]:
                print("     @%x: %s" % (i, ascii_ctx(data,i)))
