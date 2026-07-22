$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$publishDir = Join-Path $root "artifacts\publish"
$releaseExe = Join-Path $root "target\release\VoiceInsert.exe"

Push-Location $root
try {
    cargo build --release
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $releaseExe)) {
    throw "Release executable not found: $releaseExe"
}

if (Test-Path -LiteralPath $publishDir) {
    Remove-Item -LiteralPath $publishDir -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $publishDir | Out-Null
Copy-Item -LiteralPath $releaseExe -Destination (Join-Path $publishDir "VoiceInsert.exe") -Force
Copy-Item -LiteralPath (Join-Path $root "assets\VoiceInsert.ico") -Destination $publishDir -Force
Copy-Item -LiteralPath (Join-Path $root "assets\VoiceInsert.png") -Destination $publishDir -Force
Copy-Item -LiteralPath (Join-Path $root "assets\record-start.wav") -Destination $publishDir -Force
Copy-Item -LiteralPath (Join-Path $root "assets\record-stop.wav") -Destination $publishDir -Force
Copy-Item -LiteralPath (Join-Path $root "assets\record-error.wav") -Destination $publishDir -Force
