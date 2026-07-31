param(
    [string]$Revision = 'HEAD',
    [string]$OutputDirectory = 'build/source'
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $root
try {
    ./scripts/verify-repository.ps1

    $changes = @(& git status --porcelain=v1 --untracked-files=all)
    if ($LASTEXITCODE -ne 0) { throw 'Could not inspect the Git worktree.' }
    if ($changes) {
        throw 'Source archives must be created from a clean Git worktree.'
    }

    & git rev-parse --verify "$Revision^{commit}" 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Git revision does not exist: $Revision" }

    $version = (Get-Content -Raw studio-tauri/package.json | ConvertFrom-Json).version
    $output = [IO.Path]::GetFullPath((Join-Path $root $OutputDirectory))
    New-Item -ItemType Directory -Force -Path $output | Out-Null
    $archive = Join-Path $output "resubwinny-$version-source.zip"
    if (Test-Path -LiteralPath $archive) {
        Remove-Item -LiteralPath $archive -Force
    }

    & git archive --format=zip "--prefix=resubwinny-$version/" "--output=$archive" $Revision
    if ($LASTEXITCODE -ne 0) { throw 'git archive failed.' }

    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
    $hashPath = Join-Path $output 'SOURCE-SHA256SUMS.txt'
    [IO.File]::WriteAllText(
        $hashPath,
        "$hash  $([IO.Path]::GetFileName($archive))`n",
        [Text.UTF8Encoding]::new($false)
    )
    Write-Output "Created $archive"
    Write-Output "SHA-256 $hash"
} finally {
    Pop-Location
}
