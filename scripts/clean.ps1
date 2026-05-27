$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$paths = @(
    "artifacts",
    "target",
    ".appdata",
    ".localappdata",
    ".tmp"
)

foreach ($path in $paths) {
    $fullPath = Join-Path $root $path
    if (Test-Path -LiteralPath $fullPath) {
        $resolvedRoot = (Resolve-Path -LiteralPath $root).Path.TrimEnd("\")
        $resolvedPath = (Resolve-Path -LiteralPath $fullPath).Path
        if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to delete path outside repository: $resolvedPath"
        }

        Remove-Item -LiteralPath $fullPath -Recurse -Force
    }
}
