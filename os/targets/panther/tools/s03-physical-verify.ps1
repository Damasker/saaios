# S03 physical gate for Pixel 7 (panther).
# Default mode is preflight + checklist only.
# Flash / bootloader entry run ONLY when BOTH are true:
#   1) matching switch (-AllowFlash / -EnterBootloader / -RollbackToS01)
#   2) env SAAIOS_ALLOW_FLASH=yes
# Writes only init_boot_a. Never touches vendor_boot, userdata, slot B, or --set-active.

[CmdletBinding()]
param(
    [switch]$AllowFlash,
    [switch]$RollbackToS01,
    [switch]$EnterBootloader,
    [switch]$VerifyLive,
    [string]$SerialPort = "COM13",
    [string]$RepoRoot = ""
)

$ErrorActionPreference = "Stop"

if (-not $RepoRoot) {
    $here = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
    $RepoRoot = (Resolve-Path (Join-Path $here "..\..\..\..")).Path
}

$dist = Join-Path $RepoRoot "dist\panther"
$s03 = Join-Path $dist "saaios-panther-s03-init_boot.img"
$s01 = Join-Path $dist "saaios-panther-s01-init_boot.img"
$expectedS03 = "747228C90A6031BE7DCF62C0B081388DEAAA62784A8E86CAE45E232095077F55"
$expectedS01 = "18987941EE7C41A98EA9A9471287933D75A27BF9417050253800D2E937829771"

function Assert-Hash([string]$Path, [string]$Expected) {
    if (-not (Test-Path $Path)) { throw "missing image: $Path" }
    $actual = (Get-FileHash -Algorithm SHA256 $Path).Hash.ToUpperInvariant()
    if ($actual -ne $Expected.ToUpperInvariant()) {
        throw "hash mismatch for $Path`n expected $Expected`n actual   $actual"
    }
    Write-Host "OK hash $Path"
}

function Assert-FlashGate {
    if ($env:SAAIOS_ALLOW_FLASH -ne "yes") {
        throw "refusing destructive action: set SAAIOS_ALLOW_FLASH=yes"
    }
}

function Test-FastbootPresent {
    $devs = & fastboot devices 2>&1
    return [bool]($devs | Where-Object { $_ -match "\s+fastboot$" })
}

function Require-Fastboot {
    if (-not (Test-FastbootPresent)) {
        throw "no fastboot device; reboot Pixel 7 to bootloader and reconnect USB"
    }
    Write-Host "OK fastboot device present"
}

function Invoke-Serial([string[]]$Commands, [int]$WaitSeconds = 3) {
    $port = New-Object System.IO.Ports.SerialPort $SerialPort, 115200, None, 8, One
    $port.ReadTimeout = 2000
    $port.WriteTimeout = 1500
    $port.NewLine = "`r`n"
    $port.Open()
    try {
        $port.DiscardInBuffer()
        foreach ($c in $Commands) { $port.WriteLine($c) }
        Start-Sleep -Seconds $WaitSeconds
        $sb = New-Object System.Text.StringBuilder
        $deadline = (Get-Date).AddSeconds(2)
        while ((Get-Date) -lt $deadline) {
            while ($port.BytesToRead -gt 0) {
                [void]$sb.Append([char]$port.ReadByte())
            }
            Start-Sleep -Milliseconds 50
        }
        return $sb.ToString()
    } finally {
        $port.Close()
    }
}

function Show-Preflight {
    Write-Host "=== S03 preflight ==="
    Assert-Hash $s03 $expectedS03
    Assert-Hash $s01 $expectedS01
    $ping = Test-Connection -ComputerName 172.31.7.1 -Count 1 -Quiet -ErrorAction SilentlyContinue
    if ($ping) { Write-Host "OK USB NCM 172.31.7.1" } else { Write-Host "WAIT USB NCM 172.31.7.1" }
    if (Test-FastbootPresent) {
        Require-Fastboot
        & fastboot getvar product
        & fastboot getvar unlocked
        & fastboot getvar current-slot
    } else {
        Write-Host "WAIT fastboot: Pixel not in bootloader"
    }
    Write-Host ""
    Write-Host "After explicit permission:"
    Write-Host '  $env:SAAIOS_ALLOW_FLASH="yes"'
    Write-Host "  # from live SaaiOS shell path:"
    Write-Host "  .\os\targets\panther\tools\s03-physical-verify.ps1 -EnterBootloader"
    Write-Host "  .\os\targets\panther\tools\s03-physical-verify.ps1 -AllowFlash"
    Write-Host "  fastboot reboot"
    Write-Host "  .\os\targets\panther\tools\s03-physical-verify.ps1 -VerifyLive"
    Write-Host "Rollback: -RollbackToS01 -AllowFlash"
}

