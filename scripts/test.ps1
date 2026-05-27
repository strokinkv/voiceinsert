$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

Push-Location $root
try {
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test
}
finally {
    Pop-Location
}
