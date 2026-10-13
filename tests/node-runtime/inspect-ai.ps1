# Read-only acceptance inspector using the existing managed Windows identity.
# Fixed program, no arbitrary shell argument and no temporary runner files.
param([string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$source = Join-Path $PSScriptRoot 'inspect-ai.run'
$outputDir = if ($OutputDirectory) { [IO.Path]::GetFullPath($OutputDirectory) } else { Join-Path $root 'docs\validation\ai-services' }
$out = Join-Path $outputDir 'runtime-inspection.jsonl'
$err = Join-Path $outputDir 'runtime-inspection.stderr.txt'
$account = "$env:COMPUTERNAME\gnxnodesvc"
New-Item -ItemType Directory -Force $outputDir | Out-Null
Add-Type -AssemblyName System.Security
$blob = [IO.File]::ReadAllBytes('C:\ProgramData\GnX\Node\state\identity\password.dpapi')
$plain = [Security.Cryptography.ProtectedData]::Unprotect($blob, $null, [Security.Cryptography.DataProtectionScope]::LocalMachine)
$secure = $null
try {
    $secure = ConvertTo-SecureString ([Text.Encoding]::UTF8.GetString($plain)) -AsPlainText -Force
    # Transfer the fixed inspector through stdin. CreateProcessWithLogonW's
    # short command-line bound rules out large encoded scripts in arguments.
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = "$env:SystemRoot\System32\wsl.exe"
    $start.Arguments = '--distribution gnx-node --user root --exec /bin/bash -s'
    $start.WorkingDirectory = "$env:SystemRoot"
    $start.UseShellExecute = $false
    $start.UserName = 'gnxnodesvc'
    $start.Domain = $env:COMPUTERNAME
    $start.Password = $secure
    $start.LoadUserProfile = $true
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    $caddySource = [IO.File]::ReadAllText((Join-Path $root 'payload\node\gateway\routes.conf')).Replace("`r`n", "`n")
    $program = [IO.File]::ReadAllText($source).Replace("`r`n", "`n").Replace('__GNX_CADDY_SOURCE__', ($caddySource | ConvertTo-Json -Compress))
    # The native writer defaults to a Windows code page; stdin Python/Caddy
    # source must retain UTF-8 (including the installed configuration comments).
    $inputBytes = [Text.Encoding]::UTF8.GetBytes($program)
    $process.StandardInput.BaseStream.Write($inputBytes, 0, $inputBytes.Length)
    $process.StandardInput.Close()
    $process.WaitForExit()
    [IO.File]::WriteAllText($out, $stdout.GetAwaiter().GetResult())
    [IO.File]::WriteAllText($err, $stderr.GetAwaiter().GetResult())
    if ($process.ExitCode) { throw "Read-only inspection failed: exit $($process.ExitCode); see $err" }
} finally {
    [Array]::Clear($plain, 0, $plain.Length)
    if ($secure) { $secure.Dispose() }
    if ($process) { $process.Dispose() }
}
Get-Content -LiteralPath $out
