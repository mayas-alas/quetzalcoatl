[CmdletBinding()]
param(
    [string]$Bundle = (Join-Path $PSScriptRoot '..\..\dist')
)

$ErrorActionPreference = 'Stop'
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this installer from an elevated PowerShell.'
}
$bundle = (Resolve-Path -LiteralPath $Bundle).Path

function Assert-Hash {
    param([string]$Name)
    $file = Join-Path $bundle $Name
    $hashFile = "$file.sha256"
    if (-not (Test-Path -LiteralPath $file) -or -not (Test-Path -LiteralPath $hashFile)) {
        throw "FAILED HASH_MISSING_$($Name.ToUpperInvariant())"
    }
    $expected = (Get-Content -Raw -LiteralPath $hashFile).Trim()
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash
    if ($expected -notmatch '^[a-fA-F0-9]{64}$' -or $actual -ne $expected) {
        throw "FAILED HASH_$($Name.ToUpperInvariant())"
    }
}

foreach ($name in @('gnx.exe', 'gnx-service.exe', 'gnx-linux-bundle.tar')) {
    Assert-Hash $name
}

& wsl.exe --status *> $null
if ($LASTEXITCODE -ne 0) {
    & wsl.exe --install --no-distribution --web-download
    $installExit = $LASTEXITCODE
    if ($installExit -ne 0 -and $installExit -ne 3010) { throw 'FAILED WSL_INSTALL' }
    & wsl.exe --status *> $null
    if ($LASTEXITCODE -ne 0) { throw 'FAILED WSL_REBOOT_REQUIRED' }
}

$existing = Get-Service -Name 'GNXRuntime' -ErrorAction SilentlyContinue
if ($existing -and $existing.Status -ne 'Stopped') {
    Stop-Service -Name 'GNXRuntime' -Force
    $existing.WaitForStatus('Stopped', [TimeSpan]::FromSeconds(30))
}

$destination = 'C:\Program Files\GNX'
New-Item -ItemType Directory -Force -Path $destination | Out-Null
foreach ($name in @('gnx.exe', 'gnx-service.exe', 'gnx.example.toml', 'LICENSE', 'runtime.lock.json')) {
    Copy-Item -LiteralPath (Join-Path $bundle $name) -Destination $destination -Force
}
if (-not (Test-Path -LiteralPath (Join-Path $destination 'gnx.toml'))) {
    Copy-Item -LiteralPath (Join-Path $bundle 'gnx.example.toml') -Destination (Join-Path $destination 'gnx.toml')
}

$machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
if ($destination -notin ($machinePath -split ';')) {
    [Environment]::SetEnvironmentVariable('Path', "$machinePath;$destination", 'Machine')
}

$operatorSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$service = Join-Path $destination 'gnx-service.exe'
& $service --install $operatorSid
if ($LASTEXITCODE -ne 0) { throw 'FAILED SERVICE_INSTALL' }

$dataRoot = 'C:\ProgramData\GNX'
$bootstrap = Join-Path $dataRoot 'bootstrap'
New-Item -ItemType Directory -Force -Path $bootstrap | Out-Null
Copy-Item -LiteralPath (Join-Path $bundle 'gnx-linux-bundle.tar') -Destination (Join-Path $bootstrap 'gnx-linux-bundle.tar') -Force

$runtime = Get-Content -Raw -LiteralPath (Join-Path $destination 'runtime.lock.json') | ConvertFrom-Json
if ($runtime.wsl.distribution -ne 'GNX' -or $runtime.wsl.rootfs_sha256 -notmatch '^[a-fA-F0-9]{64}$') {
    throw 'FAILED RUNTIME_LOCK'
}
$wslDisk = Join-Path $dataRoot 'wsl\ext4.vhdx'
if (-not (Test-Path -LiteralPath $wslDisk)) {
    $rootfs = Join-Path $bootstrap 'ubuntu.rootfs.tar.gz'
    $valid = $false
    if (Test-Path -LiteralPath $rootfs) {
        $valid = (Get-FileHash -Algorithm SHA256 -LiteralPath $rootfs).Hash -eq $runtime.wsl.rootfs_sha256
    }
    if (-not $valid) {
        $temporary = "$rootfs.gnx-new"
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
        Invoke-WebRequest -UseBasicParsing -Uri $runtime.wsl.rootfs_url -OutFile $temporary
        if ((Get-FileHash -Algorithm SHA256 -LiteralPath $temporary).Hash -ne $runtime.wsl.rootfs_sha256) {
            Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
            throw 'FAILED WSL_ROOTFS_HASH'
        }
        Move-Item -LiteralPath $temporary -Destination $rootfs -Force
    }
}

& $service --start
if ($LASTEXITCODE -ne 0) { throw 'FAILED SERVICE_START' }

$ready = $false
for ($attempt = 0; $attempt -lt 180; $attempt++) {
    & $service --ping *> $null
    if ($LASTEXITCODE -eq 0) {
        $ready = $true
        break
    }
    Start-Sleep -Seconds 10
}
if (-not $ready) {
    $failure = Join-Path $dataRoot 'last-error.txt'
    if (Test-Path -LiteralPath $failure) {
        throw "FAILED RUNTIME_BOOTSTRAP: $((Get-Content -Raw -LiteralPath $failure).Trim())"
    }
    throw 'FAILED RUNTIME_BOOTSTRAP'
}

& (Join-Path $destination 'gnx.exe') --help | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'FAILED WINDOWS_INSTALL' }
Write-Output 'READY GNX; isolated runtime owned by gnx-runtime'
