#!/system/bin/sh
set -e
BB=/data/local/tmp/bb
if [ ! -x "$BB" ]; then
  if [ -x /saaios/busybox ]; then BB=/saaios/busybox
  elif [ -x /data/adb/ksu/bin/busybox ]; then BB=/data/adb/ksu/bin/busybox
  else BB=grep
  fi
fi

echo '=== props ==='
getprop persist.sys.usb.config
getprop sys.usb.config
getprop gsm.sim.state
getprop gsm.operator.numeric
getprop gsm.network.type
getprop gsm.operator.alpha
getprop vendor.cbd.boot_done
getprop vendor.rild.libpath
getprop init.svc.vendor.cbd
getprop init.svc.vendor.rild
getprop init.svc.vendor.rfsd
getprop init.svc.vendor.ril-daemon
getprop init.svc.rild
getprop init.svc.vendor.shared_modem_platform
getprop init.svc.shared_modem_platform
getprop persist.vendor.radio.multisim.config
getprop ro.boot.slot_suffix
getprop gsm.version.baseband
getprop vendor.ril.cbd.boot_done
getprop vendor.cbd.status
getprop vendor.rild.status

echo '=== getprop vendor radio keys ==='
getprop | grep -E 'cbd|rild|rfsd|modem|ril\.|radio\.|cpif|shannon|sit' | grep -viE 'imei|imsi|iccid|serialno|android_id'

echo '=== ps ==='
ps -A | grep -E 'cbd|rild|rfsd|shared_modem|imsda|shannon|cpboot' || true

echo '=== nodes ==='
ls -l /dev/umts_* /dev/ehci_s5123 /dev/cp_stat /dev/gnss_ipc 2>/dev/null || true
ls -l /dev/block/by-name/modem* /dev/block/by-name/efs* /dev/block/by-name/radio* 2>/dev/null || true

echo '=== efs nv ==='
ls -l /mnt/vendor/efs/nv_normal.bin* /mnt/vendor/efs/nv_protected.bin* /mnt/vendor/efs/nv_* 2>/dev/null || true
ls -ld /mnt/vendor/efs /mnt/vendor/efs/FactoryApp /mnt/vendor/efs/imei 2>/dev/null || true
find /mnt/vendor/efs -maxdepth 2 -type f -printf '%s %p\n' 2>/dev/null | grep -viE 'imei|serial|wifi|bluetooth|mac' | head -80

echo '=== ifaces ==='
ip -4 addr show | grep -E 'rmnet|ccmni|seth|v4-rmnet' || true
ip link show | grep -E 'rmnet|ccmni' || true

echo '=== telephony snip ==='
dumpsys telephony.registry 2>/dev/null | grep -E 'mServiceState|mDataConnectionState|mDataRegState|mVoiceRegState|operatorNumeric|mRilVoiceRadioTechnology|mRilDataRadioTechnology|mNrFrequencyRange|mOperatorAlpha' | head -60

echo '=== cbd rc ==='
ls -l /vendor/etc/init/cbd.rc /vendor/etc/init/init.modem.rc /vendor/etc/init/rild_exynos.rc /vendor/etc/init/*rfs* /vendor/etc/init/*ril* 2>/dev/null || true
