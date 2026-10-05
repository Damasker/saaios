#!/system/bin/sh
echo '=== smp config ==='
cat /data/vendor/radio/shared_modem_platform_config; echo
cat /data/vendor/radio/shared_modem_platform_pid; echo

echo '=== dmesg cpif/cbd (no secrets) ==='
dmesg 2>/dev/null | grep -iE 'cbd|rfsd|cpif|s5100|umts_|modem.bin|NV_NORM|NV_PROT|udl' | grep -viE 'imei|imsi|iccid' | head -80

echo '=== logcat RADIO_POWER ==='
logcat -b all -d 2>/dev/null | grep RADIO_POWER | head -20

echo '=== GET_SIM_STATUS ==='
logcat -b radio -d 2>/dev/null | grep -E 'GET_SIM_STATUS|SIM_STATUS_CHANGED|UNSOL_RESPONSE_SIM' | grep -viE 'imei|imsi|iccid' | head -20

echo '=== efs_backup nv ==='
ls -l /mnt/vendor/efs_backup/nv_* 2>/dev/null
ls /mnt/vendor/modem_img/images/default/ 2>/dev/null | head
ls -l /mnt/vendor/modem_img/images/default/modem.bin 2>/dev/null

echo '=== persist camp ==='
getprop persist.vendor.ril.camp_on_earlier
getprop vendor.ril.allow_data_0
getprop vendor.cbd.modem_bin_status
getprop vendor.cbd.modem_bin_type
getprop vendor.ril.cbd.rfs_check_done
getprop persist.radio.is_vonr_enabled_0
getprop persist.vendor.radio.target_oper
getprop persist.vendor.ril.support_nr_ds
getprop persist.vendor.ril.use_radio_hal
getprop ro.vendor.ril.use_radio_hal

echo '=== extra rfsd ==='
ps -A | grep rfsd
