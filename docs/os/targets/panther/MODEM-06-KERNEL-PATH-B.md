# Pantah s5300 / cpif rebuild notes (SaaiOS MODEM-06 path B)

## Phone ABI

```text
Linux 6.1.157-android14-11-gbd23337e42e7-ab14791245
Android (10087095, based on r487747c) clang 17.0.2
CONFIG_CFI_CLANG=y (non-permissive), CONFIG_SHADOW_CALL_STACK=y + DYNAMIC_SCS
CONFIG_KASAN_HW_TAGS=y (not GENERIC), MODULE_SIG=y (not FORCE)
```

## Clang obtained (2026-09-30)

Exact match to `/proc/version` / `CONFIG_CC_VERSION_TEXT`:

```text
URL: https://android.googlesource.com/platform/prebuilts/clang/host/linux-x86/+archive/refs/heads/android14-release/clang-r487747c.tar.gz
Host: /home/mike/local/clang/r487747c/
Verify: ./bin/clang --version → Android (10087095, … based on r487747c) clang 17.0.2
```

## Build result

`saaios_cp_poke.ko` rebuilt with clang-r487747c (`LLVM=1`, `-fsanitize=kcfi`,
phone config + **forced** `CONFIG_DEBUG_INFO_BTF_MODULES=y` matching
`phone-config.gz` / live GKI).

### Header source for `sizeof(struct module)==0x440`

| source | detail |
| --- | --- |
| phone `/proc/config.gz` → `diagnostics/phone-config.gz` | `CONFIG_DEBUG_INFO_BTF_MODULES=y` (+ `MODULE_SCMVERSION`) |
| `kernel/common@bd23337e42e7` (`…-ab14791245`) | WSL `/home/mike/kernel-work/common-bd23337` |
| ADR-024 | BTF_MODULES adds `btf_data`/`btf_data_size`; cacheline pad → **0x440** |

Measured after rebuild: `.gnu.linkonce.this_module` **0x440** (matches stock
`rfkill.ko`); `module_layout` CRC **0xea759d7f** match.

### Live dry_run / poke (2026-09-30)

| step | result |
| --- | --- |
| Header BTF_MODULES | `this_module` **0x440** |
| RC identity (live) | **modem = ch0 / 11920000.pcie** (`PCIe Channel Number:0`, s51xx); **wifi = ch1 / 14520000** |
| ATU AP base | `pcie_exynos_gs`: `res.start+0x200000` → **0x40200000** (BTL `0x14200000` = old SoC) |
| Wake | L1SS off + IPC traffic → `link_status=1`, `PM_STATE=0` |
| RO @0x414f47f4 | ATU ret=0, **read=0xff**, ONLINE |
| writeb 5 @SET#2 | claimed OK, **readback still 0xff** (not MAIN code; write ignored) |
| Soft-lock | still PIN+DISABLED; no rmnet/IPv4 |

Verdict: modem ATU path works for MMIO, but SET#2 VA is **not** a sticky PCIe DRAM
cell (open/ignored). Need CP PA alias of MAIN or non-ATU surface. Goal incomplete.

## 2026-09-30: MAIN SET#2 PA hyp (cite) — live ATU blocked

### Compute (file → VA → hyp ATU PA)

| item | value | cite |
| --- | --- | --- |
| TOC MAIN `m_off` (CP VA base) | `0x40010000` | TOC / MODEM-RUNTIME; `modem.md` MAIN row |
| MAIN `b_off` | `0x16c10` | same |
| SET#2 file | `0x14fb404` | Thumb `MOVS r0,#2` (`02 20`) stock |
| SET#2 VA | `0x40010000+(0x14fb404-0x16c10)=0x414f47f4` | prior disasm |
| BTL ATU formula | CP addr `0x47200000` + `0x40000000` → `cp_p_base=0x87200000` | `s5300-src/cp_btl.c` LINKDEV_PCIE ≈365–369 |
| `CP_CPU_BASE_ADDRESS` | `0x40000000` | `modem_utils.h` |
| ATU call | `exynos_pcie_rc_set_outbound_atu(ch, cp_p_base, off, size)` | `modem_ctrl_s5100.c` ≈1907–1914 |
| **Hyp SET#2 ATU PA** | **`0x414f47f4+0x40000000=0x814f47f4`** | extend BTL “+0x40000000” to MAIN VA |

**Missing symbol:** no `main_cp_p_base` / MAIN DRAM export — only BTL log
window hardcode. Hyp is formula extension, not a named CPIF symbol.

### Live (same ONLINE boot)

