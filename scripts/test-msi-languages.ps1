param([Parameter(Mandatory = $true)][string]$Path)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'msi-language-tools.ps1')
Import-MsiTools ''
$package = (Resolve-Path -LiteralPath $Path).Path
$scratchRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../build/msi-languages'))
$scratch = Join-Path $scratchRoot ([guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($scratch) | Out-Null
try {
    Assert-MultilingualMsi $package $scratch
    foreach ($case in @('missing-language', 'per-user')) {
        $copy = Join-Path $scratch "$case.msi"
        Copy-Item -LiteralPath $package -Destination $copy
        $database = Open-MsiDatabase $copy -Write
        try {
            if ($case -eq 'missing-language') {
                $database.Execute("DELETE FROM ``_Storages`` WHERE ``Name`` = 'zh-CN.mst'")
                $expected = 'missing embedded transform'
            } else {
                $database.Execute("UPDATE ``Property`` SET ``Value`` = '2' WHERE ``Property`` = 'ALLUSERS'")
                $expected = 'per-machine MSI'
            }
            $database.Commit()
        } finally { $database.Dispose() }
        $rejected = $false
        try { Assert-MultilingualMsi $copy $scratch } catch {
            if (-not $_.Exception.Message.Contains($expected)) { throw }
            $rejected = $true
        }
        if (-not $rejected) { throw "Accepted invalid MSI: $case" }
        Write-Output "MSI rejection verified: $case."
    }
} finally {
    $resolved = [IO.Path]::GetFullPath($scratch)
    if (-not $resolved.StartsWith($scratchRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to clean an MSI test directory outside build/msi-languages.'
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
