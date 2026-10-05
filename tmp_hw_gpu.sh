#!/system/bin/sh
echo '=== gpu nodes ==='
ls -l /dev/mali* /dev/dri/* /dev/kgsl* /dev/gsa* 2>/dev/null
ls /sys/class/misc | grep -iE 'mali|gpu|g710|panthor'
ls /sys/class | grep -iE 'mali|drm|gpu'

echo '=== modules ==='
lsmod | grep -iE 'mali|gpu|panfrost|panthor|midgard|g710|gsa|gxp'

echo '=== platform ==='
ls /sys/devices/platform | grep -iE 'mali|gpu|g710|gsa|gxp'
find /sys/devices/platform -maxdepth 2 -iname '*mali*' -o -iname '*gpu*' 2>/dev/null | head -40

echo '=== firmware ==='
ls /vendor/firmware | grep -iE 'mali|csf|g710|gpu|mgm'
ls /lib/firmware 2>/dev/null | grep -iE 'mali|csf' | head
ls /vendor/firmware/mali 2>/dev/null
find /vendor /lib -iname '*mali*' -o -iname '*csffw*' 2>/dev/null | head -40

echo '=== props ==='
getprop | grep -iE 'gpu|mali|gles|vulkan|render|hwui|skia|opengl' | grep -viE 'fingerprint|serial|imei' | head -40

echo '=== drm ==='
ls /dev/dri 2>/dev/null
cat /sys/class/drm/card0/device/vendor 2>/dev/null
for c in /sys/class/drm/card*; do echo $c; cat $c/device/uevent 2>/dev/null | head -8; done

echo '=== dt ==='
find /sys/firmware/devicetree/base -name compatible 2>/dev/null | while read c; do
  v=$(tr '\0' ' ' < "$c")
  echo "$v" | grep -qiE 'mali|gpu|g710|panthor|arm,valhall|arm,mali' || continue
  echo "$(echo $c | sed 's|.*/base||;s|/compatible||'): $v"
done
