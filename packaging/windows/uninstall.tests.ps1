#requires -Version 5.1
# Isolated contract tests: AST-load definitions only; never execute the entrypoint.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$source = Join-Path $PSScriptRoot 'uninstall.ps1'
$tokens = $null; $parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($source, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw 'Uninstaller parse failed.' }
$definitions = @($ast.EndBlock.Statements | Where-Object { $_ -is [Management.Automation.Language.FunctionDefinitionAst] })
$assignments = @($ast.EndBlock.Statements | Where-Object { $_ -is [Management.Automation.Language.AssignmentStatementAst] })

function Assert-Equal($Actual, $Expected) {
    if ($Actual -cne $Expected) { throw "Expected [$Expected], got [$Actual]." }
}
function Assert-Blocked([scriptblock]$Body, [string]$Code) {
    $script:FailureCode = 'UNINSTALL_FAILED'
    $blocked = $false
    try { & $Body } catch { $blocked = $true }
    if (-not $blocked) { throw "Expected blocker $Code." }
    Assert-Equal $script:FailureCode $Code
}

$cases = @(
    @{Name='service exact ownership'; Body={
        Assert-ServiceOwner ([pscustomobject]@{PathName='"C:\Program Files\GNX-0.3.1\gnx-service.exe"'; StartName='.\gnx-runtime'})
        foreach ($path in @('C:\Program Files\GNX-0.3.1\gnx-service.exe.bad', '"C:\Program Files\GNX-0.3.1\gnx-service.exe" --other')) {
            Assert-Blocked { Assert-ServiceOwner ([pscustomobject]@{PathName=$path; StartName='.\gnx-runtime'}) } 'SERVICE_OWNER_CONFLICT'
        }
        Assert-Blocked { Assert-ServiceOwner ([pscustomobject]@{PathName=$ServiceExe; StartName='OTHER\gnx-runtime'}) } 'SERVICE_OWNER_CONFLICT'
    }},
    @{Name='service stop revalidation'; Body={
        function Get-RuntimeService { [pscustomobject]@{PathName=$ServiceExe; StartName='.\gnx-runtime'; State='Stopped'; ProcessId=99} }
        Assert-Blocked { Stop-OwnedService } 'SERVICE_NOT_STOPPED'
    }},
    @{Name='service delete pending and retry'; Body={
        $script:present = $true
        function Get-RuntimeService { if ($script:present) { [pscustomobject]@{PathName=$ServiceExe; StartName='.\gnx-runtime'; State='Stopped'; ProcessId=0} } }
        function Invoke-Native { }
        function Start-Sleep { }
        Assert-Blocked { Remove-OwnedService } 'SERVICE_DELETE_PENDING'
        $script:present = $false
        Assert-Equal (Remove-OwnedService) $false
    }},
    @{Name='service deletion verifies disappearance'; Body={
        $script:present = $true
        function Get-RuntimeService { if ($script:present) { [pscustomobject]@{PathName=$ServiceExe; StartName='.\gnx-runtime'; State='Stopped'; ProcessId=0} } }
        function Invoke-Native { param($File,$Arguments) Assert-Equal ($Arguments -join ' ') 'delete GNXRuntime'; $script:present = $false }
        Assert-Equal (Remove-OwnedService) $true
    }},
    @{Name='account requires witness not description'; Body={
        function Read-Witness { return $null }
        $user = [pscustomobject]@{SID=[pscustomobject]@{Value='S-1-5-21-1-2-3-1001'}; Description='GNX runtime identity'}
        Assert-Blocked { Get-AccountWitness $user $null } 'ACCOUNT_WITNESS_REQUIRED'
        $service = [pscustomobject]@{PathName=$ServiceExe; StartName='.\gnx-runtime'}
        $user.Description = ''
        Assert-Equal (Get-AccountWitness $user $service) $user.SID.Value
    }},
    @{Name='journal phases and SID retry'; Body={
        $script:phase = 'PREFLIGHT'
        function Read-Witness { param($Name) if ($Name -eq 'journal.json') { [pscustomobject]@{schema=1;operation='PROVISION';phase=$script:phase} } }
        $user = [pscustomobject]@{SID=[pscustomobject]@{Value='S-1-5-21-1-2-3-1001'}}
        Assert-Blocked { Get-AccountWitness $user $null } 'ACCOUNT_WITNESS_REQUIRED'
        foreach ($phase in @('REGISTERING','SECURING','PROVISIONED')) { $script:phase=$phase; Assert-Equal (Get-AccountWitness $user $null) $user.SID.Value }
        function Read-Witness { [pscustomobject]@{schema=1;account='gnx-runtime';sid='S-1-5-21-1-2-3-1002'} }
        Assert-Blocked { Get-AccountWitness $user $null } 'ACCOUNT_SID_CONFLICT'
        Assert-Equal (Get-AccountWitness $null $null) 'S-1-5-21-1-2-3-1002'
    }},
    @{Name='account references block deletion'; Body={
        function Get-CimInstance { [pscustomobject]@{Name='Foreign';StartName='.\gnx-runtime'} }
        function Get-ScheduledTask { }
        Assert-Blocked { Assert-NoAccountReferences 'S-1-5-21-1-2-3-1001' -AllowRuntimeService } 'ACCOUNT_REFERENCED'
        function Get-CimInstance { }
        function Get-ScheduledTask { [pscustomobject]@{Principal=[pscustomobject]@{UserId='S-1-5-21-1-2-3-1001'}} }
        Assert-Blocked { Assert-NoAccountReferences 'S-1-5-21-1-2-3-1001' } 'ACCOUNT_REFERENCED'
    }},
    @{Name='account enumeration failure is not absence'; Body={
        function Get-LocalUser { throw 'provider failed' }
        Assert-Blocked { Get-RuntimeUser } 'UNINSTALL_FAILED'
    }},
    @{Name='account removal checks SID and absence'; Body={
        $script:present=$true
        function Get-RuntimeUser { if ($script:present) { [pscustomobject]@{SID=[pscustomobject]@{Value='S-1-5-21-1-2-3-1001'}} } }
        function Assert-NoAccountReferences { }
        function Remove-LocalUser { param($SID,$ErrorAction) }
        Assert-Blocked { Remove-OwnedAccount 'S-1-5-21-1-2-3-1002' } 'ACCOUNT_SID_CONFLICT'
        Assert-Blocked { Remove-OwnedAccount 'S-1-5-21-1-2-3-1001' } 'ACCOUNT_REMAINS'
        function Remove-LocalUser { param($SID,$ErrorAction) $script:present=$false }
        Assert-Equal (Remove-OwnedAccount 'S-1-5-21-1-2-3-1001') $true
        Assert-Equal (Remove-OwnedAccount 'S-1-5-21-1-2-3-1001') $false
    }},
    @{Name='distro terminate precedes unregister and retry'; Body={
        $script:running=$true; $script:present=$true; $script:calls=@()
        function Get-OwnedDistro { if ($script:present) { 'GNX-0.3.1' } }
        function Get-Distros { param([switch]$Running) if (($Running -and $script:running) -or (-not $Running -and $script:present)) { 'GNX-0.3.1' } }
        function Invoke-Native { param($File,$Arguments) $script:calls += $Arguments[0]; if ($Arguments[0] -eq '--terminate') { $script:running=$false } else { $script:present=$false } }
        Assert-Equal (Remove-OwnedDistro) $true
        Assert-Equal ($script:calls -join ',') '--terminate,--unregister'
        Assert-Equal (Remove-OwnedDistro) $false
    }},
    @{Name='stopped distro skips terminate'; Body={
        $script:present=$true; $script:calls=@()
        function Get-OwnedDistro { 'GNX-0.3.1' }
        function Get-Distros { param([switch]$Running) if (-not $Running -and $script:present) { 'GNX-0.3.1' } }
        function Invoke-Native { param($File,$Arguments) $script:calls += $Arguments[0]; $script:present=$false }
        Assert-Equal (Remove-OwnedDistro) $true
        Assert-Equal ($script:calls -join ',') '--unregister'
    }},
    @{Name='failed terminate never unregisters'; Body={
        $script:calls=@()
        function Get-OwnedDistro { 'GNX-0.3.1' }
        function Get-Distros { 'GNX-0.3.1' }
        function Invoke-Native { param($File,$Arguments) $script:calls += $Arguments[0] }
        Assert-Blocked { Remove-OwnedDistro } 'DISTRO_STILL_RUNNING'
        Assert-Equal ($script:calls -join ',') '--terminate'
    }},
    @{Name='unregister residual blocks success'; Body={
        function Get-OwnedDistro { 'GNX-0.3.1' }
        function Get-Distros { param([switch]$Running) if (-not $Running) { 'GNX-0.3.1' } }
        function Invoke-Native { }
        Assert-Blocked { Remove-OwnedDistro } 'DISTRO_REMAINS'
    }},
    @{Name='distro enumeration failure blocks'; Body={
        function Test-Path { $true }
        function Invoke-Native { Stop-Uninstall 'NATIVE_OPERATION_FAILED' }
        Assert-Blocked { Get-Distros } 'NATIVE_OPERATION_FAILED'
    }},
    @{Name='distro encoding normalized'; Body={
        function Test-Path { $true }
        function Invoke-Native { ([string][char]0xFEFF + "G`0N`0X`0-`00`0.`03`0.`01`0") }
        Assert-Equal ((Get-Distros) -join ',') 'GNX-0.3.1'
    }},
    @{Name='distro exact storage witness'; Body={
        function Get-Distros { 'GNX-0.3.1' }
        function Get-ChildItem { [pscustomobject]@{PSPath='test'} }
        $script:base='C:\Foreign'
        function Get-ItemProperty { [pscustomobject]@{DistributionName='GNX-0.3.1'; BasePath=$script:base} }
        function Assert-PlainPath { }
        Assert-Blocked { Get-OwnedDistro } 'DISTRO_OWNER_CONFLICT'
        $script:base='\\?\C:\ProgramData\GNX-0.3.1\wsl'
        Assert-Equal (Get-OwnedDistro) 'GNX-0.3.1'
    }},
    @{Name='other user distro requires its context'; Body={
        function Get-CimInstance { }
        function Test-Path { $true }
        function Get-ChildItem { [pscustomobject]@{PSPath='test'} }
        function Get-ItemProperty { [pscustomobject]@{DistributionName='GNX-0.3.1'} }
        Assert-Blocked { Assert-DistroContext 'S-1-5-21-1-2-3-1001' } 'WSL_RUNTIME_CONTEXT_REQUIRED'
        function Get-ItemProperty { [pscustomobject]@{DistributionName='Foreign'} }
        Assert-Blocked { Assert-DistroContext 'S-1-5-21-1-2-3-1001' } 'PROFILE_FOREIGN_DISTRO'
    }},
    @{Name='offline hive always unloaded on blocker'; Body={
        $script:calls=@()
        function Get-CimInstance { [pscustomobject]@{SID='S-1-5-21-1-2-3-1001'} }
        function Get-OwnedProfile { }
        function Test-Path { param($LiteralPath,$PathType) $LiteralPath -notmatch '^Registry::HKEY_USERS\\S-' }
        function Invoke-Native { param($File,$Arguments) $script:calls += $Arguments[0] }
        function Get-ChildItem { [pscustomobject]@{PSPath='test'} }
        function Get-ItemProperty { [pscustomobject]@{DistributionName='GNX-0.3.1'} }
        Assert-Blocked { Assert-DistroContext 'S-1-5-21-1-2-3-1001' } 'WSL_RUNTIME_CONTEXT_REQUIRED'
        Assert-Equal ($script:calls -join ',') 'load,unload'
    }},
    @{Name='profile SID exact path loaded and special guards'; Body={
        $script:profile=[pscustomobject]@{SID='S-1-5-21-1-2-3-1001';LocalPath='C:\Users\gnx-runtime';Loaded=$false;Special=$false}
        function Get-CimInstance { $script:profile }
        function Assert-PlainTree { }
        Assert-Equal (Get-OwnedProfile 'S-1-5-21-1-2-3-1001').LocalPath $ProfileRoot
        $script:profile.Loaded=$true
        Assert-Blocked { Get-OwnedProfile 'S-1-5-21-1-2-3-1001' } 'PROFILE_LOADED'
        $script:profile.Loaded=$false; $script:profile.Special=$true
        Assert-Blocked { Get-OwnedProfile 'S-1-5-21-1-2-3-1001' } 'PROFILE_OWNER_CONFLICT'
        $script:profile.Special=$false; $script:profile.LocalPath='C:\Users\gnx-runtime.OTHER'
        Assert-Blocked { Get-OwnedProfile 'S-1-5-21-1-2-3-1001' } 'PROFILE_OWNER_CONFLICT'
    }},
    @{Name='unregistered profile directory remains blocked'; Body={
        function Get-CimInstance { }
        function Test-Path { $true }
        Assert-Blocked { Get-OwnedProfile '' } 'PROFILE_OWNER_CONFLICT'
    }},
    @{Name='profile removal validates absence'; Body={
        $script:profile=[pscustomobject]@{SID='test'}
        function Get-OwnedProfile { $script:profile }
        function Remove-CimInstance { param([Parameter(ValueFromPipeline)]$InputObject) process { } }
        Assert-Blocked { Remove-OwnedProfile 'test' } 'PROFILE_REMAINS'
        function Remove-CimInstance { param([Parameter(ValueFromPipeline)]$InputObject) process { $script:profile=$null } }
        Assert-Equal (Remove-OwnedProfile 'test') $true
        Assert-Equal (Remove-OwnedProfile 'test') $false
    }},
    @{Name='autorun command exact ownership'; Body={
        Assert-Equal (Test-OwnedCommand '"C:\Program Files\GNX-0.3.1\gnx.exe" status') $true
        Assert-Equal (Test-OwnedCommand 'C:\Program Files\GNX-0.3.1\gnx.exe') $true
        foreach ($value in @('C:\Program Files\GNX-0.3.1\gnx.exe status','"C:\Program Files\GNX-0.3.1\gnx.exe.bad"','cmd.exe /c "C:\Program Files\GNX-0.3.1\gnx.exe"','"C:\Program Files\GNX-0.3.10\gnx.exe"')) {
            Assert-Equal (Test-OwnedCommand $value) $false
        }
    }},
    @{Name='exact roots and reparse guards'; Body={
        Assert-Blocked { Assert-AllowedRoot 'C:\Program Files\GNX-0.3.1-other' } 'ROOT_CONFLICT'
        function Test-Path { $true }
        function Get-Item { [pscustomobject]@{Attributes=[IO.FileAttributes]::ReparsePoint} }
        Assert-Blocked { Assert-PlainTree $ProgramRoot } 'REPARSE_POINT'
    }},
    @{Name='foreign roots require install witness'; Body={
        function Read-Witness { }
        function Test-Path { param($LiteralPath) $LiteralPath -eq $ProgramRoot }
        Assert-Blocked { Assert-InstallationWitness $null } 'INSTALLATION_WITNESS_REQUIRED'
    }},
    @{Name='setup witness removed last'; Body={
        $script:removed=@()
        function Assert-PlainTree { }
        function Get-ChildItem { foreach ($name in @('journal.json','other','uninstall-owner.json')) { [pscustomobject]@{Name=$name;FullName=(Join-Path $SetupRoot $name)} } }
        function Test-Path { param($LiteralPath) $LiteralPath -ne $SetupRoot }
        function Remove-Item { param($LiteralPath,[switch]$Recurse,[switch]$Force,$ErrorAction) $script:removed += (Split-Path $LiteralPath -Leaf) }
        Remove-SetupRoot
        Assert-Equal ($script:removed -join ',') 'other,uninstall-owner.json,journal.json,GNX-Setup-0.3.1'
    }},
    @{Name='witness ACL rejects untrusted writers'; Body={
        function Assert-PlainPath { }
        $script:security=[Security.AccessControl.DirectorySecurity]::new()
        $script:security.SetSecurityDescriptorSddlForm('O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)')
        function Get-Acl { $script:security }
        Assert-ProtectedMetadata 'test'
        $script:security.SetSecurityDescriptorSddlForm('O:BAG:BAD:P(A;OICI;FA;;;WD)')
        Assert-Blocked { Assert-ProtectedMetadata 'test' } 'WITNESS_ACL_CONFLICT'
    }},
    @{Name='child reparse prevents tree removal'; Body={
        function Assert-PlainPath { }
        function Test-Path { $true }
        function Get-Item { param($LiteralPath,[switch]$Force)
            if ($LiteralPath -eq $ProgramRoot) { [pscustomobject]@{Attributes=[IO.FileAttributes]::Directory; PSIsContainer=$true} }
            else { [pscustomobject]@{Attributes=[IO.FileAttributes]::ReparsePoint;PSIsContainer=$true} }
        }
        function Get-ChildItem { [pscustomobject]@{FullName=(Join-Path $ProgramRoot 'redirect')} }
        Assert-Blocked { Assert-PlainTree $ProgramRoot } 'REPARSE_POINT'
    }},
    @{Name='autorun values require exact executable ownership'; Body={
        function Test-Path { param($LiteralPath) $LiteralPath -eq 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run' }
        $script:key=[pscustomobject]@{}
        $script:key | Add-Member ScriptMethod GetValueNames { @('GNX','Foreign') }
        $script:key | Add-Member ScriptMethod GetValue { param($Name,$Default,$Options) if ($Name -eq 'GNX') { '"C:\Program Files\GNX-0.3.1\gnx.exe"' } else { 'C:\Foreign\app.exe' } }
        function Get-Item { $script:key }
        $owned=@(Get-OwnedAutoruns)
        Assert-Equal $owned.Count 1
        Assert-Equal $owned[0].Name 'GNX'
        $script:key | Add-Member ScriptMethod GetValue { 'C:\Foreign\app.exe' } -Force
        Assert-Blocked { Get-OwnedAutoruns } 'AUTORUN_OWNER_CONFLICT'
    }},
    @{Name='final gate refuses every registration residual'; Body={
        function Get-RuntimeService { if ($script:residual -eq 'service') { $true } }
        function Get-RuntimeUser { if ($script:residual -eq 'account') { $true } }
        function Get-OwnedProfile { if ($script:residual -eq 'profile') { $true } }
        function Get-Distros { if ($script:residual -eq 'distro') { 'GNX-0.3.1' } }
        function Get-OwnedAutoruns { if ($script:residual -eq 'autorun') { $true } }
        function Get-OwnedShortcuts { if ($script:residual -eq 'shortcut') { $true } }
        foreach ($residual in @('service','account','profile','distro','autorun','shortcut')) {
            $script:residual=$residual
            Assert-Blocked { Assert-ResourcesRemoved 'test' } 'RESOURCES_REMAIN'
        }
    }}
)

foreach ($case in $cases) {
    & {
        foreach ($statement in $assignments) { . ([scriptblock]::Create($statement.Extent.Text)) }
        foreach ($definition in $definitions) { . ([scriptblock]::Create($definition.Extent.Text)) }
        & $case.Body
    }
    Write-Output "PASS $($case.Name)"
}

# Execute the actual entrypoint with a replacement workflow in a child process.
# This checks sanitized JSON and exit status without touching host resources.
$entrypoint = @($ast.EndBlock.Statements | Where-Object { $_ -is [Management.Automation.Language.TryStatementAst] })[-1].Extent.Text
$harness = Join-Path ([IO.Path]::GetTempPath()) ('gnx-uninstall-test-' + [Guid]::NewGuid().ToString('N') + '.ps1')
try {
    $prefix = "[CmdletBinding(SupportsShouldProcess)]param()`r`n" + '$FailureCode = ''UNINSTALL_FAILED''' + "`r`n"
    $failureBody = 'function Invoke-Uninstall { throw ''SANITIZATION_TEST_SENTINEL'' }' + "`r`n"
    [IO.File]::WriteAllText($harness, ($prefix + $failureBody + $entrypoint))
    $output = @(& "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File $harness 2>&1)
    Assert-Equal $LASTEXITCODE 1
    Assert-Equal $output.Count 1
    Assert-Equal ([string]$output[0]) '{"result":"BLOCKED","code":"UNINSTALL_FAILED"}'
    Write-Output 'PASS entrypoint sanitized failure JSON and nonzero exit'
    $output = @(& "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File $harness -WhatIf 2>&1)
    Assert-Equal $LASTEXITCODE 0
    if (($output -join '') -match 'BLOCKED|REMOVED|SANITIZATION_TEST_SENTINEL') { throw 'WhatIf executed workflow.' }
    Write-Output 'PASS WhatIf does not execute workflow'
} finally {
    if (Test-Path -LiteralPath $harness) { Remove-Item -LiteralPath $harness -Force }
}
Write-Output "Windows uninstall isolated checks passed: $($cases.Count + 2). Host lifecycle acceptance remains separate."
