[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$distributionDirectory = Join-Path $repositoryRoot 'dist'
$target = 'x86_64-pc-windows-msvc'
$source = Join-Path $repositoryRoot "target\$target\release\gnx.exe"
$destination = Join-Path $distributionDirectory 'gnx-windows-x86_64.exe'

Push-Location $repositoryRoot
try {
    cargo test --locked --all-targets
    if ($LASTEXITCODE -ne 0) { throw "Las pruebas Rust fallaron con código $LASTEXITCODE." }

    cargo build --locked --release --target $target
    if ($LASTEXITCODE -ne 0) { throw "El build Windows falló con código $LASTEXITCODE." }

    New-Item -ItemType Directory -Force -Path $distributionDirectory | Out-Null
    Copy-Item -LiteralPath $source -Destination $destination -Force

    $checksum = (Get-FileHash -Algorithm SHA256 -LiteralPath $destination).Hash.ToLowerInvariant()
    Set-Content -LiteralPath (Join-Path $distributionDirectory 'SHA256SUMS.windows') -Encoding ascii -NoNewline -Value "$checksum *gnx-windows-x86_64.exe`n"

    & $destination version
    if ($LASTEXITCODE -ne 0) { throw "El binario Windows no superó la verificación." }

    Write-Host "Creado: $destination"
}
finally {
    Pop-Location
}
