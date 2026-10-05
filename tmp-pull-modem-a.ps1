# Host: listen and receive modem_a over USB NCM
$out = 'C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw\saaios-probe-a-modem.bin'
New-Item -ItemType Directory -Force -Path (Split-Path $out) | Out-Null
# Use Python listener on Windows for reliability
$py = @'
import socket, sys
out = sys.argv[1]
srv = socket.socket(); srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(("172.31.7.2", 9876)); srv.listen(1)
print("LISTEN 172.31.7.2:9876", flush=True)
conn, addr = srv.accept()
print("PEER", addr, flush=True)
n = 0
with open(out, "wb") as f:
    while True:
        b = conn.recv(1024 * 1024)
        if not b: break
        f.write(b); n += len(b)
        if n % (8 * 1024 * 1024) == 0: print("got", n, flush=True)
conn.close(); srv.close()
print("DONE", n, flush=True)
'@
$pyPath = 'C:\Users\Admin\Projects\saaios-som\tmp-recv-a.py'
Set-Content -Path $pyPath -Value $py -Encoding ASCII
Start-Process -FilePath python -ArgumentList $pyPath, $out -WindowStyle Hidden
Start-Sleep -Seconds 1
# Phone: mount A RO and push
$cmd = 'mkdir -p /dev/block /mnt/modem_ro; set -- $(cat /sys/block/sda/sda19/dev | tr : " "); mknod /dev/block/sda19 b $1 $2 2>/dev/null; mount -t ext4 -o ro,noload /dev/block/sda19 /mnt/modem_ro; VER=$(ls /mnt/modem_ro/images | grep g5300q | head -1); echo VER:$VER; ls -l /mnt/modem_ro/images/$VER/modem.bin; busybox nc 172.31.7.2 9876 < /mnt/modem_ro/images/$VER/modem.bin; echo NC:$?; umount /mnt/modem_ro; rm -f /dev/block/sda19; echo SENT'
& 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1' -Port COM13 -WaitSeconds 180 -Cmd $cmd
Start-Sleep -Seconds 3
if (Test-Path $out) { Get-FileHash $out -Algorithm SHA256 | Format-List; (Get-Item $out).Length }
