#!/usr/bin/env python3
"""Exhaust SitOem protobuf surface from carved factory oemipc .so files.

Lists every sit_ipc_message::* type and ModemData wrapper / PayloadCase
hint visible in dynstr/mangled names. Does not invent wire frames.
"""
from __future__ import annotations

import re
import struct
import sys
from collections import defaultdict
from pathlib import Path

OUT = Path(__file__).with_suffix(".out")
SOS = [
    Path(__file__).parent
    / "fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-25e7b000.so",
    Path(__file__).parent
    / "fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-cf64000.so",
]

# Avoid substring false friends: Ping⊃pin, StatsAtom⊃sat, sitInitModem⊃init.
SIMISH = re.compile(
    r"(?i)(SIM_|USIM_|UICC_|SimInit|SIMINIT|CardPower|VerifyPin|"
    r"TransferAtr|TransferApdu|CardReader|sim_init|simInit|uicc)"
)


def elf_dynstr(data: bytes) -> bytes:
    if data[:4] != b"\x7fELF":
        return data
    e_shoff = struct.unpack_from("<Q", data, 40)[0]
    e_shentsize = struct.unpack_from("<H", data, 58)[0]
    e_shnum = struct.unpack_from("<H", data, 60)[0]
    e_shstrndx = struct.unpack_from("<H", data, 62)[0]
    shstr = e_shoff + e_shstrndx * e_shentsize
    shstrtab = struct.unpack_from("<Q", data, shstr + 24)[0]
    names = data[shstrtab:]
    for i in range(e_shnum):
        sh = e_shoff + i * e_shentsize
        name_off = struct.unpack_from("<I", data, sh)[0]
        name = names[name_off : names.find(b"\0", name_off)].decode("ascii", "ignore")
        if name == ".dynstr":
            off = struct.unpack_from("<Q", data, sh + 24)[0]
            size = struct.unpack_from("<Q", data, sh + 32)[0]
            return data[off : off + size]
    return data


def cstrings(blob: bytes, min_len: int = 4):
    cur = bytearray()
    for b in blob:
        if 32 <= b < 127:
            cur.append(b)
        else:
            if len(cur) >= min_len:
                yield bytes(cur).decode("ascii")
            cur.clear()
    if len(cur) >= min_len:
        yield bytes(cur).decode("ascii")


def demangle_sit_types(names: list[str]) -> dict[str, set[str]]:
    """Extract sit_ipc_message::<Type> from Itanium mangled names."""
    types: dict[str, set[str]] = defaultdict(set)
    # N15sit_ipc_messageNN<Type>E
    pat = re.compile(r"N15sit_ipc_message(\d+)([A-Za-z0-9_]+)")
    for n in names:
        for m in pat.finditer(n):
            ln = int(m.group(1))
            typ = m.group(2)[:ln]
            if typ:
                types["sit_ipc_message"].add(typ)
        # PayloadCase as nested enum name in mangling
        if "PayloadCase" in n:
            types["_meta"].add("PayloadCase_seen")
        if "IpcMessageType" in n:
            types["_meta"].add("IpcMessageType_seen")
    return types


def modemdata_wrappers(names: list[str]) -> set[str]:
    out = set()
    pat = re.compile(r"\d+([A-Za-z0-9_]+MessageModemData)")
    for n in names:
        for m in pat.finditer(n):
            out.add(m.group(1))
        # also catch unmangled-ish
        m2 = re.search(r"([A-Za-z0-9_]+MessageModemData)", n)
        if m2:
            out.add(m2.group(1))
    return out


def sitoem_send_apis(names: list[str]) -> list[str]:
    apis = []
    for n in names:
        if "SitOemHandler" in n and ("sitSend" in n or "sitInit" in n or "sitRegister" in n):
            # strip leading junk
            i = n.find("_ZN13SitOemHandler")
            if i >= 0:
                apis.append(n[i:])
            else:
                apis.append(n)
    return sorted(set(apis))


