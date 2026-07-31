# Supported toolchains

The repository pins Rust `1.97.1` through `rust-toolchain.toml`. CI and local
release-candidate builds must use that file rather than an unqualified
`stable`. Rustfmt and Clippy from the same toolchain are part of the gate.

The desktop frontend supports Node.js 22 LTS with the committed npm lockfile.
Use `npm ci`, not an unconstrained dependency refresh, for verification and
packaging. A newer Node version may work locally but is not the release
baseline.

Windows 11 x86-64 is the Alpha package and native-preview acceptance platform.
Worker, Tauri compile, and frontend checks continue on Windows, macOS, and
Linux, but macOS/Linux native preview backends are deferred.

Toolchain upgrades are deliberate dependency changes. They require:

1. release-note and compatibility review;
2. Worker, desktop, frontend, and fuzz compile gates;
3. lockfile review without unrelated package churn;
4. Windows packaged preview and long-sample regression; and
5. updated CI, this document, and contributor instructions in one change.

No application component installs compilers, package managers, or build tools
at runtime.

