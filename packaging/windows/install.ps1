[CmdletBinding()]
param(
 [Parameter(Mandatory)][string]$ManifestSha256,
 [Parameter(Mandatory)][string]$Rootfs,
 [Parameter(Mandatory)][string]$RootfsSha256,
 [Parameter(Mandatory)][PSCredential]$RuntimeCredential,
 [string]$Bundle=(Join-Path $PSScriptRoot '../../dist')
)
# Development bootstrap. The supplied manifest hash must come from a trusted channel.
# Account rights must be provisioned by the host administrator before installation.
$ErrorActionPreference='Stop'
$principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (!$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Elevation required'}
if ($RuntimeCredential.UserName -ne '.\gnx-runtime'){throw 'Use the dedicated .\gnx-runtime account'}
if (!(Get-LocalUser gnx-runtime -ErrorAction SilentlyContinue)){throw 'Provision gnx-runtime with service logon and denied interactive/network/remote logon first'}
if (Get-Service GNXRuntime -ErrorAction SilentlyContinue){throw 'Existing installation detected; automated updates are not implemented'}
$Bundle=(Resolve-Path -LiteralPath $Bundle).Path
if ($ManifestSha256 -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash "$Bundle/manifest.json").Hash -ne $ManifestSha256){throw 'Manifest authentication failed'}
$m=Get-Content "$Bundle/manifest.json" -Raw | ConvertFrom-Json
foreach($name in @('gnx.exe','gnx-service.exe','gnx-linux-bundle.tar')){if((Get-FileHash "$Bundle/$name").Hash -ne $m.artifacts.$name){throw "Artifact verification failed: $name"}}
if($RootfsSha256 -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash -LiteralPath $Rootfs).Hash -ne $RootfsSha256){throw 'Rootfs verification failed'}
$data='C:\ProgramData\GNX'
$bin='C:\Program Files\GNX'
New-Item -ItemType Directory -Force $data,$bin | Out-Null
$sid=(Get-LocalUser gnx-runtime).SID.Value
& icacls $data /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' "*$($sid):(OI)(CI)F" | Out-Null
if($LASTEXITCODE){throw 'Private ACL failed'}
[Security.Principal.WindowsIdentity]::GetCurrent().User.Value | Set-Content "$data/operator.sid"
Copy-Item "$Bundle/gnx.exe","$Bundle/gnx-service.exe" $bin
Copy-Item -LiteralPath $Rootfs -Destination "$data/rootfs.tar"
Copy-Item "$Bundle/gnx-linux-bundle.tar" "$data/bundle.tar"
New-Service -Name GNXRuntime -BinaryPathName ('"'+$bin+'\gnx-service.exe"') -Credential $RuntimeCredential -StartupType Automatic | Out-Null
& sc.exe failure GNXRuntime reset= 86400 actions= restart/5000/restart/15000/restart/60000 | Out-Null
if($LASTEXITCODE){throw 'Recovery configuration failed'}
Start-Service GNXRuntime
Write-Output 'Development service installed. Bootstrap and doctor must be checked before declaring readiness.'
