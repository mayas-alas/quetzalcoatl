[CmdletBinding()]
param([ValidatePattern('^[A-Za-z0-9._-]+$')][string]$Distribution = 'Ubuntu-24.04')
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$state = Join-Path $env:ProgramData 'GNX\compute'
$userState = Join-Path $env:LOCALAPPDATA 'GNX\compute'
$report = Join-Path $env:ProgramData 'GNX\compute-status.json'
$passwordFile = Join-Path $state 'root.password'
$gate = 'ELEVATION'
try {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not ([Security.Principal.WindowsPrincipal]::new($identity)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Elevation required.' }
    if ((Get-Volume -DriveLetter $env:SystemDrive.TrimEnd(':')).SizeRemaining -lt 32GB) { throw 'At least 32 GiB free on the Windows system volume is required.' }
    foreach ($directory in @($state, $userState)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
        $acl = [Security.AccessControl.DirectorySecurity]::new()
        $acl.SetAccessRuleProtection($true, $false)
        $sids = @('S-1-5-18', 'S-1-5-32-544')
        if ($directory -eq $userState) { $sids += $identity.User.Value }
        foreach ($sid in $sids) {
            $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
                [Security.Principal.SecurityIdentifier]::new($sid), 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
        }
        Set-Acl -LiteralPath $directory -AclObject $acl
    }
    $gate = 'CREDENTIAL'
    $helper = Join-Path $state 'gnx-compute.exe'
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'target\release\gnx-compute.exe') -Destination $helper -Force
    $config = Join-Path $state 'compute.toml'
    Copy-Item -LiteralPath (Join-Path $repo 'runtime\compute\compute.toml') -Destination $config -Force
    $credentialFile = Join-Path $userState 'owner.credential.xml'
    if (Test-Path -LiteralPath $credentialFile) {
        $credential = Import-Clixml -LiteralPath $credentialFile
        [IO.File]::WriteAllText($passwordFile, $credential.GetNetworkCredential().Password, [Text.UTF8Encoding]::new($false))
    } else {
        & $helper render $config $state
        if ($LASTEXITCODE -ne 0) { throw 'Credential generation failed.' }
        $secret = ConvertTo-SecureString ([IO.File]::ReadAllText($passwordFile)) -AsPlainText -Force
        [PSCredential]::new('root@pam', $secret) | Export-Clixml -LiteralPath $credentialFile
        $secret = $null
    }
    $gate = 'WSL_DEPLOY'
    $linuxRepo = (& wsl.exe -d $Distribution --exec wslpath -u $repo).Trim()
    $linuxState = (& wsl.exe -d $Distribution --exec wslpath -u $state).Trim()
    & wsl.exe -d $Distribution --user root --exec bash "$linuxRepo/ops/compute/install-wsl.sh" $linuxRepo $linuxState
    if ($LASTEXITCODE -ne 0) { throw 'Compute deployment failed.' }
    $gate = 'HOST_TLS'
    $repair = Join-Path $env:ProgramData 'GNX\control\repair-host.ps1'
    Copy-Item -LiteralPath (Join-Path $repo 'ops\control\repair-host.ps1') -Destination $repair -Force
    & $repair -Distribution $Distribution
    $gate = 'LOGIN'
    & $helper verify $config $state
    if ($LASTEXITCODE -ne 0) { throw 'Compute login failed.' }
    $gate = 'RESTART'
    & wsl.exe -d $Distribution --user root --exec systemctl restart gnx-compute.service
    if ($LASTEXITCODE -ne 0) { throw 'Compute restart failed.' }
    $ready = $false
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        & $helper verify $config $state
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        Start-Sleep -Seconds 3
    }
    if (-not $ready) { throw 'Compute login did not recover.' }
    $gate = 'CONTROL_HEALTH'
    & (Join-Path $env:ProgramData 'GNX\control\gnx.exe') connect --config (Join-Path $env:ProgramData 'GNX\control\gnx.toml')
    if ($LASTEXITCODE -ne 0) { throw 'Control-plane connection failed.' }
    @{ result='READY'; gate='COMPLETE'; endpoint='https://proxmox.mesh.gnx'; authenticated=$true; service_restart=$true; full_reboot=$false } |
        ConvertTo-Json | Set-Content -LiteralPath $report
    'READY compute-connected'
} catch {
    @{ result='FAILED'; gate=$gate } | ConvertTo-Json | Set-Content -LiteralPath $report
    "FAILED COMPUTE_$gate"
    exit 1
} finally {
    if (Test-Path -LiteralPath $passwordFile) { Remove-Item -LiteralPath $passwordFile }
}
