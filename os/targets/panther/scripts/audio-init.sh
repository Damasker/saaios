#!/saaios/busybox sh
set -eu

M=/saaios/tinymix

$M -D 0 'TDM_0_RX Chan' Four
$M -D 0 'TDM_0_RX Format' S32_LE
$M -D 0 'TDM_0_RX Sample Rate' SR_48K
$M -D 0 'TDM_0_RX nSlot' Four
$M -D 0 'R ASPRX1 Slot Position' 1
$M -D 0 'R ASPRX2 Slot Position' 0
$M -D 0 'DRE DRE Switch' 1
$M -D 0 'R DRE DRE Switch' 1
$M -D 0 'AMP PCM Gain' 8
$M -D 0 'R AMP PCM Gain' 8
$M -D 0 'Digital PCM Volume' 600
$M -D 0 'R Digital PCM Volume' 600
$M -D 0 'DSP1 Firmware' Protection
$M -D 0 'R DSP1 Firmware' Protection
$M -D 0 'DSP1 Preload Switch' 1
$M -D 0 'R DSP1 Preload Switch' 1
/saaios/busybox sleep 1
$M -D 0 'PCM Source' DSP
$M -D 0 'R PCM Source' DSP
$M -D 0 'Boost Peak Current Limit' 2.50A
$M -D 0 'R Boost Peak Current Limit' 2.50A
$M -D 0 'TDM_0_RX Mixer EP1' 0
$M -D 0 'TDM_0_RX Mixer EP2' 1
$M -D 0 'Main AMP Enable Switch' 1
$M -D 0 'R Main AMP Enable Switch' 1

printf '600\n' > /run/audio-volume
: > /run/audio-ready
