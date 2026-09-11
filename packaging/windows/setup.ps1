$ErrorActionPreference='Stop'
try {
 $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
 if (!$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'ELEVATION_REQUIRED'}
 $bundle=(Resolve-Path -LiteralPath $env:GNX_INSTALL_BUNDLE).Path
 $manifestHash=$env:GNX_INSTALL_MANIFEST
 if ($manifestHash -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash "$bundle/manifest.json").Hash -ne $manifestHash){throw 'MANIFEST_AUTHENTICATION_FAILED'}
 $manifest=Get-Content "$bundle/manifest.json" -Raw | ConvertFrom-Json
 if ($manifest.schema -ne 1){throw 'MANIFEST_SCHEMA_INVALID'}
 foreach($name in @('gnx.exe','gnx-service.exe','gnx-install.exe','gnx-linux-bundle.tar')) {
  $hash=$manifest.artifacts.$name
  if ($hash -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash "$bundle/$name").Hash -ne $hash){throw "ARTIFACT_MISMATCH: $name"}
 }
 $rootfs=(Resolve-Path -LiteralPath $env:GNX_INSTALL_ROOTFS).Path
 if ($manifest.rootfs.sha256 -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash -LiteralPath $rootfs).Hash -ne $manifest.rootfs.sha256){throw 'ROOTFS_MISMATCH'}
 & wsl.exe --status | Out-Null
 if ($LASTEXITCODE){throw 'WSL_REQUIRED: Install WSL 2 machine-wide and reboot before retrying.'}
 # Refuse updates until an authenticated rollback of account, service and WSL is available.
 $data='C:\ProgramData\GNX\runtime'
 $installRoot='C:\Program Files\GNX'
 $bin=Join-Path $installRoot 'runtime'
 $cli=Join-Path $installRoot 'gnx.exe'
 $pending=Join-Path $data 'install.pending'
 $resume=(Test-Path -LiteralPath $pending) -and ((Get-Content -LiteralPath $pending -Raw).Trim() -eq 'GNX-INSTALL-1')
 $service=Get-CimInstance Win32_Service -Filter "Name='GNXRuntime'"
 if ($service -and !($resume -and $service.State -eq 'Stopped' -and $service.StartName -eq '.\gnx-runtime' -and $service.PathName -eq ('"'+$bin+'\gnx-service.exe"'))){throw 'EXISTING_SERVICE: preserve this installation; in-place upgrade is not accepted yet.'}
 if ((Get-LocalUser gnx-runtime -ErrorAction SilentlyContinue) -and !$resume){throw 'EXISTING_ACCOUNT: refusing to rotate an unowned account credential.'}
 if ((Test-Path -LiteralPath $data) -and !$resume){throw 'EXISTING_STATE: preserve and inspect the previous installation.'}
 New-Item -ItemType Directory -Path $data -Force | Out-Null
 & icacls.exe $data /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' | Out-Null
 if ($LASTEXITCODE){throw 'PRIVATE_ACL_FAILED'}
 'GNX-INSTALL-1' | Set-Content -LiteralPath $pending -Encoding ascii
 New-Item -ItemType Directory -Path $installRoot,$bin -Force | Out-Null
 & icacls.exe $installRoot /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-32-545:(OI)(CI)RX' | Out-Null
 if ($LASTEXITCODE){throw 'INSTALL_ROOT_ACL_FAILED'}
 & icacls.exe $bin /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-32-545:(OI)(CI)RX' | Out-Null
 if ($LASTEXITCODE){throw 'BINARY_ACL_FAILED'}
 Copy-Item "$bundle/gnx.exe" $cli -Force
 Copy-Item "$bundle/gnx.exe","$bundle/gnx-service.exe" $bin -Force
 $result=& "$bin/gnx-service.exe" --install
 if ($LASTEXITCODE){throw "ACCOUNT_INSTALL_FAILED: $result"}
 $sid=($result | ConvertFrom-Json).account_sid
 if ($sid -notmatch '^S-1-[0-9-]+$'){throw 'ACCOUNT_SID_INVALID'}
 & icacls.exe $data /grant:r "*$($sid):(OI)(CI)F" | Out-Null
 if ($LASTEXITCODE){throw 'ACCOUNT_ACL_FAILED'}
 [Security.Principal.WindowsIdentity]::GetCurrent().User.Value | Set-Content "$data/operator.sid" -Encoding ascii
 Copy-Item -LiteralPath $rootfs -Destination "$data/rootfs.tar"
 Copy-Item "$bundle/gnx-linux-bundle.tar" "$data/bundle.tar"
 Start-Service GNXRuntime
 $intent='schema=1'+[Environment]::NewLine+'instance="gnx"'+[Environment]::NewLine+'node="compute"'+[Environment]::NewLine+'[network]'+[Environment]::NewLine+'subnet="10.90.0.0/24"'
 $deadline=[DateTime]::UtcNow.AddMinutes(8)
 do {
  Start-Sleep -Seconds 3
  $report=$intent | & "$bin/gnx.exe" doctor --stdin | ConvertFrom-Json
  if ($report.code -notin @('BROKER_UNAVAILABLE','WSL_RUNTIME_UNAVAILABLE')){break}
 } while ([DateTime]::UtcNow -lt $deadline)
 if ($report.code -in @('BROKER_UNAVAILABLE','WSL_RUNTIME_UNAVAILABLE')){throw 'BOOTSTRAP_NOT_READY: preserve ProgramData/GNX and inspect the service.'}
 Remove-Item -LiteralPath $pending
 $report | ConvertTo-Json -Depth 8
 exit $(if($report.state -eq 'READY'){0}else{2})
} catch {
 @{schema=1;operation='install';state='FAILED';code=$_.Exception.Message} | ConvertTo-Json -Compress
 exit 1
}
