# S03 physical gate for Pixel 7 (panther).
# Default mode is preflight + checklist only.
# Flash runs ONLY when BOTH are true:
#   1) -AllowFlash switch
#   2) env SAAIOS_ALLOW_FLASH=yes
# Writes only init_boot_a. Never touches vendor_boot, userdata, slot B, or --set-active.

[CmdletBinding()]
param(
    [switch]$AllowFlash,
    [switch]$RollbackToS01,
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
$expectedS03 = "9CB39C849F36EF2C317AC55A2595DD5BA7142FC6610BA0112BF4ED60D1D0C880"
$expectedS01 = "18987941EE7C41A98EA9A9471287933D75A27BF9417050253800D2E937829771"

function Assert-Hash([string]$Path, [string]$Expected) {
    if (-not (Test-Path $Path)) { throw "missing image: $Path" }
    $actual = (Get-FileHash -Algorithm SHA256 $Path).Hash.ToUpperInvariant()
    if ($actual -ne $Expected.ToUpperInvariant()) {
        throw "hash mismatch for $Path`n expected $Expected`n actual   $actual"
    }
    Write-Host "OK hash $Path"
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

function Show-Preflight {
    Write-Host "=== S03 preflight ==="
    Assert-Hash $s03 $expectedS03
    Assert-Hash $s01 $expectedS01
    if (Test-FastbootPresent) {
        Require-Fastboot
        & fastboot getvar product
        & fastboot getvar unlocked
        & fastboot getvar current-slot
    } else {
        Write-Host "WAIT fastboot: Pixel not in bootloader (or USB disconnected)"
    }
    Write-Host ""
    Write-Host "Physical checklist after flash (manual + host):"
    Write-Host "  1. splash then fullscreen 4-color Wayland frame 1080x2400x60"
    Write-Host "  2. touch four regions -> white marker + CLIENT_TOUCH"
    Write-Host "  3. ping 172.31.7.1 ; runtime TCP 38127 ; USB console"
    Write-Host "  4. kill only saai-displayd -> drm-splash + CompositorExited"
    Write-Host "  5. cold reboot repeats takeover without manual recovery"
    Write-Host "Rollback: this script -RollbackToS01 -AllowFlash with SAAIOS_ALLOW_FLASH=yes"
}

function Invoke-Flash([string]$Image, [string]$Label) {
    if (-not $AllowFlash) {
        throw "refusing flash: pass -AllowFlash"
    }
    if ($env:SAAIOS_ALLOW_FLASH -ne "yes") {
        throw "refusing flash: set SAAIOS_ALLOW_FLASH=yes"
    }
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
    Write-Host "Flash done. Reboot with: fastboot reboot"
    Write-Host "Do NOT run --set-active; do NOT flash vendor_boot/userdata/slot B."
}

Show-Preflight
if ($RollbackToS01) {
    Invoke-Flash $s01 "S01"
} elseif ($AllowFlash) {
    Invoke-Flash $s03 "S03"
} else {
    Write-Host ""
    Write-Host "No flash performed. To flash S03 after explicit permission:"
    Write-Host '  $env:SAAIOS_ALLOW_FLASH="yes"'
    Write-Host "  .\os\targets\panther\tools\s03-physical-verify.ps1 -AllowFlash"
}
