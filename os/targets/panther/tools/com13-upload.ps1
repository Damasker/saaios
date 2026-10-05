param(
    [Parameter(Mandatory = $true)][string]$LocalPath,
    [Parameter(Mandatory = $true)][string]$RemotePath,
    [string]$Mode = "755",
    [string]$Port = "COM13",
    [int]$ChunkSize = 1200,
    [int]$IdleTimeoutSec = 90
)

$ErrorActionPreference = "Stop"
$bytes = [IO.File]::ReadAllBytes($LocalPath)
$b64 = [Convert]::ToBase64String($bytes)
$tmp = "/tmp/saaios-up-$([guid]::NewGuid().ToString('N').Substring(0, 8)).b64"
$marker = "ENDUP_$([guid]::NewGuid().ToString('N').Substring(0, 8))"

$p = New-Object System.IO.Ports.SerialPort $Port, 115200, None, 8, one
$p.ReadTimeout = 400
$p.WriteTimeout = 8000
$p.DtrEnable = $true
$p.RtsEnable = $true
$p.NewLine = "`n"
$p.Open()
Start-Sleep -Milliseconds 200
$p.DiscardInBuffer()
$p.Write("`n")
Start-Sleep -Milliseconds 150
$p.DiscardInBuffer()

function Send-Line([string]$line) {
    $p.Write($line + "`n")
}

function Wait-Marker([string]$m, [int]$sec) {
    $buf = New-Object System.Text.StringBuilder
    $deadline = [DateTime]::UtcNow.AddSeconds($sec)
    while ([DateTime]::UtcNow -lt $deadline) {
        try {
            $chunk = $p.ReadExisting()
            if ($chunk) {
                [void]$buf.Append($chunk)
                if ($buf.ToString() -match [regex]::Escape($m)) {
                    return ($buf.ToString() -replace "`r", "")
                }
            }
        } catch {}
        Start-Sleep -Milliseconds 40
    }
    throw "TIMEOUT waiting for $m; got=$($buf.ToString().Substring([Math]::Max(0, $buf.Length - 200)))"
}

Write-Host "upload $LocalPath ($($bytes.Length) bytes) -> $RemotePath via $tmp"
Send-Line "rm -f $tmp; : > $tmp; printf '%s\n' ${marker}_prep"
Wait-Marker "${marker}_prep" 30 | Out-Null

$n = 0
for ($i = 0; $i -lt $b64.Length; $i += $ChunkSize) {
    $len = [Math]::Min($ChunkSize, $b64.Length - $i)
    $part = $b64.Substring($i, $len)
    Send-Line "printf '%s' '$part' >> $tmp"
    $n++
    if (($n % 25) -eq 0) {
        $ack = "${marker}_a$n"
        Send-Line "printf '%s\n' $ack"
        Wait-Marker $ack 40 | Out-Null
        Write-Host ("  {0}/{1}" -f $i, $b64.Length)
    }
    Start-Sleep -Milliseconds 15
}

$fin = "${marker}_fin"
Send-Line "base64 -d $tmp > $RemotePath.tmp && mv $RemotePath.tmp $RemotePath && chmod $Mode $RemotePath && rm -f $tmp && wc -c $RemotePath && ls -la $RemotePath && printf '%s\n' $fin"
$out = Wait-Marker $fin $IdleTimeoutSec
$p.Close()
Write-Host $out
if ($out -notmatch [regex]::Escape($fin)) { exit 2 }
