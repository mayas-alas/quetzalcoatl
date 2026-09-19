#requires -Version 5.1
<##
.SYNOPSIS
  Provisions the GNX-owned WSL runtime without operator-supplied rootfs input.

.DESCRIPTION
  WSL obtains the pinned Ubuntu distribution through its own official install
  path. The temporary base distribution is exported, verified locally, removed,
  and imported under the GNX runtime identity with the GNX-owned node name.
  No download URL, credential, rootfs path, or secret is accepted from argv.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$distribution = 'GNX-0.3.1'
$baseDistribution = 'Ubuntu-24.04'
$runtimeUser = 'gnx-runtime'
$runtimeRoot = 'C:\ProgramData\GNX-0.3.1\wsl'
$reportRoot = 'C:\ProgramData\GNX-0.3.1'
$journalRoot = 'C:\ProgramData\GNX-Setup-0.3.1'
$journalPath = Join-Path $journalRoot 'journal.json'
$stageTar = Join-Path $reportRoot 'ubuntu-base.tar'
$phase = 'PREFLIGHT'
$mutationStarted = $false
$createdRuntimeUser = $false
$createdRuntimeRoot = $false
$createdBaseDistribution = $false
$createdRuntimeDistribution = $false
$runtimeSid = $null
$password = $null

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

function Write-Journal([string]$Phase, [string]$Code = $null, [string]$StagingHash = $null) {
    $entry = [ordered]@{
        schema = 1
        operation = 'PROVISION'
        phase = $Phase
        account = $runtimeUser
        distribution = $distribution
        runtime_root = $runtimeRoot
        source = $baseDistribution
    }
    if ($Code) { $entry.code = $Code }
    if ($StagingHash) { $entry.staging_sha256 = $StagingHash }
    $temp = Join-Path $journalRoot 'journal.gnx-new'
    $bytes = [Text.Encoding]::UTF8.GetBytes(($entry | ConvertTo-Json -Compress))
    $file = [IO.File]::Open($temp, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $file.Write($bytes, 0, $bytes.Length); $file.Flush($true) } finally { $file.Dispose() }
    Move-Item -LiteralPath $temp -Destination $journalPath -Force
    $script:phase = $Phase
}

function Invoke-Rollback {
    $rollbackFailed = $false
    if ($createdRuntimeDistribution) {
        try { & "$env:SystemRoot\System32\wsl.exe" '--unregister' $distribution; if ($LASTEXITCODE -ne 0) { $rollbackFailed = $true } } catch { $rollbackFailed = $true }
    }
    if ($createdBaseDistribution) {
        try { & "$env:SystemRoot\System32\wsl.exe" '--unregister' $baseDistribution; if ($LASTEXITCODE -ne 0) { $rollbackFailed = $true } } catch { $rollbackFailed = $true }
    }
    if ($createdRuntimeUser -and $runtimeSid) {
        try {
            $user = Get-LocalUser -Name $runtimeUser -ErrorAction SilentlyContinue
            if ($user -and $user.SID.Value -eq $runtimeSid) { Remove-LocalUser -Name $runtimeUser -ErrorAction Stop }
        } catch { $rollbackFailed = $true }
    }
    if ($createdRuntimeRoot) {
        try {
            if (Test-Path -LiteralPath $runtimeRoot) { Remove-Item -LiteralPath $runtimeRoot -Recurse -Force -ErrorAction Stop }
        } catch { $rollbackFailed = $true }
    }
    return $rollbackFailed
}

