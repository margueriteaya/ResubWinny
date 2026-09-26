param(
    [Parameter(Mandatory = $true)][string]$Path,
    [string]$WixToolsDirectory
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'msi-language-tools.ps1')
Import-MsiTools $WixToolsDirectory
$package = (Resolve-Path -LiteralPath $Path).Path
$scratchRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../build/msi-languages'))
$scratch = Join-Path $scratchRoot ([guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($scratch) | Out-Null
try {
    Assert-MultilingualMsi $package $scratch
} finally {
    $resolved = [IO.Path]::GetFullPath($scratch)
    if (-not $resolved.StartsWith($scratchRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to clean an MSI verification directory outside build/msi-languages.'
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
