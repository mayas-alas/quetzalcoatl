[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('check', 'provision')][string]$Mode,
    [Parameter(Mandatory)][string]$SetupExe,
    [string]$Bundle, [string]$ManifestSha256, [string]$Rootfs, [string]$RootfsSha256
)

# Minimal native-host presentation boundary. It consumes only the allowlisted
# NDJSON projection from gnx-setup and never duplicates setup policy.
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path -LiteralPath $SetupExe -ErrorAction Stop).Path
$arguments = [Collections.Generic.List[string]]::new()
[void]$arguments.Add('--json-progress'); [void]$arguments.Add("--$Mode")
if ($Mode -eq 'provision') {
    foreach ($value in @($Bundle, $ManifestSha256, $Rootfs, $RootfsSha256)) { if ([string]::IsNullOrWhiteSpace($value)) { throw 'Provision requires trust inputs' } }
    foreach ($pair in @(@('--bundle', $Bundle), @('--manifest-sha256', $ManifestSha256), @('--rootfs', $Rootfs), @('--rootfs-sha256', $RootfsSha256))) { [void]$arguments.Add($pair[0]); [void]$arguments.Add($pair[1]) }
}
$psi = [Diagnostics.ProcessStartInfo]::new()
$psi.FileName = $exe; $psi.UseShellExecute = $false; $psi.RedirectStandardOutput = $true; $psi.RedirectStandardError = $true
function Add-ProcessArgument([Diagnostics.ProcessStartInfo]$Info, [string]$Value) {
    if ($null -ne $Info.PSObject.Properties['ArgumentList']) { [void]$Info.ArgumentList.Add($Value); return }
    $escaped = $Value -replace '(\\*)"', '$1$1\\"'
    $escaped = $escaped -replace '(\\+)$', '$1$1'
    $Info.Arguments += '"' + $escaped + '" '
}
foreach ($arg in $arguments) { Add-ProcessArgument $psi $arg }
$process = [Diagnostics.Process]::new(); $process.StartInfo = $psi
if (!$process.Start()) { throw 'Unable to start gnx-setup' }
$events = @()
while (!$process.StandardOutput.EndOfStream) {
    $line = $process.StandardOutput.ReadLine()
    try { $event = $line | ConvertFrom-Json -ErrorAction Stop } catch { throw 'Malformed setup progress event' }
    if ($event.schema -ne 1 -or $event.operation -notin @('setup-check', 'setup-provision') -or $event.phase -notin @('started', 'completed')) { throw 'Unsupported setup progress event' }
    if ($event.phase -eq 'started' -and ($null -ne $event.state -or $null -ne $event.exit_code)) { throw 'Invalid started event' }
    if ($event.phase -eq 'completed' -and $event.state -notin @('READY', 'FAILED', 'ACTION_REQUIRED')) { throw 'Invalid terminal state' }
    if ($event.apply_available -ne $false) { throw 'Apply must remain unavailable' }
    $events += $event
    $label = if ($event.phase -eq 'started') { 'RUNNING' } elseif ($event.state -eq 'READY') { 'READY' } elseif ($event.state -eq 'ACTION_REQUIRED') { 'ACTION_REQUIRED' } else { 'FAILED' }
    Write-Output (([ordered]@{ schema = 1; phase = $event.phase; state = $label } | ConvertTo-Json -Compress))
}
$stderr = $process.StandardError.ReadToEnd(); $process.WaitForExit()
if ($stderr.Length -gt 0 -or $events.Count -ne 2 -or $events[-1].phase -ne 'completed') { throw 'Setup transport failed' }
$expectedExit = @{ READY = 0; FAILED = 1; ACTION_REQUIRED = 2 }[$events[-1].state]
if ($process.ExitCode -ne $expectedExit) { throw 'Setup exit state mismatch' }
exit $process.ExitCode
