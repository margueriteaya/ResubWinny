$ErrorActionPreference = 'Stop'

function Import-MsiTools([string]$WixToolsDirectory) {
    if (-not $IsWindows) { throw 'MSI language tools require Windows.' }
    if (-not $WixToolsDirectory) {
        $WixToolsDirectory = Join-Path $env:LOCALAPPDATA 'tauri/WixTools314'
    }
    Add-Type -Path (Join-Path $WixToolsDirectory 'Microsoft.Deployment.WindowsInstaller.dll')
    if (-not ('MsiStorage' -as [type])) { Add-Type -Path (Join-Path $PSScriptRoot 'msi-storage.cs') }
}

function Open-MsiDatabase([string]$Path, [switch]$Write) {
    $mode = if ($Write) { [Microsoft.Deployment.WindowsInstaller.DatabaseOpenMode]::Transact } else { [Microsoft.Deployment.WindowsInstaller.DatabaseOpenMode]::ReadOnly }
    return [Microsoft.Deployment.WindowsInstaller.Database]::new($Path, $mode)
}

function Get-MsiTableFingerprint($Database, [string]$Table) {
    $rows = @($Database.ExecuteQuery("SELECT * FROM ``$Table``"))
    return ConvertTo-Json -InputObject $rows -Compress -Depth 5
}

function Assert-MsiPayload($Base, $Other) {
    foreach ($property in @('ProductCode', 'ProductVersion', 'UpgradeCode', 'ALLUSERS')) {
        if ($Base.ExecutePropertyQuery($property) -cne $Other.ExecutePropertyQuery($property)) {
            throw "Localized MSI changes $property."
        }
    }
    foreach ($table in @('Component', 'File', 'Media', 'InstallExecuteSequence', 'CustomAction')) {
        if ((Get-MsiTableFingerprint $Base $table) -cne (Get-MsiTableFingerprint $Other $table)) {
            throw "Localized MSI changes the $table table."
        }
    }
}

function Assert-MultilingualMsi([string]$Path, [string]$ScratchDirectory) {
    $languages = [ordered]@{ 'zh-CN' = '2052'; 'zh-TW' = '1028'; 'ja-JP' = '1041' }
    $installWords = @{ 'zh-CN' = '安装'; 'zh-TW' = '安裝'; 'ja-JP' = 'インストール' }
    $base = Open-MsiDatabase $Path
    try {
        if ($base.ExecutePropertyQuery('ALLUSERS') -ne '1' -or $base.ExecutePropertyQuery('ProductLanguage') -ne '1033') {
            throw 'Expected a per-machine MSI with an English base language.'
        }
        $names = @($base.ExecuteStringQuery('SELECT `Name` FROM `_Storages`'))
        foreach ($entry in $languages.GetEnumerator()) {
            $name = "$($entry.Key).mst"
            if ($name -cnotin $names) { throw "MSI is missing embedded transform $name." }
            $transform = Join-Path $ScratchDirectory "embedded-$name"
            [MsiStorage]::Extract($Path, $name, $transform)
            $copy = Join-Path $ScratchDirectory "$($entry.Key)-verify.msi"
            Copy-Item -LiteralPath $Path -Destination $copy -Force
            $localized = Open-MsiDatabase $copy -Write
            try {
                $localized.ApplyTransform($transform)
                if ($localized.ExecutePropertyQuery('ProductLanguage') -ne $entry.Value) {
                    throw "Embedded transform $name has the wrong ProductLanguage."
                }
                Assert-MsiPayload $base $localized
                if ((Get-MsiTableFingerprint $base 'Control') -ceq (Get-MsiTableFingerprint $localized 'Control')) {
                    throw "Embedded transform $name does not translate the installer controls."
                }
                $text = $localized.ExecuteStringQuery('SELECT `Text` FROM `Control`') -join "`n"
                if (-not $text.Contains($installWords[$entry.Key])) {
                    throw "Embedded transform $name lost its localized Unicode text."
                }
                Write-Output "MSI language verified: $($entry.Key), ProductLanguage=$($entry.Value), unchanged payload and per-machine scope."
            } finally { $localized.Dispose() }
        }
    } finally { $base.Dispose() }
}
