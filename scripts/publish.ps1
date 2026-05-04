param(
    [string]$Runtime = "win-x64"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$publishDir = Join-Path $root "artifacts\publish"

$env:DOTNET_CLI_HOME = Join-Path $root ".dotnet"
$env:DOTNET_CLI_TELEMETRY_OPTOUT = "1"
$env:APPDATA = Join-Path $root ".appdata"
$env:LOCALAPPDATA = Join-Path $root ".localappdata"
$env:NUGET_PACKAGES = Join-Path $root ".nuget\packages"
$env:NUGET_HTTP_CACHE_PATH = Join-Path $root ".nuget\http-cache"

dotnet publish (Join-Path $root "src\VoiceInsert.App\VoiceInsert.App.csproj") `
    --configuration Release `
    --runtime $Runtime `
    --self-contained false `
    --output $publishDir
