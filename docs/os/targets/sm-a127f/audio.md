# SM-A127F — ALSA / speaker (Phase E1)

Speaker is **SMA1303 on UAIF1**, not the AUD3004X class-D `SPK` path. Headset/EP/HP stay off. Detail from vendor `mixer_paths.xml` + `exynos3830_aud3004x.c`.

## Card / PCM

| | |
|--|--|
| Card | `Exynos3830-Madera` (`/proc/asound/cards`) |
| Mixer | `/dev/snd/controlC0` |
| Beep PCM | `/dev/snd/pcmC0D1p` (RDMA1 → SIFS1 → UAIF1). **LIVE v026** — this is what the speaker hears |
| Silent PCM | `pcmC0D3p` (RDMA3 → SIFS0 → UAIF1) — vendor `media-speaker`. v021–v025 used this and stayed silent |
| Format | 48 kHz, S16_LE, stereo |

PCM open with no muxes is silent. PCM open **without ABOX firmware in the ramdisk hangs**. v021–v025 silence was **wrong FE/SIFS**, not missing firmware. TONEGEN on RDMA3 (v025) proved the digital path attached; D1+SIFS1 is the ear path.

## Firmware

DT chosen cmdline already has `firmware_class.path=/vendor/firmware`. Ramdisk packs these vendor blobs under `/vendor/firmware/` and `/lib/firmware` → `/vendor/firmware`:

- `calliope_sram.bin` / `calliope_sram_2.bin`
- `calliope_dram.bin` / `calliope_dram_2.bin`
- `AP_AUDIO_SLSI.bin` / `APDV_AUDIO_SLSI.bin`
- `abox_tplg.bin`

Extract: `make -f os/Makefile abox-fw` (`debugfs dump` from `os/build/stock-super/vendor.img`). Skip slog/camera bins.

## Mixer (exact names)

Userspace `/sbin/beep` and `/sbin/play` (tinyalsa) before `pcm_open`:

```text
ABOX Sound Type                   = SPEAKER   (live default VOICE — receiver)
ABOX UAIF0 SPK                    = RESERVED
HP HP On / EP EP On               = 0
ABOX SPUS OUT1                    = SIFS1     (LIVE v026)
ABOX SIFS1                        = SPUS OUT1
ABOX UAIF1 SPK                    = SIFS1     (not SIFS0)
ABOX UAIF1 Width / Channel / Rate = 16 / 2 / 48000
ABOX UAIF1 Extend BCLK            = 1
ABOX SIFS1 Width                  = 16
ABOX RDMA1_A                      = TONEGEN_1KHZ
ABOX RDMA3_A                      = None      (vendor media-speaker FE — silent)
ABOX SPUS OUT3                    = RESERVED
ABOX SPUS ASRC3                   = leave On (Off wedged pcm_writei on v021)
Speaker Volume                    = 160  (TLV invert: 167=loudest, stock shows 118)
Speaker Mute Switch               = 0
Power Up(1:Up_0:Down)             = 1
Speaker Mode                      = 1 Mono after pcm_open (idle Off is shutdown)
```

Skip Android-only `ABOX RDMA3_A` = `BD_MIXER` / VPCM.

## Jack vs speaker

AUD3004X 5-pin ADC (`adc-gdet` / `adc-ear`, `AUD3004X Headset Input` / `event7`) reports `SW_HEADPHONE_INSERT` / `SW_MICROPHONE_INSERT`. `aud3004x_jackstate_register` writes **codec** FSM/mic only. It does **not** mute SMA1303 or drop DAPM `SPEAKER`.

False jack would make Android HAL pick headset (`UAIF0` + `HP`). Our beep never used HAL. The matching firmware gate is **`ABOX Sound Type`** (`ABOX_SET_TYPE` IPC). Live v021: **VOICE**. `communication-speaker` sets `SPEAKER`. v023 does that and logs jack switches.

## Do not

- Toggle `Codec Enable`
- Pulse SMA1303 `I2C Reg Reset` or `Force AMP Power Down`
- Auto-beep at boot (watchdog + surprise). Short Power only; beep is a child so PID 1 keeps petting WDT
- Unbind TSP, second `FBIOBLANK`, maze Image

## Play WAV (boot-v027)

`/sbin/play` opens the same v026 path (`pcmC0D1p` / SIFS1 / UAIF1 / SMA1303). PCM 16-bit WAV only (mono or stereo; resampled to 48 kHz stereo). Packed clip is a generated Ode to Joy phrase (sine + envelope, not a square beep). **Not** started at boot. Power tap stays `/sbin/beep`.

```text
/sbin/play /usr/share/sounds/test.wav
```

Or scp/wget any `.wav` to `/tmp` and `/sbin/play /tmp/file.wav`. BusyBox has no mp3; ogg/mp3 are not in this ramdisk.

**LIVE 2026-08-29:** user confirmed **работает**. `/sbin/play` WAV on the v026 speaker path (`pcmC0D1p` / SIFS1 / UAIF1 / SMA1303). Clip `/usr/share/sounds/test.wav` (Ode to Joy). First heard via live `/tmp/play /tmp/test.wav` scp’d onto still-running v026 (`play: ok rdma1 sifs1`); then confirmed again (v027 packed `/sbin/play`, or still `/tmp`). Replay: `/sbin/play /usr/share/sounds/test.wav`. Power tap stays beep. Do not regress D1/SIFS1.

