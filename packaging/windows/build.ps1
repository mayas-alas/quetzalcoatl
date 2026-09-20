[CmdletBinding()]
param(
 [string]$BuildDistro='gnx-node',
 [string]$Rootfs=$env:GNX_ROOTFS,
 [string]$RootfsSource=$env:GNX_ROOTFS_SOURCE,
 [string]$RootfsVersion=$env:GNX_ROOTFS_VERSION,
 [string]$RootfsArch=$env:GNX_ROOTFS_ARCH
)
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
 Copy-Item packaging/windows/install.ps1,packaging/windows/uninstall.ps1,packaging/windows/uninstall-checklist.md,packaging/windows/gnx-progress-ui.ps1 dist -Force
 Copy-Item packaging/windows/assets/branding-install-logo.ico,packaging/windows/assets/branding-install-logo.png,packaging/windows/assets/banner-install-side.png,packaging/windows/assets/bg-installer-banner.png,packaging/windows/assets/tray-icon.ico,packaging/windows/assets/tray-icon.png dist/assets -Force
 $linuxPath=(& wsl -d $BuildDistro --exec wslpath -a (Get-Location).Path).Trim()
 & wsl -d $BuildDistro --cd $linuxPath --exec bash -lc 'set -eu; cargo_bin=$(command -v cargo); "$cargo_bin" test --locked --all-targets --target-dir /tmp/gnx-build && "$cargo_bin" build --release --locked --bin gnx --target-dir /tmp/gnx-build && cp /tmp/gnx-build/release/gnx dist/gnx-linux && sh packaging/linux/build.sh'
 if ($LASTEXITCODE) { throw 'Linux build failed' }
 if ([string]::IsNullOrWhiteSpace($Rootfs) -or -not (Test-Path -LiteralPath $Rootfs -PathType Leaf)) { throw 'A verified rootfs is required to build a complete candidate.' }
 if ([string]::IsNullOrWhiteSpace($RootfsSource) -or [string]::IsNullOrWhiteSpace($RootfsVersion) -or [string]::IsNullOrWhiteSpace($RootfsArch)) { throw 'Rootfs source, version and architecture metadata are required.' }
 $rootfsSha256=(Get-FileHash -LiteralPath $Rootfs -Algorithm SHA256).Hash.ToLowerInvariant()
 $rootfsLinuxPath=(& wsl -d $BuildDistro --exec wslpath -a (Resolve-Path -LiteralPath $Rootfs).Path).Trim()
 & wsl -d $BuildDistro --cd $linuxPath --exec sh packaging/linux/validate-rootfs.sh --rootfs $rootfsLinuxPath --sha256 $rootfsSha256 --source $RootfsSource --version $RootfsVersion --arch $RootfsArch --metadata dist/rootfs.metadata.json
 if ($LASTEXITCODE) { throw 'Rootfs validation failed' }
 $artifacts=[ordered]@{}
 foreach($name in @('gnx.exe','gnx-service.exe','gnx-setup.exe','gnx-linux','gnx-linux-bundle.tar','gnx-linux.run')) {
   $path = Join-Path (Resolve-Path dist).Path $name
   if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing release artifact: $name" }
   $artifacts[$name]=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
 }
 # A hash-only manifest is intentionally not a release trust anchor. The
 # signing/promotion step must replace status with sealed and attach the
 # authenticated release metadata before install.ps1 may consume it.
 $manifest=[ordered]@{schema=1;version='0.3.1';platform='windows+linux-amd64';status='unsealed';rootfs_sha256=$rootfsSha256;artifacts=$artifacts}
 $json=$manifest | ConvertTo-Json -Depth 5 -Compress
 [IO.File]::WriteAllText((Join-Path (Resolve-Path dist).Path 'manifest.json'), $json, [Text.UTF8Encoding]::new($false))
 Get-FileHash -LiteralPath (Join-Path (Resolve-Path dist).Path 'manifest.json') -Algorithm SHA256
} finally {Pop-Location}
