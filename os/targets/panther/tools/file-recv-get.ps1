param([string]$Remote, [string]$Local)
$c = New-Object Net.Sockets.TcpClient('172.31.7.1', 7777)
$s = $c.GetStream()
$h = [Text.Encoding]::ASCII.GetBytes("GET $Remote`n")
$s.Write($h, 0, $h.Length)
$s.Flush()
$line = New-Object Collections.Generic.List[byte]
while (($b = $s.ReadByte()) -ge 0 -and $b -ne 10) { $line.Add([byte]$b) }
$hdr = [Text.Encoding]::ASCII.GetString($line.ToArray())
if ($hdr -notmatch '^OK (\d+)$') { $c.Close(); throw "GET $Remote -> $hdr" }
$size = [long]$Matches[1]
$f = [IO.File]::Create($Local)
$buf = New-Object byte[] 65536
$left = $size
while ($left -gt 0) {
    $n = $s.Read($buf, 0, [Math]::Min($buf.Length, $left))
    if ($n -le 0) { break }
    $f.Write($buf, 0, $n); $left -= $n
}
$f.Close(); $c.Close()
if ($left -ne 0) { throw "GET $Remote short by $left bytes" }
"$Remote -> $Local ($size B)"
