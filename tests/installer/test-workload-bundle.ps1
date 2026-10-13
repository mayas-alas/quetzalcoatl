[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$hostScript = Join-Path $repo 'payload\host\workload-bundle.ps1'
$installedPayload = Join-Path $repo 'payload\node'
$fixtureRoot = Join-Path (Join-Path $repo 'build\tests') ('workload-bundle-' + [Guid]::NewGuid().ToString('N'))
$sourceRoot = Join-Path $fixtureRoot 'source'
$bomSourceRoot = Join-Path $fixtureRoot 'source-bom'
$stateRoot = Join-Path $fixtureRoot 'state'
$zipPath = Join-Path $fixtureRoot 'valid.zip'

function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Assert-Equal([object]$Actual, [object]$Expected, [string]$Message) { if ($Actual -ne $Expected) { throw "$Message (actual=$Actual expected=$Expected)" } }
function Invoke-Admission([string]$Mode, [string]$Source) {
    $arguments = @(
        '-NoLogo', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $hostScript,
        '-Mode', $Mode, '-SourcePath', $Source, '-NodePayload', $installedPayload, '-StateDir', $stateRoot
    ) | ForEach-Object { '"' + ([string]$_).Replace('"', '\"') + '"' }
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = (Join-Path $PSHOME 'powershell.exe')
    $startInfo.Arguments = $arguments -join ' '
    $startInfo.WorkingDirectory = $repo
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    [void]$process.Start()
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    $exitCode = $process.ExitCode
    $process.Dispose()
    [pscustomobject]@{ ExitCode = $exitCode; Stdout = $stdout.TrimEnd(); Stderr = $stderr.TrimEnd() }
}
function Write-Text([string]$Path, [string]$Text) {
    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    [IO.File]::WriteAllText($Path, $Text, (New-Object Text.UTF8Encoding($false)))
}
function Write-Json([string]$Path, [object]$Value, [bool]$Bom) {
    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $encoding = New-Object System.Text.UTF8Encoding -ArgumentList @($Bom)
    [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 20), $encoding)
}
function Copy-Config([string]$RelativePath) {
    $installed = Join-Path $installedPayload ('workloads\ai\' + $RelativePath.Replace('/', '\'))
    $target = Join-Path $sourceRoot ($RelativePath.Replace('/', '\'))
    $parent = Split-Path -Parent $target
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    Copy-Item -LiteralPath $installed -Destination $target -Force
}
function Make-ValidSource {
    New-Item -ItemType Directory -Path $sourceRoot -Force | Out-Null
    foreach ($file in @('profile.json', 'images.json', 'offline.sha256', 'ai.network', 'voice.container', 'oryx.container', 'omniroute.container', 'continuity.container', 'netbird-proxy.container', 'workload-mesh.container')) { Copy-Config $file }
    foreach ($file in @('offline.url', 'proxy-enroll.run', 'mesh.nft', 'install.run', 'enroll.run', 'workload-tls-renew.timer', 'workload-tls-renew.service')) {
        $installed = Join-Path $installedPayload ('workloads\ai\' + $file)
        if (Test-Path -LiteralPath $installed) { Copy-Config $file }
    }
    $profilePath = Join-Path $sourceRoot 'profile.json'
    $profile = Get-Content $profilePath -Raw | ConvertFrom-Json
    $profile.cores = 8
    $profile.memory_mib = 8192
    $profile.rootfs_gib = 32
    $profile.release = '1.2.3'
    Write-Json $profilePath $profile $false
}
function Add-ZipEntry([IO.Compression.ZipArchive]$Archive, [string]$Root, [string]$RelativePath) {
    $entry = $Archive.CreateEntry($RelativePath)
    $input = [IO.File]::OpenRead((Join-Path $Root ($RelativePath.Replace('/', '\'))))
    $output = $entry.Open()
    try { $input.CopyTo($output) } finally { $output.Dispose(); $input.Dispose() }
}
function Make-ValidZip {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $stream = [IO.File]::Create($zipPath)
    $archive = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create, $false)
    try { foreach ($file in (Get-ChildItem $sourceRoot -File | Sort-Object Name)) { Add-ZipEntry $archive $sourceRoot $file.Name } }
    finally { $archive.Dispose(); $stream.Dispose() }
}
function Make-RawZip([string]$Path, [object[]]$Entries) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $stream = [IO.File]::Create($Path)
    $archive = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create, $false)
    try {
        foreach ($item in $Entries) {
            $entry = $archive.CreateEntry([string]$item.Name)
            if ($item.Contains('Attributes')) { $entry.ExternalAttributes = [int]$item.Attributes }
            $writer = New-Object IO.StreamWriter($entry.Open(), (New-Object Text.UTF8Encoding($false)))
            try { $writer.Write([string]$item.Text) } finally { $writer.Dispose() }
        }
    } finally { $archive.Dispose(); $stream.Dispose() }
}
function Assert-RejectedZip([string]$Path, [string]$Code) {
    $result = Invoke-Admission 'Validate' $Path
    Assert-Equal $result.ExitCode 1 "Expected ZIP rejection for $Code"
    Assert-True ($result.Stderr -match ('"code":"' + $Code + '"')) "Expected stable ZIP error $Code"
    Assert-True ([string]::IsNullOrWhiteSpace($result.Stdout)) 'Rejected ZIP admission wrote stdout'
}
function Assert-Rejected([string]$Source, [string]$Code) {
    $result = Invoke-Admission 'Validate' $Source
    Assert-Equal $result.ExitCode 1 "Expected rejection for $Code"
    Assert-True ($result.Stderr -match ('"code":"' + $Code + '"')) "Expected stable error $Code"
    Assert-True ([string]::IsNullOrWhiteSpace($result.Stdout)) "Rejected admission wrote stdout"
}

try {
    Add-Type -AssemblyName System.IO.Compression,System.IO.Compression.FileSystem
    New-Item -ItemType Directory -Path $fixtureRoot -Force | Out-Null
    New-Item -ItemType Directory -Path $stateRoot -Force | Out-Null
    Make-ValidSource
    $validFolder = Invoke-Admission 'Validate' $sourceRoot
    Assert-Equal $validFolder.ExitCode 0 'Valid folder was rejected'
    $folderJson = $validFolder.Stdout | ConvertFrom-Json
    Assert-Equal $folderJson.workload 'ai' 'Folder workload mismatch'
    Assert-True ($folderJson.revision -match '^[0-9a-f]{64}$') 'Folder revision is not SHA-256'
    Assert-True ([int64]$folderJson.bytes -gt 0) 'Folder config byte count missing'
    Assert-True ($folderJson.files -contains 'profile.json') 'Folder file list missing profile'

    Copy-Item -LiteralPath $sourceRoot -Destination $bomSourceRoot -Recurse
    $bomProfilePath = Join-Path $bomSourceRoot 'profile.json'
    $bomProfile = Get-Content $bomProfilePath -Raw | ConvertFrom-Json
    Write-Json $bomProfilePath $bomProfile $true
    $bomFolder = Invoke-Admission 'Validate' $bomSourceRoot
    Assert-Equal $bomFolder.ExitCode 0 'UTF-8 BOM folder was rejected'
    $bomJson = $bomFolder.Stdout | ConvertFrom-Json
    Assert-Equal $bomJson.revision $folderJson.revision 'BOM changes normalized revision'

    Make-ValidZip
    $validZip = Invoke-Admission 'Validate' $zipPath
    Assert-Equal $validZip.ExitCode 0 'Valid ZIP was rejected'
    $zipJson = $validZip.Stdout | ConvertFrom-Json
    Assert-Equal $zipJson.revision $folderJson.revision 'Folder and ZIP revisions differ'
    Assert-True ($validZip.Stdout -notmatch '\r?\n.+') 'Valid admission emitted multiple stdout records'

    Copy-Item -LiteralPath $sourceRoot -Destination (Join-Path $fixtureRoot 'traversal') -Recurse
    Write-Text (Join-Path $fixtureRoot 'traversal\evil.txt') 'no'
    Assert-Rejected (Join-Path $fixtureRoot 'traversal') 'FILE_NOT_ALLOWED'

    $unsafe = Join-Path $fixtureRoot 'unsafe'
    Copy-Item -LiteralPath $sourceRoot -Destination $unsafe -Recurse
    Write-Text (Join-Path $unsafe 'evil.run') '#!/bin/sh'
    Assert-Rejected $unsafe 'SCRIPT_NOT_TRUSTED'

    $badConfig = Join-Path $fixtureRoot 'bad-config'
    Copy-Item -LiteralPath $sourceRoot -Destination $badConfig -Recurse
    Write-Text (Join-Path $badConfig 'config\voice\secret.env') 'TOKEN=inline'
    Assert-Rejected $badConfig 'DIRECTORY_NOT_ALLOWED'

    $badQuadlet = Join-Path $fixtureRoot 'bad-quadlet'
    Copy-Item -LiteralPath $sourceRoot -Destination $badQuadlet -Recurse
    Add-Content -LiteralPath (Join-Path $badQuadlet 'voice.container') "`n[Container]`nPodmanArgs=--privileged"
    Assert-Rejected $badQuadlet 'QUADLET_NOT_EXACT'

    $zipTraversal = Join-Path $fixtureRoot 'traversal.zip'
    Make-RawZip $zipTraversal @([ordered]@{ Name = '../profile.json'; Text = '{}' })
    Assert-RejectedZip $zipTraversal 'PATH_TRAVERSAL'

    $zipRooted = Join-Path $fixtureRoot 'rooted.zip'
    Make-RawZip $zipRooted @([ordered]@{ Name = '/profile.json'; Text = '{}' })
    Assert-RejectedZip $zipRooted 'PATH_INVALID'

    $zipDuplicate = Join-Path $fixtureRoot 'duplicate.zip'
    Make-RawZip $zipDuplicate @([ordered]@{ Name = 'profile.json'; Text = '{}' }, [ordered]@{ Name = 'PROFILE.JSON'; Text = '{}' })
    Assert-RejectedZip $zipDuplicate 'DUPLICATE_ENTRY'

    $zipLink = Join-Path $fixtureRoot 'link.zip'
    Make-RawZip $zipLink @([ordered]@{ Name = 'evil.run'; Text = '#!/bin/sh'; Attributes = (0xA000 -shl 16) })
    Assert-RejectedZip $zipLink 'ZIP_LINK_NOT_ALLOWED'

    $adsRoot = Join-Path $fixtureRoot 'ads'
    Copy-Item -LiteralPath $sourceRoot -Destination $adsRoot -Recurse
    Set-Content -LiteralPath (Join-Path $adsRoot 'profile.json') -Stream 'untrusted' -Value 'blocked'
    Assert-Rejected $adsRoot 'ADS_NOT_ALLOWED'

    $import = Invoke-Admission 'Import' $sourceRoot
    Assert-Equal $import.ExitCode 0 "Protected import failed: $($import.Stderr)"
    $importJson = $import.Stdout | ConvertFrom-Json
        Assert-True (Test-Path -LiteralPath $importJson.bundle_dir) 'Imported bundle directory missing'
        $revisionReceiptPath = Join-Path $importJson.bundle_dir 'receipt.json'
        Assert-True (Test-Path -LiteralPath $revisionReceiptPath) 'Revision receipt missing'
        $revisionReceipt = Get-Content $revisionReceiptPath -Raw | ConvertFrom-Json
        Assert-Equal $revisionReceipt.schema 1 'Revision receipt schema mismatch'
        Assert-Equal $revisionReceipt.revision $importJson.revision 'Revision receipt revision mismatch'
        Assert-True ($revisionReceipt.files[0].sha256 -match '^[0-9a-f]{64}$') 'Revision receipt file hash missing'
        Assert-True (Test-Path -LiteralPath (Join-Path $stateRoot 'workload-bundles\ai\current.json')) 'Current receipt missing'
        Assert-True ((Get-Content (Join-Path $stateRoot 'workload-bundles\ai\current.json') -Raw) -match [string]$importJson.revision) 'Receipt revision mismatch'
    $repeat = Invoke-Admission 'Import' $sourceRoot
    Assert-Equal $repeat.ExitCode 0 "Repeated protected import failed: $($repeat.Stderr)"
    $repeatJson = $repeat.Stdout | ConvertFrom-Json
    Assert-Equal $repeatJson.revision $importJson.revision 'Idempotent import changed revision'
    Assert-Equal $repeatJson.bundle_dir $importJson.bundle_dir 'Idempotent import changed protected staging path'
    Write-Output 'PASS: folder, ZIP, BOM normalization, protected/idempotent import receipts and negative admission fixtures.'
} finally {
    if (Test-Path -LiteralPath $fixtureRoot) { Remove-Item -LiteralPath $fixtureRoot -Recurse -Force -ErrorAction SilentlyContinue }
}
