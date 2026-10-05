# Pixel 7 (panther) audio in detail

Live stock **2026-10-05**. One ALSA card, everything else is
**Google AoC** plus two Cirrus speaker amps and a Cirrus haptic.

SaaiOS already plays a short stereo tone on **EP2** via static tinyplay
and CS35L41 DSP firmware. It does **not** implement capture, voice,
hotword, USB headset, or BT offload.

## SoC: Always-On Coprocessor

| Item | Live |
|---|---|
| DT | `/aoc@19000000` `google,aoc` |
| Firmware | `/vendor/firmware/aoc.bin` **21328448** bytes |
| Version | `vendor.aoc.firmware.version=14022723-polygon` |
| Daemons | `aocd`, `aocxd` |
| Char node | `/dev/aoc` plus many `/dev/acd-*` |
| Drivers | `aoc_core`, `aoc_alsa_dev`, `aoc_alsa_dev_util`, `aoc_pcm_drv`, `aoc_compr_drv`, `aoc_voice_drv`, `aoc_voip_drv`, `aoc_incall_drv`, `aoc_nohost`, `aoc_usb_driver`, `aoc_channel_dev`, `aoc_char_dev`, `aoc_control_dev` |
| Also on AoC | CHRE + USF (`/dev/acd-com.google.chre*`, `usf*`) — IMU/prox, not PCM |
| Modem link | `google_modemctl` used by `aoc_alsa_dev_util` **and** `cpif` |

Do not treat AoC as “just speakers”. Voice RX/TX, hotword, ultrasonic,
and USB audio all go through it. Shared with Shannon via modemctl.

## ALSA card `google,aoc-snd-card` (card 0)

Control: `/dev/snd/controlC0`. PCM endpoints from `/proc/asound/pcm`:

### Playback (p)

| PCM | Name | Role |
|---|---|---|
| 0 | EP1 playback | earpiece / first endpoint |
| 1 | **EP2 playback** | stereo speaker path SaaiOS uses |
| 2 | EP3 playback | extra endpoint |
| 3 | EP4 playback | |
| 4 | EP5 playback | |
| 5 | EP6 playback | |
| 7 | EP8 playback | (no EP7 in this table) |
| 14 | nohost1 playback | no-host / DSP |
| 16 | audio_voip_rx | VoIP downlink |
| 18–19, 29 | audio_incall_pb_0/1/2 | in-call playback (dummy DAI) |
| 23 | audio_raw | raw |
| 24 | haptic nohost playback | haptics over AoC |
| 25 | audio_hifiout | |
| 28 | audio_ultrasonic | ultrasound TX (Pixel spatial / occupancy) |
| 30 | audio_immersive | spatializer |
| 31 | audio_capture_inject | inject into capture |

Compressed: `/dev/snd/comprC0D6`. Offload props:
`aaudio.mmap_policy=2`, spatializer enabled.

### Capture (c)

| PCM | Name | Role |
|---|---|---|
| 8–13 | EP1–EP6 capture | endpoint mics / refs |
| 15 | nohost1 capture | |
| 17 | audio_voip_tx | VoIP uplink |
| 20–22 | audio_incall_cap_0/1/2 | in-call capture |
| 26 | audio_hifiin | |
| 27 | audio_android_aec | AEC reference |
| 32 | audio_hotword_tap | hotword |

AoC char devices that match: `acd-hotword_pcm`, `acd-hotword_notification`,
`acd-sound_trigger`, `acd-ambient_pcm`, `acd-audio_android_aec` (via
sysfs `audio_android_aec`), `acd-audio_rtp_rx/tx` (**radio** group —
VoLTE RTP), `acd-audio_haptaudio`, `acd-mel_processor`, taps 0–11.

## Speakers and earpiece

Two **Cirrus CS35L41** on SPI:

