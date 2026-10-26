param([switch]$Apply)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$root = Join-Path $repo 'build/releases'
function Assert-Regular($Item) {
    if ($Item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "Refusing linked release artifact: $($Item.FullName)"
    }
}
Assert-Regular (Get-Item -LiteralPath $root)
$candidates = @(Get-ChildItem -LiteralPath $root -Directory | ForEach-Object {
    if ($_.Name -match '^v(\d+\.\d+\.\d+)$') {
        $version = [version]$Matches[1]
        $setup = Join-Path $_.FullName 'GnxCoreSetup.exe'
        if (Test-Path -LiteralPath $setup -PathType Leaf) {
            Assert-Regular $_
            $file = Get-Item -LiteralPath $setup
            Assert-Regular $file
            [pscustomobject]@{ Version=$version; Directory=$_.FullName; Setup=$file }
        }
    }
})
if ($candidates.Count -eq 0) { throw 'No completed setup EXE to preserve; nothing removed' }
$latest = $candidates | Sort-Object Version -Descending | Select-Object -First 1
$version = $latest.Version.ToString()
$msi = Get-Item -LiteralPath (Join-Path $latest.Directory 'GnxCore.msi')
$release = Join-Path $repo "releases/v$version"
Assert-Regular (Get-Item -LiteralPath $release)
$binary = Get-Item -LiteralPath (Join-Path $release 'gnx.exe')
Assert-Regular $msi
Assert-Regular $binary
if (-not (Test-Path -LiteralPath (Join-Path $release 'CHANGELOG.md') -PathType Leaf)) { throw 'Latest changelog missing' }
# Preserve a complete, matching release, not merely the highest directory name.
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase($msi.FullName,0)
try {
    $query = $database.OpenView("SELECT Value FROM Property WHERE Property='ProductVersion'")
    $query.Execute()
    $record = $query.Fetch()
    if ($null -eq $record -or $record.StringData(1) -ne $version) { throw 'Latest MSI version mismatch' }
    $query.Close()
    $query = $database.OpenView("SELECT HashPart1,HashPart2,HashPart3,HashPart4 FROM MsiFileHash WHERE File_='CoreExe'")
    $query.Execute()
    $record = $query.Fetch()
    $hash = $installer.FileHash($binary.FullName,0)
    if ($null -eq $record) { throw 'Latest MSI binary hash missing' }
    foreach ($i in 1..4) { if ($record.IntegerData($i) -ne $hash.IntegerData($i)) { throw 'Latest MSI binary mismatch' } }
    $query.Close()
} finally { [Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) | Out-Null }
"Keep: $($latest.Setup.FullName)"
foreach ($old in $candidates | Where-Object { $_.Version -lt $latest.Version }) {
    [pscustomobject]@{ Action=$(if ($Apply) { 'Remove' } else { 'WouldRemove' }); Version=$old.Version.ToString(); Path=$old.Setup.FullName; Bytes=$old.Setup.Length }
    if ($Apply) { Remove-Item -LiteralPath $old.Setup.FullName -Force }
}
if (-not $Apply) { 'Preview only. Use -Apply to remove only the listed old setup EXEs.' }
