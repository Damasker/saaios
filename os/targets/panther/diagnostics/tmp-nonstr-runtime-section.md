## 2026-10-01: non-string dispatcher / catalog / nanopb RE (no send)

Scripts: `tmp-oem-dispatcher-nonstr-re.py` / `.out`,
`tmp-oem-dispatcher-nonstr-re2.py` / `.out`.

### Live brief (COM13 / USB NCM `172.31.7.1`)

modem_state=**ONLINE**; `sit-sim-status`: PRESENT apps=1 **app=PIN** pin1=2
remain=3; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; rmnet* rx=0; wlan0 LAN
IPv4 only; usb0=`172.31.7.1` -- **no rmnet bearer**. ADB absent; SSH pubkey
denied; brief via COM13.

### Catalog walk (28B stride; non-string)

| Item | Result |
| --- | --- |
| SIM `*_REQ` bank | 43 entries; stride **0x1c** confirmed |
| `flags/body_hint==2` cohort | **3**: `SIM_INFO_REQ` (`0x2f57`, `+18=0`), `SIM_INIT_REQ` (`0x2f50`, `+18=4`), `SIM_STOP_REQ` (`0x2f51`, `+18=4`) |
| `@0x6de740` raw | `02 00 50 2f ... 04 00 00 00` -- body=2 msgid=`0x2f50` meta=`0x10104` rsp=0 **`+0x18=4`** |
| meta `0x10104` | lo16=`0x104` hi16=`1` -- still opaque enum; common across SIM REQ bank |

### Catalog `+0x18` refined (was "maybe token len")

Cross-domain OEM catalog REQs with `+0x18!=0`:

| `+0x18` | Examples |
| --- | --- |
| **4** | `SIM_INIT_REQ` / `SIM_STOP_REQ`, `CC_INIT_REQ`, `SMS_INIT_REQ`, `SS_INIT_REQ`, `SMREG_INIT_REQ`, `NS_EMM_STOP_NETWORK_REQ`, `NS_MM_STOP_NETWORK_REQ` |
| **5** | Phonebook `PB_*_REQ` bank |
| **1** | Various CC/IMS disconnect/setup REQs |

**Verdict:** `+0x18` behaves as a **class / family tag** (INIT-family=`4`),
**not** proven wire token/seq length. Do not treat `+18=4` as "4-byte token".

### Catalog code consumers (MOVW to entry VA only; litpool=0)

| Entry | Site | Behavior |
| --- | --- | --- |
| `SIM_INIT` `@0x6de740` | `@0x32e4494` | `BL 0x2ca893e` object helper; **STRB #5 to obj+8**, STRB from other to obj+9 |
| `SIM_STOP` `@0x6de9c4` | `@0x32e4508` | sibling; **STRB #1 to obj+8** |
| `SIM_INFO` `@0x6de724` | `@0x3388338` | same helper pattern; builds **other** msgid `0x2f79` into local `{u16,u16,u32}` |
| `SIM_VERIFYPIN` | `@0x331af14` | object helper; `MOVS #12` = error return (not hdr len) |

**No** proven STR of wire msgid/len/token/2B body to TX buffer. msgid-to-fn
tables in catalog island: **0**.

### Nanopb / `[OEM][PB]`

| Check | Result |
| --- | --- |
| `pb_decode` / OEM island | present; logs `[OEM][PB] pb_decode_varint_cb...` |
| `[OEM][PB]` string count | **3** -- decode/varint/buf only |
| SIM / INIT / `0x2f50` in OEM[PB] | **0** |

Nanopb OEM[PB] is not the SIM catalog dialect (consistent with SitOem demux falsified).

### DBT / preprocess (reconfirmed non-string)

MOVW/MOVT/litpool to DBT **record** VAs for msgid_nf / not-REQUEST /
preprocess: **0**. Logs remain DBT-indexed; preprocess emit path still opaque.

### Frame / live try

**App header recovered?** **no**. **2B body contents?** **no** (size hint only).
**SIM_INIT sent?** **no** (no invent). **Bearer verified?** **no**.

### Exact missing (unchanged + refined)

1. OEM **app-layer header** field order/size (dispatcher preprocess still
   opaque without stock capture or proven encode emit)
2. **2-byte body** contents for `body_hint=2` (zeros unproven)
3. Token/seq rules -- catalog `+0x18` is **class tag**, not token proof
4. Which `oem_ipcN` carries **catalog** SIM_INIT (SitOem/protobuf owns a
   different dialect on `oem_ipc0`)

### Next

Stock catalog OEM capture on `oem_ipc*`, or host encoder binary that is not
cbd / SitOem protobuf / internal object-helper. Then **ONE** soft `SIM_INIT`.
Same bans.
