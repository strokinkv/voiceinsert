$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$candidatePaths = @(
    (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe"),
    (Join-Path $env:ProgramFiles "Inno Setup 6\ISCC.exe"),
    (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe")
) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }

$iscc = $candidatePaths | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $iscc)
{
    $command = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
    if ($command)
    {
        $iscc = $command.Source
    }
}

if (-not $iscc)
{
    throw "Inno Setup compiler not found. Install Inno Setup 6 or make ISCC.exe available in PATH."
}

[xml]$buildProps = Get-Content -LiteralPath (Join-Path $root "Directory.Build.props")
$appVersion = $buildProps.Project.PropertyGroup.Version
if ([string]::IsNullOrWhiteSpace($appVersion))
{
    throw "Version not found in Directory.Build.props."
}

& (Join-Path $PSScriptRoot "publish.ps1")
& $iscc "/DAppVersion=$appVersion" (Join-Path $root "installer\VoiceInsert.iss")
