param([string]$MsiArtifact = (Join-Path $PSScriptRoot '..\..\build\installer\GnxNode.msi'))
# Read/evaluate the compiled MSI only. Never call Install, DoAction or cleanup.
$ErrorActionPreference='Stop'
$installer=New-Object -ComObject WindowsInstaller.Installer
$db=$installer.OpenDatabase((Resolve-Path $MsiArtifact).Path,0)
$conditions=@{}
foreach($action in @('CleanupFreshResidue','CleanupOnUninstall')) {
 $view=$db.OpenView("SELECT ``Condition`` FROM ``InstallExecuteSequence`` WHERE ``Action`` = '$action'")
 $view.Execute(); $row=$view.Fetch()
 if(!$row){throw "Missing compiled action: $action"}
 $conditions[$action]=$row.StringData(1); $view.Close()
 $view=$db.OpenView("SELECT ``Type``, ``Target`` FROM ``CustomAction`` WHERE ``Action`` = '$action'")
 $view.Execute(); $row=$view.Fetch(); $type=$row.IntegerData(1)
 if(($type -band 1024) -eq 0 -or ($type -band 2048) -eq 0 -or ($type -band 64) -ne 0 -or $row.StringData(2) -ne '--cleanup'){throw "Unsafe compiled cleanup: $action"}
 $view.Close()
}
# OpenPackage option 1 ignores the installed product machine state. These are
# isolated property scenarios; no installer transaction or custom action runs.
$session=$installer.OpenPackage((Resolve-Path $MsiArtifact).Path,1)
function Set-Property([string]$Name,[string]$Value) {
 [void]$session.GetType().InvokeMember('Property',[Reflection.BindingFlags]::SetProperty,$null,$session,@($Name,$Value))
}
$cases=@(
 @{name='fresh_cleanup_enabled'; action='CleanupFreshResidue'; installed=''; upgrade=''; upgrading=''; remove=''; expected=1},
 @{name='repair_cleanup_suppressed'; action='CleanupFreshResidue'; installed='1'; upgrade=''; upgrading=''; remove=''; expected=0},
 @{name='new_upgrade_cleanup_suppressed'; action='CleanupFreshResidue'; installed=''; upgrade='{00000000-0000-0000-0000-000000000001}'; upgrading=''; remove=''; expected=0},
 @{name='complete_uninstall_cleanup_enabled'; action='CleanupOnUninstall'; installed='1'; upgrade=''; upgrading=''; remove='ALL'; expected=1},
 @{name='old_upgrade_removal_cleanup_suppressed'; action='CleanupOnUninstall'; installed='1'; upgrade=''; upgrading='{00000000-0000-0000-0000-000000000001}'; remove='ALL'; expected=0},
 @{name='partial_removal_cleanup_suppressed'; action='CleanupOnUninstall'; installed='1'; upgrade=''; upgrading=''; remove='MainFeature'; expected=0}
)
foreach($case in $cases) {
 Set-Property 'Installed' $case.installed
 Set-Property 'WIX_UPGRADE_DETECTED' $case.upgrade
 Set-Property 'UPGRADINGPRODUCTCODE' $case.upgrading
 Set-Property 'REMOVE' $case.remove
 if($session.EvaluateCondition($conditions[$case.action]) -ne $case.expected){throw "Compiled condition failed: $($case.name)"}
 [pscustomobject]@{test=$case.name;result='pass';scope='compiled-MSI-condition-only-no-uninstall'} | ConvertTo-Json -Compress
}
