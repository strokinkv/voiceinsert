param(
    [switch]$SkipUninstall
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$installer = Join-Path $root "artifacts\installer\VoiceInsertSetup.exe"
$installedExe = Join-Path $env:APPDATA "VoiceInsert\VoiceInsert.exe"
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\strokinkv.VoiceInsert_is1"

if (-not (Test-Path -LiteralPath $installer)) {
    throw "Installer not found: $installer"
}

Get-Process -Name "VoiceInsert" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Process -FilePath $installer -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -Wait

if (-not (Test-Path -LiteralPath $installedExe)) {
    throw "Installed executable not found: $installedExe"
}

if (-not (Test-Path -LiteralPath $uninstallKey)) {
    throw "Uninstall registry key not found: $uninstallKey"
}

$quietUninstall = (Get-ItemProperty -LiteralPath $uninstallKey).QuietUninstallString
if ([string]::IsNullOrWhiteSpace($quietUninstall) -or $quietUninstall -notmatch "/VERYSILENT") {
    throw "QuietUninstallString is missing or not silent."
}

if (-not $SkipUninstall) {
    $uninstallExe = $quietUninstall -replace '^"([^"]+)".*$', '$1'
    Start-Process -FilePath $uninstallExe -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -Wait
}
