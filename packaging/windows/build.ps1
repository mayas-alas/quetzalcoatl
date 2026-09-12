[CmdletBinding()]
param(
 [Parameter(Mandatory)][string]$Rootfs,
 [Parameter(Mandatory)][string]$SigningKey
)
$ErrorActionPreference='Stop'
Push-Location (Join-Path $PSScriptRoot '../..')
try {
 cargo test --locked --all-targets
 if ($LASTEXITCODE) { throw 'Windows tests failed' }
 cargo build --release --locked --bins
 if ($LASTEXITCODE) { throw 'Windows build failed' }
 New-Item -ItemType Directory -Force dist | Out-Null
 $dist=(Resolve-Path dist).Path
 $rootfsSource=(Resolve-Path -LiteralPath $Rootfs).Path
 $rootfsName=[IO.Path]::GetFileName($rootfsSource)
 $rootfsTarget=Join-Path $dist $rootfsName
 if ($rootfsSource -cne $rootfsTarget){Copy-Item -LiteralPath $rootfsSource -Destination $rootfsTarget -Force}
 Copy-Item target/release/gnx.exe,target/release/gnx-service.exe,target/release/gnx-install.exe dist -Force
 $sysroot=(rustc --print sysroot).Trim()
 $linuxLinker=Join-Path $sysroot 'lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe'
 if (!(Test-Path -LiteralPath $linuxLinker)){throw 'Linux linker rust-lld is missing'}
 $oldLinker=$env:CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER
 try {
  $env:CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=$linuxLinker
  cargo build --release --locked --target x86_64-unknown-linux-musl --bin gnx
  if ($LASTEXITCODE) { throw 'Linux musl build failed' }
 } finally {
  $env:CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=$oldLinker
 }
 Copy-Item target/x86_64-unknown-linux-musl/release/gnx dist/gnx-linux -Force
 $stage=Join-Path ([IO.Path]::GetTempPath()) ("gnx-bundle-"+[guid]::NewGuid().ToString('N'))
 New-Item -ItemType Directory -Path "$stage/usr/local/bin","$stage/usr/local/share/gnx","$stage/etc/gnx" -Force | Out-Null
 try {
  Copy-Item dist/gnx-linux "$stage/usr/local/bin/gnx"
  Copy-Item runtime "$stage/usr/local/share/gnx/runtime" -Recurse
  Copy-Item config/gnx.example.toml "$stage/etc/gnx/gnx.example.toml"
  $epoch=[DateTime]::SpecifyKind([DateTime]'2000-01-01T00:00:00',[DateTimeKind]::Utc)
  Get-ChildItem -LiteralPath $stage -Recurse | ForEach-Object {$_.LastWriteTimeUtc=$epoch}
  (Get-Item -LiteralPath $stage).LastWriteTimeUtc=$epoch
  & tar.exe -C $stage -cf dist/gnx-linux-bundle.tar .
  if ($LASTEXITCODE){throw 'Linux bundle creation failed'}
 } finally {
  $resolvedStage=[IO.Path]::GetFullPath($stage)
  $resolvedTemp=[IO.Path]::GetFullPath([IO.Path]::GetTempPath())
  if (!$resolvedStage.StartsWith($resolvedTemp,[StringComparison]::OrdinalIgnoreCase)){throw 'Unsafe temporary bundle path'}
  Remove-Item -LiteralPath $resolvedStage -Recurse -Force
 }
 $bundleHash=(Get-FileHash dist/gnx-linux-bundle.tar -Algorithm SHA256).Hash.ToLowerInvariant()
 $header=@'
#!/bin/sh
set -eu
[ "$(id -u)" = 0 ] || { echo 'Run as root' >&2; exit 2; }
umask 077
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
tail -n +15 "$0" > "$stage/bundle.tar"
echo '__BUNDLE_HASH__  ' "$stage/bundle.tar" | sha256sum -c - >&2
tar -xf "$stage/bundle.tar" -C /
[ -e /etc/gnx/gnx.toml ] || cp /etc/gnx/gnx.example.toml /etc/gnx/gnx.toml
chmod 600 /etc/gnx/gnx.toml
printf '%s\n' 'GNX installed. Run gnx doctor, then gnx apply to configure this node.' >&2
exit 0
# payload
'@
 $header=$header.Replace('__BUNDLE_HASH__',$bundleHash).Replace("`r`n","`n")+"`n"
 $run=$dist+'\gnx-linux.run'
 [IO.File]::WriteAllText($run,$header,[Text.UTF8Encoding]::new($false))
 $output=[IO.File]::Open($run,[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::None)
 try {
  $input=[IO.File]::OpenRead((Resolve-Path dist/gnx-linux-bundle.tar).Path)
  try {$input.CopyTo($output)} finally {$input.Dispose()}
 } finally {$output.Dispose()}
 $artifacts=[ordered]@{}
 foreach($name in @('gnx.exe','gnx-service.exe','gnx-install.exe','gnx-linux','gnx-linux-bundle.tar','gnx-linux.run')) {$artifacts[$name]=(Get-FileHash "dist/$name" -Algorithm SHA256).Hash.ToLowerInvariant()}
 $version=(cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages[0].version
 [long]$releaseSerial=(cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages[0].metadata.gnx.release_serial
 if ($releaseSerial -lt 1){throw 'Cargo release serial is invalid'}
 $keyInfo=& target/release/gnx-sign.exe public-key --private-key (Resolve-Path -LiteralPath $SigningKey).Path | ConvertFrom-Json
 if ($LASTEXITCODE -or $keyInfo.state -ne 'READY'){throw 'Release signing key is invalid'}
 $trusted=(Get-Content packaging/release/trusted-release.pub -Raw).Trim()
 if ($keyInfo.public_key -cne $trusted){throw 'Release signing key does not match the public key pinned in gnx-install.exe'}
 $manifest=[ordered]@{schema=1;version=$version;release_serial=$releaseSerial;signing_key_id=$keyInfo.key_id;artifacts=$artifacts}
 $manifest.rootfs=[ordered]@{name=$rootfsName;sha256=(Get-FileHash -LiteralPath $rootfsTarget).Hash.ToLowerInvariant()}
 $manifest | ConvertTo-Json -Depth 5 | Set-Content dist/manifest.json -Encoding ascii
 & target/release/gnx-sign.exe sign --private-key (Resolve-Path -LiteralPath $SigningKey).Path --manifest dist/manifest.json --output dist/manifest.json.sig
 if ($LASTEXITCODE){throw 'Release manifest signing failed'}
 $manifestDigest=(Get-FileHash dist/manifest.json -Algorithm SHA256).Hash.ToLowerInvariant()
 $verified=& dist/gnx-install.exe verify --manifest-sha256 $manifestDigest | ConvertFrom-Json
 if ($LASTEXITCODE -or $verified.state -ne 'READY' -or $verified.code -ne 'RELEASE_AUTHENTIC' -or $verified.key_id -ne $keyInfo.key_id){throw 'Pinned installer rejected the signed release'}
 $packagePaths=@('dist/gnx.exe','dist/gnx-service.exe','dist/gnx-install.exe','dist/gnx-linux-bundle.tar','dist/manifest.json','dist/manifest.json.sig',$rootfsTarget)
 foreach($path in $packagePaths){(Get-Item -LiteralPath $path).LastWriteTimeUtc=$epoch}
 Compress-Archive -Path $packagePaths -DestinationPath "dist/gnx-$version-windows-amd64.zip" -Force
 Get-FileHash dist/manifest.json,dist/manifest.json.sig,"dist/gnx-$version-windows-amd64.zip" -Algorithm SHA256
} finally {Pop-Location}
