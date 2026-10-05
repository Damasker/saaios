from pathlib import Path

p = Path("/mnt/c/Users/Admin/Projects/saaios-som/docs/os/targets/panther/MODEM-BLOCKER.md")
text = p.read_text(encoding="utf-8")
section = """
### 2026-10-01 — VerifyPin A+AID (pin1=1) opens READY (live)

**Breakthrough (reproduced x2):** with overnight pin1=**1**, VerifyPin A+AID
**without CardPower** → app **READY(5)**, pin1 **2**, remain unchanged.
Late unsolicited `0x0201` err0 + `0x0200` app=5; also saw `0x0210`,
`0x4604`, `0x4602`, `0x0303`, `0x000d` (metadata only).

**Post-edge:** `0x0704` auto selection **err=0** under READY (falsifies
prior \"0x0704 always NACK while PIN\" as the permanent camp gate). AllowData /
LTE preferred err0. **Still** `registration_raw=0`, rmnet rx=0, no IPv4.

**SetupDataCall:** deferred — on-device APN `internet` fails dotted
`apn_usable` check; need operator APN (do not invent).

**Chicken-egg update:** GET_APP PIN soft-lock is **not** immutable — pin1=1
VerifyPin A+AID reaches READY. Prior \"Present=2 / FN_A only path to READY\"
framing is **too strong** for this live path. Goal still incomplete:
**no verified bearer**.

**Bearer verified?** **no**. Still blocked on **camp/reg** (and usable APN
after camp), not on PIN→READY.

"""
marker = "## Constraints (unchanged)"
if marker not in text:
    raise SystemExit("marker missing")
if "VerifyPin A+AID (pin1=1) opens READY" in text:
    print("section already present")
else:
    text = text.replace(marker, section + marker, 1)
    p.write_text(text, encoding="utf-8")
    print("appended ok")

rt = Path("/mnt/c/Users/Admin/Projects/saaios-som/docs/os/targets/panther/MODEM-RUNTIME-2026-09-24.md")
t = rt.read_text(encoding="utf-8", errors="replace")
print("runtime READY section:", "VerifyPin A+AID (pin1=1, no CardPower)" in t)
