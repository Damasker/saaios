param(
  [string]$Path = 'C:\Users\Admin\Projects\saaios-som\os\targets\panther\diagnostics\stock-missing-gets',
  [string]$Bind = '172.31.7.2',
  [int]$Port = 8765
)
$bytes = [IO.File]::ReadAllBytes($Path)
$l = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Parse($Bind), $Port)
$l.Start()
[Console]::Error.WriteLine("READY size=$($bytes.Length)")
$client = $l.AcceptTcpClient()
$ns = $client.GetStream()
$ns.Write($bytes, 0, $bytes.Length)
$ns.Flush()
Start-Sleep -Milliseconds 400
$client.Close()
$l.Stop()
[Console]::Error.WriteLine('DONE')
