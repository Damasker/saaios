param(
  [Parameter(Mandatory=$true)][string]$Cmd,
  [int]$TimeoutMs = 20000,
  [string]$Port = "COM13",
  [int]$Baud = 115200
)
$ErrorActionPreference = "Stop"
$sp = New-Object System.IO.Ports.SerialPort $Port,$Baud,'None',8,'One'
$sp.ReadTimeout = 1000
$sp.WriteTimeout = 2000
$sp.NewLine = "`n"
$sp.Open()
try {
  # Reset any partial/continuation input, then disable terminal echo so typed
  # command lines (which contain the marker strings) don't trip detection.
  $sp.Write([string][char]3)      # Ctrl-C
  Start-Sleep -Milliseconds 200
  $sp.Write("`n")
  $sp.Write("stty -echo 2>/dev/null`n")
  Start-Sleep -Milliseconds 300
  try { $sp.DiscardInBuffer() } catch {}
  $m = "MARK" + ([guid]::NewGuid().ToString("N").Substring(0,8))
  $start = "__S_${m}__"
  $end = "__E_${m}__"
  # flush any pending input
  try { $sp.DiscardInBuffer() } catch {}
  $sp.Write("echo $start`n")
  $sp.Write("$Cmd`n")
  $sp.Write("echo $end`:`$?`n")
  $sb = New-Object System.Text.StringBuilder
  $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
  $seenStart = $false
  while ([DateTime]::UtcNow -lt $deadline) {
    try {
      $line = $sp.ReadLine()
    } catch [TimeoutException] {
      continue
    }
    $line = $line.TrimEnd("`r")
    if ($line -like "*$start*") { $seenStart = $true; continue }
    if ($line -like "*$end*") { break }
    if ($seenStart) { [void]$sb.AppendLine($line) }
  }
  Write-Output $sb.ToString()
} finally {
  $sp.Close()
}
