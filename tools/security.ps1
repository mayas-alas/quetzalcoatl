[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$expectedAuditVersion = '0.22.2'
$versionOutput = (& cargo audit --version 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "cargo-audit $expectedAuditVersion is required by the release gate."
}
$match = [regex]::Match(
    $versionOutput,
    '^cargo-audit(?:-audit)?\s+(?<version>\d+\.\d+\.\d+)'
)
if (-not $match.Success -or $match.Groups['version'].Value -ne $expectedAuditVersion) {
    throw "Expected cargo-audit $expectedAuditVersion, received '$versionOutput'."
}

& cargo audit --deny warnings
if ($LASTEXITCODE -ne 0) {
    throw 'RustSec dependency audit failed.'
}
