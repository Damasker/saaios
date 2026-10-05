param(
    [Parameter(Mandatory = $true)][string]$Cmd,
    [int]$WaitSeconds = 30,
    [string]$Port = "COM13"
)

$p = New-Object System.IO.Ports.SerialPort $Port, 115200, None, 8, one
$p.ReadTimeout = 400
$p.WriteTimeout = 5000
$p.DtrEnable = $true
$p.RtsEnable = $true
$p.NewLine = "`n"
$p.Open()
Start-Sleep -Milliseconds 150
$p.DiscardInBuffer()
$p.Write([byte[]]@(3), 0, 1)
Start-Sleep -Milliseconds 100
$p.Write("`n")
Start-Sleep -Milliseconds 150
$p.DiscardInBuffer()

$marker = "ENDCOM13_$([guid]::NewGuid().ToString('N').Substring(0, 8))"
# Disable ash bracketed paste / cursor query noise if possible; still strip CSI.
$wrapped = "($Cmd); printf '%s\n' $marker"
$p.Write($wrapped + "`n")

$buf = New-Object System.Text.StringBuilder
$deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
$done = $false
while ([DateTime]::UtcNow -lt $deadline) {
    try {
        $chunk = $p.ReadExisting()
        if ($chunk) {
            [void]$buf.Append($chunk)
            $text = $buf.ToString() -replace "`r", ""
            # Strip CSI / OSC sequences for matching
            $clean = [regex]::Replace($text, "\x1b\[[0-9;?]*[A-Za-z]", "")
            $clean = [regex]::Replace($clean, "\x1b\][^\x07]*\x07", "")
            foreach ($line in ($clean -split "`n")) {
                if ($line.Trim() -eq $marker) { $done = $true; break }
            }
            if ($done) { break }
        }
    } catch {}
    Start-Sleep -Milliseconds 40
}
$p.Close()

$text = ($buf.ToString() -replace "`r", "")
$clean = [regex]::Replace($text, "\x1b\[[0-9;?]*[A-Za-z]", "")
$lines = New-Object System.Collections.Generic.List[string]
foreach ($line in ($clean -split "`n")) {
    if ($line.Trim() -eq $marker) { break }
    # skip echoed command prefix
    if ($line.Contains($wrapped.Substring(0, [Math]::Min(20, $wrapped.Length)))) { continue }
    [void]$lines.Add($line)
}
Write-Output ($lines -join "`n")
if (-not $done) {
    Write-Output "TIMEOUT waiting for $marker"
    exit 2
}
