[CmdletBinding()]
param(
    [ValidateSet('QaSigning', 'Unsigned')]
    [string] $SigningMode = 'QaSigning',
    [switch] $SkipBuild,
    [switch] $Publish,
    [string] $Repo = 'mayas-alas/quetzalcoatl',
    [string] $NotesFile = '',
    [string] $OutputRoot = ''
)

$ErrorActionPreference = 'Stop'
$installerRoot = $PSScriptRoot
$repoRoot = (Resolve-Path (Join-Path $installerRoot '..')).Path
$moduleRoot = Join-Path $installerRoot 'modules'
. (Join-Path $moduleRoot 'release.ps1')

$releaseManifestPath = Join-Path $repoRoot 'release\manifest.toml'
$releaseFacts = Read-ReleaseManifest -Path $releaseManifestPath
$releaseVersion = [string] (Get-ReleaseFact $releaseFacts 'version')

if (-not $OutputRoot) {
    $OutputRoot = Join-Path $repoRoot 'release'
}
$artifactsRoot = Join-Path $OutputRoot 'artifacts'
New-Item -ItemType Directory -Force -Path $artifactsRoot | Out-Null

if (-not $SkipBuild) {
    $buildScript = Join-Path $installerRoot 'build.ps1'
    if ($SigningMode -eq 'Unsigned') {
        & $buildScript -AllowUnsigned
    } else {
        & $buildScript -QaSigning
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Installer build failed with exit code $LASTEXITCODE."
    }
}

$buildOutput = Join-Path $repoRoot 'target\installer'
$setupPath = Join-Path $buildOutput 'QuetzalcoatlSetup.exe'
$msiPath = Join-Path $buildOutput 'Quetzalcoatl.msi'
$platformSource = Join-Path $buildOutput 'platform-payload'
$runtimeLockSource = Join-Path $buildOutput 'runtime-payload\payload.lock.json'
foreach ($path in @($setupPath, $msiPath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Build output is absent: $path"
    }
}
if (-not (Test-Path -LiteralPath $platformSource -PathType Container)) {
    throw "Platform payload is absent: $platformSource"
}
if (-not (Test-Path -LiteralPath $runtimeLockSource -PathType Leaf)) {
    throw "Runtime payload lock is absent: $runtimeLockSource"
}

$staging = Join-Path $artifactsRoot '.staging'
if (Test-Path -LiteralPath $staging -PathType Container) {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $staging | Out-Null
Copy-Item -LiteralPath $setupPath -Destination (Join-Path $staging 'QuetzalcoatlSetup.exe') -Force
Copy-Item -LiteralPath $msiPath -Destination (Join-Path $staging 'Quetzalcoatl.msi') -Force
Copy-Item -LiteralPath $runtimeLockSource -Destination (Join-Path $staging 'payload.lock.json') -Force
Copy-Item -LiteralPath $platformSource -Destination (Join-Path $staging 'platform-payload') -Recurse -Force

$checksums = [System.Collections.Generic.List[string]]::new()
$files = Get-ChildItem -LiteralPath $staging -Recurse -File -Force | Sort-Object FullName
foreach ($file in $files) {
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $file.FullName).Hash
    $relative = $file.FullName.Substring($staging.Length + 1)
    $checksums.Add(('{0}  {1}' -f $hash, $relative))
}

$modeSuffix = if ($SigningMode -eq 'Unsigned') { 'dev' } else { 'qa' }
$zipName = 'Quetzalcoatl-{0}-{1}.zip' -f $releaseVersion, $modeSuffix
$zipPath = Join-Path $OutputRoot $zipName
Add-Type -AssemblyName System.IO.Compression.FileSystem
if (Test-Path -LiteralPath $zipPath -PathType Leaf) {
    Remove-Item -LiteralPath $zipPath -Force
}
[IO.Compression.ZipFile]::CreateFromDirectory($staging, $zipPath, [IO.Compression.CompressionLevel]::Optimal, $false)

$zipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $zipPath).Hash
$checksums.Add(('{0}  {1}' -f $zipHash, $zipName))
$checksumPath = Join-Path $OutputRoot 'SHA256SUMS.txt'
[IO.File]::WriteAllLines($checksumPath, $checksums, [Text.UTF8Encoding]::new($false))

[pscustomobject]@{
    version = $releaseVersion
    signing_mode = $SigningMode
    setup = $setupPath
    msi = $msiPath
    zip = $zipPath
    zip_sha256 = $zipHash
    checksums = $checksumPath
    file_count = $files.Count
} | ConvertTo-Json

if ($Publish) {
    $tag = 'v{0}-{1}' -f $releaseVersion, $modeSuffix
    $title = 'Quetzalcoatl {0} ({1})' -f $releaseVersion, $modeSuffix
    $notes = if ($NotesFile -and (Test-Path -LiteralPath $NotesFile -PathType Leaf)) {
        (Get-Content -LiteralPath $NotesFile -Raw)
    } else {
        "Build $zipName`nSHA-256: $zipHash"
    }
    gh release create $tag $zipPath `
        --repo $Repo `
        --title $title `
        --notes $notes
    if ($LASTEXITCODE -ne 0) {
        throw "gh release create failed with exit code $LASTEXITCODE."
    }
}