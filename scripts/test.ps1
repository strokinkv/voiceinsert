param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Release"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$env:DOTNET_CLI_HOME = Join-Path $root ".dotnet"
$env:DOTNET_CLI_TELEMETRY_OPTOUT = "1"
$env:APPDATA = Join-Path $root ".appdata"
$env:LOCALAPPDATA = Join-Path $root ".localappdata"
$env:NUGET_PACKAGES = Join-Path $root ".nuget\packages"
$env:NUGET_HTTP_CACHE_PATH = Join-Path $root ".nuget\http-cache"

dotnet test (Join-Path $root "tests\VoiceInsert.App.Tests\VoiceInsert.App.Tests.csproj") --configuration $Configuration
