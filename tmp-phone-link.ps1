$ErrorActionPreference = 'Continue'
Write-Host '=== host IPs ==='
Get-NetIPAddress -AddressFamily IPv4 |
  Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' } |
  Format-Table IPAddress, InterfaceAlias -AutoSize

$targets = @('172.31.7.1','172.31.7.2','192.168.0.104')
foreach ($ip in $targets) {
  $r = Test-Connection -ComputerName $ip -Count 1 -Quiet -TimeoutSeconds 2
  Write-Host ("ping {0} : {1}" -f $ip, $r)
}

Write-Host '=== COM ports ==='
try {
  Get-CimInstance Win32_SerialPort | Select-Object DeviceID, Name | Format-Table -AutoSize
} catch { Write-Host $_.Exception.Message }
[System.IO.Ports.SerialPort]::GetPortNames() | ForEach-Object { Write-Host ("port {0}" -f $_) }

Write-Host '=== adb ==='
adb start-server 2>&1 | Out-Host
adb devices -l 2>&1 | Out-Host

Write-Host '=== USB phone-ish ==='
Get-PnpDevice -Status OK -ErrorAction SilentlyContinue |
  Where-Object { $_.FriendlyName -match 'Pixel|Android|ADB|RNDIS|Gadget|NCM|ECM|CDC|Saai|Serial|USB Serial|tty' } |
  Select-Object FriendlyName, InstanceId |
  Format-Table -AutoSize

Write-Host '=== TCP 172.31.7.1 ports ==='
foreach ($port in @(22, 8765, 38127, 38128)) {
  try {
    $tcp = New-Object System.Net.Sockets.TcpClient
    $iar = $tcp.BeginConnect('172.31.7.1', $port, $null, $null)
    $ok = $iar.AsyncWaitHandle.WaitOne(1500, $false)
    if ($ok -and $tcp.Connected) { Write-Host ("{0}:{1} OPEN" -f '172.31.7.1', $port) }
    else { Write-Host ("{0}:{1} closed/timeout" -f '172.31.7.1', $port) }
    $tcp.Close()
  } catch {
    Write-Host ("{0}:{1} err {2}" -f '172.31.7.1', $port, $_.Exception.Message)
  }
}
