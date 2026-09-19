#requires -Version 5.1
[CmdletBinding(SupportsShouldProcess)]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$ProgramRoot = 'C:\Program Files\GNX-0.3.1'
$DataRoot = 'C:\ProgramData\GNX-0.3.1'
$SetupRoot = 'C:\ProgramData\GNX-Setup-0.3.1'
$ServiceName = 'GNXRuntime'
$ServiceExe = Join-Path $ProgramRoot 'gnx-service.exe'
$RuntimeAccount = 'gnx-runtime'
$Distribution = 'GNX-0.3.1'
$ProfileRoot = 'C:\Users\gnx-runtime'
$AllowedRoots = @($ProgramRoot, $DataRoot, $SetupRoot, $ProfileRoot)
$FailureCode = 'UNINSTALL_FAILED'

function Stop-Uninstall([string]$Code) {
    $script:FailureCode = $Code
    throw 'GNX uninstall blocked.'
}

function Assert-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Stop-Uninstall 'ELEVATION_REQUIRED'
    }
}

function Assert-AllowedRoot([string]$Path) {
    if ($Path -notin $AllowedRoots -or [IO.Path]::GetFullPath($Path) -ne $Path) { Stop-Uninstall 'ROOT_CONFLICT' }
}

function Assert-PlainPath([string]$Path) {
    $current = $Path
    while ($current) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { Stop-Uninstall 'REPARSE_POINT' }
        }
        $current = Split-Path -Path $current -Parent
    }
}

function Assert-PlainTree([string]$Path) {
    Assert-AllowedRoot $Path
    Assert-PlainPath $Path
    if (-not (Test-Path -LiteralPath $Path)) { return }
    if (-not (Get-Item -LiteralPath $Path -Force).PSIsContainer) { Stop-Uninstall 'ROOT_CONFLICT' }
    $pending = [Collections.Generic.Stack[string]]::new()
    $pending.Push($Path)
    while ($pending.Count -gt 0) {
        $current = $pending.Pop()
        if ([IO.Path]::GetFullPath($current) -ne $current -or ($current -ne $Path -and -not $current.StartsWith($Path + '\', [StringComparison]::OrdinalIgnoreCase))) {
            Stop-Uninstall 'CHILD_PATH_CONFLICT'
        }
        $item = Get-Item -LiteralPath $current -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { Stop-Uninstall 'REPARSE_POINT' }
        if ($item.PSIsContainer) {
            foreach ($child in Get-ChildItem -LiteralPath $current -Force) { $pending.Push($child.FullName) }
        }
    }
}

function Assert-ProtectedMetadata([string]$Path) {
    Assert-PlainPath $Path
    $acl = Get-Acl -LiteralPath $Path
    $trusted = @('S-1-5-18', 'S-1-5-32-544')
    if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin $trusted) { Stop-Uninstall 'WITNESS_ACL_CONFLICT' }
    $writeMask = [Security.AccessControl.FileSystemRights]'Write,Delete,DeleteSubdirectoriesAndFiles,ChangePermissions,TakeOwnership'
    foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
        if ($rule.AccessControlType -eq 'Allow' -and ($rule.FileSystemRights -band $writeMask) -and $rule.IdentityReference.Value -notin $trusted) {
            Stop-Uninstall 'WITNESS_ACL_CONFLICT'
        }
    }
}

function Read-Witness([string]$Name) {
    $path = Join-Path $SetupRoot $Name
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    Assert-ProtectedMetadata $SetupRoot
    Assert-ProtectedMetadata $path
    $item = Get-Item -LiteralPath $path -Force
    if ($item.PSIsContainer -or $item.Length -gt 4096) { Stop-Uninstall 'WITNESS_INVALID' }
    return (Get-Content -LiteralPath $path -Raw | ConvertFrom-Json)
}

function Get-RuntimeUser {
    # Enumeration distinguishes absence from access/provider failure.
    $users = @(Get-LocalUser -ErrorAction Stop | Where-Object { $_.Name -eq $RuntimeAccount })
    if ($users.Count -gt 1) { Stop-Uninstall 'ACCOUNT_CONFLICT' }
    if ($users.Count) { return $users[0] }
    return $null
}

