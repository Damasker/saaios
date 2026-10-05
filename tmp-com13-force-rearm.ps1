param(
    [string]$Port = "COM13",
    [int]$WaitSeconds = 40
)
$ErrorActionPreference = "Stop"
$p = New-Object System.IO.Ports.SerialPort $Port, 115200, None, 8, one
$p.ReadTimeout = 400
$p.WriteTimeout = 4000
$p.DtrEnable = $true
$p.RtsEnable = $true
$p.NewLine = "`n"
$p.Open()
Start-Sleep -Milliseconds 200
$p.Write([byte[]](0x03), 0, 1)
Start-Sleep -Milliseconds 400
$p.Write([byte[]](0x03), 0, 1)
Start-Sleep -Milliseconds 500
$p.DiscardInBuffer()
$p.Write("`n")
Start-Sleep -Milliseconds 200
$p.DiscardInBuffer()

$marker = "ENDCOM13_$([guid]::NewGuid().ToString('N').Substring(0, 8))"
# BusyBox: kill by matching cmdline PIDs (killall misses `sh /path/script`).
$cmd = @'
echo FORCE; ps w | grep -E "tray-bearer|sit-sim-status" | grep -v grep; kill -9 5257 6292 11920 11921 12479 2>/dev/null; for p in $(ps w | grep -E "tray-bearer-chase|sit-sim-status" | grep -v grep | sed -n "s/^ *\([0-9][0-9]*\).*/\1/p"); do echo K:$p; kill -9 $p; done; sleep 2; echo AFTER; ps w | grep -E "tray-bearer|sit-sim" | grep -v grep || echo CLEARED; rm -f /run/saaios-tray-chase.lock; nohup env PERSIST=1 WATCH_ROUNDS=12 OUT=/data/saaios/var/tray-bearer.log sh /data/saaios/bin/tray-bearer-chase.sh >/data/saaios/var/tray-bearer.nohup 2>&1 & echo STARTED:$!; sleep 4; echo ALIVE; cat /data/saaios/var/tray-bearer.alive; echo PS; ps w | grep tray-bearer | grep -v grep; echo LOG; tail -n 16 /data/saaios/var/tray-bearer.log; echo DONE
'@
$wrapped = "($cmd); printf '%s\n' $marker"
$p.Write($wrapped + "`n")

$buf = New-Object System.Text.StringBuilder
$deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
$done = $false
while ([DateTime]::UtcNow -lt $deadline) {
    try {
        $chunk = $p.ReadExisting()
        if ($chunk) {
            [void]$buf.Append($chunk)
            if ($buf.ToString() -match [regex]::Escape($marker)) { $done = $true; break }
        }
    } catch {}
    Start-Sleep -Milliseconds 60
}
$p.Close()
$text = ($buf.ToString() -replace "`r", "")
Write-Output $text
if (-not $done) { Write-Output "TIMEOUT waiting for $marker"; exit 2 }
