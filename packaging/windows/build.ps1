[CmdletBinding()]
param(
    [string]$OutputDirectory,
    [string]$MeshClientMsi,
    [string]$MeshClientVersion,
    [string]$MeshClientLicense,
    [string]$MeshClientSbom
)

$ErrorActionPreference = 'Stop'
$projectRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $projectRoot 'dist\windows'
}

& cargo build --locked --release --manifest-path (Join-Path $projectRoot 'Cargo.toml')
if ($LASTEXITCODE -ne 0) {
    throw 'Rust release build failed.'
}

$output = New-Item -ItemType Directory -Force -Path $OutputDirectory
Copy-Item -Force (Join-Path $projectRoot 'target\release\gnx.exe') $output.FullName
Copy-Item -Force (Join-Path $projectRoot 'config\gnx.example.toml') (Join-Path $output 'gnx.example.toml')

$inputs = @($MeshClientMsi, $MeshClientVersion, $MeshClientLicense, $MeshClientSbom)
$completeRelease = ($inputs | Where-Object { $_ }).Count -eq $inputs.Count
if (($inputs | Where-Object { $_ }).Count -notin @(0, $inputs.Count)) {
    throw 'Provide all mesh client release inputs or none.'
}

if ($completeRelease) {
    if ($MeshClientVersion -notmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$') {
        throw 'The mesh client version must be SemVer.'
    }
    $msi = Resolve-Path -LiteralPath $MeshClientMsi
    $license = Resolve-Path -LiteralPath $MeshClientLicense
    $sbom = Resolve-Path -LiteralPath $MeshClientSbom
    $signature = Get-AuthenticodeSignature -LiteralPath $msi
    if ($signature.Status -ne 'Valid') {
        throw 'The mesh client MSI does not have a valid Authenticode signature.'
    }

    $artifacts = New-Item -ItemType Directory -Force -Path (Join-Path $output 'artifacts')
    $legal = New-Item -ItemType Directory -Force -Path (Join-Path $output 'legal')
    $bundledMsi = Join-Path $artifacts 'mesh-client.msi'
    Copy-Item -Force -LiteralPath $msi -Destination $bundledMsi
    Copy-Item -Force -LiteralPath $license -Destination (Join-Path $legal 'mesh-client.LICENSE')
    Copy-Item -Force -LiteralPath $sbom -Destination (Join-Path $legal 'mesh-client.cdx.json')
    $digest = (Get-FileHash -Algorithm SHA256 -LiteralPath $bundledMsi).Hash.ToLowerInvariant()

    @"
version = 1

[windows.mesh_client]
package = "artifacts/mesh-client.msi"
version = "$MeshClientVersion"
sha256 = "$digest"
license = "legal/mesh-client.LICENSE"
sbom = "legal/mesh-client.cdx.json"
"@ | Set-Content -Encoding utf8 -NoNewline (Join-Path $output 'release.toml')
    $exampleManifest = Join-Path $output 'release.example.toml'
    if (Test-Path -LiteralPath $exampleManifest) {
        Remove-Item -LiteralPath $exampleManifest
    }
} else {
    $releaseManifest = Join-Path $output 'release.toml'
    if (Test-Path -LiteralPath $releaseManifest) {
        Remove-Item -LiteralPath $releaseManifest
    }
    Copy-Item -Force (Join-Path $projectRoot 'runtime\release.example.toml') $output.FullName
}

$exeDigest = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $output 'gnx.exe')).Hash.ToLowerInvariant()
Set-Content -Encoding ascii -NoNewline -Path (Join-Path $output 'gnx.exe.sha256') -Value $exeDigest
Write-Output $output.FullName
