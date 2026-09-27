param(
    [string]$MsiDirectory = 'build/cargo/release/bundle/msi',
    [string]$WixToolsDirectory
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'msi-language-tools.ps1')
Import-MsiTools $WixToolsDirectory
$directory = (Resolve-Path -LiteralPath $MsiDirectory).Path
$english = @(Get-ChildItem -LiteralPath $directory -File -Filter '*_en-US.msi')
if ($english.Count -ne 1) { throw 'Expected exactly one English MSI from the current build.' }
$stem = $english[0].BaseName -replace '_en-US$', ''
$output = Join-Path $directory "$stem.msi"
$languages = [ordered]@{ 'zh-CN' = '2052'; 'zh-TW' = '1028'; 'ja-JP' = '1041' }
$inputs = @($english[0].FullName)
foreach ($locale in $languages.Keys) {
    $inputPath = Join-Path $directory "${stem}_${locale}.msi"
    if (-not (Test-Path -LiteralPath $inputPath -PathType Leaf)) { throw "Missing localized MSI: $inputPath" }
    $inputs += $inputPath
}
# Keep all intermediate databases outside the installer discovery tree.
$scratchRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../build/msi-languages'))
$scratch = Join-Path $scratchRoot ([guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($scratch) | Out-Null
try {
    $candidate = Join-Path $scratch "$stem.msi"
    Copy-Item -LiteralPath $english[0].FullName -Destination $candidate
    $base = Open-MsiDatabase $candidate -Write
    try {
        foreach ($entry in $languages.GetEnumerator()) {
            $locale = $entry.Key
            $localizedPath = Join-Path $scratch "$locale.msi"
            Copy-Item -LiteralPath (Join-Path $directory "${stem}_${locale}.msi") -Destination $localizedPath
            $localized = Open-MsiDatabase $localizedPath -Write
            try {
                if ($localized.ExecutePropertyQuery('ProductLanguage') -ne $entry.Value) { throw "Wrong language in $locale MSI." }
                # Tauri generates a fresh ProductCode for each language. These
                # are UI variants of one product, not separately installed apps.
                $identity = [Microsoft.Deployment.WindowsInstaller.Record]::new(2)
                try {
                    $identity.SetString(1, $base.ExecutePropertyQuery('ProductCode'))
                    $identity.SetString(2, 'ProductCode')
                    $localized.Execute('UPDATE `Property` SET `Value` = ? WHERE `Property` = ?', $identity)
                } finally { $identity.Dispose() }
                $localized.Commit()
                Assert-MsiPayload $base $localized
                $transform = Join-Path $scratch "$locale.mst"
                if (-not $localized.GenerateTransform($base, $transform)) { throw "No language changes for $locale." }
                $localized.CreateTransformSummaryInfo($base, $transform,
                    [Microsoft.Deployment.WindowsInstaller.TransformErrors]::ChangeCodePage,
                    [Microsoft.Deployment.WindowsInstaller.TransformValidations]'Product,NewEqualBaseVersion,UpgradeCode')
            } finally { $localized.Dispose() }
            $view = $base.OpenView('SELECT `Name`, `Data` FROM `_Storages`')
            $record = [Microsoft.Deployment.WindowsInstaller.Record]::new(2)
            try {
                $view.Execute()
                $record.SetString(1, "$locale.mst")
                $record.SetStream(2, $transform)
                $view.Modify([Microsoft.Deployment.WindowsInstaller.ViewModifyMode]::Insert, $record)
            } finally { $record.Dispose(); $view.Dispose() }
        }
        $base.Commit()
    } finally { $base.Dispose() }
    Assert-MultilingualMsi $candidate $scratch
    # Publish only after extracting and applying every embedded transform.
    Copy-Item -LiteralPath $candidate -Destination $output -Force
    foreach ($inputPath in $inputs) { Remove-Item -LiteralPath $inputPath }
    Write-Output "Single multilingual MSI ready: $output"
} finally {
    $resolvedScratch = [IO.Path]::GetFullPath($scratch)
    if (-not $resolvedScratch.StartsWith($scratchRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to clean an MSI scratch directory outside build/msi-languages.'
    }
    Remove-Item -LiteralPath $resolvedScratch -Recurse -Force
}
