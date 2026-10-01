param(
  [Parameter(Mandatory=$true)][string]$LocalPath,
  [int]$WaitSeconds = 120,
  [string]$Port = 'COM13'
)

# Stage a file in /tmp only; installation is a separate, hash-checked action.
# Never send Ctrl-C or use this while the serial shell is running a probe.
$source = (Resolve-Path -LiteralPath $LocalPath -ErrorAction Stop).Path
$bytes = [IO.File]::ReadAllBytes($source)
$expected = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
$payload = [Convert]::ToBase64String($bytes)
$nonce = [guid]::NewGuid().ToString('N').Substring(0, 16)
$remoteBase = "/tmp/saaios-rfs-stage-$nonce"
$remoteText = "$remoteBase.b64"
$remoteBinary = "$remoteBase.bin"
$initMarker = "READY_$nonce"
$endMarker = "END_$nonce"
$portHandle = New-Object System.IO.Ports.SerialPort $Port, 115200, None, 8, one
$portHandle.ReadTimeout = 400
$portHandle.WriteTimeout = 30000
$portHandle.DtrEnable = $true
$portHandle.RtsEnable = $true
$buffer = New-Object System.Text.StringBuilder

function Wait-Marker([string]$marker, [int]$seconds) {
  $deadline = [DateTime]::UtcNow.AddSeconds($seconds)
  while ([DateTime]::UtcNow -lt $deadline) {
    $chunk = $portHandle.ReadExisting()
    if ($chunk) {
      [void]$buffer.Append($chunk)
      $lines = ($buffer.ToString() -replace "`r", '') -split "`n"
      if ($lines -contains $marker) { return $true }
    }
    Start-Sleep -Milliseconds 40
  }
  return $false
}

try {
  $portHandle.Open()
  Start-Sleep -Milliseconds 200
  $portHandle.DiscardInBuffer()
  $portHandle.Write("/saaios/busybox stty -echo`n")
  Start-Sleep -Milliseconds 150
  $portHandle.DiscardInBuffer()
  $init = "if [ ! -e '$remoteText' ] && [ ! -e '$remoteBinary' ]; then (umask 077; : > '$remoteText'); printf '%s\n' '$initMarker'; fi`n"
  $portHandle.Write($init)
  if (-not (Wait-Marker $initMarker 10)) { throw 'Remote staging refused or serial prompt unavailable' }
  for ($offset = 0; $offset -lt $payload.Length; $offset += 3000) {
    $count = [Math]::Min(3000, $payload.Length - $offset)
    $piece = $payload.Substring($offset, $count)
    $portHandle.Write("printf '%s' '$piece' >> '$remoteText'`n")
    Start-Sleep -Milliseconds 2
  }
  $finish = "if /saaios/busybox base64 -d '$remoteText' > '$remoteBinary'; then /saaios/busybox sha256sum '$remoteBinary'; /saaios/busybox wc -c '$remoteBinary'; fi; printf '%s\n' '$endMarker'`n"
  $portHandle.Write($finish)
  if (-not (Wait-Marker $endMarker $WaitSeconds)) { throw 'Remote decode did not complete' }
} finally {
  if ($portHandle.IsOpen) {
    try { $portHandle.Write("/saaios/busybox stty echo`n") } catch {}
    $portHandle.Close()
  }
}

$result = $buffer.ToString() -replace "`r", ''
$hashLine = "$expected  $remoteBinary"
$sizeLine = [regex]::Escape("$($bytes.Length) $remoteBinary")
if (-not $result.Contains($hashLine) -or $result -notmatch $sizeLine) {
  throw 'Remote staged file failed SHA-256 or size verification'
}
Write-Output "staged=$remoteBinary bytes=$($bytes.Length) sha256=$expected"
