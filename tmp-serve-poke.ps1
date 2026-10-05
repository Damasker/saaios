$ErrorActionPreference = 'Stop'
$root = 'C:\Users\Admin\Projects\saaios-som\os\targets\panther\diagnostics'
$prefix = 'http://172.31.7.2:8765/'
$listener = New-Object System.Net.HttpListener
$listener.Prefixes.Add($prefix)
$listener.Start()
Write-Host "serving $root at $prefix"
while ($listener.IsListening) {
  $ctx = $listener.GetContext()
  $rel = $ctx.Request.Url.AbsolutePath.TrimStart('/')
  if ([string]::IsNullOrWhiteSpace($rel)) { $rel = 'index.html' }
  $path = Join-Path $root $rel
  Write-Host ("{0} {1}" -f $ctx.Request.HttpMethod, $path)
  if (Test-Path -LiteralPath $path -PathType Leaf) {
    $bytes = [System.IO.File]::ReadAllBytes($path)
    $ctx.Response.StatusCode = 200
    $ctx.Response.ContentLength64 = $bytes.Length
    $ctx.Response.ContentType = 'application/octet-stream'
    $ctx.Response.OutputStream.Write($bytes, 0, $bytes.Length)
  } else {
    $msg = [Text.Encoding]::UTF8.GetBytes('not found')
    $ctx.Response.StatusCode = 404
    $ctx.Response.OutputStream.Write($msg, 0, $msg.Length)
  }
  $ctx.Response.Close()
}
