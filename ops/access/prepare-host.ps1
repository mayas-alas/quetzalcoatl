[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$state = Join-Path $env:LOCALAPPDATA 'GNX\access'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$mutex = [Threading.Mutex]::new($false, 'Local\GNX-Access-Apply')
$held = $false
$gate = 'LOCK'
try {
    $held = $mutex.WaitOne(0)
    if (-not $held) { throw 'An access operation is already running.' }
    $gate = 'DIRECTORY'
    New-Item -ItemType Directory -Path $state -Force | Out-Null
    $acl = [Security.AccessControl.DirectorySecurity]::new()
    $acl.SetAccessRuleProtection($true, $false)
    $sids = @('S-1-5-18', 'S-1-5-32-544', [Security.Principal.WindowsIdentity]::GetCurrent().User.Value)
    foreach ($sid in $sids) {
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
            [Security.Principal.SecurityIdentifier]::new($sid), 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
    }
    $directory = [IO.DirectoryInfo]::new($state)
    if ($PSVersionTable.PSVersion.Major -ge 6) {
        [IO.FileSystemAclExtensions]::SetAccessControl($directory, $acl)
    } else { $directory.SetAccessControl($acl) }
    $gate = 'KEY_ACL'
    $key = Join-Path $state 'enrollment.key'
    $arguments = @('apply', '--config', (Join-Path $repo 'runtime\access\access.toml'))
    if (Test-Path -LiteralPath $key) {
        $fileAcl = [Security.AccessControl.FileSecurity]::new()
        $fileAcl.SetAccessRuleProtection($true, $false)
        foreach ($sid in $sids) {
            $fileAcl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
                [Security.Principal.SecurityIdentifier]::new($sid), 'FullControl', 'Allow'))
        }
        $file = [IO.FileInfo]::new($key)
        if ($PSVersionTable.PSVersion.Major -ge 6) {
            [IO.FileSystemAclExtensions]::SetAccessControl($file, $fileAcl)
        } else { $file.SetAccessControl($fileAcl) }
        $arguments += @('--key-file', $key)
    }
    $gate = 'APPLY'
    $ErrorActionPreference = 'Continue'
    & (Join-Path $PSScriptRoot 'target\release\gnx-access.exe') @arguments
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} catch {
    "FAILED ACCESS_HOST_$gate"
    exit 1
} finally {
    if ($held) { $mutex.ReleaseMutex() }
    $mutex.Dispose()
}
