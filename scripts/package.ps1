$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$iscc = Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe"

if (-not (Test-Path -LiteralPath $iscc)) {
    throw "Inno Setup compiler not found at $iscc"
}

& (Join-Path $PSScriptRoot "publish.ps1")
& $iscc (Join-Path $root "installer\VoiceInsert.iss")
