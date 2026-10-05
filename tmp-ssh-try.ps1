$ErrorActionPreference = 'Continue'
Write-Host '=== projects ==='
Get-ChildItem 'C:\Users\Admin\Projects' -Directory | Select-Object -ExpandProperty Name
Write-Host '=== ssh keys ==='
Get-ChildItem 'C:\Users\Admin\.ssh' -File | Select-Object -ExpandProperty Name
$keys = @(
  'C:\Users\Admin\.ssh\laptop_key',
  'C:\Users\Admin\.ssh\id_rsa',
  'C:\Users\Admin\.ssh\id_ed25519',
  'C:\Users\Admin\.ssh\saaios-odin-win',
  'C:\Users\Admin\.ssh\ruta_cloud'
)
foreach ($k in $keys) {
  if (Test-Path $k) {
    Write-Host ("try {0}" -f $k)
    & ssh -o ConnectTimeout=4 -o StrictHostKeyChecking=no -o BatchMode=yes -i $k root@172.31.7.1 'echo OK' 2>&1 |
      ForEach-Object { Write-Host $_ }
  }
}
Write-Host '=== find com13 ==='
Get-ChildItem -Path 'C:\Users\Admin\Projects','C:\Users\Admin' -Filter 'com13.ps1' -Recurse -ErrorAction SilentlyContinue |
  Select-Object -First 10 -ExpandProperty FullName