## Console (boot-v027)

Banner `SaaiOS v027`. Same USB as v024 (LF `rcS`) and same LIVE D1/SIFS1 route as v026. Adds `/sbin/play` + `/usr/share/sounds/test.wav`. Pack: `make -f os/Makefile boot-v027`. Tar SHA256 `4fce1292e80bb7153c9113466a1f01ba34a8eef61c4de1e5dca9bbe6150b1101`. Does not overwrite v021–v026. Rollback: `saaios-boot-v026.tar`.

**LIVE 2026-08-29:** user said **работает**. `/sbin/play` on D1/SIFS1. Replay `/sbin/play /usr/share/sounds/test.wav`. First `/tmp` on v026, then confirmed. Do not regress to `pcmC0D3p` / SIFS0.

## Console (boot-v026)

Banner `SaaiOS v026`. Same USB as v024/v025 (LF `rcS`). Working PCM is **`pcmC0D1p` / SIFS1 / UAIF1**, not vendor `media-speaker` RDMA3/SIFS0. v021–v025 were silent for that reason (wrong FE/SIFS), not missing firmware. USB fix was CRLF `rcS` (v024). TONEGEN on RDMA3 (v025) proved the digital path; D1+SIFS1 is what the speaker hears. Pack: `make -f os/Makefile boot-v026`. Tar SHA256 `094a8c06e4794f1094c36cbfb6ee0ee528449444f96f602048af289ba9c0baa1`. Does not overwrite v021–v025. Rollback: `saaios-boot-v025.tar` (USB+TONEGEN) or `saaios-boot-v024.tar`.

**LIVE 2026-08-29:** human flashed AP; ear heard the tone. `strings /init` = `SaaiOS v026`. Telnet `:23`/`:2323`. Cards `Exynos3830-Madera` / `abox_vdma` / `abox_dump`. `pcmC0D1p` present. Beep dmesg: DAPM `TONEGEN_1KHZ` / `RDMA1_A` / `SPUS OUT1` / `SIFS1` / `UAIF1 SPK`, `reset sifs1_cnt_val`, Calliope `NFB0`, SMA UNMUTE, `abox_rdma_trigger[1](1)`. Mixer after beep: `Sound Type=SPEAKER`, `UAIF1 SPK=SIFS1`, `SPUS OUT1=SIFS1`, `SIFS1=SPUS OUT1`, `RDMA1_A=TONEGEN_1KHZ`, `RDMA3_A=None`, `OUT3=RESERVED`, volume 160, Mode Mono, Power Up On, mute On (idle). Dropbear `-R` did not write host keys on first boot (`:22` empty until a later `dropbear -R`).

## Console (boot-v025)

Banner `SaaiOS v025`. Same USB as v024 (LF `rcS`). Beep keeps SPEAKER and sets `ABOX RDMA3_A=TONEGEN_1KHZ` (vendor `media-speaker` uses `BD_MIXER` + VPCM; idle mux is `None`). Hold Power Up during write. Pack: `make -f os/Makefile boot-v025`. Does not overwrite v021–v024. Rollback: `saaios-boot-v024.tar` (USB) or `saaios-boot-v021.tar`.

**LIVE 2026-08-29:** USB `rndis0` UP, telnet `:23`/`:2323`. TONEGEN DAPM up, Calliope `NFB0`, UNMUTE, `abox_rdma_trigger[3](1)`. Mixer after beep: `Sound Type=SPEAKER`, `RDMA3_A=TONEGEN_1KHZ`, `UAIF1 SPK=SIFS0`, `SPUS OUT3=SIFS0`, volume 160, Mode Mono, Power Up On. Telnet `/sbin/beep` `pcm_writei` EIO. Ear silent. Dropbear process up but not listening (`:22` empty — host keys never written).

## Console (boot-v024)

Banner `SaaiOS v024`. Same SPEAKER beep as v023. USB: LF `rcS`, listeners before gadget iface, `ifconfig up` for RNDIS carrier. Pack: `make -f os/Makefile boot-v024`. Rollback: `saaios-boot-v021.tar`.

## Console (boot-v023)

Banner `SaaiOS v023`. v022 amp settings plus `ABOX Sound Type=SPEAKER`, UAIF0 reserved, jack evdev log. Pack: `make -f os/Makefile boot-v023`. Rollback: `saaios-boot-v022.tar`. Packed `rcS` was CRLF — ash never ran `ifconfig` (Windows RNDIS “cable unplugged”). Use v024.

## Console (boot-v022)

Banner `SaaiOS v022`. Same as v021 plus amp Mode/unmute/ASRC bypass. Pack: `make -f os/Makefile boot-v022`. Rollback: `saaios-boot-v021.tar`.

**v021 LIVE kernel, silent to the ear:** cards up, RDMA3 triggered, Calliope `NFB0`. Idle mixer `Speaker Mode=Off` / mute On is `sma1303_shutdown`, not the playback state. Stock volume 118 = hardware `0x31`. v022: volume **160** (TLV invert — 32 is quieter, do not use), Power Up, Mono after `pcm_open`, 800 ms / louder PCM. Do not turn `ABOX SPUS ASRC3` Off.
