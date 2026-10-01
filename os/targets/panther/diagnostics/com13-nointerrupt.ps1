param(
  [Parameter(Mandatory=$true)][string]$Cmd,
  [int]$WaitSeconds = 20,
  [string]$Port = 'COM13'
)

# Do not send Ctrl-C: that can interrupt the CP firmware transfer. Invoke
# only at an idle shell prompt, never while a boot/probe command is running.
$nonce = [guid]::NewGuid().ToString('N').Substring(0, 12)
$begin = "BEGIN_$nonce"
$end = "END_$nonce"
$remoteScript = "printf '%s\n' '$begin'; $Cmd; printf '%s\n' '$end'"
$encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($remoteScript))
$wireCommand = "printf '%s' '$encoded' | /saaios/busybox base64 -d | /bin/sh`n"
# Keep the entire input line well below the console's canonical line limit.
$wireBytes = [Text.Encoding]::ASCII.GetByteCount($wireCommand)
if ($wireBytes -gt 1024) {
  throw "Serial command is $wireBytes bytes (limit 1024); split it into shorter commands"
}
$portHandle = New-Object System.IO.Ports.SerialPort $Port, 115200, None, 8, one
$portHandle.ReadTimeout = 400
$portHandle.WriteTimeout = 4000
$portHandle.DtrEnable = $true
$portHandle.RtsEnable = $true
$buffer = New-Object System.Text.StringBuilder
$found = $false
try {
  $portHandle.Open()
  Start-Sleep -Milliseconds 200
  $portHandle.DiscardInBuffer()
  $portHandle.Write($wireCommand)
  $deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
  while ([DateTime]::UtcNow -lt $deadline) {
    $chunk = $portHandle.ReadExisting()
    if ($chunk) {
      [void]$buffer.Append($chunk)
      $lines = ($buffer.ToString() -replace "`r", '') -split "`n"
      if ($lines -contains $end) { $found = $true; break }
    }
    Start-Sleep -Milliseconds 80
  }
} finally {
  if ($portHandle.IsOpen) { $portHandle.Close() }
}
$all = ($buffer.ToString() -replace "`r", '') -split "`n"
$started = $false
foreach ($line in $all) {
  if ($line -eq $begin) { $started = $true; continue }
  if ($line -eq $end) { break }
  if ($started) { Write-Output $line }
}
if (-not $found) { throw "Serial command timed out without completion marker (BEGIN seen: $started)" }
