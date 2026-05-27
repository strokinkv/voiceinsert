param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

Push-Location $root
try {
    if ($Configuration -eq "Release") {
        cargo build --release
    }
    else {
        cargo build
    }
}
finally {
    Pop-Location
}
