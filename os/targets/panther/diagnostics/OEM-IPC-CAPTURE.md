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

See `docs/os/targets/panther/MODEM-BLOCKER.md` (armed-for-frame) and
`tray-bearer-chase.sh` for the post-edge pipeline.
