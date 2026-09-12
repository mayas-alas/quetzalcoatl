$ErrorActionPreference='Stop'
try {
 if ($env:GNX_UNINSTALL_CONFIRM -ne 'REMOVE-GNX-AND-DATA'){throw 'CONFIRMATION_REQUIRED: use --confirm REMOVE-GNX-AND-DATA'}
 $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
 if (!$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'ELEVATION_REQUIRED'}
 $data='C:\ProgramData\GNX\runtime'
 $installRoot='C:\Program Files\GNX'
 $serviceBinary="$installRoot\runtime\gnx-service.exe"
 $receiptPath="$installRoot\install-receipt.json"
 if (!(Test-Path -LiteralPath $receiptPath)){throw 'OWNERSHIP_RECEIPT_MISSING: refusing to remove unverified state'}
 $receipt=Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
 if ($receipt.schema -ne 1 -or $receipt.service -ne 'GNXRuntime' -or $receipt.account -ne '.\gnx-runtime' -or $receipt.distro -ne 'GNX' -or $receipt.install_root -ne $installRoot -or $receipt.data_root -ne $data){throw 'OWNERSHIP_RECEIPT_INVALID'}
 $service=Get-CimInstance Win32_Service -Filter "Name='GNXRuntime'"
 if (!$service){throw 'OWNED_SERVICE_MISSING: refusing partial removal'}
 if ($service.StartName -ne '.\gnx-runtime' -or $service.PathName.Trim('"') -ne $serviceBinary){throw 'SERVICE_OWNERSHIP_MISMATCH'}
 if (!(Get-LocalUser gnx-runtime -ErrorAction SilentlyContinue)){throw 'OWNED_ACCOUNT_MISSING: refusing partial removal'}
 $resultPath="$data/uninstall-result.json"
 if (Test-Path -LiteralPath $resultPath){Remove-Item -LiteralPath $resultPath -Force}
 'GNX-UNINSTALL-1' | Set-Content -LiteralPath "$installRoot/uninstall.request" -Encoding ascii
 if ($service.State -ne 'Running'){
  Start-Service GNXRuntime
  (Get-Service GNXRuntime).WaitForStatus('Running',[TimeSpan]::FromSeconds(30))
 }
 Stop-Service GNXRuntime
 (Get-Service GNXRuntime).WaitForStatus('Stopped',[TimeSpan]::FromSeconds(90))
 if (!(Test-Path -LiteralPath $resultPath)){throw 'WSL_UNREGISTER_RESULT_MISSING: preserve remaining state'}
 $runtimeResult=Get-Content -LiteralPath $resultPath -Raw | ConvertFrom-Json
 if ($runtimeResult.state -ne 'READY' -or $runtimeResult.code -ne 'WSL_UNREGISTERED'){throw "WSL_UNREGISTER_FAILED: preserve remaining state"}
 Remove-Item -LiteralPath "$installRoot/uninstall.request" -Force
 & sc.exe delete GNXRuntime | Out-Null
 if ($LASTEXITCODE){throw 'SERVICE_DELETE_FAILED: preserve remaining state'}
 $deadline=[DateTime]::UtcNow.AddSeconds(30)
 do {
  Start-Sleep -Milliseconds 250
  $remaining=Get-CimInstance Win32_Service -Filter "Name='GNXRuntime'" -ErrorAction SilentlyContinue
 } while ($remaining -and [DateTime]::UtcNow -lt $deadline)
 if ($remaining){throw 'SERVICE_DELETE_TIMEOUT: preserve remaining state'}
 Remove-LocalUser gnx-runtime
 Remove-Item -LiteralPath $data -Recurse -Force
 if (Test-Path -LiteralPath $installRoot){Remove-Item -LiteralPath $installRoot -Recurse -Force}
 @{schema=1;operation='uninstall';state='READY';code='UNINSTALLED';removed=@('GNXRuntime','.\gnx-runtime','WSL:GNX',$data,$installRoot)} | ConvertTo-Json -Compress
 exit 0
} catch {
 @{schema=1;operation='uninstall';state='FAILED';code=$_.Exception.Message} | ConvertTo-Json -Compress
 exit 1
}
