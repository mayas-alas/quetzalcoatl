[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+$')][string]$OwnerEmail,
    [string]$Distribution = 'Ubuntu-24.04'
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$state = Join-Path $env:ProgramData 'GNX\control'
$report = Join-Path $env:ProgramData 'GNX\control-status.json'
$gate = 'ELEVATION'
try {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not ([Security.Principal.WindowsPrincipal]::new($identity)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Elevation required.'
    }
    New-Item -ItemType Directory -Path $state -Force | Out-Null
    $acl = [Security.AccessControl.DirectorySecurity]::new()
    $acl.SetAccessRuleProtection($true, $false)
    foreach ($sid in @('S-1-5-18', 'S-1-5-32-544')) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new(
            [Security.Principal.SecurityIdentifier]::new($sid), 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
        $acl.AddAccessRule($rule)
    }
    Set-Acl -LiteralPath $state -AclObject $acl
    $gate = 'ARTIFACTS'
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'target\release\gnx-control.exe') -Destination $state -Force
    Copy-Item -LiteralPath (Join-Path $repo 'dist\windows\gnx.exe') -Destination $state -Force
    Copy-Item -LiteralPath (Join-Path $repo 'config\gnx.example.toml') -Destination (Join-Path $state 'gnx.toml') -Force
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'repair-host.ps1') -Destination $state -Force
    $config = Join-Path $state 'control.toml'
    if (-not (Test-Path -LiteralPath $config)) {
        [IO.File]::WriteAllText($config, "endpoint = `"https://mesh.gnx`"`nowner_email = `"$OwnerEmail`"", [Text.UTF8Encoding]::new($false))
    }
    $helper = Join-Path $state 'gnx-control.exe'
    if (-not (Test-Path -LiteralPath (Join-Path $state 'complete'))) {
        $gate = 'RENDER'
        & $helper render $config $state
        if ($LASTEXITCODE -ne 0) { throw 'Render failed.' }
        $templates = (& wsl.exe -d $Distribution --exec wslpath -u (Join-Path $repo 'runtime\control')).Trim()
        $sourceState = (& wsl.exe -d $Distribution --exec wslpath -u $state).Trim()
        $installer = (& wsl.exe -d $Distribution --exec wslpath -u (Join-Path $PSScriptRoot 'install-wsl.sh')).Trim()
        $gate = 'WSL_DEPLOY'
        & wsl.exe -d $Distribution --user root --exec bash $installer $templates $sourceState
        if ($LASTEXITCODE -ne 0) { throw 'WSL deployment failed.' }
        $certificate = & wsl.exe -d $Distribution --user root --exec cat /var/lib/gnx/control/tls/root.crt
        if ($LASTEXITCODE -ne 0) { throw 'CA export failed.' }
        $certificate | Set-Content -LiteralPath (Join-Path $state 'root.crt') -Encoding ascii
    }
    $gate = 'HOST_TLS'
    & (Join-Path $state 'repair-host.ps1') -Distribution $Distribution
    $gate = 'LOGON_ROUTINE'
    $action = New-ScheduledTaskAction -Execute "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -Argument "-NoProfile -WindowStyle Hidden -File `"$state\repair-host.ps1`" -Distribution `"$Distribution`" -StayActive"
    $trigger = New-ScheduledTaskTrigger -AtLogOn -User $identity.Name
    $principal = New-ScheduledTaskPrincipal -UserId $identity.Name -LogonType Interactive -RunLevel Highest
    $settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    Register-ScheduledTask -TaskName 'GNX Control Host' -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null
    Start-ScheduledTask -TaskName 'GNX Control Host'
    if (-not (Test-Path -LiteralPath (Join-Path $state 'complete'))) {
        $gate = 'BOOTSTRAP'
        & $helper bootstrap $config $state
        if ($LASTEXITCODE -ne 0) { throw 'Bootstrap failed.' }
        $owner = Get-Content -LiteralPath (Join-Path $state 'owner.json') -Raw | ConvertFrom-Json
        $password = ConvertTo-SecureString $owner.password -AsPlainText -Force
        [PSCredential]::new($owner.email, $password) | Export-Clixml -LiteralPath (Join-Path $state 'owner.credential.xml')
        $owner = $null
        $password = $null
        $gate = 'CONNECT'
        Restart-Service -Name NetBird
        & (Join-Path $state 'gnx.exe') connect --config (Join-Path $state 'gnx.toml') --setup-key-file (Join-Path $state 'join.key')
        if ($LASTEXITCODE -ne 0) { throw 'Connect failed.' }
        $gate = 'VERIFY'
        & $helper verify $config $state
        if ($LASTEXITCODE -ne 0) { throw 'Peer verification failed.' }
        $gate = 'RECONNECT'
        Restart-Service -Name NetBird
        & (Join-Path $state 'gnx.exe') connect --config (Join-Path $state 'gnx.toml')
        if ($LASTEXITCODE -ne 0) { throw 'Reconnect failed.' }
        & $helper verify $config $state
        if ($LASTEXITCODE -ne 0) { throw 'Identity verification failed.' }
        $gate = 'FINALIZE'
        & $helper finalize $config $state
        if ($LASTEXITCODE -ne 0) { throw 'Credential revocation failed.' }
        Remove-Item -LiteralPath (Join-Path $state 'owner.json')
        & wsl.exe -d $Distribution --user root --exec sh -c 'printf "NB_SETUP_PAT_ENABLED=false\n" > /var/lib/gnx/control/bootstrap.env'
        if ($LASTEXITCODE -ne 0) { throw 'Bootstrap closure failed.' }
        & wsl.exe -d $Distribution --user root --exec systemctl restart gnx-control.service
        if ($LASTEXITCODE -ne 0) { throw 'Control restart failed.' }
        'complete' | Set-Content -LiteralPath (Join-Path $state 'complete')
    }
    $gate = 'FINAL_HEALTH'
    & (Join-Path $state 'gnx.exe') connect --config (Join-Path $state 'gnx.toml')
    if ($LASTEXITCODE -ne 0) { throw 'Final connection health check failed.' }
    $credentialDirectory = Join-Path $env:LOCALAPPDATA 'GNX\control'
    New-Item -ItemType Directory -Path $credentialDirectory -Force | Out-Null
    $credentialAcl = [Security.AccessControl.DirectorySecurity]::new()
    $credentialAcl.SetAccessRuleProtection($true, $false)
    foreach ($sid in @($identity.User.Value, 'S-1-5-18', 'S-1-5-32-544')) {
        $credentialAcl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
            [Security.Principal.SecurityIdentifier]::new($sid), 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
    }
    Set-Acl -LiteralPath $credentialDirectory -AclObject $credentialAcl
    Copy-Item -LiteralPath (Join-Path $state 'owner.credential.xml') -Destination $credentialDirectory -Force
    @{ result = 'READY'; gate = 'COMPLETE' } | ConvertTo-Json | Set-Content -LiteralPath $report
    'READY control-connected'
} catch {
    # Keep diagnostic labels only; never serialize an exception or HTTP body.
    if (Test-Path -LiteralPath $state) {
        @{ result = 'FAILED'; gate = $gate } | ConvertTo-Json | Set-Content -LiteralPath $report
    }
    Write-Output "FAILED CONTROL_$gate"
    exit 1
}
