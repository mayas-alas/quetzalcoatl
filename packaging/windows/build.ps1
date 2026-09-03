[CmdletBinding()]
param(
    [string]$OutputDirectory,
    [string]$BuildDistribution = 'Ubuntu-24.04'
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $root 'dist' }
$output = New-Item -ItemType Directory -Force -Path $OutputDirectory

$gates = @(
    [pscustomobject]@{ Name = 'TEST'; Args = @('test', '--locked') },
    [pscustomobject]@{ Name = 'CLIPPY'; Args = @('clippy', '--locked', '--all-targets', '--', '-D', 'warnings') },
    [pscustomobject]@{ Name = 'BUILD'; Args = @('build', '--locked', '--release') }
)
foreach ($gate in $gates) {
    & cargo @($gate.Args)
    if ($LASTEXITCODE -ne 0) { throw "FAILED WINDOWS_$($gate.Name)" }
}

$repo = (& wsl.exe -d $BuildDistribution --exec wslpath -u $root).Trim()
if ($LASTEXITCODE -ne 0 -or -not $repo.StartsWith('/')) { throw 'FAILED WSL_PATH' }
$outputWsl = (& wsl.exe -d $BuildDistribution --exec wslpath -u $output.FullName).Trim()
if ($LASTEXITCODE -ne 0 -or -not $outputWsl.StartsWith('/')) { throw 'FAILED OUTPUT_PATH' }
$container = 'gnx-build-' + [Guid]::NewGuid().ToString('N')
$image = 'docker.io/library/rust@sha256:3ffeca71d0e4fc30f5537f76b7243e87ac99726b6d3d66591dfc5e497078b9fc'
try {
    & wsl.exe -d $BuildDistribution --user root --exec podman run --name $container --pull=missing `
        -v "${repo}:/work" -v gnx-build-registry:/usr/local/cargo/registry `
        -v gnx-build-linux:/work/target -w /work $image sh -c `
        'rustup component add clippy && cargo test --locked && cargo clippy --locked --all-targets -- -D warnings && cargo build --locked --release'
    if ($LASTEXITCODE -ne 0) { throw 'FAILED LINUX_BUILD' }
    & wsl.exe -d $BuildDistribution --user root --exec podman cp "${container}:/work/target/release/gnx" "$outputWsl/gnx"
    if ($LASTEXITCODE -ne 0) { throw 'FAILED LINUX_COPY' }
} finally {
    & wsl.exe -d $BuildDistribution --user root --exec podman rm -f $container 2>$null | Out-Null
}

Copy-Item -LiteralPath (Join-Path $root 'target\release\gnx.exe') -Destination $output -Force
Copy-Item -LiteralPath (Join-Path $root 'config\gnx.example.toml') -Destination $output -Force
Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination $output -Force
Copy-Item -LiteralPath (Join-Path $root 'runtime') -Destination $output -Recurse -Force
Copy-Item -LiteralPath (Join-Path $root 'packaging\linux\install.sh') -Destination (Join-Path $output 'install-linux.sh') -Force

& (Join-Path $output 'gnx.exe') --version | Out-Null
& wsl.exe -d $BuildDistribution --exec "$outputWsl/gnx" --version | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'FAILED ARTIFACT_EXECUTION' }

foreach ($name in @('gnx.exe', 'gnx')) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $output $name)).Hash.ToLowerInvariant() |
        Set-Content -Encoding ascii -NoNewline -LiteralPath (Join-Path $output "$name.sha256")
}
Write-Output "READY $($output.FullName)"
