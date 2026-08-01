[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $SetupPath,
    [Parameter(Mandatory)][string] $ExpectedVersion,
    [Parameter(Mandatory)][string] $ExpectedController,
    [Parameter(Mandatory)][string] $EvidencePath,
    [int] $OperationTimeoutSeconds = 900,
    [int] $ReadyTimeoutSeconds = 300
)

$ErrorActionPreference = 'Stop'
$setup = (Resolve-Path -LiteralPath $SetupPath).Path
$evidence = [IO.Path]::GetFullPath($EvidencePath)
$evidenceDirectory = Split-Path -Parent $evidence
New-Item -ItemType Directory -Force -Path $evidenceDirectory | Out-Null
Set-Content -LiteralPath $evidence -Value '' -Encoding utf8

function Write-Evidence {
    param(
        [Parameter(Mandatory)][string] $Stage,
        [Parameter(Mandatory)][string] $Status,
        [Parameter(Mandatory)] $Detail
    )

    [pscustomobject]@{
        timestamp_utc = [DateTime]::UtcNow.ToString('o')
        stage = $Stage
        status = $Status
        detail = $Detail
    } | ConvertTo-Json -Compress -Depth 8 |
        Add-Content -LiteralPath $evidence -Encoding utf8
}

function Assert-Administrator {
    $principal = [Security.Principal.WindowsPrincipal]::new(
        [Security.Principal.WindowsIdentity]::GetCurrent()
    )
    if (-not $principal.IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator
    )) {
        throw 'QA lifecycle requires an elevated administrator process.'
    }
}

