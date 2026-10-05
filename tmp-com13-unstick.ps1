$p = New-Object System.IO.Ports.SerialPort COM13,115200,None,8,One
$p.ReadTimeout = 500
$p.WriteTimeout = 2000
$p.DtrEnable = $true
$p.RtsEnable = $true
$p.Open()
for ($i = 0; $i -lt 10; $i++) {
    $p.Write([string][char]3)
    Start-Sleep -Milliseconds 120
}
$p.Write("`n")
Start-Sleep -Milliseconds 250
$p.DiscardInBuffer()
$p.Write("echo UNSTUCK; killall -9 tmp-sitoem-ping-once 2>/dev/null; echo AFTER`n")
Start-Sleep -Seconds 3
Write-Output $p.ReadExisting()
$p.Close()
