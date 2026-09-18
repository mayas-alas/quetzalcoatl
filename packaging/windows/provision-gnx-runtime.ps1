#requires -Version 5.1
<##
.SYNOPSIS
  Provisions the GNX-owned WSL runtime without operator-supplied rootfs input.

.DESCRIPTION
  WSL obtains the pinned Ubuntu distribution through its own official install
  path. The temporary base distribution is exported, verified locally, removed,
  and imported under the GNX runtime identity with the GNX-owned name.
  No download URL, credential, rootfs path, or secret is accepted from argv.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$distribution = 'gnx-pihole'
$baseDistribution = 'Ubuntu-24.04'
$runtimeUser = 'gnx-runtime'
$runtimeRoot = 'C:\ProgramData\GNX\runtime'
$reportRoot = 'C:\ProgramData\GNX'
$stageTar = Join-Path $reportRoot 'ubuntu-base.tar'

function Assert-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'GNX runtime provisioning requires an elevated administrator.'
    }
}

function Assert-PlainDirectory([string]$Path) {
    if (Test-Path -LiteralPath $Path) {
        $item = Get-Item -LiteralPath $Path -Force
        if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw "Refusing redirected runtime path: $Path"
        }
    }
}

function Get-Distros {
    @(wsl.exe --list --quiet 2>$null | ForEach-Object { $_.Trim() } | Where-Object { $_ })
}

function Invoke-Wsl([string[]]$Arguments) {
    & "$env:SystemRoot\System32\wsl.exe" @Arguments
    if ($LASTEXITCODE -eq 3010) { throw 'WSL requires a restart before GNX provisioning can continue.' }
    if ($LASTEXITCODE -ne 0) { throw "WSL operation failed with code $LASTEXITCODE." }
}

function New-RuntimePassword {
    $bytes = New-Object byte[] 32
    [Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
    $text = [Convert]::ToBase64String($bytes)
    $bytes = $null
    ConvertTo-SecureString $text -AsPlainText -Force
}

Assert-Admin
Assert-PlainDirectory $reportRoot
Assert-PlainDirectory $runtimeRoot
New-Item -ItemType Directory -Force -Path $reportRoot | Out-Null
New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null

$existingUser = Get-LocalUser -Name $runtimeUser -ErrorAction SilentlyContinue
if ($existingUser) {
    if ($existingUser.Description -ne 'GNX runtime identity') {
        throw "Existing local user '$runtimeUser' is not owned by GNX."
    }
    throw "GNX runtime identity already exists; refusing an unattended credential reset."
}

$existingDistros = Get-Distros
if ($existingDistros -contains $distribution) {
    throw "WSL distribution '$distribution' already exists; refusing adoption."
}
if ($existingDistros -contains $baseDistribution) {
    throw "Base distribution '$baseDistribution' already exists; refusing to reuse an unrelated distribution."
}

$password = New-RuntimePassword
try {
    New-LocalUser -Name $runtimeUser -Password $password -Description 'GNX runtime identity' -AccountNeverExpires -PasswordNeverExpires -UserMayNotChangePassword | Out-Null

    $acl = Get-Acl -LiteralPath $runtimeRoot
    $acl.SetAccessRuleProtection($true, $false)
    $runtimeSid = (Get-LocalUser -Name $runtimeUser).SID
    $rule = [Security.AccessControl.FileSystemAccessRule]::new($runtimeSid, 'Modify', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    $acl.SetAccessRule($rule)
    Set-Acl -LiteralPath $runtimeRoot -AclObject $acl

    # WSL performs the official distribution download. No URL or rootfs is
    # accepted from the operator, and --no-launch prevents an interactive shell.
    & "$env:SystemRoot\System32\wsl.exe" --install $baseDistribution --no-launch --web-download
    if ($LASTEXITCODE -eq 3010) { throw 'WSL requires a restart before GNX provisioning can continue.' }
    if ($LASTEXITCODE -ne 0) { throw "WSL distribution installation failed with code $LASTEXITCODE." }

    Invoke-Wsl @('--export', $baseDistribution, $stageTar)
    if (-not (Test-Path -LiteralPath $stageTar -PathType Leaf)) { throw 'WSL export did not produce the expected staging archive.' }
    $hash = (Get-FileHash -LiteralPath $stageTar -Algorithm SHA256).Hash.ToLowerInvariant()

    # The temporary base name is fixed and was created by this run; only that
    # exact distribution is removed before the GNX-owned import.
    Invoke-Wsl @('--unregister', $baseDistribution)

    $credential = [PSCredential]::new("$env:COMPUTERNAME\$runtimeUser", $password)
    $import = Start-Process -FilePath "$env:SystemRoot\System32\wsl.exe" -Credential $credential -Wait -PassThru -WindowStyle Hidden -ArgumentList @('--import', $distribution, $runtimeRoot, $stageTar, '--version', '2')
    if ($import.ExitCode -ne 0) { throw "GNX WSL import failed with code $($import.ExitCode)." }

    $report = [ordered]@{
        result = 'READY'
        distribution = $distribution
        runtime_user = $runtimeUser
        runtime_root = $runtimeRoot
        source = $baseDistribution
        staging_sha256 = $hash
    }
    $report | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $reportRoot 'runtime-provisioning.json') -Encoding UTF8
    Write-Output "READY GNX runtime '$distribution' provisioned under '$runtimeUser'."
} finally {
    $password.Dispose()
    Remove-Item -LiteralPath $stageTar -Force -ErrorAction SilentlyContinue
}