function Get-RuntimeService {
    return (Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction Stop)
}

function Test-RuntimeIdentity([string]$Identity, [string]$Sid) {
    return ($Identity -in @($RuntimeAccount, ".\$RuntimeAccount", "$env:COMPUTERNAME\$RuntimeAccount", $Sid))
}

function Assert-ServiceOwner($Service) {
    if ($null -eq $Service) { return }
    if ($Service.PathName -notin @($ServiceExe, ('"' + $ServiceExe + '"')) -or
        $Service.StartName -notin @(".\$RuntimeAccount", "$env:COMPUTERNAME\$RuntimeAccount")) {
        Stop-Uninstall 'SERVICE_OWNER_CONFLICT'
    }
}

function Get-AccountWitness($User, $Service) {
    $saved = Read-Witness 'uninstall-owner.json'
    if ($saved) {
        if ($saved.schema -ne 1 -or $saved.account -ne $RuntimeAccount -or $saved.sid -notmatch '^S-1-5-21-\d+-\d+-\d+-\d+$') { Stop-Uninstall 'WITNESS_INVALID' }
        if ($User -and $User.SID.Value -ne $saved.sid) { Stop-Uninstall 'ACCOUNT_SID_CONFLICT' }
        return [string]$saved.sid
    }
    if (-not $User) { return $null }
    # Description alone is not ownership. Native setup leaves Description empty.
    if ($Service) { Assert-ServiceOwner $Service; return $User.SID.Value }
    $journal = Read-Witness 'journal.json'
    if ($journal -and $journal.schema -eq 1 -and $journal.operation -eq 'PROVISION' -and $journal.phase -in @('REGISTERING', 'SECURING', 'PROVISIONED')) {
        return $User.SID.Value
    }
    Stop-Uninstall 'ACCOUNT_WITNESS_REQUIRED'
}

function Assert-InstallationWitness($Service) {
    if ($Service) { Assert-ServiceOwner $Service; return }
    if (Read-Witness 'uninstall-owner.json') { return }
    $journal = Read-Witness 'journal.json'
    if ($journal -and $journal.schema -eq 1 -and $journal.operation -eq 'PROVISION' -and
        $journal.phase -in @('STAGING', 'PUBLISHING', 'REGISTERING', 'SECURING', 'PROVISIONED')) { return }
    foreach ($root in @($ProgramRoot, $DataRoot, $SetupRoot)) {
        if (Test-Path -LiteralPath $root) {
            # A previous run may have removed its last witness before the final
            # empty-directory removal failed. No foreign contents are adopted.
            if ($root -eq $SetupRoot -and -not @(Get-ChildItem -LiteralPath $root -Force).Count) {
                Assert-ProtectedMetadata $root
                continue
            }
            Stop-Uninstall 'INSTALLATION_WITNESS_REQUIRED'
        }
    }
}

function Save-AccountWitness([string]$Sid) {
    if (-not $Sid) { return }
    $path = Join-Path $SetupRoot 'uninstall-owner.json'
    if (Test-Path -LiteralPath $path) { return }
    Assert-ProtectedMetadata $SetupRoot
    $staging = Join-Path $SetupRoot 'uninstall-owner.gnx-new'
    if (Test-Path -LiteralPath $staging) {
        Assert-ProtectedMetadata $staging
        if ((Get-Item -LiteralPath $staging -Force).PSIsContainer) { Stop-Uninstall 'WITNESS_INVALID' }
        Remove-Item -LiteralPath $staging -Force -ErrorAction Stop
    }
    $bytes = [Text.Encoding]::UTF8.GetBytes((@{schema=1; account=$RuntimeAccount; sid=$Sid} | ConvertTo-Json -Compress))
    $file = [IO.File]::Open($staging, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $file.Write($bytes, 0, $bytes.Length); $file.Flush($true) } finally { $file.Dispose() }
    Move-Item -LiteralPath $staging -Destination $path -ErrorAction Stop
    Assert-ProtectedMetadata $path
}