| probe | result |
| --- | --- |
| RO `cp_phys=0x814f47f4` | ATU **ret=-32** ×8; `link_status=0`; no MMIO |
| RO `cp_phys=0x87200000` (BTL base) | ATU **ret=-32** ×8 — not MAIN-specific |
| Prior RO VA `0x414f47f4` | ATU ret=0, **read=0xff** (not `02 20`) |
| PCIe EP `0000:01:00.0` | config **all 0xff**, `current_link_width=0`, `PM_STATE=2` |
| SIT `query-sim-status` | `observed_frames=0` (IPC dead with link) |
| Write `05 20` | **not attempted** (no RO `02 20`) |

No random PA spray. Soft-lock / no bearer.

### Next RO (single)

1. Restore modem PCIe (SIT frames>0, `PM_STATE=0`, link width≠0) —
   proven path: stock reonline after clean link (sysrq→`stock-online-bringup`
   if needed). No POWER_OFF ioctl.
2. **One** RO ATU-map `@0x814f47f4` only; expect Thumb `02 20`.
3. Iff RO=`02 20`: **one** writeb `05 20`; readback; refresh SIM; chase
   READY→reg→rmnet. Else document PA still unknown (BTL formula ≠ MAIN).

## 2026-09-30: PCIe restored; hyp PA RO=0xff (not MAIN)

### Restore

sysrq → CPIF → `run-ho.sh` / probe-handover stock (`449eeab3…`) →
**ONLINE**, soft-lock PIN+DISABLED. SIT frames>0 after burst.

### Live RO (link warm: width=2, ATU ret=0)

| `cp_phys` | ATU | RO byte | notes |
| --- | --- | --- | --- |
| `0x87200000` BTL | 0 | `0xff` | log window; empty unless dump |
| `0x814f47f4` hyp | 0 | **`0xff`** | ≠ stock `02 20` — **reject hyp for SET#2** |
| `0x414f47f4` VA | 0 / -32 | `0xff` / fail | race on link |
| `0x014f47f4` VA−`CP_CPU_BASE` | 0 | `0xff` | refine; not MAIN |

**Write not done.** Missing: PCIe-exported MAIN code PA (BTL
`+0x40000000` only cites log `0x47200000`→`0x87200000`, not MAIN
`0x414f…`). Next RO: other proven CP-DRAM map (DT/OEM), not more
formula guesses. Goal incomplete (no rmnet/IPv4).

## 2026-09-30: UDL MAIN placement — no AP PA after ONLINE

### How SIT UDL delivers MAIN (s5300-src)

| step | where bytes go | cite |
| --- | --- | --- |
| userspace write | `/dev/umts_boot0` → `bootdump_write` | `bootdump_io_device.c` ≈454–607 |
| link TX | `xmit_to_cp` bootdump ch → `xmit_to_legacy_link` | `link_device.c` ≈1425–1426 |
| AP staging | `circ_write` into **SHMEM_IPC NORM_RAW TX** ring | `link_device_memory_legacy.c` ≈271–332 |
| live ring | RAW buff `@+0x3000` size `0x1FD000` (sysfs `legacy/region`) | live ONLINE |
| CP side | BOOTLDR consumes TX ring over PCIe; writes MAIN into **CP-private DRAM** at TOC `m_off` | no AP map of that DRAM |

`LOAD_CP_IMAGE` / `LINK_ATTR_XMIT_BTDLR_PCIE` only stages **BOOT** in IPC
carveout; live `boot_img addr:0xEA410000 size:0x16800`. MAIN
(`0x5917acc`) **cannot** fit remaining IPC — never AP-DMA’d to a known
MAIN PA.

### ATU windows after ONLINE

| window | symbol / cite | covers MAIN text? |
| --- | --- | --- |
| outbound BTL | `exynos_pcie_rc_set_outbound_atu` only (`s51xx_pcie.h`); `cp_p_base=0x87200000` ← CP `0x47200000`+`0x40000000` (`cp_btl.c`) | **No** — SET#2 VA `0x414f47f4` ∉ `[0x47200000,+30M)` |
| inbound ATU (AP→CP DRAM) | **absent** in s5300 + live kallsyms (`set_outbound_atu` only) | — |
| hyp `0x814f47f4` | ATU ret=0, RO `0xff` | open/ignored, not `02 20` |

### Missing map (precise)

**CP DRAM PA of MAIN `@TOC 0x40010000` / SET#2 `@VA 0x414f47f4` after
ONLINE** — not exported by CPIF; UDL leaves no AP alias. No proven PA
range to RO-scan for `02 20`. **No write.**

### Next RO

1. OEM/DT / `pcie_exynos_gs` source for any **second** outbound target that
   covers `0x40xxxxxx` CP code (not BTL log), **or**
2. non-ATU path (tray / PresentObj) — already soft-locked.
Do not spray RO/writes outside a cited window. Goal incomplete.

