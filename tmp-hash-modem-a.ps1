& "C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1" -Port COM13 -WaitSeconds 60 -Cmd @'
mkdir -p /mnt/modem_ro; set -- $(cat /sys/block/sda/sda19/dev | tr : " "); rm -f /dev/block/sda19; mknod /dev/block/sda19 b $1 $2; mount -t ext4 -o ro,noload /dev/block/sda19 /mnt/modem_ro; echo A_VER; ls /mnt/modem_ro/images; VER=$(ls /mnt/modem_ro/images | head -1); echo A_SHA; sha256sum /mnt/modem_ro/images/$VER/modem.bin; umount /mnt/modem_ro; rm -f /dev/block/sda19; echo DONE
'@
