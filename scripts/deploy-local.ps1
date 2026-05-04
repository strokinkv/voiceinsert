$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$installer = Join-Path $root "artifacts\installer\VoiceInsertSetup.exe"
$installedExe = Join-Path $env:APPDATA "VoiceInsert\VoiceInsert.exe"

Get-Process -Name "VoiceInsert" -ErrorAction SilentlyContinue | Stop-Process -Force

if (-not (Test-Path -LiteralPath $installer)) {
    throw "Installer not found: $installer"
}

Start-Process -FilePath $installer -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -Wait

if (-not (Test-Path -LiteralPath $installedExe)) {
    throw "Installed executable not found: $installedExe"
}

Start-Process -FilePath $installedExe
