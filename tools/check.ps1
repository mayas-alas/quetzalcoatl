[CmdletBinding()]
param(
    [switch] $SourceOnly
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$validationRoot = Join-Path $PSScriptRoot 'validation'

Push-Location $repoRoot
try {
    foreach ($validator in @(
        'repository.py',
        'contracts.py',
        'remote_execution.py',
        'runtime.py',
        'platform.py',
        'installer.py'
    )) {
        & python (Join-Path $validationRoot $validator)
        if ($LASTEXITCODE -ne 0) {
            throw "Validation failed: $validator"
        }
    }

    & (Join-Path $PSScriptRoot 'security.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'Security gate failed.' }

    & cargo fmt --all --check
    if ($LASTEXITCODE -ne 0) { throw 'cargo fmt failed.' }

    & cargo clippy --workspace --all-targets --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'cargo clippy failed.' }

    & cargo test --workspace --all-targets --locked
    if ($LASTEXITCODE -ne 0) { throw 'cargo test failed.' }

    if (-not $SourceOnly) {
        & (Join-Path $repoRoot 'installer\build.ps1')
        if ($LASTEXITCODE -ne 0) { throw 'Installer build failed.' }
    }
} finally {
    Pop-Location
}