function Assert-NoAccountReferences([string]$Sid, [switch]$AllowRuntimeService) {
    if (-not $Sid) { return }
    $services = @(Get-CimInstance Win32_Service -ErrorAction Stop | Where-Object {
        (Test-RuntimeIdentity $_.StartName $Sid) -and (-not $AllowRuntimeService -or $_.Name -ne $ServiceName)
    })
    $tasks = @(Get-ScheduledTask -ErrorAction Stop | Where-Object { Test-RuntimeIdentity $_.Principal.UserId $Sid })
    if ($services.Count -or $tasks.Count) { Stop-Uninstall 'ACCOUNT_REFERENCED' }
}

function Stop-OwnedService {
    $service = Get-RuntimeService
    if (-not $service) { return }
    Assert-ServiceOwner $service
    if ($service.State -ne 'Stopped') {
        Stop-Service -Name $ServiceName -ErrorAction Stop
        $handle = Get-Service -Name $ServiceName -ErrorAction Stop
        try { $handle.WaitForStatus('Stopped', [TimeSpan]::FromSeconds(30)) } finally { $handle.Dispose() }
    }
    $service = Get-RuntimeService
    if ($service) {
        Assert-ServiceOwner $service
        if ($service.State -ne 'Stopped' -or $service.ProcessId -ne 0) { Stop-Uninstall 'SERVICE_NOT_STOPPED' }
    }
}

function Invoke-Native([string]$File, [string[]]$Arguments) {
    # Capture both streams: native diagnostics can contain paths or secrets.
    $output = @(& $File @Arguments 2>&1)
    if ($LASTEXITCODE -ne 0) { Stop-Uninstall 'NATIVE_OPERATION_FAILED' }
    return $output
}

function Remove-OwnedService {
    $service = Get-RuntimeService
    if (-not $service) { return $false }
    Assert-ServiceOwner $service
    if ($service.State -ne 'Stopped' -or $service.ProcessId -ne 0) { Stop-Uninstall 'SERVICE_NOT_STOPPED' }
    $null = Invoke-Native "$env:SystemRoot\System32\sc.exe" @('delete', $ServiceName)
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        if (-not (Get-RuntimeService)) { return $true }
        Start-Sleep -Milliseconds 200
    }
    Stop-Uninstall 'SERVICE_DELETE_PENDING'
}

function Get-Distros([switch]$Running) {
    $wsl = Join-Path $env:SystemRoot 'System32\wsl.exe'
    if (-not (Test-Path -LiteralPath $wsl)) {
        if (Test-Path -LiteralPath (Join-Path $DataRoot 'wsl')) { Stop-Uninstall 'WSL_UNAVAILABLE' }
        return @()
    }
    $arguments = @('--list', '--quiet')
    if ($Running) { $arguments += '--running' }
    return @(Invoke-Native $wsl $arguments | ForEach-Object { ([string]$_).Replace([string][char]0, '').Trim([char]0xFEFF).Trim() } | Where-Object { $_ })
}

function Assert-DistroContext([string]$Sid) {
    # WSL registrations are per-user. Never infer another user's absence from
    # the elevated operator's list, and never delete an offline hive to hide it.
    if (-not $Sid -or $Sid -eq [Security.Principal.WindowsIdentity]::GetCurrent().User.Value) { return }
    $profiles = @(Get-CimInstance Win32_UserProfile -ErrorAction Stop | Where-Object { $_.SID -eq $Sid })
    $hive = "Registry::HKEY_USERS\$Sid"
    $mounted = $null
    if (-not (Test-Path -LiteralPath $hive) -and $profiles.Count) {
        # Inspect an offline hive only after SID, exact profile path and tree
        # validation. Mount to a fresh name and always unload before returning.
        $null = Get-OwnedProfile $Sid
        $hiveFile = Join-Path $ProfileRoot 'NTUSER.DAT'
        if (-not (Test-Path -LiteralPath $hiveFile -PathType Leaf)) { Stop-Uninstall 'WSL_RUNTIME_CONTEXT_UNVERIFIED' }
        $mounted = 'HKU\GNXUninstall-' + [Guid]::NewGuid().ToString('N')
        $null = Invoke-Native "$env:SystemRoot\System32\reg.exe" @('load', $mounted, $hiveFile)
        $hive = 'Registry::HKEY_USERS\' + $mounted.Substring(4)
    }
    try {
      if (Test-Path -LiteralPath $hive) {
        $lxss = Join-Path $hive 'Software\Microsoft\Windows\CurrentVersion\Lxss'
        if (Test-Path -LiteralPath $lxss) {
            foreach ($entry in Get-ChildItem -LiteralPath $lxss -ErrorAction Stop) {
                $registration = Get-ItemProperty -LiteralPath $entry.PSPath -ErrorAction Stop
                if ($registration.DistributionName -eq $Distribution) { Stop-Uninstall 'WSL_RUNTIME_CONTEXT_REQUIRED' }
                # Other distributions make deleting this identity/profile unsafe.
                Stop-Uninstall 'PROFILE_FOREIGN_DISTRO'
            }
        }
      } elseif ($profiles.Count -or (Test-Path -LiteralPath $ProfileRoot)) {
        Stop-Uninstall 'WSL_RUNTIME_CONTEXT_UNVERIFIED'
      }
    } finally {
        if ($mounted) { $null = Invoke-Native "$env:SystemRoot\System32\reg.exe" @('unload', $mounted) }
    }
}

