#!/usr/bin/env python3
"""Public FMT / Shannon IPC header mapping vs catalog SIM_INIT_REQ (0x2f50).

Evidence-only: refuses to invent a sendable frame. Prints closest public
matches and remaining gaps for OEM catalog INIT (body_hint=2) / twin VerifyPin
(0x2f52, body_hint=10).
"""

from __future__ import annotations

OUT = []


def log(s: str = "") -> None:
    OUT.append(s)
    print(s)


def main() -> None:
    log("=== Public FMT / app-header layouts (evidence) ===")
    log()
    log("A) Classic samsung-ipc / Replicant / osmocom Wireshark FMT")
    log("   Source: libsamsung-ipc include/radio.h (morphis); Wireshark-dev")
    log("           202202/msg00028 (GNUtoo packet-samsung-ipc); Replicant")
    log("           SamsungIpcDissector wiki.")
    log("   struct ipc_header / ipc_fmt_header (packed 7B):")
    log("     u16 length;  /* total incl. header */")
    log("     u8  mseq;")
    log("     u8  aseq;")
    log("     u8  group;   /* command high */")
    log("     u8  index;   /* command low  */")
    log("     u8  type;    /* REQ: EXEC=1 GET=2 SET=3 CFRM=4 EVENT=5")
    log("                    RSP: INDI=1 RESP=2 NOTI=3 */")
    log("   command = (group<<8)|index")
    log("   SEC bank (include/sec.h): IPC_SEC_SIM_STATUS=0x0501, ...")
    log("     NO SIM_INIT_REQ; NO 0x2f50 / 0x2f52 in public SEC table.")
    log("   Closest hypothetical map of catalog msgid onto this header:")
    log("     group=0x2f index=0x50  => command word 0x2f50")
    log("     length = 7 + body_hint(=2) = 9")
    log("     type = ? (EXEC/GET/SET unproven for Shannon OEM catalog)")
    log("     mseq/aseq = ?")
    log("     body[2] = ? (zeros unproven)")
    log("   Verdict: STRUCT known publicly; BINDING to Shannon OEM catalog")
    log("            msgid 0x2f50 NOT evidenced (SEC group is 0x05, not 0x2f).")
    log()
    log("B) Soft SIT FMT on Pixel umts_ipc0 (our factory / live)")
    log("   App 12B: type:u8 pad:u8 id:u16le len:u16le token:u32le [body]")
    log("   Evidenced: GetSimStatus id=0x0200 len=12; VerifyPin id=0x0201")
    log("   Twin check: soft 0x0201 body != OEM catalog 0x2f52 body_hint=10")
    log("   No SIT dual 0x0200<->0x2f50 / 0x0201<->0x2f52 in MAIN.")
    log("   Verdict: NOT the OEM catalog wire dialect.")
    log()
    log("C) Kernel EXYNOS 12B wrap (CPIF / sipc5 link_header)")
    log("   Public kernel: exynos_build_header (modem_v1 / modem_if families)")
    log("   Live kprobe (SitOem Ping on oem_ipc0):")
    log("     sync=0xABCD | frame_seq | frag_cfg=0xC000 | len=12+count")
    log("     | ch=0x81 | pad | <userspace app bytes passthrough>")
    log("   Userspace write = app only; NO msgid in EXYNOS header.")
    log("   Verdict: outer transport ONLY; does not supply catalog app hdr.")
    log()
    log("D) SitOem protobuf on oem_ipc0")
    log("   Live Ping REQUEST/RESPONSE proven; schema exhaust: no SIM/INIT.")
    log("   Verdict: different dialect from catalog 0x2f50.")
    log()
    log("E) Comsecuris / Hardwear.io / ShannonBaseband")
    log("   Document SHM FMT/RAW rings + internal qitem_header (src/dst/msg).")
    log("   Do NOT publish Shannon OEM catalog app header or 0x2f50 body.")
    log()
    log("F) AOSP / Google sepolicy")
    log("   /dev/oem_ipc[0-7] labeled radio_device — node existence only;")
    log("   no app-header layout comments for catalog SIM_INIT.")
    log()
    log("=== MAIN catalog stride-28 cross-check ===")
    log("  @0x6de740 SIM_INIT_REQ:     body_hint=2  msgid=0x2f50 meta=0x10104")
    log("                              rsp=0  +0x18=4 (class tag, not token len)")
    log("  @0x6de874 SIM_VERIFYPIN_REQ: body_hint=10 msgid=0x2f52 meta=0x10104")
    log("                              rsp=0x2fa1 +0x18=0")
    log("  meta 0x10104 common across SIM REQ bank — opaque; NOT proven as")
    log("  classic sipc type / SIT type / EXYNOS frag_cfg.")
    log()
    log("=== Map attempt: 0x2f50 onto public headers ===")
    rows = [
        ("classic 7B sipc_fmt", "group/index=0x2f/0x50", "mseq,aseq,type,body[2]"),
        ("soft SIT 12B", "would put id=0x2f50", "SIT dialect; twin sizes contradict"),
        ("EXYNOS 12B", "kernel only", "no msgid; app still missing"),
        ("SitOem protobuf", "n/a", "no SIM_INIT in schema"),
        ("catalog meta alone", "body_hint=2", "body CONTENTS unknown"),
    ]
    for name, match, gap in rows:
        log(f"  - {name}: closest={match}; gap={gap}")
    log()
    log("=== SENDABLE? ===")
    log("NO — not every app-header byte evidenced; 2B body CONTENTS unproven.")
    log("Do NOT invent zeros / classic type=EXEC / SIT-shaped OEM frame.")
    log()
    log("=== Frame sent this turn? ===")
    log("NO")
    log("=== Bearer verified? ===")
    log("NO")
    log("DONE")


if __name__ == "__main__":
    main()