def payload_case_immediates(data: bytes) -> list[tuple[int, int, str]]:
    """Find MOVZ wN,#imm near initialMessageHeader / Ping ctor patterns.

    AArch64 MOVZ Wd, #imm16: 0x5280iiii with Rd in low 5 bits of last byte-ish.
    Encoding: 5280XXXX where imm is bits.
    MOVZ Wd,#imm: bits [20:5]=imm16, [4:0]=Rd, opc=10, sf=0 => 0x52800000 | (imm<<5) | Rd
    """
    hits = []
    # scan for call sites that pass small immediates as PayloadCase (w0/w1)
    # Known Ping: MOVZ w1,#0x5 before initialMessageHeader from Ping ctor
    for off in range(0, len(data) - 4, 4):
        w = struct.unpack_from("<I", data, off)[0]
        if (w & 0xFF800000) != 0x52800000:
            continue
        imm = (w >> 5) & 0xFFFF
        rd = w & 0x1F
        if imm == 0 or imm > 64:
            continue
        # look ahead 0..24 insn for BL to ~initialMessageHeader band or STR of case
        ctx = data[off : off + 64]
        # classify by nearby ascii
        window = data[max(0, off - 32) : off + 96]
        ascii_bits = "".join(chr(b) if 32 <= b < 127 else "." for b in window)
        hits.append((off, imm, f"rd=w{rd}"))
    # dedupe by imm, keep first few offs
    by_imm: dict[int, list[int]] = defaultdict(list)
    for off, imm, _ in hits:
        if len(by_imm[imm]) < 8:
            by_imm[imm].append(off)
    return [(imm, offs[0], f"n={len(offs)} first={hex(offs[0])}") for imm, offs in sorted(by_imm.items())]