- `spi7.0` and `spi7.1`, driver `cs35l41`
- Firmware: `cs35l41-dsp1-spk-{cali,diag,prot}.{bin,wmfw}` plus `R-cs35l41-*` for the second amp
- Live metrics (persist, not secrets): impedances `100,100`, temps `25,25` °C, heartbeats ~11151/11158

Android ports:

- `AUDIO_DEVICE_OUT_EARPIECE` — “Earpiece”
- `AUDIO_DEVICE_OUT_SPEAKER` — “Speaker”
- `AUDIO_DEVICE_OUT_SPEAKER_SAFE`

SaaiOS `audio-init.sh` routes **EP2**. Earpiece is a different EP
(EP1 in the PCM map). Do not assume one CS35L41 is the earpiece;
earpiece on panther is the usual Pixel receiver path through AoC.

BCL can throttle speaker playback
(`BatteryThrottle=…BCL_AUDIO_BAACL…MediaSpeakerAndScreenOn` in
`audio_platform_configuration.xml`).

## Haptics

**CS40L26** I2C `8-0043` `cs40l26a`, input `event5`, firmware
`cs40l26.wmfw` / `cs40l26.bin` / calib / dvl / svc. ALSA PCM 24
`haptic nohost playback` plus `/dev/acd-audio_haptaudio`.
Factory F0/ReDC/Q from `persist` (already SaaiOS).

## Microphones (three)

Platform XML (`/vendor/etc/audio_platform_configuration.xml`) marks
the curves as “fake data” but the **count, addresses, and geometry
are the live mapping**:

| id | Android type | address | orientation | geometric_location (m) |
|---|---|---|---|---|
| builtin_mic_1 | `IN_BUILTIN_MIC` | **bottom** | 0,0,1 | 0.0269 0.0058 0.0079 |
| builtin_mic_2 | `IN_BACK_MIC` | **back** | 0,1,0 | 0.0546 0.1456 0.00415 |
| builtin_mic_3 | `IN_BUILTIN_MIC` | **top** | 0,0,1 | 0.0274 0.14065 0.0079 |

All omni, sensitivity −37 dB, SPL 28.5–132.5. Policy ports currently
exposed: “Built-In Mic” `@bottom`, “Built-In Back Mic” `@back`. Top is
in the mapping for beamform/voice, not always a separate AudioPort.

Capture also: telephony RX, echo reference, remote submix, USB/BT/BLE
headset mics.

There are **no** separate I2C codec nodes for the mics — they are
digital mics into AoC. Do not look for an analog codec like on A12.

## Voice / modem

In-call PCM uses dummy DAIs into AoC voice/incall drivers. RTP char
devs are `radio`. `google_modemctl` ties ALSA util to CPIF. Shannon
IMS can use this after a bearer exists. USB TTY HCO/VCO paths are in
the platform XML (`voice-speaker`, `usb-headset-mic`).

## USB and Bluetooth audio

- USB headset: `aoc_usb_driver` + `xhci_exynos` (AoC is on the DWC3
  graph). Policy: USB device/headset in and out.
- BT: A2DP offload `sbc-aac-aptx-aptxhd-ldac-opus`; LE Audio offload
  supported, LC3 on BLE headset/speaker ports. SaaiOS has HCI but not
  A2DP/LE Audio playback (documented).

Dolby: `vendor-dolby-media-c2-hal`, `ro.vendor.dolby.ddpm.version=DDPM_1.1.0_r1`.

## Userspace (stock)

`vendor.audio-hal`, `audioserver`, `audiometricext`. Policy XMLs under
`/vendor/etc/audio_policy_configuration.xml` (not `vendor/etc/audio/`).

## What will break if copied wrong

- Playing on EP1 vs EP2 vs “Speaker Safe”.
- Capture without AoC `acd-*` nodes and EP capture PCMs.
- Voice without `google_modemctl` + incall PCMs + radio `acd-audio_rtp_*`.
- Skipping CS35L41 `spk-prot` firmware — protection DSP.
- Clocking AoC PD (`dbgdev-pd-aoc`) independently of CPIF QoS.
