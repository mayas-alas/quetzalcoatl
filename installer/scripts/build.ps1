param([string]$Configuration = "release", [switch]$BuildPodmanInput)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$out = Join-Path $root "build\installer"
if ($BuildPodmanInput) {
    $podman = Get-Content (Join-Path $root 'installer\inputs\podman.json') -Raw | ConvertFrom-Json
    if ($podman.schema -ne 1 -or $podman.version -ne '6.1.3' -or $podman.architecture -ne 'amd64' -or $podman.build.recipe -ne 'upstream Makefile' -or (@($podman.build.targets) -join ',') -ne 'podman,rootlessport,quadlet' -or $podman.build.prefix -ne '/usr' -or $podman.source.commit -notmatch '^[0-9a-f]{40}$' -or $podman.toolchain.version -notmatch '^go[0-9]+\.[0-9]+\.[0-9]+$' -or $podman.build.build_info -notmatch '^[0-9]+$' -or $podman.build.tags -notmatch '^[a-z0-9_,]+$') { throw 'Invalid Podman build manifest' }
    $inputDir = Join-Path $root 'build\podman\inputs'; $sourceName = "podman-$($podman.source.commit).tar.gz"; $goName = "$($podman.toolchain.version).linux-amd64.tar.gz"
    if ($podman.source.archive -ne $sourceName -or $podman.toolchain.archive -ne $goName -or $podman.source.sha256 -notmatch '^[0-9a-f]{64}$' -or $podman.toolchain.sha256 -notmatch '^[0-9a-f]{64}$') { throw 'Invalid sealed Podman input names or hashes' }
    $inputHashes = [ordered]@{}
    foreach ($item in @(@{ Name = $sourceName; Hash = $podman.source.sha256 }, @{ Name = $goName; Hash = $podman.toolchain.sha256 })) {
        $path = Join-Path $inputDir $item.Name; if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing sealed Podman input: $path" }
        $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant(); if ($actual -ne $item.Hash) { throw "Sealed Podman input digest mismatch: $($item.Name)" }; $inputHashes[$item.Name] = $actual
    }
    $podmanOut = Join-Path $root 'build\podman'; New-Item -ItemType Directory -Force $podmanOut | Out-Null
    $wslRoot = "/mnt/$($root.Substring(0, 1).ToLowerInvariant())/$($root.Substring(3).Replace('\', '/'))"; $wslInput = "$wslRoot/build/podman/inputs"; $buildRoot = '/var/lib/gnx-build/podman'; $source = "$buildRoot/source"; $toolchain = "$buildRoot/toolchain"; $cache = "$buildRoot/cache"; $stage = "$buildRoot/stage"; $archive = Join-Path $podmanOut 'podman-6.1.3-linux-amd64.tar.gz'
    $wsl = @('--distribution', 'GnX-LinuxLab', '--user', 'root', '--exec'); $run = { param([string[]]$Arguments) & wsl.exe @wsl @Arguments; if ($LASTEXITCODE) { throw "Podman WSL command failed: $($Arguments -join ' ')" } }
    & $run @('rm', '-rf', $stage); & $run @('mkdir', '-p', $source, $toolchain, $cache, $stage)
    & $run @('tar', '--extract', '--gzip', '--file', "$wslInput/$sourceName", '--directory', $source, '--strip-components', '1'); & $run @('tar', '--extract', '--gzip', '--file', "$wslInput/$goName", '--directory', $toolchain)
    $env = @('env', 'GOTOOLCHAIN=local', 'GOPROXY=off', 'GOSUMDB=off', "GOCACHE=$cache", 'GOMAXPROCS=2', 'GOFLAGS=-mod=vendor', "GO=$toolchain/go/bin/go", "PREFIX=$($podman.build.prefix)", "GIT_COMMIT=$($podman.source.commit)", "BUILD_INFO=$($podman.build.build_info)", "BUILDTAGS=$($podman.build.tags)")
    & $run ($env + @('make', '--directory', $source, '-j2', 'podman', 'rootlessport', 'quadlet'))
    $binaryPaths = @("$source/bin/podman", "$source/bin/rootlessport", "$source/bin/quadlet"); $names = @('nativepodman', 'rootlessport', 'quadlet'); $hashLines = & $run (@('sha256sum') + $binaryPaths); $outputs = [ordered]@{}
    for ($i = 0; $i -lt $binaryPaths.Count; $i++) { if ($hashLines[$i] -notmatch '^([0-9a-f]{64})\s+') { throw 'Invalid Podman output hash' }; $outputs[$names[$i]] = [ordered]@{ path = "source/bin/$($names[$i] -replace 'nativepodman','podman')"; sha256 = $Matches[1] } }
    & $run ($env + @('make', '--directory', $source, '-j2', 'install.bin', 'install.systemd', "DESTDIR=$stage")); Remove-Item -LiteralPath $archive -Force -ErrorAction SilentlyContinue
    foreach ($name in @('netavark', 'aardvark-dns')) {
        $helper = $podman.helpers.$name
        if ($helper.version -ne '2.1.0' -or $helper.archive -ne "$name-2.1.0.gz" -or $helper.sha256 -notmatch '^[a-f0-9]{64}$') { throw "Invalid network helper manifest: $name" }
        $helperPath = Join-Path $inputDir $helper.archive
        $helperHash = (Get-FileHash -LiteralPath $helperPath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($helperHash -ne $helper.sha256) { throw "Network helper digest mismatch: $name" }
        $inputHashes[$helper.archive] = $helperHash
        & $run @('gzip', '--decompress', '--keep', '--force', "$wslInput/$($helper.archive)")
        & $run @('install', '-m', '0755', "$wslInput/$name-2.1.0", "$stage/usr/libexec/podman/$name")
        $helperOutput = & $run @('sha256sum', "$stage/usr/libexec/podman/$name")
        if ($helperOutput -notmatch '^([a-f0-9]{64})\s+') { throw 'Invalid helper output hash' }
        $outputs[$name] = [ordered]@{ path = "stage/usr/libexec/podman/$name"; sha256 = $Matches[1]; provenance = $helper.provenance }
    }
    & $run @('tar', '--create', '--gzip', '--file', "$wslRoot/build/podman/podman-6.1.3-linux-amd64.tar.gz", '--directory', $stage, '--sort=name', '--mtime=@0', '--owner=0', '--group=0', '--numeric-owner', '.')
    if (!(Test-Path -LiteralPath $archive -PathType Leaf)) { throw "Podman archive missing: $archive" }
    $outputs['archive'] = [ordered]@{ path = 'podman-6.1.3-linux-amd64.tar.gz'; sha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() }
    [ordered]@{ schema = 1; builder = 'GnX-LinuxLab'; inputs = $inputHashes; source = [ordered]@{ commit = $podman.source.commit; tag = $podman.source.tag }; build = [ordered]@{ recipe = 'upstream Makefile'; targets = @('podman', 'rootlessport', 'quadlet'); build_info = $podman.build.build_info; build_tags = $podman.build.tags }; outputs = $outputs; prerequisites = 'GnX-LinuxLab provides gcc-14, libseccomp-dev, systemd development files, and libgpgme-dev from official Debian; exact runtime dependencies still require a manifest and test.'; runtime_dependency_bundle = 'not complete' } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $podmanOut 'build-manifest.json') -Encoding UTF8
    return
}
$hostPayloadSource = Join-Path $root "payload\host"
$rootfsArtifact = Join-Path $hostPayloadSource "gnx-node-rootfs.tar"
$rootfsManifest = Join-Path $hostPayloadSource "gnx-node-rootfs.tar.sha256"
$expectedRootfsDigest = "5215c762cd1562b9b6836b01f6e1a2207cee7bae0ee334e50316af72b9a871af"
if (!(Test-Path -LiteralPath $rootfsArtifact -PathType Leaf)) {
    throw "Missing sealed WSL rootfs artifact: $rootfsArtifact"
}
if (!(Test-Path -LiteralPath $rootfsManifest -PathType Leaf)) {
    throw "Missing sealed WSL rootfs manifest: $rootfsManifest"
}
$manifestLine = (Get-Content -LiteralPath $rootfsManifest -Raw).Trim()
if ($manifestLine -notmatch '^[0-9a-f]{64}  gnx-node-rootfs\.tar$') {
    throw "Invalid sealed WSL rootfs manifest format: $rootfsManifest"
}
$manifestDigest = $manifestLine.Substring(0, 64)
if ($manifestDigest -ne $expectedRootfsDigest) {
    throw "Unexpected sealed WSL rootfs digest: $manifestDigest"
}
$actualRootfsDigest = (Get-FileHash -LiteralPath $rootfsArtifact -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualRootfsDigest -ne $manifestDigest) {
    throw "Sealed WSL rootfs digest mismatch: expected $manifestDigest, got $actualRootfsDigest"
}
# Build-time sealed input only: no registry/OCI download path is added to the product.
$offlineDir = Join-Path $root 'payload\node\workloads\private-services'
$offlineArtifact = Join-Path $offlineDir 'offline.tar.gz'
$offlineManifest = (Get-Content (Join-Path $offlineDir 'offline.sha256') -Raw).Trim()
if ($offlineManifest -notmatch '^([0-9a-f]{64})  offline\.tar\.gz$') { throw 'Invalid workload input manifest' }
$offlineDigest = $Matches[1]
if (!(Test-Path $offlineArtifact)) {
    $offlineUrl = (Get-Content (Join-Path $offlineDir 'offline.url') -Raw).Trim()
    if ($offlineUrl -notmatch '^https://huggingface\.co/datasets/gnx-labs-mx/gnx-node-releases/resolve/[0-9a-f]{40}/build-inputs/v[0-9.]+/offline\.tar\.gz$') { throw 'Invalid sealed input URL' }
    Invoke-WebRequest -UseBasicParsing -Uri $offlineUrl -OutFile $offlineArtifact
}
if ((Get-FileHash $offlineArtifact -Algorithm SHA256).Hash.ToLowerInvariant() -ne $offlineDigest) { throw 'Sealed workload input digest mismatch' }
$continuity = Join-Path $root 'payload\node\workloads\continuity'
foreach ($name in @('bootstrap', 'offline')) {
    $line = (Get-Content (Join-Path $continuity "$name.sha256") -Raw).Trim()
    if ($line -notmatch "^([a-f0-9]{64})  $name\.tar\.gz$") { throw 'Invalid Continuity input manifest' }
    if ((Get-FileHash (Join-Path $continuity "$name.tar.gz") -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Matches[1]) { throw 'Continuity input hash mismatch' }
}
$runtimeInput = Get-Content (Join-Path $continuity 'runtime-input.json') -Raw | ConvertFrom-Json
if ((Get-FileHash (Join-Path $continuity $runtimeInput.archive) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $runtimeInput.sha256) { throw 'Continuity engine hash mismatch' }
$acceptance = (Get-Content (Join-Path $continuity 'images.json') -Raw | ConvertFrom-Json).acceptance
if ((Get-FileHash (Join-Path $continuity $acceptance.binary) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $acceptance.sha256) { throw 'Continuity acceptance binary hash mismatch' }
$authDir = Join-Path $root 'payload\node\auth'
$authArtifact = Join-Path $authDir 'offline.tar.gz'
$authManifest = (Get-Content (Join-Path $authDir 'offline.sha256') -Raw).Trim()
if ($authManifest -notmatch '^([0-9a-f]{64})  offline\.tar\.gz$') { throw 'Invalid app auth input manifest' }
$authDigest = $Matches[1]
if (!(Test-Path $authArtifact)) {
    $authUrl = (Get-Content (Join-Path $authDir 'offline.url') -Raw).Trim()
    if ($authUrl -notmatch '^https://huggingface\.co/datasets/gnx-labs-mx/gnx-node-releases/resolve/[0-9a-f]{40}/build-inputs/v[0-9.]+/app-auth/offline\.tar\.gz$') { throw 'Invalid app auth sealed input URL' }
    Invoke-WebRequest -UseBasicParsing -Uri $authUrl -OutFile $authArtifact
}
if ((Get-FileHash $authArtifact -Algorithm SHA256).Hash.ToLowerInvariant() -ne $authDigest) { throw 'Sealed app auth digest mismatch' }
New-Item -ItemType Directory -Force $out | Out-Null
$msi = Join-Path $out "GnxNode.msi"
$localizedMsi = Join-Path $out "en-us\GnxNode.msi"
$exe = Join-Path $out "GnxNodeSetup.exe"
foreach ($artifact in @($msi, $localizedMsi, $exe)) {
    if (Test-Path -LiteralPath $artifact -PathType Leaf) {
        Remove-Item -LiteralPath $artifact -Force
    }
}
Push-Location $root
try {
    cargo build --workspace --release --locked
    if ($LASTEXITCODE) { throw "cargo build failed" }
}
finally { Pop-Location }
$node = Join-Path $out "..\payload\node"; if (Test-Path -LiteralPath $node) { Remove-Item -LiteralPath $node -Recurse -Force }; New-Item -ItemType Directory -Force $node | Out-Null
# Immutable sealed archives are shared by NTFS link in disposable staging.
# Text remains copied/normalized; never link runtime state, disks or secrets.
function Copy-SealedTree([string]$Source, [string]$Destination) {
    $sourcePath = (Resolve-Path $Source).Path
    Get-ChildItem $sourcePath -File -Recurse | ForEach-Object {
        $relative = $_.FullName.Substring($sourcePath.Length + 1)
        if ($relative -eq 'workloads\ai\offline.tar.gz') { return }
        $target = Join-Path $Destination $relative
        New-Item -ItemType Directory -Force (Split-Path $target) | Out-Null
        if ($_.Extension -in @('.gz', '.tar', '.zst', '.deb', '.bin')) {
            New-Item -ItemType HardLink -Path $target -Target $_.FullName | Out-Null
        } else { Copy-Item -LiteralPath $_.FullName -Destination $target }
    }
}
Copy-SealedTree (Join-Path $root 'payload\node') $node
$hostPayload = Join-Path $out "..\payload\host"; if (Test-Path -LiteralPath $hostPayload) { Remove-Item -LiteralPath $hostPayload -Recurse -Force }; New-Item -ItemType Directory -Force $hostPayload | Out-Null
Copy-SealedTree $hostPayloadSource $hostPayload
$web = Join-Path $node "web-app"; New-Item -ItemType Directory -Force $web | Out-Null
Copy-Item (Join-Path $root "apps\web-app\*") $web -Recurse -Force
# Linux consumes shell scripts, Quadlets, nftables, Caddy and web assets from
# this staged tree. Normalize before hashing so Windows checkout settings can
# never inject CRLF into parsers or systemd units.
Get-ChildItem $node -File -Recurse | Where-Object { $_.Extension -notin @('.gz', '.tar', '.zst', '.deb', '.bin') } | ForEach-Object {
    $text = [IO.File]::ReadAllText($_.FullName)
    $normalized = $text.Replace("`r`n", "`n").Replace("`r", "`n")
    [IO.File]::WriteAllText($_.FullName, $normalized, [Text.UTF8Encoding]::new($false))
}
$nodeRoot = (Resolve-Path $node).Path
$marker = Join-Path $nodeRoot ".gnx-payload.sha256"
$payloadLines = Get-ChildItem $nodeRoot -File -Recurse | Where-Object { $_.FullName -ne $marker } | Sort-Object FullName | ForEach-Object {
    $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $relative = $_.FullName.Substring($nodeRoot.Length + 1).Replace('\', '/')
    "$hash  /opt/gnx/payload/node/$relative"
}
$payloadLines | Set-Content $marker -Encoding ASCII
$manifest = Join-Path $out "..\manifests\payload.sha256"; New-Item -ItemType Directory -Force (Split-Path $manifest) | Out-Null
Get-ChildItem (Join-Path $out "..\payload") -File -Recurse | Get-FileHash -Algorithm SHA256 | ForEach-Object { "$($_.Hash)  $($_.Path.Substring($root.Length + 1))" } | Set-Content $manifest -Encoding UTF8
$project = Join-Path $PSScriptRoot "..\GnxNode.wixproj"
if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) { throw "WiX build unavailable: dotnet SDK not found" }
dotnet build $project -p:Configuration=$Configuration -p:OutputPath=$out
if ($LASTEXITCODE) { throw "WiX build failed" }
if (!(Test-Path -LiteralPath $msi) -and (Test-Path -LiteralPath $localizedMsi)) {
    Move-Item -LiteralPath $localizedMsi -Destination $msi -Force
}
if (!(Test-Path -LiteralPath $msi)) { throw "WiX did not produce the MSI: $msi" }

# Cabinets are already embedded in the complete validated MSI. Discard only
# those reproducible WiX intermediates before Burn needs its own cabinet.
Get-ChildItem (Join-Path $root 'installer\obj') -Filter '*.cab' -File -Recurse | Remove-Item -Force

# Burn is intentionally built after MSI so the bundle embeds this exact MSI.
$bundle = Join-Path $PSScriptRoot "..\bundle\Bundle.wixproj"
dotnet build $bundle -p:Configuration=$Configuration -p:OutputPath=$out
if ($LASTEXITCODE) { throw "Burn build failed" }
if (!(Test-Path -LiteralPath $exe)) { throw "Burn did not produce the setup bundle: $exe" }
Write-Output "Installer build complete: $out"
Write-Output "MSI: $msi"
Write-Output "Burn: $exe"