function Get-OwnedDistro {
    if ((Get-Distros) -notcontains $Distribution) { return $null }
    $lxss = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Lxss'
    $matches = @(Get-ChildItem -LiteralPath $lxss -ErrorAction Stop | ForEach-Object {
        Get-ItemProperty -LiteralPath $_.PSPath -ErrorAction Stop
    } | Where-Object { $_.DistributionName -eq $Distribution })
    if ($matches.Count -ne 1) { Stop-Uninstall 'DISTRO_OWNER_CONFLICT' }
    $base = ([string]$matches[0].BasePath).TrimEnd('\')
    if ($base.StartsWith('\\?\')) { $base = $base.Substring(4) }
    if ($base -ne (Join-Path $DataRoot 'wsl')) { Stop-Uninstall 'DISTRO_OWNER_CONFLICT' }
    Assert-PlainPath $base
    return $Distribution
}

function Remove-OwnedDistro {
    if (-not (Get-OwnedDistro)) { return $false }
    $wsl = Join-Path $env:SystemRoot 'System32\wsl.exe'
    if ((Get-Distros -Running) -contains $Distribution) {
        $null = Invoke-Native $wsl @('--terminate', $Distribution)
        if ((Get-Distros -Running) -contains $Distribution) { Stop-Uninstall 'DISTRO_STILL_RUNNING' }
    }
    $null = Invoke-Native $wsl @('--unregister', $Distribution)
    if ((Get-Distros) -contains $Distribution) { Stop-Uninstall 'DISTRO_REMAINS' }
    return $true
}

function Get-OwnedProfile([string]$Sid, [switch]$AllowLoaded) {
    $profiles = @(Get-CimInstance Win32_UserProfile -ErrorAction Stop | Where-Object { $_.SID -eq $Sid -or $_.LocalPath -eq $ProfileRoot })
    if (-not $profiles.Count) {
        if (Test-Path -LiteralPath $ProfileRoot) { Stop-Uninstall 'PROFILE_OWNER_CONFLICT' }
        return $null
    }
    if (-not $Sid -or $profiles.Count -ne 1 -or $profiles[0].SID -ne $Sid -or $profiles[0].LocalPath -ne $ProfileRoot -or $profiles[0].Special) {
        Stop-Uninstall 'PROFILE_OWNER_CONFLICT'
    }
    if ($profiles[0].Loaded -and -not $AllowLoaded) { Stop-Uninstall 'PROFILE_LOADED' }
    Assert-PlainTree $ProfileRoot
    return $profiles[0]
}

function Remove-OwnedProfile([string]$Sid) {
    $profile = Get-OwnedProfile $Sid
    if (-not $profile) { return $false }
    $profile | Remove-CimInstance -ErrorAction Stop
    if (Get-OwnedProfile $Sid) { Stop-Uninstall 'PROFILE_REMAINS' }
    return $true
}

function Remove-OwnedAccount([string]$Sid) {
    $user = Get-RuntimeUser
    if (-not $user) { return $false }
    if (-not $Sid -or $user.SID.Value -ne $Sid) { Stop-Uninstall 'ACCOUNT_SID_CONFLICT' }
    Assert-NoAccountReferences $Sid
    Remove-LocalUser -SID $user.SID -ErrorAction Stop
    if (Get-RuntimeUser) { Stop-Uninstall 'ACCOUNT_REMAINS' }
    return $true
}

function Test-OwnedCommand([string]$Command) {
    # Exact executable, optional arguments; unquoted paths with spaces are unsafe.
    foreach ($name in @('gnx.exe', 'gnx-service.exe', 'gnx-setup.exe')) {
        $exe = Join-Path $ProgramRoot $name
        if ($Command -eq $exe -or $Command -match ('^"' + [regex]::Escape($exe) + '"(?:\s|$)')) { return $true }
    }
    return $false
}

function Get-OwnedAutoruns {
    foreach ($key in @('HKLM:\Software\Microsoft\Windows\CurrentVersion\Run', 'HKLM:\Software\Microsoft\Windows\CurrentVersion\RunOnce',
        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run', 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\RunOnce',
        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run', 'HKCU:\Software\Microsoft\Windows\CurrentVersion\RunOnce')) {
        if (-not (Test-Path -LiteralPath $key)) { continue }
        $registryKey = Get-Item -LiteralPath $key -ErrorAction Stop
        foreach ($name in $registryKey.GetValueNames()) {
            $value = [string]$registryKey.GetValue($name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            if (Test-OwnedCommand $value) { [pscustomobject]@{ Key=$key; Name=$name } }
            elseif ($name -match '^GNX' -or $value.IndexOf($ProgramRoot, [StringComparison]::OrdinalIgnoreCase) -ge 0) { Stop-Uninstall 'AUTORUN_OWNER_CONFLICT' }
        }
    }
}

function Get-OwnedShortcuts {
    $roots = @([Environment]::GetFolderPath('CommonStartMenu'), [Environment]::GetFolderPath('CommonDesktopDirectory')) | Where-Object { $_ }
    $shell = New-Object -ComObject WScript.Shell
    try {
        foreach ($root in $roots) {
            Assert-PlainPath $root
            if (-not (Test-Path -LiteralPath $root)) { continue }
            $pending = [Collections.Generic.Stack[string]]::new()
            $pending.Push($root)
            while ($pending.Count) {
                foreach ($item in Get-ChildItem -LiteralPath $pending.Pop() -Force -ErrorAction Stop) {
                    # Never traverse junctions in the shared Start Menu.
                    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { Stop-Uninstall 'SHORTCUT_REPARSE_POINT' }
                    if ($item.PSIsContainer) { $pending.Push($item.FullName) }
                    elseif ($item.Extension -eq '.lnk') {
                        $link = $shell.CreateShortcut($item.FullName)
                        try { if (Test-OwnedCommand $link.TargetPath) { $item.FullName } }
                        finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($link) }
                    }
                }
            }
        }
    } finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell) }
}

function Remove-SetupRoot {
    Assert-PlainTree $SetupRoot
    # Remove witness files last so partial deletion remains retryable.
    $witnessNames = @('uninstall-owner.json', 'journal.json')
    foreach ($child in Get-ChildItem -LiteralPath $SetupRoot -Force) {
        if ($child.Name -notin $witnessNames) { Remove-Item -LiteralPath $child.FullName -Recurse -Force -ErrorAction Stop }
    }
    foreach ($name in $witnessNames) {
        $path = Join-Path $SetupRoot $name
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force -ErrorAction Stop }
    }
    Remove-Item -LiteralPath $SetupRoot -Force -ErrorAction Stop
    if (Test-Path -LiteralPath $SetupRoot) { Stop-Uninstall 'ROOT_REMAINS' }
}

function Assert-ResourcesRemoved([string]$Sid) {
    if ((Get-RuntimeService) -or (Get-RuntimeUser) -or (Get-OwnedProfile $Sid) -or ((Get-Distros) -contains $Distribution) -or
        @(Get-OwnedAutoruns).Count -or @(Get-OwnedShortcuts).Count) { Stop-Uninstall 'RESOURCES_REMAIN' }
    $remainingPath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    if (@($remainingPath -split ';' | Where-Object { $_.Trim().Trim('"').TrimEnd('\') -eq $ProgramRoot }).Count) { Stop-Uninstall 'PATH_REMAINS' }
}

function Invoke-Uninstall {
    Assert-Admin
    foreach ($root in @($ProgramRoot, $DataRoot, $SetupRoot)) { Assert-PlainTree $root }
    $service = Get-RuntimeService
    Assert-ServiceOwner $service
    $sid = Get-AccountWitness (Get-RuntimeUser) $service
    Assert-InstallationWitness $service
    Assert-NoAccountReferences $sid -AllowRuntimeService
    # Block before mutation when the registration is inaccessible or ambiguous.
    Assert-DistroContext $sid
    $null = Get-OwnedDistro
    $null = Get-OwnedProfile $sid -AllowLoaded
    $autoruns = @(Get-OwnedAutoruns)
    $shortcuts = @(Get-OwnedShortcuts)
    $lock = $null
    try {
        if (-not (Test-Path -LiteralPath $SetupRoot)) {
            $security = [Security.AccessControl.DirectorySecurity]::new()
            $security.SetSecurityDescriptorSddlForm('O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)')
            $null = [IO.Directory]::CreateDirectory($SetupRoot, $security)
        }
        Assert-ProtectedMetadata $SetupRoot
        $lockPath = Join-Path $SetupRoot 'setup.lock'
        Assert-PlainPath $lockPath
        $script:FailureCode = 'SETUP_BUSY'
        $lock = [IO.File]::Open($lockPath, [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
        $script:FailureCode = 'UNINSTALL_FAILED'
        # Preflight can race setup before lock acquisition. Recheck the identity
        # and service under the lock before recording ownership or mutating.
        $service = Get-RuntimeService
        Assert-ServiceOwner $service
        if ((Get-AccountWitness (Get-RuntimeUser) $service) -ne $sid) { Stop-Uninstall 'ACCOUNT_SID_CONFLICT' }
        Assert-NoAccountReferences $sid -AllowRuntimeService
        Save-AccountWitness $sid
        Stop-OwnedService
        Assert-DistroContext $sid
        $removedDistro = Remove-OwnedDistro
        $removedService = Remove-OwnedService
        $removedProfile = Remove-OwnedProfile $sid
        $removedAccount = Remove-OwnedAccount $sid
        foreach ($entry in $autoruns) { Remove-ItemProperty -LiteralPath $entry.Key -Name $entry.Name -ErrorAction Stop }
        foreach ($shortcut in $shortcuts) { Assert-PlainPath $shortcut; Remove-Item -LiteralPath $shortcut -Force -ErrorAction Stop }
        $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
        if ($null -ne $machinePath) {
            $parts = @($machinePath -split ';' | Where-Object { $_.Trim().Trim('"').TrimEnd('\') -ne $ProgramRoot })
            [Environment]::SetEnvironmentVariable('Path', ($parts -join ';'), 'Machine')
        }
        Assert-ResourcesRemoved $sid
        foreach ($root in @($ProgramRoot, $DataRoot)) {
            Assert-PlainTree $root
            if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction Stop }
            if (Test-Path -LiteralPath $root) { Stop-Uninstall 'ROOT_REMAINS' }
        }
        # Retain the SID witness until all other resources have been verified.
        $lock.Dispose(); $lock = $null
        Remove-SetupRoot
        [ordered]@{ result='REMOVED'; service_removed=$removedService; distro_removed=$removedDistro;
            account_removed=$removedAccount; profile_removed=$removedProfile; shortcuts_removed=$shortcuts.Count; autoruns_removed=$autoruns.Count }
    } finally { if ($lock) { $lock.Dispose() } }
}

try {
    if ($PSCmdlet.ShouldProcess('GNX Windows installation', 'fully uninstall')) {
        Invoke-Uninstall | ConvertTo-Json -Compress | Write-Output
    }
} catch {
    # Never serialize exception text, native output, paths, registry values or credentials.
    [ordered]@{result='BLOCKED'; code=$FailureCode} | ConvertTo-Json -Compress | Write-Output
    exit 1
}
