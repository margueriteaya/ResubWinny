param(
    [switch]$SkipFrontend,
    [switch]$SkipFuzz,
    [switch]$RequireLibmpvRuntime
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $root
try {
    cargo fmt --check
    if ($LASTEXITCODE -ne 0) { throw 'Worker formatting check failed.' }

    cargo fmt --manifest-path studio-tauri/src-tauri/Cargo.toml --check
    if ($LASTEXITCODE -ne 0) { throw 'Desktop formatting check failed.' }

    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { throw 'Worker tests failed.' }

    cargo clippy --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Worker lint failed.' }

    cargo test --manifest-path studio-tauri/src-tauri/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw 'Desktop tests failed.' }

    cargo clippy --manifest-path studio-tauri/src-tauri/Cargo.toml --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Desktop lint failed.' }

    if (-not $SkipFrontend) {
        if (-not (Test-Path -LiteralPath studio-tauri/node_modules)) {
            throw 'Frontend dependencies are missing. Run npm ci --prefix studio-tauri.'
        }
        npm run build --prefix studio-tauri
        if ($LASTEXITCODE -ne 0) { throw 'Frontend build failed.' }
    }

    if (-not $SkipFuzz) {
        cargo check --manifest-path fuzz/Cargo.toml
        if ($LASTEXITCODE -ne 0) { throw 'Fuzz target check failed.' }
    }

    if ($RequireLibmpvRuntime) {
        if (-not $IsWindows) {
            throw '-RequireLibmpvRuntime is supported only on Windows.'
        }
        ./scripts/check-upstreams.ps1 -RequireRuntime
    } else {
        ./scripts/check-upstreams.ps1
    }

    ./scripts/generate-license-report.ps1 -Check
    if ($LASTEXITCODE -ne 0) { throw 'Dependency license inventory check failed.' }

    Write-Output 'ResubWinny quality gate passed.'
} finally {
    Pop-Location
}
