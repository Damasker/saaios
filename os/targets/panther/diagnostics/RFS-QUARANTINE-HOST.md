# Host-only RFS file-3 transaction model

`rfs-quarantine-host.mjs` is an isolated, opt-in model of the observed
protected-NV exchange. It imports only Node's cryptographic hash API. It has
no filesystem, modem-device, network, original-EFS, or promotion interface.
Do not install it on a phone or connect its returned frames to a device.

The pinned input is exactly 524,288 bytes and must match an independently
supplied SHA-256. The constructor copies it into private memory. On the
exact cmd7 request, the model prepares and verifies a fresh 524,288-byte
candidate clone **before** returning status-0 for cmd7. The candidate is not
externally inspectable until completion. The model overwrites its first
189,446 bytes, never the baseline; its tail remains unchanged. A malformed
input aborts the session and wipes the candidate. A completed candidate can
only be inspected through a detached in-memory copy. There is no commit,
rename, or candidate-promotion interface.

The strict modeled wire contract is:

| Stage | Incoming validation | Modeled output |
|---|---|---|
| cmd7 | Exact 12-byte file-3 unprotect request | Proven 16-byte status-0 reply |
| cmd3 | Exact 20-byte file-3 status request | None |
| cmd6 | Exact 24-byte file-3 op2 request: offset 0, length 189,446 | First 20-byte cmd2 grant |
| cmd2 data | Status 0, file 3, seq 1, exact granted length and bounded frame | Next grant, or exact 16-byte cmd3 success after verified completion |

The factory grant format is `u16 cmd=2, u16 seq, u32 payload_len=12,
u32 file_id=3, u32 offset, u32 chunk_len`. The data-frame payload is
`u32 status, u32 file_id, u32 chunk_len, bytes[chunk_len]`. The source
sequence is copied from cmd6 (observed value 1) and must be echoed by cmd2
data. In the factory binary, the cmd3 path bypasses the general sequence
comparison; cmd2 data does not. Its normal dispatch compares incoming seq
with the stored cmd6 value before the protected-file handler.
Each grant is at most 2,012 bytes: 94 full grants and one 318-byte grant,
95 in total. Requiring each data chunk to equal its advertised grant is a
stricter SaaiOS fail-closed policy: stock code checks that the chunk does not
exceed the remaining total and may accept shorter chunks. The model rejects
short, extra, reordered, or failed frames. After the 95th exact chunk and
in-memory baseline/candidate integrity checks, it models the factory final
status as `03 00 01 00 08 00 00 00 00 00 00 00 03 00 00 00` (cmd3,
seq1, payload length 8, status 0, file 3). No success status is returned if
validation fails.

The stock `rfsd` route still depends on local file-descriptor, size, seek,
state, temporary-file, fsync, and OnWriteDone behavior. This in-memory model
does not provide durability and deliberately does not claim to reproduce
those decisions. In particular, its modeled final ACK and a successful host
fixture are **not** evidence that any reply or write is safe on the phone.
The relevant source of current reverse-engineering evidence is
`docs/os/targets/panther/MODEM-RUNTIME-2026-09-24.md`; the stock `rfsd`
SHA-256 recorded there is
`58d7f885e7533a328268f0de47ef9eb9995cdfa6b317d755b57973d4f5dfb71b`.

Run only the synthetic host fixtures:

```sh
node --test os/targets/panther/diagnostics/rfs-quarantine-host.test.mjs
```

The fixture baseline is synthetic (`0x5a` repeated) and independently pinned;
no real NV contents or subscriber identifiers are included.
