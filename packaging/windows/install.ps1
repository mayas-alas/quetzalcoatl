[CmdletBinding()]
param(
 [Parameter(Mandatory)][string]$ManifestSha256,
 [Parameter(Mandatory)][string]$Rootfs,
 [string]$Bundle=(Join-Path $PSScriptRoot '../../dist')
)
$ErrorActionPreference='Stop'
$installer=Join-Path (Resolve-Path -LiteralPath $Bundle).Path 'gnx-install.exe'
& $installer --manifest-sha256 $ManifestSha256 --rootfs (Resolve-Path -LiteralPath $Rootfs).Path
exit $LASTEXITCODE
