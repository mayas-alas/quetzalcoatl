# Export the existing, digest-verified Continuity image as one durable build input.
# No deployment mutation, secrets in argv/output or temporary runner files.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$directory = Join-Path $root 'build\ai\images'
New-Item -ItemType Directory -Force $directory | Out-Null
$archive = Join-Path $directory 'continuity.oci.tar'
Add-Type -AssemblyName System.Security
$plain = [Security.Cryptography.ProtectedData]::Unprotect([IO.File]::ReadAllBytes('C:\ProgramData\GnX\Node\state\identity\password.dpapi'), $null, [Security.Cryptography.DataProtectionScope]::LocalMachine)
$secure = $null
try {
    $secure = ConvertTo-SecureString ([Text.Encoding]::UTF8.GetString($plain)) -AsPlainText -Force
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = "$env:SystemRoot\System32\wsl.exe"
    # The inspected production image. Do not package an arbitrary image or the journal.
    $start.Arguments = '--distribution gnx-node --user root --exec podman exec gnx-compute pct exec 100 -- podman save --format oci-archive localhost/continuity@sha256:1afd94a9bccb26985160f3557ce4477f246247f2d93619d3d0add11298e0cd56'
    $start.WorkingDirectory = "$env:SystemRoot"
    $start.UseShellExecute = $false
    $start.UserName = 'gnxnodesvc'
    $start.Domain = $env:COMPUTERNAME
    $start.Password = $secure
    $start.LoadUserProfile = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    $stderr = $process.StandardError.ReadToEndAsync()
    $stream = [IO.File]::Create($archive)
    try { $process.StandardOutput.BaseStream.CopyTo($stream); $stream.Flush() } finally { $stream.Dispose() }
    $process.WaitForExit()
    if ($process.ExitCode) { throw "Continuity image export failed: exit $($process.ExitCode)" }
    [IO.File]::WriteAllText((Join-Path $directory 'continuity-export.stderr.txt'), $stderr.GetAwaiter().GetResult())
    Get-FileHash -Algorithm SHA256 $archive | Select-Object Algorithm,Hash,Path
} finally {
    [Array]::Clear($plain, 0, $plain.Length)
    if ($secure) { $secure.Dispose() }
    if ($process) { $process.Dispose() }
}
