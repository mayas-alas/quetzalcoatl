[CmdletBinding()]
param([string]$Bundle = (Join-Path $PSScriptRoot '..\..\dist\windows'))

$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSEdition -eq 'Desktop') { $env:PSModulePath = $PSHOME + '\Modules' }
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this host installer from an elevated PowerShell.'
}

$bundlePath = (Resolve-Path -LiteralPath $Bundle).Path
$destination = 'C:\Program Files\GNX'
$oldProgram = 'C:\Program Files\QuetzalcoatlNext'
$oldExe = Join-Path $oldProgram 'gnx.exe'
$oldServiceName = 'QuetzalcoatlNext'
$runKey = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run'
$runName = 'QuetzalcoatlNextTray'
$oldRoots = @($oldProgram, 'C:\ProgramData\QuetzalcoatlNext', 'C:\ProgramData\Quetzalcoatl', 'C:\ProgramData\Quetzalcoatl.Runtime')
$files = @('gnx.exe', 'gnx.exe.sha256', 'access.toml', 'gnx.example.toml')
$report = Join-Path $env:ProgramData 'GNX\host-install-status.json'
$backupRoot = Join-Path $env:ProgramData 'GNX\retired-host'
$backup = Join-Path $backupRoot ([DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ'))
$gate = 'PREFLIGHT'

function Assert-PlainDirectory([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $item = Get-Item -LiteralPath $Path -Force
    if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'Refusing a redirected or non-directory installation path.'
    }
    foreach ($child in Get-ChildItem -LiteralPath $Path -Recurse -Force) {
        if ($child.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Refusing a redirected child.' }
    }
}

function Open-RetiredDirectory([string]$Path) {
    if ($Path -notin $oldRoots -or [IO.Path]::GetFullPath($Path) -ne $Path) { throw 'Unexpected retirement path.' }
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $pending = [Collections.Generic.Stack[string]]::new()
    $pending.Push($Path)
    while ($pending.Count) {
        $current = $pending.Pop()
        if ([IO.Path]::GetFullPath($current) -ne $current -or
            ($current -ne $Path -and -not $current.StartsWith($Path + '\', [StringComparison]::OrdinalIgnoreCase))) { throw 'Unsafe retired child path.' }
        $item = Get-Item -LiteralPath $current -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Unsafe retired directory link.' }
        if (-not $item.PSIsContainer) { continue }
        try { $children = @(Get-ChildItem -LiteralPath $current -Force) } catch [UnauthorizedAccessException] {
            # One validated directory at a time; never recursive ownership across links.
            $null = & "$env:SystemRoot\System32\takeown.exe" /F $current /A
            if ($LASTEXITCODE -ne 0) { throw 'Could not recover retired directory ownership.' }
            $null = & "$env:SystemRoot\System32\icacls.exe" $current /grant '*S-1-5-32-544:(OI)(CI)F' /L /Q
            if ($LASTEXITCODE -ne 0) { throw 'Could not grant retired directory access.' }
            $children = @(Get-ChildItem -LiteralPath $current -Force)
        }
        foreach ($child in $children) { $pending.Push($child.FullName) }
    }
}

function Protect-Backup([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force
    if ($item.PSIsContainer) {
        $acl = [Security.AccessControl.DirectorySecurity]::new()
        $inherit = [Security.AccessControl.InheritanceFlags]'ContainerInherit,ObjectInherit'
    } else {
        $acl = [Security.AccessControl.FileSecurity]::new()
        $inherit = [Security.AccessControl.InheritanceFlags]::None
    }
    $acl.SetAccessRuleProtection($true, $false)
    foreach ($sidText in @('S-1-5-18','S-1-5-32-544')) {
        $sid = [Security.Principal.SecurityIdentifier]::new($sidText)
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', $inherit, 'None', 'Allow'))
    }
    Set-Acl -LiteralPath $Path -AclObject $acl
}

try {
    foreach ($file in $files) {
        $item = Get-Item -LiteralPath (Join-Path $bundlePath $file)
        if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Invalid bundle file.' }
    }
    $digest = (Get-Content -LiteralPath (Join-Path $bundlePath 'gnx.exe.sha256') -Raw).Trim()
    if ($digest -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash -LiteralPath (Join-Path $bundlePath 'gnx.exe')).Hash -ne $digest) {
        throw 'Executable digest mismatch.'
    }
    Assert-PlainDirectory $destination
    if (Test-Path -LiteralPath $destination) {
        $foreign = @(Get-ChildItem -LiteralPath $destination -Force | Where-Object { $_.Name -notin $files })
        if ($foreign.Count) { throw 'The destination contains unmanaged files.' }
    }
    foreach ($oldRoot in $oldRoots) {
        if ([IO.Path]::GetFullPath($oldRoot) -ne $oldRoot) { throw 'Unexpected retirement path.' }
        Open-RetiredDirectory $oldRoot
        Assert-PlainDirectory $oldRoot
        if (Get-ChildItem -LiteralPath $oldRoot -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.Extension -in @('.vhd','.vhdx','.qcow2','.img') }) {
            throw 'Machine disks require separate review; nothing has been retired.'
        }
    }
    $oldService = Get-CimInstance Win32_Service -Filter "Name='$oldServiceName'"
    if ($oldService -and $oldService.PathName -notmatch '^"?C:\\Program Files\\QuetzalcoatlNext\\gnx\.exe(?:"|\s|$)') {
        throw 'The old service points to an unexpected executable.'
    }
    if ($oldService -and (Get-Service $oldServiceName).DependentServices.Count) { throw 'The old service has dependents.' }
    $oldAutorun = (Get-ItemProperty -LiteralPath $runKey).PSObject.Properties[$runName].Value
    if ($oldAutorun -and $oldAutorun -notmatch '^"?C:\\Program Files\\QuetzalcoatlNext\\gnx\.exe(?:"|\s|$)') { throw 'Unexpected retired startup entry.' }
    Assert-PlainDirectory $backupRoot
    if ($backupRoot -ne 'C:\ProgramData\GNX\retired-host') { throw 'Unexpected backup root.' }
    New-Item -ItemType Directory -Path $backup -Force | Out-Null
    Protect-Backup $backupRoot
    Protect-Backup $backup
    $machinePath = [Environment]::GetEnvironmentVariable('Path','Machine')
    @{ machine_path_before=$machinePath; retired_service=$oldServiceName; retired_autorun=$oldAutorun } | ConvertTo-Json |
        Set-Content -LiteralPath (Join-Path $backup 'restore.json') -Encoding UTF8
    Protect-Backup (Join-Path $backup 'restore.json')

    $gate = 'INSTALL'
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    foreach ($file in $files) {
        $target = Join-Path $destination $file
        if ($file -in @('access.toml','gnx.example.toml') -and (Test-Path -LiteralPath $target)) { continue }
        Copy-Item -LiteralPath (Join-Path $bundlePath $file) -Destination $target -Force
    }
    $installedExe = Join-Path $destination 'gnx.exe'
    if ((Get-FileHash -LiteralPath $installedExe).Hash -ne $digest) { throw 'Installed executable does not match.' }
    $null = & $installedExe credentials --help
    if ($LASTEXITCODE -ne 0) { throw 'Installed CLI validation failed.' }

    $gate = 'RETIRE'
    if ($oldAutorun) {
        if ((Get-ItemProperty -LiteralPath $runKey).PSObject.Properties[$runName].Value -ne $oldAutorun) { throw 'Startup entry changed.' }
        Remove-ItemProperty -LiteralPath $runKey -Name $runName
    }
    if ($oldService) {
        Set-Service -Name $oldServiceName -StartupType Disabled
        Stop-Service -Name $oldServiceName
        (Get-Service $oldServiceName).WaitForStatus('Stopped', [TimeSpan]::FromSeconds(30))
        $null = & "$env:SystemRoot\System32\sc.exe" delete $oldServiceName
        if ($LASTEXITCODE -ne 0) { throw 'Could not remove the retired service.' }
    }
    Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq $oldExe } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
    $retired = @()
    foreach ($oldRoot in $oldRoots) {
        if (-not (Test-Path -LiteralPath $oldRoot)) { continue }
        # Resolve both ends immediately before every move; never move a workspace/root.
        $resolved = (Resolve-Path -LiteralPath $oldRoot).Path
        if ($resolved -ne $oldRoot -or $resolved -notin $oldRoots) { throw 'Retirement target changed.' }
        Assert-PlainDirectory $resolved
        $label = if ($resolved -eq $oldProgram) { 'program' } else { Split-Path $resolved -Leaf }
        $target = [IO.Path]::GetFullPath((Join-Path $backup $label))
        if ([IO.Path]::GetDirectoryName($target) -ne $backup -or (Test-Path -LiteralPath $target)) { throw 'Unsafe retirement destination.' }
        Move-Item -LiteralPath $resolved -Destination $target
        Protect-Backup $target
        Get-ChildItem -LiteralPath $target -Recurse -Force | ForEach-Object { Protect-Backup $_.FullName }
        $retired += $resolved
    }
    $retiredAccount = Get-LocalUser -Name 'gnx-runtime' -ErrorAction SilentlyContinue
    $accountDisabled = $false
    if ($retiredAccount -and $retiredAccount.Description -eq 'Quetzalcoatl Next isolated runtime') {
        $services = @(Get-CimInstance Win32_Service | Where-Object { $_.StartName -match '(?:^|\\)gnx-runtime$' })
        $tasks = @(Get-ScheduledTask | Where-Object { $_.Principal.UserId -in @('gnx-runtime', '.\gnx-runtime', $retiredAccount.SID.Value) })
        if (-not $services.Count -and -not $tasks.Count) {
            Disable-LocalUser -Name 'gnx-runtime'
            $accountDisabled = $true
        }
    }
    # Preserve the retired profile and any machine disks until separately authorized.
    $profile = if ($retiredAccount) { Get-CimInstance Win32_UserProfile | Where-Object { $_.SID -eq $retiredAccount.SID.Value } }
    $retiredMachines = @()
    if ($retiredAccount) {
        $registry = 'Registry::HKEY_USERS\' + $retiredAccount.SID.Value + '\Software\Microsoft\Windows\CurrentVersion\Lxss'
        if (Test-Path -LiteralPath $registry) {
            $retiredMachines = @(Get-ChildItem -LiteralPath $registry | Get-ItemProperty | Select-Object DistributionName,BasePath)
        }
    }

    $gate = 'PATH'
    $pathParts = @($machinePath -split ';' | Where-Object { $_.Trim().TrimEnd('\') -notin @($oldProgram,$destination) })
    [Environment]::SetEnvironmentVariable('Path', (($pathParts + $destination) -join ';'), 'Machine')
    $env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
    if ((Get-Command gnx.exe).Source -ne $installedExe) { throw 'A different CLI still shadows the installation.' }
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class GnxEnvironmentNotice {
    [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern IntPtr SendMessageTimeout(IntPtr window, uint message, UIntPtr param, string value, uint flags, uint timeout, out UIntPtr result);
}
'@
    $broadcastResult = [UIntPtr]::Zero
    $null = [GnxEnvironmentNotice]::SendMessageTimeout([IntPtr]0xffff, 0x1a, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$broadcastResult)
    @{ result='READY'; executable=$installedExe; sha256=$digest; retired=$retired; backup=$backup;
       old_account_disabled=$accountDisabled; preserved_profile=$profile.LocalPath; profile_loaded=$profile.Loaded;
       retired_machines=$retiredMachines; restart_terminal=$true } |
        ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $report -Encoding UTF8
    Write-Output 'READY host-cli; reopen your terminal'
} catch {
    @{ result='FAILED'; gate=$gate; backup=$backup; reason=$_.Exception.Message } | ConvertTo-Json | Set-Content -LiteralPath $report -Encoding UTF8
    throw "FAILED HOST_INSTALL_$gate"
}
