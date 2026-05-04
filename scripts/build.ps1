param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$env:DOTNET_CLI_HOME = Join-Path $root ".dotnet"
$env:DOTNET_CLI_TELEMETRY_OPTOUT = "1"
$env:APPDATA = Join-Path $root ".appdata"
$env:LOCALAPPDATA = Join-Path $root ".localappdata"
$env:NUGET_PACKAGES = Join-Path $root ".nuget\packages"
$env:NUGET_HTTP_CACHE_PATH = Join-Path $root ".nuget\http-cache"

dotnet build (Join-Path $root "src\VoiceInsert.App\VoiceInsert.App.csproj") --configuration $Configuration
dotnet build (Join-Path $root "tests\VoiceInsert.App.Tests\VoiceInsert.App.Tests.csproj") --configuration $Configuration
