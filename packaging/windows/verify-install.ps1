$ErrorActionPreference='Stop'
$bundle=(Resolve-Path (Join-Path $PSScriptRoot '../../dist')).Path
$digest=(Get-FileHash "$bundle/manifest.json").Hash
& "$bundle/gnx-install.exe" --manifest-sha256 $digest --rootfs "$bundle/gnx-wsl-rootfs.tar.gz" *> "$bundle/windows-install-result.txt"
$result=$LASTEXITCODE
@{exit=$result;service=(Get-Service GNXRuntime -ErrorAction SilentlyContinue).Status.ToString()} | ConvertTo-Json | Set-Content "$bundle/windows-install-status.json"
exit $result
