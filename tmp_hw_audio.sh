#!/system/bin/sh
echo '=== alsa cards ==='
cat /proc/asound/cards 2>/dev/null
ls /proc/asound 2>/dev/null
ls /dev/snd 2>/dev/null

echo '=== pcm ==='
cat /proc/asound/pcm 2>/dev/null

echo '=== aoc devices ==='
ls -l /dev/aoc /dev/acd-* 2>/dev/null | head -80
ls /sys/class | grep -iE 'aoc|snd|sound'
ls /sys/devices/platform | grep -iE 'aoc|abox|audio'

echo '=== aoc fw / props ==='
getprop | grep -iE 'aoc|audio|offload|dolby|spatial|mic' | grep -viE 'fingerprint|serial|imei' | head -50
cat /sys/devices/platform/*/firmware_node/name 2>/dev/null | head
ls /vendor/firmware | grep -iE 'aoc|cs35|cs40|dsp|audio|call' | head -40

echo '=== tinymix if any ==='
which tinymix 2>/dev/null
ls /vendor/etc/audio 2>/dev/null | head
ls /vendor/etc/audio_param 2>/dev/null | head
ls /vendor/etc | grep -i audio | head

echo '=== sound modules ==='
lsmod | grep -iE 'snd|aoc|cs35|cs40|cirrus|dmic|mic|abox|voice'

echo '=== i2c audio ==='
for s in /sys/bus/i2c/devices/*; do
  n=$(cat "$s/name" 2>/dev/null)
  echo "$n" | grep -qiE 'cs35|cs40|aoc|codec|tfa|max9' || continue
  echo "$(basename $s) name=$n drv=$(basename $(readlink $s/driver 2>/dev/null))"
done
for s in /sys/bus/spi/devices/*; do
  echo -n "$(basename $s) "
  cat "$s/modalias" 2>/dev/null
done

echo '=== input audio related ==='
for d in /sys/class/input/event*; do
  echo "$(basename $d) $(cat $d/device/name 2>/dev/null)"
done
