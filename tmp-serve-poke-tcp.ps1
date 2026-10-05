$ErrorActionPreference = 'Stop'
$root = 'C:\Users\Admin\Projects\saaios-som\os\targets\panther\diagnostics'
$ip = [System.Net.IPAddress]::Parse('172.31.7.2')
$listener = [System.Net.Sockets.TcpListener]::new($ip, 8765)
$listener.Start()
Write-Host "tcp serving $root on 172.31.7.2:8765"
while ($true) {
  $client = $listener.AcceptTcpClient()
  $stream = $client.GetStream()
  $reader = New-Object System.IO.StreamReader($stream)
  $req = $reader.ReadLine()
  while ($true) {
    $line = $reader.ReadLine()
    if ([string]::IsNullOrEmpty($line)) { break }
  }
  Write-Host $req
  $parts = $req -split ' '
  $rel = $parts[1].TrimStart('/')
  if ([string]::IsNullOrWhiteSpace($rel)) { $rel = 'index.html' }
  $path = Join-Path $root ($rel -replace '/','\')
  if (Test-Path -LiteralPath $path -PathType Leaf) {
    $bytes = [System.IO.File]::ReadAllBytes($path)
    $hdr = "HTTP/1.0 200 OK`r`nContent-Length: $($bytes.Length)`r`nContent-Type: application/octet-stream`r`nConnection: close`r`n`r`n"
    $hb = [Text.Encoding]::ASCII.GetBytes($hdr)
    $stream.Write($hb, 0, $hb.Length)
    $stream.Write($bytes, 0, $bytes.Length)
  } else {
    $body = [Text.Encoding]::ASCII.GetBytes('not found')
    $hdr = "HTTP/1.0 404 Not Found`r`nContent-Length: $($body.Length)`r`nConnection: close`r`n`r`n"
    $hb = [Text.Encoding]::ASCII.GetBytes($hdr)
    $stream.Write($hb, 0, $hb.Length)
    $stream.Write($body, 0, $body.Length)
  }
  $stream.Flush()
  $client.Close()
}
