param(
    [Alias("Artifact")]
    [string]$MsiArtifact = (Join-Path $PSScriptRoot "..\..\build\installer\GnxNode.msi"),
    [string]$BurnArtifact = (Join-Path $PSScriptRoot "..\..\build\installer\GnxNodeSetup.exe"),
    [switch]$PublishRelease
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$rootfsArtifact = Join-Path $root "payload\host\gnx-node-rootfs.tar"
$rootfsManifest = Join-Path $root "payload\host\gnx-node-rootfs.tar.sha256"
$expectedRootfsDigest = "5215c762cd1562b9b6836b01f6e1a2207cee7bae0ee334e50316af72b9a871af"
if (!(Test-Path -LiteralPath $rootfsArtifact -PathType Leaf)) { throw "Missing sealed WSL rootfs artifact: $rootfsArtifact" }
if (!(Test-Path -LiteralPath $rootfsManifest -PathType Leaf)) { throw "Missing sealed WSL rootfs manifest: $rootfsManifest" }
$manifestLine = (Get-Content -LiteralPath $rootfsManifest -Raw).Trim()
if ($manifestLine -notmatch '^[0-9a-f]{64}  gnx-node-rootfs\.tar$') { throw "Invalid sealed WSL rootfs manifest format: $rootfsManifest" }
$manifestDigest = $manifestLine.Substring(0, 64)
if ($manifestDigest -ne $expectedRootfsDigest) { throw "Unexpected sealed WSL rootfs digest: $manifestDigest" }
$actualRootfsDigest = (Get-FileHash -LiteralPath $rootfsArtifact -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualRootfsDigest -ne $manifestDigest) { throw "Sealed WSL rootfs digest mismatch: expected $manifestDigest, got $actualRootfsDigest" }
function Assert-Artifact([string]$Path, [string]$Label) {
    if (!(Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Missing $Label artifact: $Path" }
    $item = Get-Item -LiteralPath $Path
    if ($item.Length -le 0) { throw "$Label artifact is empty: $Path" }
    $hash = Get-FileHash -LiteralPath $Path -Algorithm SHA256
    [pscustomobject]@{
        Artifact = $Label
        Path = $item.FullName
        Bytes = $item.Length
        SHA256 = $hash.Hash
    }
}

$artifacts = @(Assert-Artifact $MsiArtifact "MSI"; Assert-Artifact $BurnArtifact "Burn")
$artifacts
if ($PublishRelease) {
    $cargo = Get-Content (Join-Path $root 'Cargo.toml') -Raw
    if ($cargo -notmatch '(?m)^version = "(\d+\.\d+\.\d+)"') { throw 'Workspace version unavailable' }
    $version = $Matches[1]
    $releases = Join-Path $root 'releases'
    $latest = Join-Path $releases "v$version"
    if (!(Test-Path (Join-Path $latest 'RELEASE_NOTES.md'))) { throw 'Write the validated release notes before publishing' }
    # Promote only the MSI built for this workspace version, not stale output.
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $database = $installer.OpenDatabase((Resolve-Path $MsiArtifact).Path, 0)
    $view = $database.OpenView("SELECT ``Value`` FROM ``Property`` WHERE ``Property`` = 'ProductVersion'")
    $view.Execute()
    if ($view.Fetch().StringData(1) -ne "$version.0") { throw 'MSI version does not match the workspace' }
    $view.Close()
    $sums = @()
    foreach ($artifact in $artifacts) {
        $name = Split-Path $artifact.Path -Leaf
        $destination = Join-Path $latest $name
        if (Test-Path $destination) {
            if ((Get-FileHash $destination -Algorithm SHA256).Hash -ne $artifact.SHA256) { throw "Published artifact is immutable: $destination" }
        } else { Copy-Item -LiteralPath $artifact.Path -Destination $destination }
        $sums += "$($artifact.SHA256.ToLowerInvariant())  $name"
    }
    [IO.File]::WriteAllText((Join-Path $latest 'SHA256SUMS.txt'), ($sums -join "`n") + "`n", [Text.UTF8Encoding]::new($false))
    # Archive notes outside releases; retain only the latest immutable artifacts.
    # Never rewrite Git/LFS history or replace an existing historical note.
    $history = Join-Path $root 'docs\releases'
    Get-ChildItem $releases -Directory | Where-Object { $_.Name -ne "v$version" } | ForEach-Object {
        $old = $_
        Get-ChildItem $old.FullName -File -Recurse | Where-Object { $_.Extension -eq '.md' } | ForEach-Object {
            $relative = $_.FullName.Substring($old.FullName.Length + 1)
            $destination = Join-Path (Join-Path $history $old.Name) $relative
            if (Test-Path $destination) {
                if ((Get-FileHash $_.FullName).Hash -ne (Get-FileHash $destination).Hash) { throw "Historical note conflict: $destination" }
            } else {
                New-Item -ItemType Directory -Force (Split-Path $destination -Parent) | Out-Null
                Copy-Item -LiteralPath $_.FullName -Destination $destination
            }
        }
        Remove-Item -LiteralPath $old.FullName -Recurse -Force
    }
    Write-Output "Release prepared: $latest (historical Markdown: docs/releases)"
}
