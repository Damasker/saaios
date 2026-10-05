#!/usr/bin/env python3
"""RE stock radio-on -> SIM READY SIT sequence from libsitril + sit-stream.

Prints Build* opcode/length where recoverable, and libsitril handler names.
No live I/O. No secrets.
"""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics"
)
# fallback if research tree mount differs
if not (DIAG / "sit-stream.so").exists():
    DIAG = Path(__file__).resolve().parent


def strings(data: bytes, min_len: int = 5):
    out = []
    cur = bytearray()
    start = 0
    for i, b in enumerate(data):
        if 32 <= b < 127:
            if not cur:
                start = i
            cur.append(b)
        else:
            if len(cur) >= min_len:
                out.append((start, bytes(cur).decode()))
            cur = bytearray()
    return out


def parse_dynsym(data: bytes) -> dict[str, int]:
    if data[:4] != b"\x7fELF":
        return {}
    ei = data[4]
    if ei != 2:
        return {}
    e_shoff = struct.unpack_from("<Q", data, 0x28)[0]
    e_shentsize = struct.unpack_from("<H", data, 0x3A)[0]
    e_shnum = struct.unpack_from("<H", data, 0x3C)[0]
    e_shstrndx = struct.unpack_from("<H", data, 0x3E)[0]

    def sh(i: int):
        o = e_shoff + i * e_shentsize
        return {
            "name": struct.unpack_from("<I", data, o)[0],
            "typ": struct.unpack_from("<I", data, o + 4)[0],
            "addr": struct.unpack_from("<Q", data, o + 16)[0],
            "off": struct.unpack_from("<Q", data, o + 24)[0],
            "size": struct.unpack_from("<Q", data, o + 32)[0],
            "link": struct.unpack_from("<I", data, o + 40)[0],
            "entsize": struct.unpack_from("<Q", data, o + 56)[0],
        }

    shstr = sh(e_shstrndx)
    names = data[shstr["off"] : shstr["off"] + shstr["size"]]
    syms: dict[str, int] = {}
    for i in range(e_shnum):
        s = sh(i)
        nm = names[s["name"] :].split(b"\0", 1)[0]
        if nm not in (b".dynsym", b".symtab"):
            continue
        link = sh(s["link"])
        strtab = data[link["off"] : link["off"] + link["size"]]
        ents = s["entsize"] or 24
        for j in range(0, s["size"], ents):
            o = s["off"] + j
            st_name = struct.unpack_from("<I", data, o)[0]
            st_value = struct.unpack_from("<Q", data, o + 8)[0]
            name = strtab[st_name:].split(b"\0", 1)[0].decode("ascii", "replace")
            if name and st_value:
                syms[name] = st_value
    return syms


def demangle_build(name: str) -> str:
    # crude: keep readable fragment
    for key in (
        "BuildSim",
        "BuildSet",
        "BuildGet",
        "BuildRadio",
        "BuildNetwork",
        "BuildAllow",
        "BuildQuery",
        "BuildSetup",
        "BuildOperator",
    ):
        if key in name:
            i = name.find(key)
            return name[i : i + 80]
    return name[:80]


