[CmdletBinding()]
param(
 [Parameter(Mandatory)][string]$Archive,
 [string]$SigningKey,
 [string]$Signer = (Join-Path $PSScriptRoot '..\target\release\gnx-sign.exe')
)
$ErrorActionPreference='Stop'

function Invoke-Gnx([string]$File,[string[]]$Arguments) {
 $lines=@(& $File @Arguments 2>$null)
 $exitCode=$LASTEXITCODE
 $text=($lines -join [Environment]::NewLine).Trim()
 try {$result=$text | ConvertFrom-Json} catch {throw "UNPARSEABLE_RESULT exit=$exitCode"}
 [pscustomobject]@{exit_code=$exitCode;result=$result}
}

function Assert-Case($Case,[int]$ExitCode,[string]$State,[string]$Code) {
 if($Case.exit_code -ne $ExitCode -or $Case.result.state -ne $State -or $Case.result.code -ne $Code){
  throw "UNEXPECTED_RESULT expected=$ExitCode/$State/$Code"
 }
 if($State -eq 'FAILED' -and [string]::IsNullOrWhiteSpace($Case.result.next_action)){
  throw 'MISSING_NEXT_ACTION'
 }
}

$archivePath=(Resolve-Path -LiteralPath $Archive).Path
$testRoot=Join-Path ([IO.Path]::GetTempPath()) ('gnx-release-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot | Out-Null
try {
 Expand-Archive -LiteralPath $archivePath -DestinationPath $testRoot
 $installer=Join-Path $testRoot 'gnx-install.exe'
 $manifest=Join-Path $testRoot 'manifest.json'
 $signature=Join-Path $testRoot 'manifest.json.sig'
 $rootfs=Join-Path $testRoot 'gnx-wsl-rootfs.tar.gz'
 $digest=(Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash.ToLowerInvariant()
 $cases=[ordered]@{}

 $cases.authentic_manifest=Invoke-Gnx $installer @('verify','--manifest-sha256',$digest)
 Assert-Case $cases.authentic_manifest 0 'READY' 'RELEASE_AUTHENTIC'

 $cases.wrong_manifest_digest=Invoke-Gnx $installer @('verify','--manifest-sha256',('0' * 64))
 Assert-Case $cases.wrong_manifest_digest 1 'FAILED' 'MANIFEST_AUTHENTICATION_FAILED'

 $originalSignature=[IO.File]::ReadAllBytes($signature)
 $corruptSignature=[byte[]]$originalSignature.Clone()
 $corruptSignature[0]=$corruptSignature[0] -bxor 1
 [IO.File]::WriteAllBytes($signature,$corruptSignature)
 $cases.corrupt_signature=Invoke-Gnx $installer @('verify','--manifest-sha256',$digest)
 Assert-Case $cases.corrupt_signature 1 'FAILED' 'RELEASE_SIGNATURE_INVALID'
 [IO.File]::WriteAllBytes($signature,$originalSignature)

 $gnxExe=Join-Path $testRoot 'gnx.exe'
 $originalGnx=[IO.File]::ReadAllBytes($gnxExe)
 [IO.File]::WriteAllBytes($gnxExe,[byte[]](0x47,0x4e,0x58))
 $cases.corrupt_artifact=Invoke-Gnx $installer @('--manifest-sha256',$digest,'--rootfs',$rootfs)
 Assert-Case $cases.corrupt_artifact 1 'FAILED' 'ARTIFACT_MISMATCH'
 [IO.File]::WriteAllBytes($gnxExe,$originalGnx)

 if($SigningKey) {
  $signerPath=(Resolve-Path -LiteralPath $Signer).Path
  $keyPath=(Resolve-Path -LiteralPath $SigningKey).Path
  $originalManifest=[IO.File]::ReadAllBytes($manifest)
  $document=Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
  $document.schema=2
  [IO.File]::WriteAllText($manifest,($document | ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false))
  & $signerPath sign --private-key $keyPath --manifest $manifest --output $signature | Out-Null
  if($LASTEXITCODE){throw 'TEST_MANIFEST_SIGNING_FAILED'}
  $unsupportedDigest=(Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash.ToLowerInvariant()
  $cases.unsupported_schema=Invoke-Gnx $installer @('verify','--manifest-sha256',$unsupportedDigest)
  Assert-Case $cases.unsupported_schema 1 'FAILED' 'MANIFEST_SCHEMA_INVALID'
  [IO.File]::WriteAllBytes($manifest,$originalManifest)
 }

 [ordered]@{
  schema=1
  operation='verify-release-negative-matrix'
  state='READY'
  code='G0_RELEASE_MATRIX_PASSED'
  archive_sha256=(Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
  manifest_sha256=$digest
  cases=$cases
  next_action=$null
 } | ConvertTo-Json -Depth 12 -Compress
} finally {
 $resolved=[IO.Path]::GetFullPath($testRoot)
 $temp=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\'
 if(!$resolved.StartsWith($temp,[StringComparison]::OrdinalIgnoreCase)){throw 'UNSAFE_TEMP_CLEANUP_TARGET'}
 Remove-Item -LiteralPath $resolved -Recurse -Force
}