## 2026-09-30: ATU inventory — no MAIN cover; no BTL PresentObj

### Outbound windows (`pcie-exynos-rc.c` / live `pcie-exynos-gs.ko`)

| viewport | role | AP base | CP/EP target | covers MAIN `0x40010000`? |
| --- | --- | --- | --- | --- |
| **OB0** | CFG0 | `cfg0_base` | busdev CFG | no |
| **OB1** | EP MEM (BAR) | `res.start` … `+2MiB-1` (=`0x40000000`…) | `res.start - offset` (doorbell/BAR) | no |
| **OB2** | **BTL only** (`/* Only for BTL */`) | `res.start+SZ_2M` (=**`0x40200000`**) | `target_addr+offset` via `exynos_pcie_rc_set_outbound_atu` | **no fixed** — retargetable; BTL uses `0x87200000`←CP `0x47200000` |

Live ko strings dump **only** `PCIE_ATU_*_OUTBOUND2` for the programmable path; **no INBOUND** in module. `dw_pcie_prog_inbound_atu` exists in DWC core but unused by modem BTL/MAIN.

Test hook `pcie_rc_test` case 16: `set_outbound_atu(1, 0x47200000, …)` — wifi ch1 / raw CP addr, still **log window**, not MAIN text.

### Remap / PresentObj in BTL?

| ask | answer |
| --- | --- |
| CP remap MAIN into BTL range? | **No** API — BTL base hardcoded (`cp_btl.c`); SHMEM path SMC only enables logging, not MAIN alias |
| PresentObj / `+0xBF6` in BTL `[0x47200000,+30M)`? | **Unproven** — PresentObj is CP **heap** (`#0x636c`); no PA. Prior peeks: GAP |
| SET#2 in OB2 if retargeted? | ATU ok @`0x814f47f4`, RO **`0xff`** ≠ `02 20` |

**No write.** Missing unchanged: CP DRAM PA of MAIN / PresentObj after ONLINE.
Next: tray or OEM-signed MAIN / new CPIF export. Goal incomplete.

## 2026-09-30: OB1 RO ≠ MAIN; ATU poke path dead

### Live OB1 (AP `0x40000000` window, width=2 after SIT)

| AP addr | RO | stock MAIN @file `0x16c10` |
| --- | --- | --- |
| `0x40000000`..`+3` | **`0xff`** | would be `88 f0 9f e5` vectors |
| `0x40010000`..`+7` | **`0xff`** | same header if TOC mapped | 

OB1 target = EP BAR (`res.start - offset`), BAR0 live ~4KiB doorbell —
**not** CP DRAM @ TOC `0x40010000`.

### Grow / extra outbound?

| ask | cite / live |
| --- | --- |
| Grow OB1 LIMIT past 2MiB? | Source hardcodes `start+SZ_2M-1`; RC window is 6MiB (`0x40000000–0x405fffff`) but still **EP BAR target**, not MAIN |
| Cover SET#2 `0x414f47f4` (~+21MiB)? | Needs ≫6MiB; not possible with OB1 |
| Extra viewport? | Only OB2 programmable (`Only for BTL`); no OB3 |

### Verdict

**MODEM-06 poke-via-ATU = dead** for SET#2/`02 20`. Soft-lock PIN+DISABLED
unchanged (SIM poll). **Next: physical tray** or signed MAIN / CPIF export.
No write. Goal incomplete (no rmnet/IPv4).

## 2026-09-30: OB2 systematic RO sig-scan — still no MAIN

### Signatures (stock MAIN)

| site | file | bytes |
| --- | --- | --- |
| TOC | `0x0` | `54 4f 43 00` |
| MAIN vectors | `0x16c10` | `88 f0 9f e5`… |
| SET#2 `MOVS r0,#2` | `0x14fb404` | **`02 20 96 f0 64 f1 6e 78`** (VA `0x414f47f4`) |

### Live (ONLINE, PCIe width warm, OB2 AP `0x40200000`)

Extended `saaios_cp_poke.ko`: RO DUMP16 + `sig_hex` probes at window
offsets `{0, SET2_low20=0xf47f4, …}`; skip full probe if win[0..63]=`ff`.

**Bands scanned (1MiB ATU retarget, RO only):**

| band | range | step | result |
| --- | --- | --- | --- |
| priors | `0x814f47f4`, `0x87200000`, `0x414f47f4`, `0x40010000`, `0x80010000` | — | DUMP **all `ff`** / ATU ok or retry |
| b800 | `0x80000000`–`0x90000000` | 16MiB | all-ff or FAIL(EPIPE race); **no HIT** |
| b400 | `0x40000000`–`0x50000000` | 16MiB | all-ff; **no HIT** |
| b000 | `0x00000000`–`0x10000000` | 16MiB | all-ff; **no HIT** |
| ba00 | `0xa0000000`–`0xb0000000` | 16MiB | all-ff; **no HIT** |
| bc00 | `0xc0000000`–`0xd0000000` | 16MiB | all-ff; **no HIT** |
| fBTL | `0x86000000`–`0x89000000` | 1MiB | all-ff; **no HIT** |
| fHYP | `0x80c00000`–`0x81c00000` | 1MiB | all-ff; **no HIT** |

