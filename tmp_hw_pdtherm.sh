#!/system/bin/sh
echo '=== dumpsys audio excerpt ==='
dumpsys audio 2>/dev/null | grep -E 'AudioMixer|Output thread|Input thread|Devices:|IO handle|Sample rate|AudioPort|Microphone|Speaker|Voice|aoc|AOC' | head -80

echo '=== audio policy devices ==='
dumpsys media.audio_policy 2>/dev/null | grep -E 'AudioPort|deviceName|type:|role:|mic|speaker' | head -80

echo '=== microphones api ==='
dumpsys audio 2>/dev/null | grep -A 80 'Available microphones' | head -90

echo '=== usb pd detailed ==='
ls /sys/class/typec/port0 2>/dev/null
for f in data_role power_role preferred_role port_type power_operation_mode usb_power_delivery_revision usb_typec_revision supported_data_partner_revision; do
  [ -e /sys/class/typec/port0/$f ] || continue
  echo -n "port0 $f="
  cat /sys/class/typec/port0/$f 2>/dev/null
done
ls /sys/class/typec/port0-partner 2>/dev/null
ls /sys/class/usb_power_delivery 2>/dev/null
find /sys/class/usb_power_delivery -type f 2>/dev/null | head -40 | while read f; do
  echo -n "$f="
  cat "$f" 2>/dev/null | tr '\n' ' '; echo
done

echo '=== thermal all ==='
for z in /sys/class/thermal/thermal_zone*; do
  t=$(cat "$z/type" 2>/dev/null)
  tmp=$(cat "$z/temp" 2>/dev/null)
  echo "$t $tmp"
done
ls /sys/class/thermal/cooling_device* 2>/dev/null | head
for c in /sys/class/thermal/cooling_device*; do
  echo "$(basename $c) type=$(cat $c/type 2>/dev/null) max=$(cat $c/max_state 2>/dev/null) cur=$(cat $c/cur_state 2>/dev/null)"
done | head -40
