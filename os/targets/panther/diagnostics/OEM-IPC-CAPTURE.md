# OEM IPC catalog `0x2f50` — capture recipe (policy-gated)

**Status:** Soft-lock still needs an **external** catalog OEM `SIM_INIT_REQ`
(`0x2f50`) userspace frame for bearer. Host RE has **not** recovered a sendable
app header + 2-byte body. Do **not** invent zeros / guessed headers.

**Ban (current):** do **not** start `rild` / `cbd` on the diagnostic phone
until an explicit policy grant. This file is the **ready procedure** for that
grant — not authorization to run it now.

## Armed path (once a frame file exists)

1. Place the capture as a file on-device (raw bytes **or** hex text).
2. ONLINE soft stack only (existing probe/handover) — **no** stock RIL daemon.
3. One inject + chase:

```sh
# build on host
aarch64-linux-gnu-gcc -O2 -static -Wall -Wextra -Werror \
  os/targets/panther/diagnostics/oem-ipc-inject.c \
  -o /tmp/oem-ipc-inject
# host parser check
gcc -std=c11 -Wall -Wextra -Werror \
  os/targets/panther/diagnostics/oem-ipc-inject.c \
  -o /tmp/oem-ipc-inject-host && /tmp/oem-ipc-inject-host self-test

# on device (after deploy binaries + scripts under /data/saaios/bin)
sh /data/saaios/bin/post-init-chase.sh --frame /data/saaios/var/oem-2f50.frame
# or poll after a prior inject:
#   sh /data/saaios/bin/post-init-chase.sh --poll-only
```

`oem-ipc-inject` refuses empty and all-zero frames and has **no** default
`0x2f50` payload. Logs write length / errno only.

`post-init-chase` polls `sit-sim-status` until `app∈{1,4,5}`, then runs
`CHASE_ONCE=1` on `tray-bearer-chase.sh` (VerifyPin if needed → LTE →
AllowData → GetPs → SetupDataCall/APN → `rmnet`).

## Capture-only stock rild (when policy grants — not now)

Goal: obtain the **first** userspace write to `/dev/oem_ipc*` that carries
catalog msgid `0x2f50` (app payload only; kernel may prepend EXYNOS 12B on
link_header paths — capture **userspace** bytes written to the chardev).

Suggested one-shot outline (operator adapts to the stock image in use):

1. Fresh diagnostic ONLINE **without** our long-running SIT consumers holding
   exclusive locks, **or** a known-good stock boot where OEM IPC still works.
2. Start capture **before** RIL:
   - `strace -f -e write -o /data/saaios/var/rild-oem.strace` attached to the
     stock `rild` / vendor RIL process, **or**
   - a kprobe/`bpftrace` on `vfs_write` filtered to the `oem_ipc*` inode, **or**
   - equivalent FRIDA/interceptor on the OEM IPC write path.
3. Start stock `rild` **once** only long enough for SIM bring-up / first OEM
   SIM_INIT write. Do **not** leave it running as a service.
4. Stop / kill `rild` (and any started `cbd` helper) immediately after the
   first matching write.
5. Extract the write buffer for the `oem_ipc*` fd into
   `/data/saaios/var/oem-2f50.frame` (raw) or a hex dump.
6. Confirm msgid `0x2f50` / `SIM_INIT` against catalog evidence; do not pad or
   “fix in” missing bytes.
7. Re-enter the **armed path** above with that file. Never commit frame bytes
   that embed secrets (ICCID/IMSI/PIN). Prefer length + msgid confirmation in
   docs; keep the frame local/operator-private if unsure.

## External dump alternative

A third-party / prior stock capture of Shannon OEM IPC userspace bytes for
catalog `SIM_INIT_REQ` is equally valid. Same inject + chase path. Still **no
invent**.

## What this does not do

- Does not mark MODEM-06 / cellular goal complete without verified `rmnet`
  bearer.
- Does not authorize unsigned MAIN, EFS RW, ATU Present poke, or inventing the
  2-byte body.
- Does not treat SitOem protobuf Ping/Config as catalog `0x2f50`.

## Post-kernel EXYNOS layout (SitOem Ping teach — 2026-10-01)

Live kprobe on `exynos_build_header` (+0x50 buff dump) while writing evidenced
SitOem Ping protobuf to `/dev/oem_ipc0` (no rild/cbd):

```
userspace write (11B):  08 01 10 01 2a 05 0a 03 0a 01 78
post-kernel (23B):      CD AB | seqLE | C0 00 | 17 00 | 81 | 00 00 00 | <protobuf>
                        sync    frame   cfg     len=23   ch0x81  pad     app
```

- Kernel prepends **EXYNOS 12B**; app bytes are **passthrough**.
- EXYNOS fields: sync `0xABCD`, frame_seq, frag_cfg `0xC000`, len=12+count,
  channel=`0x81` for `oem_ipc0`. **No msgid** in this header.
- SitOem msgid/token are protobuf tags inside the app payload, not fixed
  binary offsets in the outer header.
- Therefore: catalog `0x2f50` may share the **same EXYNOS wrap** if sent on
  `oem_ipc*`, but still needs a fully evidenced **app-layer** REQUEST
  (header + `flags=2` 2-byte body). Ping does **not** supply those bytes —
  do not invent; do not soft-send.

Probe helpers (host): `tmp-run-exynos-hdr-probe.ps1`,
`tmp-run-ping-kprobe.ps1`. On-device Ping: `tmp-sitoem-ping-once`.

See `docs/os/targets/panther/MODEM-BLOCKER.md` (armed-for-frame) and
`tray-bearer-chase.sh` for the post-edge pipeline.

## Public FMT mapping attempt (2026-10-01) — still incomplete

Closest public layouts vs catalog `SIM_INIT_REQ` (`0x2f50`, `body_hint=2`) and
twin `SIM_VERIFYPIN_REQ` (`0x2f52`, `body_hint=10`):

| Layout | Public evidence | Enough for soft send? |
| --- | --- | --- |
| Classic 7B `ipc_fmt_header` (len,mseq,aseq,group,index,type) | Replicant / libsamsung-ipc / Wireshark-dev | **No** — SEC group is `0x05`, not `0x2f`; no public `SIM_INIT_REQ`; type/mseq/aseq/body contents for Shannon OEM catalog unproven |
| Soft SIT 12B (`type,pad,id,len,token`) | our Pixel soft path | **No** — different dialect; `0x0201` ≠ OEM `0x2f52` |
| EXYNOS 12B (sync `ABCD` … ch `0x81`) | live Ping kprobe + public `exynos_build_header` | **Outer only** — userspace still needs full app frame |
| SitOem protobuf | live Ping + schema exhaust | **No** — not catalog |

**Do not** soft-build a classic-FMT or SIT-shaped `0x2f50` from catalog
meta alone. Remaining blockers: every app-header byte + evidenced 2B body.
Helper notes: `tmp-public-fmt-2f50-map.py`.

## Deep host const-build miss (2026-10-01)

Factory carves + full `vendor.img` scanned for MOVZ+MOVK / MOVN / ORR /
rodata `50 2f` and oem_ipc∩msgid tables (`tmp-vendor-deep-2f50-constbuild*.py`).
**No** catalog encoder recovered. This does **not** authorize inventing a
frame; capture (or external dump) remains the prerequisite for
`oem-ipc-inject`.
