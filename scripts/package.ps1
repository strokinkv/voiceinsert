$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$candidatePaths = @(
    (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe"),
    (Join-Path $env:ProgramFiles "Inno Setup 6\ISCC.exe"),
    (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe")
) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }

$iscc = $candidatePaths | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $iscc) {
    $command = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
    if ($command) {
        $iscc = $command.Source
    }
}

if (-not $iscc) {
    throw "Inno Setup compiler not found. Install Inno Setup 6 or make ISCC.exe available in PATH."
}

$cargoToml = Get-Content -LiteralPath (Join-Path $root "Cargo.toml")
$versionLine = $cargoToml | Where-Object { $_ -match '^version\s*=\s*"([^"]+)"' } | Select-Object -First 1
if (-not $versionLine) {
    throw "Version not found in Cargo.toml."
}

$appVersion = [regex]::Match($versionLine, '^version\s*=\s*"([^"]+)"').Groups[1].Value

& (Join-Path $PSScriptRoot "publish.ps1")
& $iscc "/DAppVersion=$appVersion" (Join-Path $root "installer\VoiceInsert.iss")
