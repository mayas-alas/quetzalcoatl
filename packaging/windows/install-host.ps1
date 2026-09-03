[CmdletBinding()]
param(
    [string]$Bundle = (Join-Path $PSScriptRoot '..\..\dist'),
    [string]$Distribution = 'Ubuntu-24.04'
)

$ErrorActionPreference = 'Stop'
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this installer from an elevated PowerShell.'
}
$bundle = (Resolve-Path -LiteralPath $Bundle).Path
foreach ($name in @('gnx.exe', 'gnx')) {
    $expected = (Get-Content -Raw -LiteralPath (Join-Path $bundle "$name.sha256")).Trim()
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $bundle $name)).Hash
    if ($expected -notmatch '^[a-fA-F0-9]{64}$' -or $actual -ne $expected) {
        throw "FAILED HASH_$($name.ToUpperInvariant())"
    }
}

$destination = 'C:\Program Files\GNX'
New-Item -ItemType Directory -Force -Path $destination | Out-Null
foreach ($name in @('gnx.exe', 'gnx', 'gnx.exe.sha256', 'gnx.sha256', 'gnx.example.toml', 'LICENSE')) {
    Copy-Item -LiteralPath (Join-Path $bundle $name) -Destination $destination -Force
}
if (-not (Test-Path -LiteralPath (Join-Path $destination 'gnx.toml'))) {
    Copy-Item -LiteralPath (Join-Path $bundle 'gnx.example.toml') -Destination (Join-Path $destination 'gnx.toml')
}
$machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
if ($destination -notin ($machinePath -split ';')) {
    [Environment]::SetEnvironmentVariable('Path', "$machinePath;$destination", 'Machine')
}

$bundleWsl = (& wsl.exe -d $Distribution --exec wslpath -u $bundle).Trim()
if ($LASTEXITCODE -ne 0 -or -not $bundleWsl.StartsWith('/')) { throw 'FAILED WSL_PATH' }
& wsl.exe -d $Distribution --user root --exec sh "$bundleWsl/install-linux.sh" $bundleWsl
if ($LASTEXITCODE -ne 0) { throw 'FAILED LINUX_INSTALL' }
& (Join-Path $destination 'gnx.exe') --help | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'FAILED WINDOWS_INSTALL' }
Write-Output 'READY windows+linux; reopen the terminal'
