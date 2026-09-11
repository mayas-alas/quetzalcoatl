[CmdletBinding()]
param([string]$BuildDistro='Ubuntu-24.04', [string]$Rootfs)
$ErrorActionPreference='Stop'
Push-Location (Join-Path $PSScriptRoot '../..')
try {
 cargo test --locked --all-targets
 if ($LASTEXITCODE) { throw 'Windows tests failed' }
 cargo build --release --locked --bins
 if ($LASTEXITCODE) { throw 'Windows build failed' }
 New-Item -ItemType Directory -Force dist | Out-Null
 Copy-Item target/release/gnx.exe,target/release/gnx-service.exe,target/release/gnx-install.exe dist -Force
 $linuxPath=(& wsl -d $BuildDistro --exec wslpath -a (Get-Location).Path).Trim()
 & wsl -d $BuildDistro --cd $linuxPath --exec sh -c '$HOME/.cargo/bin/cargo test --locked --all-targets --target-dir /tmp/gnx-build && $HOME/.cargo/bin/cargo build --release --locked --bin gnx --target-dir /tmp/gnx-build && cp /tmp/gnx-build/release/gnx dist/gnx-linux && sh packaging/linux/build.sh'
 if ($LASTEXITCODE) { throw 'Linux build failed' }
 $artifacts=@{}
 foreach($name in @('gnx.exe','gnx-service.exe','gnx-install.exe','gnx-linux','gnx-linux-bundle.tar','gnx-linux.run')) {$artifacts[$name]=(Get-FileHash "dist/$name" -Algorithm SHA256).Hash.ToLowerInvariant()}
 $version=(cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages[0].version
 $manifest=@{schema=1;version=$version;artifacts=$artifacts}
 if ($Rootfs) { $manifest.rootfs=@{name=[IO.Path]::GetFileName($Rootfs);sha256=(Get-FileHash -LiteralPath $Rootfs).Hash.ToLowerInvariant()} }
 $manifest | ConvertTo-Json -Depth 5 | Set-Content dist/manifest.json -Encoding ascii
 Compress-Archive -Path dist/gnx.exe,dist/gnx-service.exe,dist/gnx-install.exe,dist/gnx-linux-bundle.tar,dist/manifest.json -DestinationPath "dist/gnx-$version-windows-amd64.zip" -Force
 Get-FileHash dist/manifest.json -Algorithm SHA256
} finally {Pop-Location}
