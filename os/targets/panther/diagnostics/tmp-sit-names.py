#!/usr/bin/env python3
# Dump the SIT opcode-name table strings from the CP image region and the
# vendor RIL, filtering for operational-mode / attach / service / PS / config
# SET and GET names. Symbol/debug strings only.
import re
CP = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
RIL = "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-sitril-builder-0x25f54000.so"

def all_sit_names(path):
    data = open(path,"rb").read()
    names=set()
    for m in re.finditer(rb"SIT_[A-Z0-9_]{3,60}", data):
        names.add(m.group().decode())
    return sorted(names)

cp = all_sit_names(CP)
ril = all_sit_names(RIL)
alln = sorted(set(cp) | set(ril))
print("CP SIT names:", len(cp), " RIL SIT names:", len(ril), " union:", len(alln))
KEY = ("OPER","OPERATION","ATTACH","SERVICE","PS_","INTPS","MODE","CONFIG","ONLINE",
       "OFFLINE","DETACH","NETWORK","CFUN","DOMAIN","NORMAL","GCF","SGC","POWER","RADIO","DATA")
print("==== candidate SET/GET names ====")
for n in alln:
    if any(k in n for k in KEY):
        where = ("CP" if n in cp else "") + ("+RIL" if n in ril else "")
        print(f"  {n}  [{where}]")
