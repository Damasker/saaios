$ErrorActionPreference = 'Continue'
$outDir = 'C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw'
$out = Join-Path $outDir 'saaios-probe-a-modem.bin'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# Mount A and start busybox httpd serving the version dir
$cmd = 'mkdir -p /dev/block /mnt/modem_ro /tmp/ahttp; set -- $(cat /sys/block/sda/sda19/dev | tr : " "); mknod /dev/block/sda19 b $1 $2 2>/dev/null; mount -t ext4 -o ro,noload /dev/block/sda19 /mnt/modem_ro; VER=$(ls /mnt/modem_ro/images | grep g5300q | head -1); echo VER:$VER; ln -sf /mnt/modem_ro/images/$VER/modem.bin /tmp/ahttp/modem.bin; killall httpd 2>/dev/null; busybox httpd -p 0.0.0.0:8088 -h /tmp/ahttp; echo HTTPD:$?; ls -l /tmp/ahttp/modem.bin'
& 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1' -Port COM13 -WaitSeconds 40 -Cmd $cmd

Write-Host 'DOWNLOAD'
curl.exe -f --connect-timeout 10 --max-time 300 -o $out "http://172.31.7.1:8088/modem.bin"
if (Test-Path $out) {
  Write-Host ("SIZE={0}" -f (Get-Item $out).Length)
  Get-FileHash $out -Algorithm SHA256 | Format-List
}

# cleanup httpd/mount
$cleanup = 'killall httpd 2>/dev/null; umount /mnt/modem_ro 2>/dev/null; rm -f /dev/block/sda19 /tmp/ahttp/modem.bin; echo CLEAN'
& 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1' -Port COM13 -WaitSeconds 15 -Cmd $cleanup
