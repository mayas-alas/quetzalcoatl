[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$distributionDirectory = Join-Path $repositoryRoot 'dist'
$windowsArtifact = Join-Path $distributionDirectory 'gnx-windows-x86_64.exe'
$linuxArtifact = Join-Path $distributionDirectory 'gnx-linux-x86_64'
$appImageArtifact = Join-Path $distributionDirectory 'gnx-x86_64.AppImage'

foreach ($artifact in @($windowsArtifact, $linuxArtifact, $appImageArtifact)) {
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) {
        throw "Falta el artefacto requerido: $artifact"
    }
}

$versionOutput = & $windowsArtifact version
if ($LASTEXITCODE -ne 0) { throw 'No se pudo obtener la versión del binario Windows.' }
$version = ($versionOutput -split '\s+')[-1]

$specifications = @(
    [ordered]@{
        name = 'gnx-windows-x86_64.exe'
        path = $windowsArtifact
        target = 'x86_64-pc-windows-msvc'
        format = 'pe-executable'
    },
    [ordered]@{
        name = 'gnx-linux-x86_64'
        path = $linuxArtifact
        target = 'x86_64-unknown-linux-musl'
        format = 'static-pie-elf'
    },
    [ordered]@{
        name = 'gnx-x86_64.AppImage'
        path = $appImageArtifact
        target = 'x86_64-unknown-linux-musl'
        format = 'appimage-type2'
    }
)

$checksumLines = @()
$artifacts = foreach ($specification in $specifications) {
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $specification.path).Hash.ToLowerInvariant()
    $checksumLines += "$hash *$($specification.name)"
    [ordered]@{
        name = $specification.name
        target = $specification.target
        format = $specification.format
        sha256 = $hash
        signed = $false
    }
}

$manifest = [ordered]@{
    schema = 1
    product = 'Quetzalcoatl Next'
    version = $version
    channel = 'development'
    artifacts = @($artifacts)
    limitations = @(
        'unsigned development artifacts',
        'Docktail and Headscale Services require physical MESH-SVC-01 evidence',
        'Dockur Proxmox and LXC require nested KVM evidence',
        'Windows virtual service identity requires WIN-ID-01 evidence'
    )
}

$checksumPath = Join-Path $distributionDirectory 'SHA256SUMS'
$checksumText = ($checksumLines -join "`n") + "`n"
[System.IO.File]::WriteAllText($checksumPath, $checksumText, [System.Text.Encoding]::ASCII)

$manifestPath = Join-Path $distributionDirectory 'release.json'
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText(
    $manifestPath,
    ($manifest | ConvertTo-Json -Depth 5) + "`n",
    $utf8WithoutBom
)

Write-Host "Metadata de desarrollo creada en $distributionDirectory"
