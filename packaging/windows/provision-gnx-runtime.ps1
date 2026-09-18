#requires -Version 5.1
<##
.SYNOPSIS
  Provisions the GNX-owned WSL runtime under a dedicated local identity.

.DESCRIPTION
  This script is intentionally explicit: it never downloads a distribution,
  accepts arbitrary resource names, or writes credentials to disk, argv, or logs.
  The caller supplies a trusted rootfs tarball and the script imports it as the
  GNX runtime identity.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$RootfsTar,
    [string]$DistributionName = 'gnx-pihole',
    [string]$RuntimeUser = 'gnx-runtime',
    [string]$RuntimeRoot = 'C:\ProgramData\GNX\runtime'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$fixedDistribution = 'gnx-pihole'
$fixedRuntimeUser = 'gnx-runtime'
$fixedRuntimeRoot = 'C:\ProgramData\GNX\runtime'
$reportRoot = 'C:\ProgramData\GNX'

function Assert-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'GNX runtime provisioning requires an elevated administrator.'
    }
}

function Assert-FixedIdentity {
    if ($DistributionName -cne $fixedDistribution) { throw "DistributionName must be '$fixedDistribution'." }
    if ($RuntimeUser -cne $fixedRuntimeUser) { throw "RuntimeUser must be '$fixedRuntimeUser'." }
    if ($RuntimeRoot -cne $fixedRuntimeRoot) { throw "RuntimeRoot must be '$fixedRuntimeRoot'." }
}

function Assert-PlainDirectory([string]$Path) {
    if (Test-Path -LiteralPath $Path) {
        $item = Get-Item -LiteralPath $Path -Force
        if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw "Refusing redirected runtime path: $Path"
        }
    }
}

function New-RuntimePassword {
    $bytes = New-Object byte[] 32
    [Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
    $chars = [Convert]::ToBase64String($bytes).ToCharArray()
    $bytes = $null
    # SecureString never leaves this process and is not persisted.
    ConvertTo-SecureString (-join $chars) -AsPlainText -Force
}

Assert-Admin
Assert-FixedIdentity
$rootfs = (Resolve-Path -LiteralPath $RootfsTar).Path
Assert-PlainDirectory $reportRoot
Assert-PlainDirectory $RuntimeRoot
New-Item -ItemType Directory -Force -Path $reportRoot | Out-Null
New-Item -ItemType Directory -Force -Path $RuntimeRoot | Out-Null

$existingUser = Get-LocalUser -Name $RuntimeUser -ErrorAction SilentlyContinue
if ($existingUser) {
    if ($existingUser.Description -ne 'GNX runtime identity') {
        throw "Existing local user '$RuntimeUser' is not owned by GNX."
    }
} else {
    $password = New-RuntimePassword
    New-LocalUser -Name $RuntimeUser -Password $password -Description 'GNX runtime identity' -AccountNeverExpires -PasswordNeverExpires -UserMayNotChangePassword | Out-Null
}

$acl = Get-Acl -LiteralPath $RuntimeRoot
$acl.SetAccessRuleProtection($true, $false)
$runtimeSid = (Get-LocalUser -Name $RuntimeUser).SID
$rule = [Security.AccessControl.FileSystemAccessRule]::new($runtimeSid, 'Modify', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
$acl.SetAccessRule($rule)
Set-Acl -LiteralPath $RuntimeRoot -AclObject $acl

$existing = @(wsl.exe --list --quiet 2>$null | ForEach-Object { $_.Trim() } | Where-Object { $_ })
if ($existing -contains $DistributionName) {
    throw "WSL distribution '$DistributionName' already exists; refusing adoption."
}

# Import is executed as the dedicated identity so WSL ownership is not tied to
# the administrator who provisions the host.
if ($existingUser) {
    $password = Read-Host -Prompt "Password for existing GNX runtime identity" -AsSecureString
}
try {
    $credential = [PSCredential]::new("$env:COMPUTERNAME\$RuntimeUser", $password)
    $import = Start-Process -FilePath "$env:SystemRoot\System32\wsl.exe" -Credential $credential -Wait -PassThru -WindowStyle Hidden -ArgumentList @('--import', $DistributionName, $RuntimeRoot, $rootfs, '--version', '2')
} finally {
    $password.Dispose()
}
if ($import.ExitCode -ne 0) { throw "GNX WSL import failed with code $($import.ExitCode)." }

$report = [ordered]@{
    result = 'READY'
    distribution = $DistributionName
    runtime_user = $RuntimeUser
    runtime_root = $RuntimeRoot
}
$report | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $reportRoot 'runtime-provisioning.json') -Encoding UTF8
Write-Output "READY GNX runtime '$DistributionName' provisioned under '$RuntimeUser'."