def decode_builder_id_len(data: bytes, addr: int, window: int = 0xC0) -> tuple[int | None, int | None]:
    """Heuristic: scan ARM64 builder for movz/movk of id and length stores.

    Look for immediate halfwords commonly used as SIT ids (0x02xx/07xx/08xx/09xx)
    and nearby length immediates (12..80).
    """
    # file offset ~= addr for ET_DYN with vaddr base often 0; try PT_LOAD map
    # For these SOs, text often at file offset == VA - load_bias; try VA as file off first.
    candidates = []
    # Prefer PT_LOAD mapping
    if data[:4] == b"\x7fELF" and data[4] == 2:
        e_phoff = struct.unpack_from("<Q", data, 0x20)[0]
        e_phentsize = struct.unpack_from("<H", data, 0x36)[0]
        e_phnum = struct.unpack_from("<H", data, 0x38)[0]
        for i in range(e_phnum):
            o = e_phoff + i * e_phentsize
            p_type = struct.unpack_from("<I", data, o)[0]
            if p_type != 1:
                continue
            p_offset = struct.unpack_from("<Q", data, o + 8)[0]
            p_vaddr = struct.unpack_from("<Q", data, o + 16)[0]
            p_filesz = struct.unpack_from("<Q", data, o + 32)[0]
            if p_vaddr <= addr < p_vaddr + p_filesz:
                file_off = p_offset + (addr - p_vaddr)
                candidates.append(file_off)
    candidates.append(addr)  # fallback

    ids = []
    lens = []
    for file_off in candidates:
        chunk = data[file_off : file_off + window]
        if len(chunk) < 8:
            continue
        # decode 32-bit A64 instructions
        for i in range(0, len(chunk) - 3, 4):
            insn = struct.unpack_from("<I", chunk, i)[0]
            # MOVZ Wd, #imm16  (sf=0): 0101 0010 1xxi iiii iiii iiii iiid dddd
            # MOVZ Xd/Wd pattern: opc=10, hw in bits
            if (insn & 0x7F800000) == 0x52800000:  # MOVZ W
                imm16 = (insn >> 5) & 0xFFFF
                hw = (insn >> 21) & 0x3
                val = imm16 << (hw * 16)
                if 0x0200 <= val <= 0x02FF or 0x0700 <= val <= 0x09FF or 0x4600 <= val <= 0x46FF:
                    ids.append(val)
                if 8 <= val <= 256:
                    lens.append(val)
            # MOV Wd, #imm via ORR immediate is harder; also look for MOVN
        if ids or lens:
            break
    id_v = ids[0] if ids else None
    # prefer length that looks like SIT total length near id
    len_v = None
    for L in lens:
        if L in (12, 13, 16, 17, 18, 21, 29, 30, 38, 63, 71, 72, 246):
            len_v = L
            break
    if len_v is None and lens:
        len_v = lens[0]
    return id_v, len_v


