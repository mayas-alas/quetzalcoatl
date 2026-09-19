[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ManifestSha256,
    [Parameter(Mandatory)][string]$Rootfs,
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$RootfsSha256,
    [string]$Bundle = (Join-Path $PSScriptRoot '../../dist')
)

# Thin host wrapper: gnx-setup owns provisioning, ACL, account and service policy.
# No credential is accepted or placed on a command line.
$ErrorActionPreference = 'Stop'
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (!$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Elevation required' }
$bundlePath = (Resolve-Path -LiteralPath $Bundle -ErrorAction Stop).Path
$rootfsPath = (Resolve-Path -LiteralPath $Rootfs -ErrorAction Stop).Path
$manifestPath = Join-Path $bundlePath 'manifest.json'
$manifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($manifestHash -ne $ManifestSha256.ToLowerInvariant()) { throw 'Manifest authentication failed' }
$manifest = ([Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($manifestPath)) | ConvertFrom-Json)
if ($manifest.schema -ne 1 -or $manifest.version -ne '0.3.1') { throw 'Manifest schema or release version is unsupported' }
$artifacts = @('gnx.exe', 'gnx-service.exe', 'gnx-setup.exe', 'gnx-linux', 'gnx-linux-bundle.tar', 'gnx-linux.run')
foreach ($name in $artifacts) {
    $expected = [string]$manifest.artifacts.$name
    if ($expected -notmatch '^[a-fA-F0-9]{64}$') { throw "Artifact hash is invalid: $name" }
    $actual = (Get-FileHash -LiteralPath (Join-Path $bundlePath $name) -Algorithm SHA256).Hash
    if ($actual -ne $expected) { throw "Artifact verification failed: $name" }
}
$rootfsActual = (Get-FileHash -LiteralPath $rootfsPath -Algorithm SHA256).Hash
if ($rootfsActual -ne $RootfsSha256.ToLowerInvariant()) { throw 'Rootfs verification failed' }

# Refuse known legacy/partial layouts; recovery is explicit and never guessed.
$conflicts = @(
    'C:\Program Files\QuetzalcoatlNext', 'C:\ProgramData\QuetzalcoatlNext',
    'C:\Program Files\GNX', 'C:\ProgramData\GNX',
    'C:\Program Files\GNX-0.3.1', 'C:\ProgramData\GNX-0.3.1',
    'C:\ProgramData\GNX-Setup-0.3.1'
)
foreach ($path in $conflicts) {
    if (Test-Path -LiteralPath $path) {
        throw "Setup conflict at $path; GNX never adopts or overwrites existing roots. Recover explicitly before retrying."
    }
}

$setup = Join-Path $bundlePath 'gnx-setup.exe'
$psi = [Diagnostics.ProcessStartInfo]::new()
$psi.FileName = (Resolve-Path -LiteralPath $setup -ErrorAction Stop).Path
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
function Add-ProcessArgument([Diagnostics.ProcessStartInfo]$Info, [string]$Value) {
    if ($null -ne $Info.PSObject.Properties['ArgumentList']) { [void]$Info.ArgumentList.Add($Value); return }
    $escaped = $Value -replace '(\\*)"', '$1$1\\"'
    $escaped = $escaped -replace '(\\+)$', '$1$1'
    $Info.Arguments += '"' + $escaped + '" '
}
foreach ($arg in @('--json-progress', '--provision', '--bundle', $bundlePath, '--manifest-sha256', $ManifestSha256.ToLowerInvariant(), '--rootfs', $rootfsPath, '--rootfs-sha256', $RootfsSha256.ToLowerInvariant())) { Add-ProcessArgument $psi $arg }
$process = [Diagnostics.Process]::new()
$process.StartInfo = $psi
if (!$process.Start()) { throw 'Unable to start gnx-setup' }
$stdout = $process.StandardOutput.ReadToEnd()
$stderr = $process.StandardError.ReadToEnd()
$process.WaitForExit()
if ($stderr.Length -gt 0) { throw 'gnx-setup produced unexpected diagnostic output' }
Write-Output $stdout.TrimEnd()
exit $process.ExitCode