def analyze(path: Path, log) -> None:
    data = path.read_bytes()
    log(f"\n{'='*60}\nFILE {path.name} size={len(data)}")
    dyn = elf_dynstr(data)
    names = list(cstrings(dyn, 8))
    log(f"dynstr strings >=8: {len(names)}")

    types = demangle_sit_types(names)
    msgs = sorted(types.get("sit_ipc_message", set()))
    log(f"\n=== sit_ipc_message::* types ({len(msgs)}) ===")
    for t in msgs:
        flag = " ***SIMISH***" if SIMISH.search(t) else ""
        log(f"  {t}{flag}")

    wrappers = sorted(modemdata_wrappers(names))
    log(f"\n=== *MessageModemData wrappers ({len(wrappers)}) ===")
    for w in wrappers:
        flag = " ***SIMISH***" if SIMISH.search(w) else ""
        log(f"  {w}{flag}")

    apis = sitoem_send_apis(names)
    log(f"\n=== SitOemHandler send/init/register APIs ({len(apis)}) ===")
    for a in apis:
        short = a
        if len(short) > 120:
            short = short[:117] + "..."
        flag = " ***SIMISH***" if SIMISH.search(a) else ""
        log(f"  {short}{flag}")

    # PayloadCase: search mangled + FileDescriptorProto-ish field names
    log("\n=== PayloadCase / oneof clues ===")
    pc = [n for n in names if "PayloadCase" in n]
    log(f"mangled with PayloadCase: {len(pc)}")
    for n in pc[:10]:
        log(f"  {n[:140]}")

    # protobuf reflection often embeds field names as bare strings in .rodata
    ro_candidates = []
    for s in cstrings(data, 4):
        if s in (
            "ping",
            "config",
            "thermal",
            "metrics",
            "device_state",
            "deviceState",
            "traffic",
            "txas",
            "scone",
            "coex",
            "debug",
            "data_flow",
            "dataFlow",
            "data_validation",
            "dataValidation",
            "mch",
            "sim",
            "sim_init",
            "simInit",
            "usim",
            "uicc",
            "card",
            "payload",
            "type",
            "token",
            "kPing",
            "kConfig",
            "PAYLOAD_NOT_SET",
            "kPayloadNotSet",
        ):
            ro_candidates.append(s)
    log(f"rodata field-name hits: {sorted(set(ro_candidates))}")

    # Arena CreateMaybeMessage<sit_ipc_message::X> — strongest type list
    arena = sorted(
        set(
            re.findall(
                r"CreateMaybeMessageIN15sit_ipc_message(\d+)([A-Za-z0-9_]+)",
                dyn.decode("latin1", "ignore"),
            )
        )
    )
    log(f"\n=== Arena CreateMaybeMessage types ({len(arena)}) ===")
    arena_types = []
    for ln_s, rest in arena:
        ln = int(ln_s)
        typ = rest[:ln]
        arena_types.append(typ)
        flag = " ***SIMISH***" if SIMISH.search(typ) else ""
        log(f"  {typ}{flag}")

    def is_simish(s: str) -> bool:
        # Exclude known non-SIM: Ping*, sitInitModem (IPC bring-up), *Msim* version
        if re.search(r"(?i)Ping|sitInitModem|MsimReq|StatsAtom|CallDrop", s):
            return False
        return bool(SIMISH.search(s))

    simish_msgs = [t for t in msgs if is_simish(t)]
    simish_wrap = [w for w in wrappers if is_simish(w)]
    simish_api = [a for a in apis if is_simish(a)]
    simish_arena = [t for t in arena_types if is_simish(t)]

    log("\n=== SIM/init/card/uicc-related (filtered; Ping/sitInitModem excluded) ===")
    log(f"sit_ipc_message SIMISH: {simish_msgs or 'NONE'}")
    log(f"MessageModemData SIMISH: {simish_wrap or 'NONE'}")
    log(f"SitOemHandler API SIMISH: {simish_api or 'NONE'}")
    log(f"Arena CreateMaybeMessage SIMISH: {simish_arena or 'NONE'}")

    if any("sitInitModem" in a for a in apis):
        log(
            "NOTE: sitInitModem = SitOem startModemIPC path (protobuf family), "
            "NOT catalog SIM_INIT_REQ 0x2f50"
        )

    movz = payload_case_immediates(data)
    log(f"\n=== MOVZ #1..64 immediates (possible PayloadCase/Type) count={len(movz)} ===")
    for imm, _off, info in movz[:40]:
        log(f"  #{imm} {info}")

    # Evidenced encode paths: fillInput / RequestInput symbols
    encodes = [
        n
        for n in names
        if ("fillInput" in n or "RequestInput" in n or "sitSend" in n)
        and ("ModemData" in n or "SitOemHandler" in n)
    ]
    log(f"\n=== Evidenced encode symbols ({len(encodes)}) ===")
    for n in sorted(set(encodes)):
        flag = " ***SIMISH***" if SIMISH.search(n) else ""
        log(f"  {n[:130]}{flag}")

    return {
        "msgs": msgs,
        "wrappers": wrappers,
        "simish_msgs": simish_msgs,
        "simish_wrap": simish_wrap,
        "simish_api": simish_api,
        "simish_arena": simish_arena,
        "arena": arena_types,
    }


def main() -> int:
    lines: list[str] = []

    def log(s: str = "") -> None:
        print(s)
        lines.append(s)

    results = []
    for so in SOS:
        if not so.exists():
            log(f"MISSING {so}")
            continue
        results.append(analyze(so, log))

    log("\n" + "=" * 60)
    log("VERDICT")
    any_sim = False
    for r in results:
        if r["simish_msgs"] or r["simish_wrap"] or r["simish_arena"]:
            any_sim = True
        # sitInitModem alone is NOT a SIM protobuf message
        api_real = [a for a in r["simish_api"] if "sitInitModem" not in a]
        if api_real:
            any_sim = True

    if not any_sim:
        log(
            "NO SitOem protobuf message/PayloadCase is SIM/init/card/uicc-related "
            "with an evidenced encode path. Catalog SIM_INIT_REQ 0x2f50 remains "
            "a separate OEM IPC table family. LIVE TRY: SKIP (no invent)."
        )
    else:
        log(
            "SIM-related SitOem surface found — review SIMISH lists above before "
            "any live try; only attempt if encode path is evidenced."
        )

    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    log(f"\nWrote {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
