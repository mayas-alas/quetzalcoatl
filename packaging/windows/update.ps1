$ErrorActionPreference='Stop'

function Wait-GnxJson([string]$Path,[int]$Seconds) {
 $deadline=[DateTime]::UtcNow.AddSeconds($Seconds)
 do {
  if (Test-Path -LiteralPath $Path){return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)}
  Start-Sleep -Milliseconds 500
 } while ([DateTime]::UtcNow -lt $deadline)
 throw "TRANSACTION_TIMEOUT: $([IO.Path]::GetFileName($Path))"
}

$operation=$env:GNX_INSTALL_ACTION
$installRoot='C:\Program Files\GNX'
$data='C:\ProgramData\GNX\runtime'
$bin=Join-Path $installRoot 'runtime'
$cli=Join-Path $installRoot 'gnx.exe'
$transactionRoot=Join-Path $installRoot 'release-transaction'
$previousRoot=Join-Path $installRoot 'release-previous'
$linuxAwaitingCommit=$false
$windowsBackup=$false

try {
 if ($operation -notin @('update','rollback')){throw 'RELEASE_OPERATION_INVALID'}
 $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
 if (!$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'ELEVATION_REQUIRED'}
 $bundle=(Resolve-Path -LiteralPath $env:GNX_INSTALL_BUNDLE).Path
 $manifestHash=$env:GNX_INSTALL_MANIFEST
 if ($manifestHash -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash "$bundle/manifest.json").Hash -ne $manifestHash){throw 'MANIFEST_AUTHENTICATION_FAILED'}
 $manifest=Get-Content "$bundle/manifest.json" -Raw | ConvertFrom-Json
 if ($manifest.schema -ne 1 -or $manifest.release_serial -lt 1 -or $manifest.signing_key_id -notmatch '^[a-f0-9]{64}$'){throw 'MANIFEST_SCHEMA_INVALID'}
 foreach($name in @('gnx.exe','gnx-service.exe','gnx-install.exe','gnx-linux-bundle.tar')) {
  $hash=$manifest.artifacts.$name
  if ($hash -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash "$bundle/$name").Hash -ne $hash){throw "ARTIFACT_MISMATCH: $name"}
 }
 $receiptPath=Join-Path $installRoot 'install-receipt.json'
 if (!(Test-Path -LiteralPath $receiptPath)){throw 'OWNERSHIP_RECEIPT_MISSING'}
 $receipt=Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
 if ($receipt.schema -ne 1 -or $receipt.release_serial -lt 1 -or $receipt.service -ne 'GNXRuntime' -or $receipt.account -ne '.\gnx-runtime' -or $receipt.distro -ne 'GNX' -or $receipt.install_root -ne $installRoot -or $receipt.data_root -ne $data){throw 'OWNERSHIP_RECEIPT_INVALID'}
 if ($operation -eq 'update' -and $manifest.release_serial -le $receipt.release_serial){throw 'RELEASE_NOT_NEWER'}
 if ($operation -eq 'rollback' -and $manifest.release_serial -ge $receipt.release_serial){throw 'RELEASE_NOT_OLDER'}
 $service=Get-CimInstance Win32_Service -Filter "Name='GNXRuntime'"
 if (!$service -or $service.StartName -ne '.\gnx-runtime' -or $service.PathName.Trim('"') -ne "$bin\gnx-service.exe"){throw 'SERVICE_OWNERSHIP_MISMATCH'}
 if (Test-Path -LiteralPath $transactionRoot){throw 'RELEASE_TRANSACTION_PENDING'}
 if (Test-Path -LiteralPath $previousRoot){throw 'RELEASE_PREVIOUS_PENDING'}
 foreach($name in @('release-result.json','release-commit-result.json','release-rollback-result.json')) {
  $path=Join-Path $data $name
  if (Test-Path -LiteralPath $path){Remove-Item -LiteralPath $path -Force}
 }
 if ($service.State -ne 'Stopped'){
  Stop-Service GNXRuntime
  (Get-Service GNXRuntime).WaitForStatus('Stopped',[TimeSpan]::FromSeconds(90))
 }
 New-Item -ItemType Directory -Path $previousRoot,$transactionRoot -Force | Out-Null
 Copy-Item -LiteralPath $cli -Destination "$previousRoot\gnx.exe"
 Copy-Item -LiteralPath "$bin\gnx.exe" -Destination "$previousRoot\runtime-gnx.exe"
 Copy-Item -LiteralPath "$bin\gnx-service.exe" -Destination "$previousRoot\gnx-service.exe"
 $windowsBackup=$true
 Copy-Item -LiteralPath "$bundle/gnx-linux-bundle.tar" -Destination "$transactionRoot\bundle.tar"
 [ordered]@{schema=1;operation=$operation;release_serial=[long]$manifest.release_serial;linux_sha256=$manifest.artifacts.'gnx-linux'} | ConvertTo-Json -Compress | Set-Content -LiteralPath "$transactionRoot\transaction.json" -Encoding ascii
 Copy-Item -LiteralPath "$bundle/gnx.exe" -Destination $cli -Force
 Copy-Item -LiteralPath "$bundle/gnx.exe" -Destination "$bin\gnx.exe" -Force
 Copy-Item -LiteralPath "$bundle/gnx-service.exe" -Destination "$bin\gnx-service.exe" -Force
 Start-Service GNXRuntime
 (Get-Service GNXRuntime).WaitForStatus('Running',[TimeSpan]::FromSeconds(30))
 $releaseResult=Wait-GnxJson (Join-Path $data 'release-result.json') 600
 if ($releaseResult.state -ne 'READY' -or $releaseResult.code -ne 'RELEASE_CANDIDATE_READY' -or $releaseResult.release_serial -ne $manifest.release_serial -or $releaseResult.linux_sha256 -ne $manifest.artifacts.'gnx-linux'){
  if ($releaseResult.rollback_code -eq 'ROLLBACK_FAILED'){throw 'RELEASE_ROLLBACK_FAILED'}
  throw 'RELEASE_CANDIDATE_REJECTED'
 }
 $linuxAwaitingCommit=$true
 $intent='schema=1'+[Environment]::NewLine+'instance="gnx"'+[Environment]::NewLine+'node="compute"'+[Environment]::NewLine+'[network]'+[Environment]::NewLine+'subnet="10.90.0.0/24"'
 $doctorText=$intent | & "$bin\gnx.exe" doctor --stdin
 $doctorExit=$LASTEXITCODE
 $doctor=$doctorText | ConvertFrom-Json
 if ($doctorExit -ne 0 -or $doctor.state -ne 'READY'){throw 'UPDATED_DOCTOR_FAILED'}
 $statusText=$intent | & "$bin\gnx.exe" status --stdin
 $statusExit=$LASTEXITCODE
 $status=$statusText | ConvertFrom-Json
 if ($statusExit -ne 0 -or $status.state -ne 'READY'){throw 'UPDATED_STATUS_FAILED'}
 'GNX-RELEASE-COMMIT-1' | Set-Content -LiteralPath "$transactionRoot\commit.request" -Encoding ascii
 $intent | & "$bin\gnx.exe" doctor --stdin | Out-Null
 $commitResult=Wait-GnxJson (Join-Path $data 'release-commit-result.json') 90
 if ($commitResult.state -ne 'READY' -or $commitResult.code -ne 'RELEASE_COMMITTED'){throw 'RELEASE_COMMIT_FAILED'}
 [ordered]@{schema=1;version=$manifest.version;release_serial=[long]$manifest.release_serial;manifest_sha256=$manifestHash.ToLowerInvariant();service='GNXRuntime';account='.\gnx-runtime';distro='GNX';install_root=$installRoot;data_root=$data} | ConvertTo-Json -Compress | Set-Content -LiteralPath $receiptPath -Encoding ascii
 Remove-Item -LiteralPath $transactionRoot -Recurse -Force
 Remove-Item -LiteralPath $previousRoot -Recurse -Force
 @{schema=1;operation=$operation;state='READY';code=$(if($operation -eq 'update'){'UPDATED'}else{'ROLLED_BACK'});previous_serial=[long]$receipt.release_serial;release_serial=[long]$manifest.release_serial;revision=$status.revision} | ConvertTo-Json -Compress
 exit 0
} catch {
 $failure=$_.Exception.Message
 $rollback='NOT_NEEDED'
 try {
  if ($windowsBackup){
   if ($linuxAwaitingCommit){
    $rollback='FAILED'
    Remove-Item -LiteralPath "$transactionRoot\commit.request" -Force -ErrorAction SilentlyContinue
    'GNX-RELEASE-ROLLBACK-1' | Set-Content -LiteralPath "$transactionRoot\rollback.request" -Encoding ascii
    $serviceNow=Get-Service GNXRuntime -ErrorAction SilentlyContinue
    if ($serviceNow -and $serviceNow.Status -ne 'Running'){Start-Service GNXRuntime}
    $intent | & "$bin\gnx.exe" doctor --stdin | Out-Null
    $rollbackResult=Wait-GnxJson (Join-Path $data 'release-rollback-result.json') 90
    if ($rollbackResult.state -ne 'READY' -or $rollbackResult.code -ne 'RELEASE_ROLLED_BACK'){throw 'Linux release rollback failed'}
    $rollback='READY'
   } else {
    $candidateResultPath=Join-Path $data 'release-result.json'
    if (Test-Path -LiteralPath $candidateResultPath){
     $candidateResult=Get-Content -LiteralPath $candidateResultPath -Raw | ConvertFrom-Json
     if ($candidateResult.rollback_code -eq 'ROLLBACK_FAILED'){throw 'Linux automatic rollback failed'}
    }
    $rollback='READY'
   }
   if ($rollback -eq 'READY'){
    $serviceNow=Get-Service GNXRuntime -ErrorAction SilentlyContinue
    if ($serviceNow -and $serviceNow.Status -ne 'Stopped'){
     Stop-Service GNXRuntime
     $serviceNow.WaitForStatus('Stopped',[TimeSpan]::FromSeconds(90))
    }
    Copy-Item -LiteralPath "$previousRoot\gnx.exe" -Destination $cli -Force
    Copy-Item -LiteralPath "$previousRoot\runtime-gnx.exe" -Destination "$bin\gnx.exe" -Force
    Copy-Item -LiteralPath "$previousRoot\gnx-service.exe" -Destination "$bin\gnx-service.exe" -Force
    Remove-Item -LiteralPath $transactionRoot -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $previousRoot -Recurse -Force
    Start-Service GNXRuntime
   }
  }
 } catch {
  $rollback='FAILED'
 }
 $code=if($rollback -eq 'FAILED'){'RELEASE_ROLLBACK_FAILED'}else{$failure}
 @{schema=1;operation=$operation;state='FAILED';code=$code;candidate_error=$failure;rollback=$rollback} | ConvertTo-Json -Compress
 exit 1
}
