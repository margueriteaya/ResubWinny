param(
    [ValidateSet('Bundle', 'Executable')]
    [string]$Target = 'Bundle',

    [ValidateSet('Bundled', 'External')]
    [string]$Libmpv = 'Bundled',

    [switch]$Check,
    [switch]$SkipNpmInstall
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Invoke-Checked([string]$Label, [scriptblock]$Command) {
    Write-Host "`n==> $Label"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE."
    }
}

Push-Location $root
try {
    if (-not $env:RESUBWINNY_RELEASE_TIER) {
        $env:RESUBWINNY_RELEASE_TIER = 'Development'
    }
    if (-not $env:RESUBWINNY_BUILD_COMMIT) {
        $env:RESUBWINNY_BUILD_COMMIT = (& git rev-parse --verify HEAD).Trim()
    }
    if (-not $SkipNpmInstall) {
        Invoke-Checked 'Install pinned frontend dependencies' {
            npm ci --prefix studio-tauri
        }
    } elseif (-not (Test-Path -LiteralPath 'studio-tauri\node_modules' -PathType Container)) {
        throw 'Frontend dependencies are missing. Remove -SkipNpmInstall or run npm ci --prefix studio-tauri.'
    }

    $tauriArguments = @('run', 'tauri', '--prefix', 'studio-tauri', '--', 'build')
    if ($Target -eq 'Executable') {
        $tauriArguments += '--no-bundle'
    }

    # Tauri copies bundled resources beside the executable. Remove only the
    # previous staged runtime so switching profiles cannot silently retain it.
    $stagedLibmpv = Join-Path $root 'build\cargo\release\libmpv-2.dll'
    if (Test-Path -LiteralPath $stagedLibmpv -PathType Leaf) {
        Remove-Item -LiteralPath $stagedLibmpv -Force
    }

    if ($Libmpv -eq 'Bundled') {
        if (-not $IsWindows) {
            throw 'The bundled libmpv profile is currently available only on Windows. Use -Libmpv External on this platform.'
        }

        Write-Host "`n==> Install and verify the pinned Windows libmpv runtime"
        ./scripts/setup-libmpv.ps1
        $tauriArguments += @('--config', 'src-tauri/tauri.windows-libmpv.conf.json')
    }

    if ($Check) {
        Write-Host "`n==> Run the quality gate"
        if ($Libmpv -eq 'Bundled') {
            ./scripts/check.ps1 -SkipFrontend -RequireLibmpvRuntime
        } else {
            ./scripts/check.ps1 -SkipFrontend
        }
    }

    # WiX derives the installed filename from each source basename and ignores
    # Tauri's resource destination alias. Stage licenses with unique basenames
    # so multiple upstream LICENSE files cannot collide in the MSI.
    $bundleLicenseDirectory = Join-Path $root 'build\bundle-resources\licenses'
    [IO.Directory]::CreateDirectory($bundleLicenseDirectory) | Out-Null
    $bundleLicenses = [ordered]@{
        'third_party\libaribcaption\LICENSE' = 'libaribcaption-MIT.txt'
        'third_party\libaribtlv\LICENSE' = 'libaribtlv-MIT.txt'
        'third_party\zlib\LICENSE' = 'zlib-License.txt'
    }
    foreach ($license in $bundleLicenses.GetEnumerator()) {
        Copy-Item -LiteralPath (Join-Path $root $license.Key) `
            -Destination (Join-Path $bundleLicenseDirectory $license.Value) `
            -Force
    }

    $profile = if ($Libmpv -eq 'Bundled') { 'bundled libmpv' } else { 'external libmpv' }
    Invoke-Checked "Build ResubWinny ($Target, $profile)" {
        npm @tauriArguments
    }

    Write-Host "`nResubWinny build completed."
    Write-Host 'Executable: build/cargo/release/resubwinny-studio.exe'
    if ($Target -eq 'Bundle') {
        Write-Host 'Installers: build/cargo/release/bundle/'
    }
    if ($Libmpv -eq 'Bundled') {
        Write-Warning 'This build contains the pinned development libmpv DLL. Do not publish it until the matching corresponding-source archive and receipt have passed the public-release gate.'
    } else {
        Write-Host 'This build does not distribute libmpv. Preview requires a compatible runtime supplied through RESUBWINNY_LIBMPV.'
    }
} finally {
    Pop-Location
}
