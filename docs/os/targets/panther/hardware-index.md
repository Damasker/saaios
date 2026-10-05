# Pixel 7 (panther) hardware map — what is documented

Live stock inventories 2026-10-05 (SKU **GVU6C**, rev **MP1.0**).
This is **not** a claim that SaaiOS drives every block.

| Domain | Doc | SaaiOS today |
|---|---|---|
| Display / touch / keys / brightness | [README.md](README.md) | Working |
| Speakers CS35L41, AoC, CS40L26 haptics | [README.md](README.md) | Working |
| Wi-Fi BCM4389, BT | [README.md](README.md) | Working |
| Cellular Shannon / CPIF / NV | [modem.md](modem.md), [modem-stock-reproduction.md](modem-stock-reproduction.md) | Not a bearer |
| Modem-adjacent PCIe, GSA, GNSS, eSE/NFC | [hardware-risks.md](hardware-risks.md) | Partial (NFC/eSE unused) |
| Cameras LWIS, UDFPS, prox, IMU/CHRE | [sensors-cameras.md](sensors-cameras.md) | Not claimed |
| GPU Mali-G710 | [gpu.md](gpu.md) | Scanout only; stock kbase r54p3 + CSF not packaged |
| Qi / USB-C / BMS | [power-charging.md](power-charging.md) | Battery % only |
| USB-PD PDOs + thermals | [usb-pd-thermal.md](usb-pd-thermal.md) | Not claimed |
| AoC audio, CS35L41, 3 mics, voice | [audio.md](audio.md) | EP2 playback + haptics only |

## Not a Pixel 7 (this SKU)

- **UWB** — Pixel 7 Pro (`cheetah`), not `panther`.
- **Telephoto** — Pro only; panther is wide + UW + front.
- **Soli** — Pixel 4.

## Seen on the SoC, not given a dedicated page

Still real, lower priority than radio/camera/WLC:

- UFS (`ufs_exynos_gs`) — userdata already mounted.
- Privacy LED / camera indicator — not dumped.
- Titan-class **GSA** — NV/keystore in the modem docs.

If something is missing from this table, it was not inventoried live yet.