function Invoke-VerifyLive {
    Write-Host "=== S03 live verify via $SerialPort ==="
    if (-not (Test-Connection -ComputerName 172.31.7.1 -Count 1 -Quiet)) {
        throw "172.31.7.1 unreachable; USB NCM required"
    }
    $text = Invoke-Serial @(
        "",
        "echo S03_VERIFY_BEGIN",
        "test -x /saaios/saai-display-supervisor && echo __S03_SUPERVISOR_YES__",
        "test -x /saaios/saai-demo-surface && echo __S03_DEMO_YES__",
        "test -x /saaios/saai-displayd && echo __S03_DISPLAYD_YES__",
        "test -x /saaios/drm-splash && echo __S03_SPLASH_YES__",
        "test -e /run/saai-displayd.ready && echo __S03_READY_YES__",
        "test -e /run/saai-displayd.heartbeat && echo __S03_HEARTBEAT_YES__",
        "ps",
        "tail -30 /run/saai-display-supervisor.log 2>/dev/null || true",
        "cat /run/saai-display-fallback.reason 2>/dev/null || true",
        "echo S03_VERIFY_END"
    ) 4
    Write-Host $text
    $failed = $false
    foreach ($mark in @(
        "__S03_SUPERVISOR_YES__",
        "__S03_DEMO_YES__",
        "__S03_DISPLAYD_YES__",
        "__S03_SPLASH_YES__"
    )) {
        if ($text -match "(?m)^$([regex]::Escape($mark))\r?$") {
            Write-Host "OK $mark"
        } else {
            Write-Host "FAIL missing $mark"
            $failed = $true
        }
    }
    if ($text -match "(?m)^__S03_READY_YES__\r?$") {
        Write-Host "OK __S03_READY_YES__"
    } else {
        Write-Host "WAIT ready marker absent"
    }
    if ($failed) {
        throw "live verify failed: not S03 candidate image"
    }
    Write-Host "OK S03 packaging markers present on live device"
}

function Invoke-EnterBootloader {
    Assert-FlashGate
    if (-not $EnterBootloader) { throw "refusing bootloader: pass -EnterBootloader" }
    Write-Host "Rebooting live SaaiOS into bootloader via /saaios/reboot-bootloader ..."
    [void](Invoke-Serial @("", "/saaios/reboot-bootloader") 1)
    Write-Host "Waiting for fastboot..."
    for ($i = 0; $i -lt 30; $i++) {
        Start-Sleep -Seconds 1
        if (Test-FastbootPresent) {
            Require-Fastboot
            return
        }
    }
    throw "timed out waiting for fastboot after reboot-bootloader"
}

function Invoke-Flash([string]$Image, [string]$Label) {
    if (-not $AllowFlash -and -not $RollbackToS01) {
        throw "refusing flash: pass -AllowFlash or -RollbackToS01"
    }
    Assert-FlashGate
    Require-Fastboot
    Assert-Hash $Image $(if ($Label -eq "S01") { $expectedS01 } else { $expectedS03 })
    $product = (& fastboot getvar product 2>&1 | Out-String)
    if ($product -notmatch "panther") { throw "refusing flash: product is not panther`n$product" }
    $unlocked = (& fastboot getvar unlocked 2>&1 | Out-String)
    if ($unlocked -notmatch "yes") { throw "refusing flash: device not unlocked`n$unlocked" }
    $slot = (& fastboot getvar current-slot 2>&1 | Out-String)
    if ($slot -notmatch "current-slot:\s*a\b") {
        throw "refusing flash: current-slot is not a`n$slot"
    }
    Write-Host "Flashing $Label ONLY to init_boot_a ..."
    & fastboot flash init_boot_a $Image
    if ($LASTEXITCODE -ne 0) { throw "fastboot flash init_boot_a failed" }
    Write-Host "Flash done. Next: fastboot reboot ; then -VerifyLive"
    Write-Host "Do NOT run --set-active; do NOT flash vendor_boot/userdata/slot B."
}

Show-Preflight
if ($VerifyLive) {
    Invoke-VerifyLive
} elseif ($EnterBootloader) {
    Invoke-EnterBootloader
} elseif ($RollbackToS01) {
    Invoke-Flash $s01 "S01"
} elseif ($AllowFlash) {
    Invoke-Flash $s03 "S03"
} else {
    Write-Host ""
    Write-Host "No destructive action performed."
}
