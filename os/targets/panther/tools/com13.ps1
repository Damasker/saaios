param(
    [Parameter(Mandatory = $true)]
    [string]$Cmd,
    [int]$WaitSeconds = 25,
    [string]$Port = "COM13"
)

$p = New-Object System.IO.Ports.SerialPort $Port, 115200, None, 8, one
$p.ReadTimeout = 400
$p.WriteTimeout = 4000
$p.DtrEnable = $true
$p.RtsEnable = $true
$p.NewLine = "`n"
$p.Open()
Start-Sleep -Milliseconds 200
$p.DiscardInBuffer()
$p.Write("`n")
Start-Sleep -Milliseconds 200
$p.DiscardInBuffer()

$marker = "ENDCOM13_$([guid]::NewGuid().ToString('N').Substring(0, 8))"
# Run via a one-liner that prints the marker only on its own line after the command.
$wrapped = "($Cmd)" + "; printf '%s\n' $marker"
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
            foreach ($line in ($text -split "`n")) {
                if ($line.Trim() -eq $marker) {
                    $done = $true
                    break
                }
            }
            if ($done) { break }
        }
    } catch {}
    Start-Sleep -Milliseconds 60
}
$p.Close()

$text = ($buf.ToString() -replace "`r", "")
$lines = New-Object System.Collections.Generic.List[string]
$skip = $true
foreach ($line in ($text -split "`n")) {
    if ($skip) {
        # drop the echoed command line(s) until we see real output or the marker
        if ($line.Contains($wrapped.Substring(0, [Math]::Min(12, $wrapped.Length)))) {
            continue
        }
        $skip = $false
    }
    if ($line.Trim() -eq $marker) { break }
    [void]$lines.Add($line)
}
Write-Output ($lines -join "`n")
if (-not $done) {
    Write-Output "TIMEOUT waiting for $marker"
    exit 2
}
