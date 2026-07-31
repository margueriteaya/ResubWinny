param(
    [switch]$Dependencies,
    [switch]$DownloadedRuntimes,
    [switch]$TestOutputs
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Remove-WorkspacePath([string]$RelativePath) {
    $candidate = Join-Path $root $RelativePath
    if (-not (Test-Path -LiteralPath $candidate)) {
        return
    }
    $resolved = (Resolve-Path -LiteralPath $candidate).Path
    if (-not $resolved.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove a path outside the workspace: $resolved"
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
    Write-Output "Removed $RelativePath"
}

Remove-WorkspacePath 'build'
Remove-WorkspacePath 'target'
Remove-WorkspacePath 'fuzz\target'
Remove-WorkspacePath 'studio-tauri\target'
Remove-WorkspacePath 'studio-tauri\dist'
Remove-WorkspacePath 'studio-tauri\src-tauri\target'
Remove-WorkspacePath 'studio-tauri\src-tauri\gen'

Get-ChildItem -LiteralPath $root -File -Filter '*.log' | Remove-Item -Force
Get-ChildItem -LiteralPath (Join-Path $root 'studio-tauri') -File -Filter '*.log' | Remove-Item -Force

if ($Dependencies) {
    Remove-WorkspacePath 'studio-tauri\node_modules'
}
if ($DownloadedRuntimes) {
    Remove-WorkspacePath 'third_party\libmpv\windows-x86_64\libmpv-2.dll'
    Remove-WorkspacePath 'third_party\libmpv\windows-x86_64\libmpv.dll.a'
}
if ($TestOutputs) {
    Remove-WorkspacePath 'output'
    Remove-WorkspacePath 'studio-tauri\output'
}
