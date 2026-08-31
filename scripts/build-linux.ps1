[CmdletBinding()]
param(
    [string]$BuilderImage = 'docker.io/library/rust@sha256:3ffeca71d0e4fc30f5537f76b7243e87ac99726b6d3d66591dfc5e497078b9fc'
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$distributionDirectory = Join-Path $repositoryRoot 'dist'
$linuxBinary = Join-Path $repositoryRoot 'target\linux-musl\release\gnx'
$destination = Join-Path $distributionDirectory 'gnx-linux-x86_64'
$mountPath = $repositoryRoot.Replace('\', '/')

Push-Location $repositoryRoot
try {
    podman run --rm --arch amd64 `
        --volume "${mountPath}:/workspace" `
        --workdir /workspace `
        --env CARGO_TARGET_DIR=/workspace/target/linux-musl `
        $BuilderImage `
        sh -c 'cargo test --locked --all-targets && cargo build --locked --release'
    if ($LASTEXITCODE -ne 0) { throw "El build Linux falló con código $LASTEXITCODE." }

    New-Item -ItemType Directory -Force -Path $distributionDirectory | Out-Null
    Copy-Item -LiteralPath $linuxBinary -Destination $destination -Force
    $checksum = (Get-FileHash -Algorithm SHA256 -LiteralPath $destination).Hash.ToLowerInvariant()
    Set-Content -LiteralPath (Join-Path $distributionDirectory 'SHA256SUMS.linux') -Encoding ascii -NoNewline -Value "$checksum *gnx-linux-x86_64`n"

    podman run --rm --arch amd64 `
        --volume "${mountPath}:/workspace:ro" `
        --workdir /workspace `
        $BuilderImage `
        /workspace/dist/gnx-linux-x86_64 version
    if ($LASTEXITCODE -ne 0) { throw "El binario Linux no superó la verificación." }

    Write-Host "Creado: $destination"
}
finally {
    Pop-Location
}