Log: `/data/saaios/var/atu-scan.log` — `SCAN_NEGATIVE hits=0`, FINAL **ONLINE**.
~150 all-ff dumps; intermittent `insmod` EPIPE (ATU busy) with STATE still ONLINE.
**Zero** `SIG HIT` for SET#2/`02 20` or TOC. **No write / no poke.**

### Verdict

OB2 retarget sees open/ignored bus (`0xff`) across plausible CP PA bands —
MAIN code **not** exposed on modem outbound ATU. **MODEM-06 ATU-discovery =
dead.** Soft-lock unchanged. Goal incomplete (no rmnet/IPv4).

## 2026-09-30: PresentObj live DRAM locate — negative (no poke)

| probe | result |
| --- | --- |
| `saaios_sim_ro` / `saaios_shm_ro` | FMT WIRE/STATUS hits = ring copies only |
| BTL ATU 8 MiB | empty/`0xff` or ATU `-32` |
| ATU RO hyp PresentObj `0x88b87b7c` + `+0xBF4`/`+0xBF6` | **`0xff`** open-bus |
| `cp-dram-mmap-probe` | iod **ENODEV** (no userspace CP DRAM) |

**No writable PA → no poke.** Soft-lock PIN+pin1=2 unchanged. Watch left
armed. Goal incomplete (no rmnet/IPv4).

## 2026-09-30: SET#5 CMP live RAM patch — same open-bus

| site | VA | hyp PA | ATU (warm) | RO |
| --- | --- | --- | --- | --- |
| LDRB +0xBF6 `@file 0x14fb5bc` | `0x414f49ac` | `0x814f49ac` | ret=0 | **`ff`×16** |
| CMP #2 `@file 0x14fb5c0` | `0x414f49b0` | `0x814f49b0` | ret=0 | **`ff`×16** |

Expect stock `90 f8 f6 0b 02 28…`. **No write.** Soft-lock PIN; rmnet rx=0.
Live MAIN text still not on OB2. Goal incomplete.

## 2026-09-30: non-ATU phys inventory — no MAIN; BAR ioremap panics

Hypothesis: wrong ATU window; MAIN still reachable via DT reserved /
PCIe BAR / iomem / direct phys.

### Inventory (live ONLINE then post-restore)

| surface | AP phys | size | role |
| --- | --- | --- | --- |
| `cp_rmem` | `0xea400000` | 8MiB | SHMEM_IPC (FMT/RAW rings) |
| `cp_rmem_1` | `0xe8000000` | 32MiB | PKTPROC (+UL) DMA |
| `cp_msi_rmem` | `0xf6200000` | 4KiB | MSI |
| `cp_aoc_rmem` | `0x197fd000` | 12KiB | VSS_AOC |
| PCIe EP BAR2 | `0x81400000`–`0x81500000` | 1MiB | EP MEM (covers hyp SET#2 PA) |
| PCIe RC | `0x40000000`–`0x405fffff` | 6MiB | OB1/OB2 windows |

`/dev/mem` absent. `use_mem_map_on_cp=0`.

### RO (tool: `saaios_phys_ro.ko` memremap via kallsyms; `saaios_shm_ro`)

| probe | result |
| --- | --- |
| memremap `cp_rmem` `@0` | magic **`aa`**, ring heads — real IPC, **not** TOC/`02 20` |
| memremap `@0x16c10` / `@0xf47f4` in IPC | `ff` / `00` — **EXPECT TOC/SET#2 MISS** |
| memremap `cp_rmem_1` `@0` | pktproc desc (`54 30 06…`) ≠ TOC `54 4f 43` |
| `cp_shmem_get_region` idx3 | SIM-PAT = FMT **0x0200 copies** only |
| sysfs `resource2` dd | **EIO** when link down |
| ioremap BAR `0x81400000` | **AP panic/reboot** — do not repeat |
| ATU OB2 (prior) | open-bus `ff` — already dead |

**No write.** No Thumb/MAIN bytes on any non-ATU map. Soft-lock PIN;
rmnet rx=0. **Hard wall:** CP MAIN DRAM has no AP alias after ONLINE.

## Do not UpdateGoal complete

Stock ONLINE restore still used after panics. Soft-lock PIN; no
rmnet/IPv4 bearer.
