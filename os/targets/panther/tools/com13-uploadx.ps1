param(
    [Parameter(Mandatory = $true)][string]$LocalPath,
    [Parameter(Mandatory = $true)][string]$RemotePath,
    [string]$Mode = "755",
    [string]$Port = "COM13",
    [int]$ChunkSize = 400
)

$ErrorActionPreference = "Stop"
$bytes = [IO.File]::ReadAllBytes($LocalPath)
$b64 = [Convert]::ToBase64String($bytes)
$expected = $bytes.Length
$tmp = "/tmp/saaios-up.b64"

$p = New-Object System.IO.Ports.SerialPort $Port, 115200, None, 8, one
$p.ReadTimeout = 400
$p.WriteTimeout = 8000
$p.DtrEnable = $true
$p.RtsEnable = $true
$p.Open()
Start-Sleep -Milliseconds 120
$p.Write([byte[]]@(3), 0, 1)
Start-Sleep -Milliseconds 80
$p.Write("`n")
Start-Sleep -Milliseconds 120
$p.DiscardInBuffer()

function Drain([int]$ms = 80) {
    Start-Sleep -Milliseconds $ms
    try { [void]$p.ReadExisting() } catch {}
}

function Wait-Tok([string]$tok, [int]$sec) {
    $buf = New-Object System.Text.StringBuilder
    $deadline = [DateTime]::UtcNow.AddSeconds($sec)
    while ([DateTime]::UtcNow -lt $deadline) {
        try {
            $c = $p.ReadExisting()
            if ($c) {
                [void]$buf.Append($c)
                $clean = [regex]::Replace($buf.ToString(), "\x1b\[[0-9;?]*[A-Za-z]", "")
                if ($clean -match [regex]::Escape($tok)) { return $clean }
            }
        } catch {}
        Start-Sleep -Milliseconds 25
    }
    throw "TIMEOUT $tok"
}

Write-Host "upload $($bytes.Length) b64=$($b64.Length) -> $RemotePath"
$p.Write("rm -f $tmp; : > $tmp; echo PREP_OK`n")
Wait-Tok "PREP_OK" 20 | Out-Null
Drain 50

$n = 0
for ($i = 0; $i -lt $b64.Length; $i += $ChunkSize) {
    $len = [Math]::Min($ChunkSize, $b64.Length - $i)
    $part = $b64.Substring($i, $len)
    $n++
    # Two-step: write chunk, then short ack (avoids huge line+marker races)
    $p.Write("printf '%s' '$part' >> $tmp`n")
    Drain 40
    if (($n % 15) -eq 0 -or ($i + $len) -ge $b64.Length) {
        $tok = "ACK$n"
        $p.Write("wc -c $tmp; echo $tok`n")
        Wait-Tok $tok 30 | Out-Null
        Write-Host "  $i/$($b64.Length)"
    }
}

$p.Write("base64 -d $tmp > $RemotePath.tmp && mv $RemotePath.tmp $RemotePath && chmod $Mode $RemotePath && rm -f $tmp && wc -c $RemotePath && echo DONE_UP`n")
$out = Wait-Tok "DONE_UP" 45
$p.Close()
Write-Host $out
if ($out -notmatch "$expected") {
    Write-Error "expected size $expected not seen"
    exit 2
}
Write-Host "OK"
