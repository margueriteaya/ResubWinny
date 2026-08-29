param([int]$MaxTrackedFileMiB = 20)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $root
try {
    $inside = (& git rev-parse --is-inside-work-tree 2>$null).Trim()
    if ($LASTEXITCODE -ne 0 -or $inside -ne 'true') {
        throw 'Repository hygiene checks require a Git worktree.'
    }

    $tracked = @(& git ls-files)
    if ($LASTEXITCODE -ne 0 -or $tracked.Count -eq 0) {
        throw 'The Git worktree does not contain tracked files.'
    }

    $forbidden = @(
        '^(build|target|output)/',
        '^studio-tauri/(node_modules|dist|output)/',
        '^studio-tauri/src-tauri/(gen|target)/',
        '^fuzz/target/',
        '(^|/)\.env($|\.)',
        '\.(dll|exe|7z|m2ts|tlv|mmts)$'
    )
    $violations = foreach ($path in $tracked) {
        if ($forbidden.Where({ $path -match $_ }, 'First').Count) { $path }
    }
    if ($violations) {
        throw "Generated, private, or downloaded files are tracked:`n$($violations -join "`n")"
    }

    $limit = $MaxTrackedFileMiB * 1MB
    $oversized = foreach ($path in $tracked) {
        $absolutePath = Join-Path $root $path
        if (Test-Path -LiteralPath $absolutePath -PathType Leaf) {
            $item = Get-Item -LiteralPath $absolutePath
            if ($item.Length -gt $limit) {
                "$path ($([math]::Round($item.Length / 1MB, 2)) MiB)"
            }
        } elseif ($path -notmatch '(^|/)\.') {
            throw "Tracked file is missing from the checkout: $path"
        }
    }
    if ($oversized) {
        throw "Tracked files exceed the $MaxTrackedFileMiB MiB source limit:`n$($oversized -join "`n")"
    }

    $nestedGit = Get-ChildItem -LiteralPath (Join-Path $root 'third_party') `
        -Recurse -Force -Directory -Filter '.git'
    if ($nestedGit) {
        throw "Nested Git metadata is not allowed:`n$($nestedGit.FullName -join "`n")"
    }

    $npmVersion = (Get-Content -Raw studio-tauri/package.json | ConvertFrom-Json).version
    $npmLock = Get-Content -Raw studio-tauri/package-lock.json |
        ConvertFrom-Json -AsHashtable
    $desktopVersion = [regex]::Match(
        (Get-Content -Raw studio-tauri/src-tauri/Cargo.toml),
        '(?m)^version\s*=\s*"([^"]+)"'
    ).Groups[1].Value
    $workerVersion = [regex]::Match(
        (Get-Content -Raw crates/arib-caption-worker/Cargo.toml),
        '(?m)^version\s*=\s*"([^"]+)"'
    ).Groups[1].Value
    if ($npmVersion -ne $desktopVersion -or $npmVersion -ne $workerVersion) {
        throw "Project versions differ: npm=$npmVersion desktop=$desktopVersion worker=$workerVersion"
    }
    if ($npmLock['version'] -ne $npmVersion -or
        $npmLock['packages']['']['version'] -ne $npmVersion) {
        throw "npm lockfile version does not match package.json: $npmVersion"
    }
    $tauriVersion = (Get-Content -Raw studio-tauri/src-tauri/tauri.conf.json |
        ConvertFrom-Json).version
    $expectedTauriVersion = $npmVersion -replace '-alpha\.', '-'
    if ($tauriVersion -ne $expectedTauriVersion) {
        throw "Tauri/MSI version differs from the numeric prerelease mapping: expected $expectedTauriVersion, found $tauriVersion"
    }

    foreach ($required in @('LICENSE', 'THIRD_PARTY_NOTICES.md', 'SECURITY.md', 'CONTRIBUTING.md')) {
        if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
            throw "Required public repository file is missing: $required"
        }
    }

    Write-Output "Repository hygiene verified for $($tracked.Count) tracked files."
} finally {
    Pop-Location
}
