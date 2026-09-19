[CmdletBinding()]
param([string]$BuildDistro='Ubuntu-24.04')
$ErrorActionPreference='Stop'
Push-Location (Join-Path $PSScriptRoot '../..')
try {
 cargo test --locked --all-targets
 if ($LASTEXITCODE) { throw 'Windows tests failed' }
 cargo build --release --locked --bins
 if ($LASTEXITCODE) { throw 'Windows build failed' }
 New-Item -ItemType Directory -Force dist | Out-Null
 New-Item -ItemType Directory -Force dist/assets | Out-Null
 Copy-Item target/release/gnx.exe,target/release/gnx-service.exe,target/release/gnx-setup.exe dist -Force
 Copy-Item packaging/windows/uninstall.ps1,packaging/windows/uninstall-checklist.md dist -Force
 Copy-Item packaging/windows/assets/branding-install-logo.ico,packaging/windows/assets/branding-install-logo.png,packaging/windows/assets/banner-install-side.png,packaging/windows/assets/bg-installer-banner.png,packaging/windows/assets/tray-icon.ico,packaging/windows/assets/tray-icon.png dist/assets -Force
 $linuxPath=(& wsl -d $BuildDistro --exec wslpath -a (Get-Location).Path).Trim()
 & wsl -d $BuildDistro --cd $linuxPath --exec sh -c 'set -eu; cargo_bin=$(command -v cargo); "$cargo_bin" test --locked --all-targets --target-dir /tmp/gnx-build && "$cargo_bin" build --release --locked --bin gnx --target-dir /tmp/gnx-build && cp /tmp/gnx-build/release/gnx dist/gnx-linux && sh packaging/linux/build.sh'
 if ($LASTEXITCODE) { throw 'Linux build failed' }
 $artifacts=[ordered]@{}
 foreach($name in @('gnx.exe','gnx-service.exe','gnx-setup.exe','gnx-linux','gnx-linux-bundle.tar','gnx-linux.run')) {
   $path = Join-Path (Resolve-Path dist).Path $name
   if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing release artifact: $name" }
   $artifacts[$name]=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
 }
 # A hash-only manifest is intentionally not a release trust anchor. The
 # signing/promotion step must replace status with sealed and attach the
 # authenticated release metadata before install.ps1 may consume it.
 $manifest=[ordered]@{schema=1;version='0.3.1';platform='windows+linux-amd64';status='unsealed';artifacts=$artifacts}
 $json=$manifest | ConvertTo-Json -Depth 5 -Compress
 [IO.File]::WriteAllText((Join-Path (Resolve-Path dist).Path 'manifest.json'), $json, [Text.UTF8Encoding]::new($false))
 Get-FileHash -LiteralPath (Join-Path (Resolve-Path dist).Path 'manifest.json') -Algorithm SHA256
} finally {Pop-Location}
