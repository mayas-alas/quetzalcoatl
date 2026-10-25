param([switch]$Package)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
function Native([scriptblock]$Command) { & $Command; if ($LASTEXITCODE -ne 0) { throw "Native command failed ($LASTEXITCODE)" } }
Push-Location $repo
try {
    Native { cargo build --release --locked }
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
