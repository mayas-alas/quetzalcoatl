[CmdletBinding()]
param([ValidatePattern('^[A-Za-z0-9._-]+$')][string]$Distribution = 'Ubuntu-24.04')
$ErrorActionPreference = 'Stop'
$root = Join-Path $env:LOCALAPPDATA 'GNX'
$backups = Join-Path $root 'backups'
$recovery = Join-Path $root 'recovery'
$gate = 'ELEVATION'
try {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not ([Security.Principal.WindowsPrincipal]::new($identity)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Elevation required for identity comparison.'
    }
    foreach ($directory in @($backups, $recovery)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
        $acl = [Security.AccessControl.DirectorySecurity]::new()
        $acl.SetAccessRuleProtection($true, $false)
        foreach ($sid in @($identity.User.Value, 'S-1-5-18', 'S-1-5-32-544')) {
            $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
                [Security.Principal.SecurityIdentifier]::new($sid), 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
        }
        Set-Acl -LiteralPath $directory -AclObject $acl
    }
    $gate = 'REBOOT_IDENTITY'
    $expected = [IO.File]::ReadAllText((Join-Path $env:ProgramData 'GNX\control\peer-id'))
    $query = 'import sqlite3; c=sqlite3.connect("file:/var/lib/gnx/control/state/store.db?mode=ro",uri=True); print("\n".join(r[0] for r in c.execute("SELECT id FROM peers")))'
    $peers = @($query | & wsl.exe -d $Distribution --user root --exec python3 -)
    if ($LASTEXITCODE -ne 0 -or $peers.Count -ne 1 -or $peers[0] -cne $expected) { throw 'Peer identity changed or could not be verified.' }
    $gate = 'SNAPSHOT'
    $stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
    $archive = Join-Path $backups "control-$stamp.tar.gz.age"
    $key = Join-Path $recovery 'control.agekey'
    $helper = Join-Path $PSScriptRoot 'target\release\gnx-snapshot.exe'
    $script = (& wsl.exe -d $Distribution --exec wslpath -u (Join-Path $PSScriptRoot 'snapshot-wsl.sh')).Trim()
    & $helper create $Distribution $script $archive $key
    if ($LASTEXITCODE -ne 0) { throw 'Snapshot failed.' }
    & $helper check $archive $key
    if ($LASTEXITCODE -ne 0) { throw 'Recovery check failed.' }
    $gate = 'POST_BACKUP_HEALTH'
    & (Join-Path $env:ProgramData 'GNX\control\gnx.exe') connect --config (Join-Path $env:ProgramData 'GNX\control\gnx.toml')
    if ($LASTEXITCODE -ne 0) { throw 'Client did not recover.' }
    $null = Invoke-RestMethod 'https://mesh.gnx/api/instance' -TimeoutSec 20 -MaximumRedirection 0
    $boot = (Get-CimInstance Win32_OperatingSystem).LastBootUpTime.ToUniversalTime().ToString('o')
    @{ result='READY'; gate='COMPLETE'; boot_utc=$boot; same_peer=$true; archive=$archive; roundtrip_verified=$true; external_copy=$false } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $backups 'latest.json')
    'READY control-backup'
} catch {
    if (Test-Path -LiteralPath $backups) {
        @{ result='FAILED'; gate=$gate } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $backups 'latest.json')
    }
    "FAILED CONTROL_BACKUP_$gate"
    exit 1
}
