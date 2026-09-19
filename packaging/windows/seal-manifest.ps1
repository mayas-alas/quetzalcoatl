[CmdletBinding()]
param(
    [string]$Manifest = (Join-Path $PSScriptRoot '../../dist/manifest.json'),
    [string]$Signature = (Join-Path $PSScriptRoot '../../dist/manifest.sig')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$keyPath = $env:GNX_RELEASE_PRIVATE_KEY_FILE
if ([string]::IsNullOrWhiteSpace($keyPath)) {
    throw 'GNX_RELEASE_PRIVATE_KEY_FILE is required; the private key must not be passed in argv or committed.'
}
$key = (Resolve-Path -LiteralPath $keyPath -ErrorAction Stop).Path
$manifestPath = (Resolve-Path -LiteralPath $Manifest -ErrorAction Stop).Path
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.schema -ne 1 -or $manifest.version -ne '0.3.1' -or $manifest.platform -ne 'windows+linux-amd64') {
    throw 'Manifest schema, version or platform is unsupported.'
}
if ($manifest.status -ne 'unsealed') { throw 'Only an unsealed build candidate may be promoted.' }

# The build emits compact, stable JSON. Replace only the controlled status token
# so the exact bytes that are signed remain deterministic and reviewable.
$bytes = [IO.File]::ReadAllBytes($manifestPath)
$text = [Text.Encoding]::UTF8.GetString($bytes)
$sealed = $text.Replace('"status":"unsealed"', '"status":"sealed"')
if ($sealed -eq $text) { throw 'Manifest status token was not found in canonical form.' }
[IO.File]::WriteAllText($manifestPath, $sealed, [Text.UTF8Encoding]::new($false))

$signaturePath = [IO.Path]::GetFullPath($Signature)
& openssl pkeyutl -sign -rawin -inkey $key -in $manifestPath -out $signaturePath
if ($LASTEXITCODE -ne 0) { throw 'Manifest signing failed.' }
$signatureItem = Get-Item -LiteralPath $signaturePath -ErrorAction Stop
if ($signatureItem.Length -ne 64) { throw 'Manifest signing produced an invalid Ed25519 signature.' }
Write-Output 'SEALED manifest and detached signature created.'
