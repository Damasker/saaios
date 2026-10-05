$ErrorActionPreference = 'Stop'
$root = 'C:\Users\Admin\Projects\saaios-som\os\targets\panther\diagnostics'
$ip = [Net.IPAddress]::Parse('172.31.7.2')
$listener = [Net.Sockets.TcpListener]::new($ip, 8765)
$listener.Start()
Write-Host "tcp serve $root on 172.31.7.2:8765"
while ($true) {
  $client = $listener.AcceptTcpClient()
  $stream = $client.GetStream()
  $reader = New-Object IO.StreamReader($stream)
  $req = ''
  while (($line = $reader.ReadLine()) -ne $null) {
    if ($line -eq '') { break }
    if ($req -eq '') { $req = $line }
  }
  $rel = 'pa-probe.sh'
  if ($req -match 'GET\s+/([^\s?]+)') { $rel = $Matches[1] }
  $path = Join-Path $root $rel
  Write-Host "REQ $rel -> $path"
  if (Test-Path -LiteralPath $path -PathType Leaf) {
    $bytes = [IO.File]::ReadAllBytes($path)
    $hdr = "HTTP/1.0 200 OK`r`nContent-Length: $($bytes.Length)`r`nContent-Type: application/octet-stream`r`nConnection: close`r`n`r`n"
    $hb = [Text.Encoding]::ASCII.GetBytes($hdr)
    $stream.Write($hb, 0, $hb.Length)
    $stream.Write($bytes, 0, $bytes.Length)
  } else {
    $msg = [Text.Encoding]::ASCII.GetBytes("HTTP/1.0 404 Not Found`r`nConnection: close`r`n`r`nnot found")
    $stream.Write($msg, 0, $msg.Length)
  }
  $stream.Close(); $client.Close()
}
