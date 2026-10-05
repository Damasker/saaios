param(
    [string]$Port = "COM13",
    [int]$WaitSeconds = 25
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
# Interrupt any foreground tray-watch / chase holding the TTY.
$p.Write([byte[]](0x03), 0, 1)  # Ctrl-C
Start-Sleep -Milliseconds 400
$p.Write([byte[]](0x03), 0, 1)
Start-Sleep -Milliseconds 500
$p.DiscardInBuffer()
$p.Write("`n")
Start-Sleep -Milliseconds 200
$p.DiscardInBuffer()

# One-line re-arm (no multiline backslash continuations).
$marker = "ENDCOM13_$([guid]::NewGuid().ToString('N').Substring(0, 8))"
$cmd = @'
killall tray-bearer-chase.sh 2>/dev/null; killall sit-sim-status 2>/dev/null; sleep 1; rm -f /run/saaios-tray-chase.lock; nohup env PERSIST=1 WATCH_ROUNDS=12 OUT=/data/saaios/var/tray-bearer.log sh /data/saaios/bin/tray-bearer-chase.sh >/data/saaios/var/tray-bearer.nohup 2>&1 & echo STARTED:$!; sleep 2; cat /data/saaios/var/tray-bearer.alive; ps | grep tray-bearer | grep -v grep; tail -n 8 /data/saaios/var/tray-bearer.log
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
