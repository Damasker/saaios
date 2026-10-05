#!/system/bin/sh
echo '=== pci drivers ==='
for d in 0000:00:00.0 0000:01:00.0 0001:00:00.0 0001:01:00.0; do
  echo "-- $d"
  ls -l /sys/bus/pci/devices/$d/driver 2>/dev/null
  cat /sys/bus/pci/devices/$d/uevent 2>/dev/null
  cat /sys/bus/pci/devices/$d/current_link_speed 2>/dev/null
  cat /sys/bus/pci/devices/$d/current_link_width 2>/dev/null
done

echo '=== gnss dt ==='
tr '\0' ' ' < /sys/firmware/devicetree/base/gnss/compatible 2>/dev/null; echo
ls /sys/firmware/devicetree/base/gnss 2>/dev/null | head
ls /sys/devices/platform/gnss 2>/dev/null | head

echo '=== nfc ese spi ==='
ls /sys/bus/spi/devices 2>/dev/null
for s in /sys/bus/spi/devices/*; do
  echo "-- $s"
  cat "$s/modalias" 2>/dev/null
  ls -l "$s/driver" 2>/dev/null
done
ps -A | grep -iE 'nfc|st21|ese|secure_element' | head

echo '=== bcl / modemctl ==='
ls /sys/devices/platform | grep -iE 'bcl|modemctl|odpm'
cat /sys/module/google_modemctl/parameters/* 2>/dev/null | head
ls /sys/class/google_modemctl 2>/dev/null

echo '=== confpack label ==='
cat /mnt/vendor/modem_img/images/default/confpack/release-label 2>/dev/null; echo
cat /mnt/vendor/modem_img/images/default/confpack/build.info 2>/dev/null | head -20

echo '=== pcie modem stats names ==='
ls /sys/devices/platform/cpif/modem 2>/dev/null
ls /sys/devices/platform/11920000.pcie 2>/dev/null | head -30
ls /sys/devices/platform/14520000.pcie 2>/dev/null | head -30

echo '=== mbox ==='
ls /sys/devices/platform | grep mbox
tr '\0' ' ' < /sys/firmware/devicetree/base/mailbox@18300000/compatible 2>/dev/null; echo

echo '=== hardware sku ==='
getprop ro.boot.hardware.sku
getprop ro.boot.hardware.revision
getprop ro.boot.hardware.radio.subtype
getprop ro.boot.hw.soc.rev
getprop ro.boot.hwdevice
getprop vendor.usb.product_string
