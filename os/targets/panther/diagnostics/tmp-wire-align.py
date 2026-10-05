#!/usr/bin/env python3
"""Align the RIL name table (12-byte adrp/add/ret stubs, index=internal id) with
the 4-byte-stride wire-opcode table to recover wire opcodes per SIT name.
Validate on anchors (RADIO_POWER=0x800, SETUP_DATA_CALL=0x600)."""
import struct, re
SO="/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-sitril-builder-0x25f54000.so"
d=open(SO,"rb").read()
def u16(o): return struct.unpack_from("<H",d,o)[0]
def u32(o): return struct.unpack_from("<I",d,o)[0]

# ---- decode aarch64 adrp + add at stub to get name address ----
def adrp_add_name(stub):
    w0=u32(stub); w1=u32(stub+4)
    # ADRP: bit31=1, bits[28:24]=10000
    if (w0>>24)&0x9f != 0x90: return None
    immlo=(w0>>29)&3; immhi=(w0>>5)&0x7ffff
    imm=(immhi<<2)|immlo
    if imm & (1<<20): imm-=(1<<21)
    page=(stub & ~0xfff) + (imm<<12)
    # ADD imm: bits[31]=0 sf, [30:24]=0100010? ADD(imm)=100100010? check opc
    if (w1>>24)&0x7f != 0x11: return None
    imm12=(w1>>10)&0xfff; sh=(w1>>22)&1
    if sh: imm12<<=12
    addr=page+imm12
    if 0<=addr<len(d):
        e=d.find(b"\x00",addr,addr+64)
        try: return d[addr:e].decode("ascii")
        except: return None
    return None

# Build name table: scan for the dense run of 12-byte adrp/add/ret producing SIT_ names
# Start from the observed region.
NT_START=0x218400
idx2name={}
i=0; o=NT_START
# extend downward to find true base (keep going back while entries resolve to SIT_)
# find base
base=NT_START
while base-12>=0:
    nm=adrp_add_name(base-12)
    if nm and nm.startswith("SIT_"): base-=12
    else: break
# forward fill
o=base; idx=0
while o+12<=len(d):
    nm=adrp_add_name(o)
    if nm and nm.startswith("SIT_"):
        idx2name[idx]=nm
    else:
        # allow a few holes then stop
        if idx>0 and all((base+12*k) and not (adrp_add_name(base+12*k) or "").startswith("SIT_") for k in range(idx,idx+3)):
            break
    o+=12; idx+=1
    if idx>1200: break
name2idx={v:k for k,v in idx2name.items()}
print("name table base",hex(base),"entries",len(idx2name))
for a in ("SIT_SET_RADIO_POWER","SIT_SETUP_DATA_CALL","SIT_SIM_IO"):
    print("  idx",a,name2idx.get(a))

# ---- wire table: stride 4, u16 wire + u16 flags, base so that idx(radio_power)->0x800
rp=name2idx.get("SIT_SET_RADIO_POWER")
# candidate wire record for radio power is at 0x314a2 (from scan). derive base:
WT_RP=0x314a2
wt_base=WT_RP-rp*4
print("wire table base guess",hex(wt_base))
def wire(idx):
    o=wt_base+idx*4
    if 0<=o<len(d)-1: return u16(o)
    return None
# validate anchors
for a,exp in (("SIT_SET_RADIO_POWER",0x800),("SIT_SETUP_DATA_CALL",0x600),("SIT_SIM_IO",0x208)):
    ix=name2idx.get(a); print(f"  anchor {a} idx={ix} wire={wire(ix) if ix is not None else None:#x} expect={exp:#x}" if ix is not None else f"  {a} missing")

print("=== targets ===")
for a in ("SIT_GET_PS_SERVICE_DOMAIN","SIT_SET_PS_SERVICE_DOMAIN","SIT_SET_INTPS_SERVICE",
          "SIT_SET_DEVICE_SERVICE","SIT_GET_DEVICE_SERVICE","SIT_SET_VOICE_OPERATION",
          "SIT_GET_VOICE_OPERATION","SIT_SET_MODEM_CONFIG","SIT_SET_DUAL_NTW_AND_PS_TYPE",
          "SIT_GET_PS_REG_STATE","SIT_SET_MOBILE_DATA_STATE"):
    ix=name2idx.get(a)
    w=wire(ix) if ix is not None else None
    print(f"  {a}: idx={ix} wire={('%#x'%w) if w is not None else None}")
