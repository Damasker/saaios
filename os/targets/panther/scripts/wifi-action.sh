#!/saaios/sh

case "$2" in
    CONNECTED)
        /saaios/busybox udhcpc -b -q -n -t 5 -i "$1" \
            -s /saaios/udhcpc.script > /run/udhcpc.log 2>&1
        if /saaios/sntp-sync time.google.com > /run/time-sync.log 2>&1 || \
           /saaios/sntp-sync pool.ntp.org > /run/time-sync.log 2>&1; then
            : > /run/time-ready
        fi
        ;;
    DISCONNECTED)
        /saaios/ip addr flush dev "$1"
        ;;
esac
