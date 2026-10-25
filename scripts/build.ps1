param([switch]$Package)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
function Native([scriptblock]$Command) { & $Command; if ($LASTEXITCODE -ne 0) { throw "Native command failed ($LASTEXITCODE)" } }
Push-Location $repo
try {
    Native { cargo build --release --locked }
    $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $version = ($metadata.packages | Where-Object name -eq 'gnx').version
    if ((& './target/release/gnx.exe' --version) -ne "gnx $version") { throw 'Release version mismatch' }
    $release = Join-Path $repo "releases/v$version"
    if (-not (Test-Path -LiteralPath (Join-Path $release 'CHANGELOG.md') -PathType Leaf)) { throw 'Release changelog missing' }
    Copy-Item -LiteralPath 'target/release/gnx.exe' -Destination (Join-Path $release 'gnx.exe') -Force
    if ((Get-FileHash target/release/gnx.exe).Hash -ne (Get-FileHash (Join-Path $release 'gnx.exe')).Hash) { throw 'Release binary mismatch' }
    if (-not $Package) { return }
    $stage = Join-Path $repo 'build/package/payload'
    New-Item -ItemType Directory -Force $stage,(Join-Path $stage 'images') | Out-Null
    Copy-Item -Recurse -Force config,scripts $stage
    $inputs = @{ 'rootfs.tar' = 'build/payload/rootfs.tar' }
    foreach ($role in 'gateway','dns','mesh','compute') { $inputs["images/$role.oci.tar"] = "build/payload/images/$role.oci.tar" }
    foreach ($relative in $inputs.Keys) {
        $source = Join-Path $repo $inputs[$relative]
        $destination = Join-Path $stage $relative
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Offline input missing: $source" }
        if (-not (Test-Path -LiteralPath $destination)) { New-Item -ItemType HardLink -Path $destination -Target $source | Out-Null }
    }
    $entries = @(Get-ChildItem -LiteralPath $stage -Recurse -File | Where-Object Name -ne 'manifest.json' | Sort-Object FullName | ForEach-Object {
        @{ path = $_.FullName.Substring($stage.Length + 1).Replace('\','/'); sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    })
    @{ version=1; architecture='x86_64'; files=$entries } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $stage 'manifest.json') -Encoding utf8
    # PowerShell 5 writes a BOM; JSON is UTF-8 without BOM for Rust serde.
    $manifest = Join-Path $stage 'manifest.json'
    [IO.File]::WriteAllText($manifest, [IO.File]::ReadAllText($manifest), [Text.UTF8Encoding]::new($false))
    Native { & './target/release/gnx.exe' --verify-package $stage }
    $output = Join-Path $repo 'build/releases/v0.0.1/'
    New-Item -ItemType Directory -Force $output | Out-Null
    Native { dotnet build installer/GnxCore.wixproj -c Release "-p:OutputPath=$output" }
    Native { dotnet build installer/bundle/Bundle.wixproj -c Release "-p:OutputPath=$output" }
    Copy-Item target/release/gnx.exe $output
} finally { Pop-Location }