function Get-QuetzalcoatlRegistrations {
    $registrations = @()
    foreach ($root in @(
        'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )) {
        foreach ($key in Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue) {
            $properties = Get-ItemProperty -LiteralPath $key.PSPath -ErrorAction SilentlyContinue
            if ($properties.DisplayName -eq 'Quetzalcoatl') {
                $registrations += [pscustomobject]@{
                    key = $key.PSChildName
                    version = $properties.DisplayVersion
                    publisher = $properties.Publisher
                    hidden = $properties.SystemComponent -eq 1
                }
            }
        }
    }
    $registrations
}

function Assert-InstalledSurface {
    $registrations = @(Get-QuetzalcoatlRegistrations)
    $visible = @($registrations | Where-Object { -not $_.hidden })
    $hidden = @($registrations | Where-Object hidden)
    if ($visible.Count -ne 1 -or $hidden.Count -ne 1) {
        throw "Expected one visible Setup and one hidden MSI registration; found visible=$($visible.Count), hidden=$($hidden.Count)."
    }
    if ($visible[0].version -ne $ExpectedVersion -or
        $visible[0].publisher -ne 'GNX Labs' -or
        $hidden[0].version -ne $ExpectedVersion -or
        $hidden[0].publisher -ne 'GNX Labs') {
        throw "Installed registration metadata differs: $($registrations | ConvertTo-Json -Compress)."
    }
    $registrations
}

function Wait-Ready {
    param([Parameter(Mandatory)][string] $Stage)

    $cli = 'C:\Program Files\Quetzalcoatl\gnx.exe'
    $deadline = [DateTime]::UtcNow.AddSeconds($ReadyTimeoutSeconds)
    $lastStatus = $null
    while ([DateTime]::UtcNow -lt $deadline) {
        if (Test-Path -LiteralPath $cli) {
            $statusText = & $cli status --json 2>$null
            if ($LASTEXITCODE -eq 0) {
                try {
                    $lastStatus = $statusText | ConvertFrom-Json
                    if ($lastStatus.overall -eq 'ready' -and
                        $lastStatus.stage -eq 'READY' -and
                        $lastStatus.controller -eq $ExpectedController -and
                        $lastStatus.cluster.joined -eq $true -and
                        $lastStatus.cluster.quorate -eq $true) {
                        Write-Evidence -Stage $Stage -Status 'ready' -Detail $lastStatus
                        return $lastStatus
                    }
                } catch {
                    $lastStatus = [pscustomobject]@{ parse_error = $_.Exception.Message }
                }
            }
        }
        Start-Sleep -Seconds 2
    }
    throw "$Stage did not converge to the expected READY controller: $($lastStatus | ConvertTo-Json -Compress -Depth 8)."
}

function Invoke-SetupOperation {
    param(
        [Parameter(Mandatory)][string] $Stage,
        [Parameter(Mandatory)][string] $Action
    )

    $log = Join-Path $evidenceDirectory "qa-$Stage.log"
    Write-Evidence -Stage $Stage -Status 'started' -Detail @{
        action = $Action
        log = $log
    }
    $arguments = @($Action, '/quiet', '/norestart', '/log', $log)
    $stopwatch = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $setup -ArgumentList $arguments -PassThru
    if (-not $process.WaitForExit($OperationTimeoutSeconds * 1000)) {
        $process.Kill()
        throw "$Stage exceeded $OperationTimeoutSeconds seconds."
    }
    $stopwatch.Stop()
    $exitCode = $process.ExitCode
    Write-Evidence -Stage $Stage -Status 'completed' -Detail @{
        exit_code = $exitCode
        elapsed_seconds = [Math]::Round($stopwatch.Elapsed.TotalSeconds, 1)
        log = $log
    }
    if ($exitCode -ne 0) {
        throw "$Stage failed with exit code $exitCode. See $log."
    }
}

Assert-Administrator
$signature = Get-AuthenticodeSignature -LiteralPath $setup
if ($signature.Status -ne 'Valid' -or
    $signature.SignerCertificate.Subject -ne 'CN=GNX Labs QA Publisher' -or
    -not $signature.TimeStamperCertificate) {
    throw 'QA lifecycle requires the valid timestamped GNX Labs Setup.'
}

$baseline = Wait-Ready -Stage 'baseline'
$baselineRegistrations = Assert-InstalledSurface
Write-Evidence -Stage 'baseline-registration' -Status 'passed' -Detail $baselineRegistrations

Invoke-SetupOperation -Stage 'repair' -Action '/repair'
Wait-Ready -Stage 'repair-convergence' | Out-Null
Write-Evidence -Stage 'repair-registration' -Status 'passed' -Detail (
    Assert-InstalledSurface
)

Invoke-SetupOperation -Stage 'uninstall' -Action '/uninstall'
$uninstallDeadline = [DateTime]::UtcNow.AddSeconds(60)
do {
    $service = Get-Service Quetzalcoatl -ErrorAction SilentlyContinue
    $registrations = @(Get-QuetzalcoatlRegistrations)
    $installRootExists = Test-Path -LiteralPath 'C:\Program Files\Quetzalcoatl'
    if (-not $service -and $registrations.Count -eq 0 -and -not $installRootExists) {
        break
    }
    Start-Sleep -Seconds 2
} while ([DateTime]::UtcNow -lt $uninstallDeadline)
if ($service -or $registrations.Count -ne 0 -or $installRootExists) {
    throw "Uninstall surface remains: service=$([bool]$service), registrations=$($registrations.Count), root=$installRootExists."
}
Write-Evidence -Stage 'uninstall-cleanup' -Status 'passed' -Detail @{
    service_absent = $true
    registrations_absent = $true
    install_root_absent = $true
}

Invoke-SetupOperation -Stage 'fresh-install' -Action '/install'
$finalStatus = Wait-Ready -Stage 'fresh-install-convergence'
$finalRegistrations = Assert-InstalledSurface
$installedCli = Get-Item -LiteralPath 'C:\Program Files\Quetzalcoatl\gnx.exe'
$trayProcesses = @(Get-Process gnx-tray -ErrorAction SilentlyContinue)
if ($installedCli.VersionInfo.ProductVersion -ne $ExpectedVersion) {
    throw "Installed CLI version differs: $($installedCli.VersionInfo.ProductVersion)."
}
if ($trayProcesses.Count -ne 1) {
    throw "Expected one tray process after fresh install; found $($trayProcesses.Count)."
}
Write-Evidence -Stage 'complete' -Status 'passed' -Detail @{
    version = $installedCli.VersionInfo.ProductVersion
    controller = $finalStatus.controller
    registrations = $finalRegistrations
    tray_processes = $trayProcesses.Count
    baseline_controller = $baseline.controller
}
