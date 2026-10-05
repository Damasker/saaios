#!/usr/bin/env python3
"""Factory cold-boot: who calls Build* after ONLINE; order from SimService/NetworkService."""
import struct
from pathlib import Path

# Parse ELF to get symbol addresses if possible, else string-xref via ARM32 BL
libs = {
    "libsitril.so": Path("libsitril.so").read_bytes(),
    "sit-stream.so": Path("sit-stream.so").read_bytes(),
}

def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]

def parse_dynsym(data):
    """Minimal ELF32 dynsym name->addr."""
    if data[:4] != b"\x7fELF":
        return {}
    ei_class = data[4]
    if ei_class != 1:
        # try ELF64
        e_shoff = struct.unpack_from("<Q", data, 0x28)[0]
        e_shentsize = struct.unpack_from("<H", data, 0x3A)[0]
        e_shnum = struct.unpack_from("<H", data, 0x3C)[0]
        e_shstrndx = struct.unpack_from("<H", data, 0x3E)[0]
        def sh(i):
            o = e_shoff + i * e_shentsize
            return {
                "name": u32(data, o),
                "type": u32(data, o + 4),
                "flags": struct.unpack_from("<Q", data, o + 8)[0],
                "addr": struct.unpack_from("<Q", data, o + 16)[0],
                "off": struct.unpack_from("<Q", data, o + 24)[0],
                "size": struct.unpack_from("<Q", data, o + 32)[0],
                "link": u32(data, o + 40),
                "entsize": struct.unpack_from("<Q", data, o + 56)[0],
            }
        shstr = sh(e_shstrndx)
        names = data[shstr["off"]: shstr["off"] + shstr["size"]]
        syms = {}
        for i in range(e_shnum):
            s = sh(i)
            nm = names[s["name"]:].split(b"\x00", 1)[0]
            if nm in (b".dynsym", b".symtab"):
                link = sh(s["link"])
                strtab = data[link["off"]: link["off"] + link["size"]]
                entsize = s["entsize"] or 24
                for j in range(0, s["size"], entsize):
                    o = s["off"] + j
                    st_name = u32(data, o)
                    st_info = data[o + 4]
                    st_value = struct.unpack_from("<Q", data, o + 8)[0]
                    name = strtab[st_name:].split(b"\x00", 1)[0].decode("ascii", "replace")
                    if name and st_value:
                        syms[name] = st_value
        return syms
    # ELF32
    e_shoff = u32(data, 0x20)
    e_shentsize = struct.unpack_from("<H", data, 0x2E)[0]
    e_shnum = struct.unpack_from("<H", data, 0x30)[0]
    e_shstrndx = struct.unpack_from("<H", data, 0x32)[0]
    def sh(i):
        o = e_shoff + i * e_shentsize
        return {
            "name": u32(data, o),
            "type": u32(data, o + 4),
            "addr": u32(data, o + 12),
            "off": u32(data, o + 16),
            "size": u32(data, o + 20),
            "link": u32(data, o + 24),
            "entsize": u32(data, o + 36),
        }
    shstr = sh(e_shstrndx)
    names = data[shstr["off"]: shstr["off"] + shstr["size"]]
    syms = {}
    for i in range(e_shnum):
        s = sh(i)
        nm = names[s["name"]:].split(b"\x00", 1)[0]
        if nm in (b".dynsym", b".symtab"):
            link = sh(s["link"])
            strtab = data[link["off"]: link["off"] + link["size"]]
            entsize = s["entsize"] or 16
            for j in range(0, s["size"], entsize):
                o = s["off"] + j
                st_name = u32(data, o)
                st_value = u32(data, o + 4)
                name = strtab[st_name:].split(b"\x00", 1)[0].decode("ascii", "replace")
                if name and st_value:
                    syms[name] = st_value
    return syms

for lib, data in libs.items():
    syms = parse_dynsym(data)
    print(f"\n######## {lib} syms={len(syms)} ########")
    keys = [
        "DoGetSimStatus", "OnGetSimStatusDone", "CheckAndAutoVerifyPin",
        "DoVerifyPin", "DoGetRadioState", "TrySetRadioPower",
        "DoSetPreferredNetworkType", "DoSetNetworkSelectionAuto",
        "GetDataRegistrationState", "BuildSimGetStatus", "BuildSimVerifyPin",
        "BuildGetRadioState", "BuildSetPreferredNetworkType",
        "BuildSetNetworkSelectionAuto", "BuildNetworkRegistrationState",
        "BuildSimGetATR", "BuildSimOpenChannel", "BuildSimGetFacilityLock",
        "BuildSimGetSlotStatus", "BuildSetSimCardPower", "BuildSetUicc",
        "STATE_ONLINE", "OnModem",
    ]
    for k, v in sorted(syms.items(), key=lambda x: x[1]):
        if any(x in k for x in keys) or ("BuildSim" in k) or ("BuildGet" in k and "Network" in k) or ("BuildSet" in k and "Network" in k) or ("BuildSetPreferred" in k) or ("BuildGetRadio" in k):
            print(f"  {hex(v)} {k[:120]}")

# From prior RUNTIME knowledge print proven order table
print("""
=== PROVEN factory cold-boot SIT order (from RUNTIME + HAL) ===
1. GetRadioState          0x0801  GET
2. GetSimStatus           0x0200  GET  (solicited after ONLINE)
3. GetPreferredNetwork    0x070b  GET
4. SetPreferredNetwork    0x070a  SET  (stock may set default RAT)
5. SetNetworkSelectionAuto 0x0704 SET
6. GetDataRegistration    0x0701  GET  (poll)
7. VerifyPin              0x0201  SET  ONLY if app_state=PIN and pin1 enabled
   (PIN-disabled: CheckAndAutoVerifyPin / DoVerifyPin NOT taken)
Also seen stock-adjacent (not required for READY on PIN-disabled):
- facility SC 0x0209, ATR 0x0212, OpenChannel 0x020d/0x0247, SlotStatus 0x024d
NO builders: SIM_INIT_REQ, START_STACK_SERVICES_REQ
""")