def main():
    stream = (DIAG / "sit-stream.so").read_bytes()
    ril = (DIAG / "libsitril.so").read_bytes()
    ssyms = parse_dynsym(stream)
    rsyms = parse_dynsym(ril)

    print(f"DIAG={DIAG}")
    print(f"sit-stream syms={len(ssyms)} libsitril syms={len(rsyms)}")

    build_keys = (
        "BuildSim",
        "BuildRadio",
        "BuildSetSim",
        "BuildSetUicc",
        "BuildGetRadio",
        "BuildSetPreferred",
        "BuildGetPreferred",
        "BuildSetNetwork",
        "BuildNetwork",
        "BuildAllow",
        "BuildGetPs",
        "BuildSetupData",
        "BuildGetData",
        "BuildOperator",
        "BuildQueryNetwork",
        "SetEngMode",
        "BuildGetVoice",
        "BuildSetVoice",
        "BuildSetMobile",
    )

    print("\n=== sit-stream builders (opcode/len heuristic) ===")
    rows = []
    for k, v in sorted(ssyms.items(), key=lambda x: x[1]):
        if not any(x in k for x in build_keys):
            continue
        nice = demangle_build(k)
        oid, olen = decode_builder_id_len(stream, v)
        rows.append((v, nice, oid, olen, k))
        print(f"  {hex(v)} id={oid and hex(oid)} len={olen}  {nice}")

    print("\n=== libsitril SIM/radio handlers ===")
    want = (
        "GetSimStatus",
        "VerifyPin",
        "RadioPower",
        "RadioState",
        "Preferred",
        "NetworkSelection",
        "Registration",
        "CheckAndAuto",
        "OnGetSim",
        "DoAuto",
        "SetUicc",
        "CardPower",
        "SlotStatus",
        "Facility",
        "OpenChannel",
        "GetATR",
        "AllowData",
        "EngMode",
        "StartNetwork",
        "Imsi",
        "Auth",
        "SimLock",
        "SimService",
        "TrySetRadio",
        "DoGetSim",
        "DoSetPreferred",
        "DoSetNetwork",
        "DoRadio",
        "FillRilCard",
    )
    for k, v in sorted(rsyms.items(), key=lambda x: x[1]):
        if any(x in k for x in want):
            print(f"  {hex(v)} {demangle_build(k) if 'Build' in k else k[:140]}")

    # ASCII fallback inventory of BuildSim*
    print("\n=== ASCII BuildSim*/BuildSet*/BuildGet* in sit-stream ===")
    seen = set()
    for off, s in strings(stream, 8):
        if not s.startswith("Build"):
            continue
        if not any(
            x in s
            for x in (
                "Sim",
                "Radio",
                "Uicc",
                "Preferred",
                "Network",
                "Allow",
                "Ps",
                "Eng",
                "Voice",
                "Data",
                "Card",
                "Slot",
                "Facility",
                "Operator",
                "Selection",
                "Registration",
            )
        ):
            continue
        if s in seen:
            continue
        seen.add(s)
        print(f"  {hex(off)} {s}")

    print("\n=== ASCII libsitril bring-up / READY path strings ===")
    keys = (
        "GetSimStatus",
        "OnGetSimStatusDone",
        "CheckAndAutoVerifyPin",
        "DoVerifyPin",
        "DoAutoVerifyPin",
        "DoRadioPower",
        "TrySetRadioPower",
        "SetRadioPower",
        "GetRadioState",
        "SetPreferredNetworkType",
        "SetNetworkSelectionAuto",
        "GetDataRegistration",
        "GetVoiceRegistration",
        "FillRilCardStatus",
        "app_state",
        "CARD_READY",
        "APPSTATE_READY",
        "SIM_READY",
        "RIL_CARDSTATE",
        "RIL_APPSTATE",
        "DoGetSimStatus",
        "RequestSimStatus",
        "SIM_STATUS",
        "CardPower",
        "SetUicc",
        "OpenChannel",
        "GetATR",
        "GetSlotStatus",
        "AllowData",
        "START_NETWORK",
        "StartNetwork",
    )
    seen = set()
    for off, s in strings(ril, 6):
        if any(k in s for k in keys):
            if s in seen:
                continue
            seen.add(s)
            print(f"  {hex(off)} {s[:140]}")

    # Diff table: stock radio-on->READY vs our soft-lock path (from prior RUNTIME + builders)
    print(
        """
=== STOCK radio-on -> SIM READY (evidenced) ===
Order (libsitril / sit-stream, prior RUNTIME + this inventory):
  1. BuildGetRadioState              0x0801 len12
  2. BuildRadioPower ON              0x0800 len18  (DoRadioPower: word@12=2)
  3. BuildSimGetStatus               0x0200 len12  (poll / OnGetSimStatusDone)
  4. BuildGetPreferredNetworkType    0x070b len12
  5. BuildSetPreferredNetworkType    0x070a len16
  6. BuildSetNetworkSelectionAuto    0x0704 len12
  7. BuildNetworkRegistrationState   0x0700/0x0701 poll
  8. IF app=PIN & pin1 enabled:
       BuildSimVerifyPin             0x0201 len38 (+AID when property set)
     ELSE IF pin1 DISABLED:
       CheckAndAutoVerifyPin returns; NO 0x0201
  Adjacent (stock may send; not READY gates on this MAIN):
       GetFacilityLock SC 0x0209, GetATR 0x0212, OpenChannel 0x020d/0x0247,
       GetSlotStatus 0x024d, CardPower 0x024c, SetUicc 0x0249, AllowData 0x0710
  NOT in builders: SIM_INIT_REQ / START_STACK / Present poke / SET_APP

=== OUR soft-lock path (saai-modemd / tray-chase) already covers ===
  0x0801, 0x0800 ON, 0x0200, 0x070b, 0x070a(11/12), 0x0704, 0x0700/0701,
  0x0201 A+AID (live err0 pin1 1->2), 0x0209, 0x0212, 0x020d/0247, 0x024d,
  0x024c, 0x0249, 0x0710, 0x0908 EngMode — still app=PIN present_infer=notin

=== CANDIDATE missing signed cmds (exist in tables; not READY-proven) ===
  Review builder dump above for any id we never live-sent on PIN+verified path.
"""
    )


if __name__ == "__main__":
    main()