try {
    Assert-Admin
    Assert-PlainDirectory $reportRoot
    Assert-PlainDirectory $journalRoot
    Assert-PlainDirectory $runtimeRoot
    New-Item -ItemType Directory -Force -Path $reportRoot | Out-Null
    New-Item -ItemType Directory -Force -Path $journalRoot | Out-Null
    Write-Journal 'PREFLIGHT'

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

    if (-not (Test-Path -LiteralPath $runtimeRoot)) {
        New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
        $createdRuntimeRoot = $true
        $mutationStarted = $true
    }
    Write-Journal 'STAGING'
    $password = New-RuntimePassword
    $mutationStarted = $true
    New-LocalUser -Name $runtimeUser -Password $password -Description 'GNX runtime identity' -AccountNeverExpires -PasswordNeverExpires -UserMayNotChangePassword | Out-Null
    $createdRuntimeUser = $true
    $runtimeSid = (Get-LocalUser -Name $runtimeUser).SID.Value
    Write-Journal 'REGISTERING'

    $acl = Get-Acl -LiteralPath $runtimeRoot
    $acl.SetAccessRuleProtection($true, $false)
    $rule = [Security.AccessControl.FileSystemAccessRule]::new($runtimeSid, 'Modify', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    $acl.SetAccessRule($rule)
    Set-Acl -LiteralPath $runtimeRoot -AclObject $acl

    # WSL performs the official distribution download. No URL or rootfs is
    # accepted from the operator, and --no-launch prevents an interactive shell.
    & "$env:SystemRoot\System32\wsl.exe" --install $baseDistribution --no-launch --web-download
    if ($LASTEXITCODE -eq 3010) { throw 'WSL requires a restart before GNX provisioning can continue.' }
    if ($LASTEXITCODE -ne 0) { throw "WSL distribution installation failed with code $LASTEXITCODE." }
    $createdBaseDistribution = $true

    Invoke-Wsl @('--export', $baseDistribution, $stageTar)
    if (-not (Test-Path -LiteralPath $stageTar -PathType Leaf)) { throw 'WSL export did not produce the expected staging archive.' }
    $hash = (Get-FileHash -LiteralPath $stageTar -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-Journal 'PUBLISHING' -StagingHash $hash

    # The temporary base name is fixed and was created by this run; only that
    # exact distribution is removed before the GNX-owned import.
    Invoke-Wsl @('--unregister', $baseDistribution)
    $createdBaseDistribution = $false

    $credential = [PSCredential]::new("$env:COMPUTERNAME\$runtimeUser", $password)
    $import = Start-Process -FilePath "$env:SystemRoot\System32\wsl.exe" -Credential $credential -Wait -PassThru -WindowStyle Hidden -ArgumentList @('--import', $distribution, $runtimeRoot, $stageTar, '--version', '2')
    if ($import.ExitCode -ne 0) { throw "GNX WSL import failed with code $($import.ExitCode)." }
    $createdRuntimeDistribution = $true
    Write-Journal 'SECURING' -StagingHash $hash

    $report = [ordered]@{
        result = 'READY'
        distribution = $distribution
        runtime_user = $runtimeUser
        runtime_root = $runtimeRoot
        source = $baseDistribution
        staging_sha256 = $hash
    }
    $report | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $reportRoot 'runtime-provisioning.json') -Encoding UTF8
    Write-Journal 'PROVISIONED' -StagingHash $hash
    Write-Output "READY GNX runtime '$distribution' provisioned under '$runtimeUser'."
} catch {
    $code = if ($mutationStarted) { 'PROVISION_FAILED' } else { 'PROVISION_BLOCKED' }
    $needsRecovery = $false
    if ($mutationStarted) {
        try {
            if ((Get-Distros) -contains $distribution) { $createdRuntimeDistribution = $true }
            if ((Get-Distros) -contains $baseDistribution) { $createdBaseDistribution = $true }
        } catch { }
        try {
            $partialUser = Get-LocalUser -Name $runtimeUser -ErrorAction SilentlyContinue
            if ($partialUser -and $partialUser.Description -eq 'GNX runtime identity') {
                $createdRuntimeUser = $true
                $runtimeSid = $partialUser.SID.Value
            }
        } catch { }
        $needsRecovery = Invoke-Rollback
    }
    $result = if ($mutationStarted -or $needsRecovery) { 'RECOVERY_REQUIRED' } else { 'BLOCKED' }
    try { Write-Journal $result $code } catch { }
    [ordered]@{ result = $result; code = $code; phase = $phase } | ConvertTo-Json -Compress | Write-Output
    exit 1
} finally {
    if ($null -ne $password) { $password.Dispose() }
    Remove-Item -LiteralPath $stageTar -Force -ErrorAction SilentlyContinue
}
