[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [string]$DistPath,

    [string]$Distribution = 'Ubuntu-24.04'
)

$ErrorActionPreference = 'Stop'

function Run-Native {
    param([string]$Command, [string]$Arguments)
    $pinfo = New-Object System.Diagnostics.ProcessStartInfo
    $pinfo.FileName = $Command
    $pinfo.Arguments = $Arguments
    $pinfo.RedirectStandardOutput = $true
    $pinfo.RedirectStandardError = $true
    $pinfo.UseShellExecute = $false
    $pinfo.CreateNoWindow = $true
    $proc = [System.Diagnostics.Process]::Start($pinfo)
    $stdout = $proc.StandardOutput.ReadToEnd()
    $stderr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    return @{ ExitCode = $proc.ExitCode; Stdout = $stdout; Stderr = $stderr }
}

function Assert-Contract {
    param(
        [string]$Label,
        [int]$ExpectedExitCode,
        [string]$ExpectedStderrPattern,
        [string]$Command,
        [string]$Arguments
    )
    $result = Run-Native $Command $Arguments
    $exitCode = $result.ExitCode
    $joined = $result.Stderr + $result.Stdout
    if ($exitCode -ne $ExpectedExitCode -or $joined -notmatch $ExpectedStderrPattern) {
        throw "FAILED $Label"
    }
}

# 1. Windows binary contract
$exe = Join-Path $DistPath 'gnx.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw 'FAILED DIST_MISSING_EXE' }
Assert-Contract 'WINDOWS_CONTRACT' 2 'FAILED CONFIG_READ' $exe '--config missing.gnx.toml access dns'

# 2. Linux binary contract (via WSL)
$distWsl = (& wsl.exe -d $Distribution --exec wslpath -u $DistPath).Trim()
if ($LASTEXITCODE -ne 0 -or -not $distWsl.StartsWith('/')) { throw 'FAILED WSL_PATH' }
$linuxBin = "$distWsl/gnx"
Assert-Contract 'LINUX_CONTRACT' 2 'FAILED CONFIG_READ' 'wsl.exe' "-d $Distribution --exec $linuxBin --config /missing.gnx.toml access dns"

# 3. Argument contract (secrets have no CLI position)
Assert-Contract 'ARGUMENTS_CONTRACT' 2 'FAILED ARGUMENTS' $exe 'access configure GNX-NONSECRET-INPUT-MARKER'

Write-Output 'READY VALIDATION'