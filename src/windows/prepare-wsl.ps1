$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object Text.UTF8Encoding $false
$env:WSL_UTF8 = '1'
$root = '@ROOT@'
$runtime = 'C:\Program Files\WSL\wsl.exe'
# Pinned stable Microsoft release; updating it requires updating the trusted hash.
$url = 'https://github.com/microsoft/WSL/releases/download/2.7.14/wsl.2.7.14.0.x64.msi'
$expected = 'db084e536279a59e90a26ec598d8aa8a4dff8309f41d078fd06242953ac1ebcd'
$package = Join-Path $root 'wsl.2.7.14.0.x64.msi'

if (!(Test-Path -LiteralPath $runtime)) {
    Write-Output 'Installing the machine-wide WSL engine from the signed Microsoft MSI (no distribution).'
    $cached = (Test-Path -LiteralPath $package) -and ((Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash -eq $expected)
    if (!$cached) {
        $partial = "$package.download"
        & 'C:\Windows\System32\curl.exe' --fail --location --proto '=https' --proto-redir '=https' --tlsv1.2 --retry 2 --connect-timeout 20 --max-time 600 --output $partial $url
        if ($LASTEXITCODE -ne 0) { throw "Microsoft WSL download failed ($LASTEXITCODE). Check network access." }
        if ((Get-FileHash -LiteralPath $partial -Algorithm SHA256).Hash -ne $expected) { throw 'WSL MSI SHA256 mismatch. Installation refused.' }
        Move-Item -LiteralPath $partial -Destination $package -Force
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $package
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(^|,\s*)O=Microsoft Corporation(,|$)') {
        throw 'WSL MSI must have a valid Microsoft Corporation Authenticode signature.'
    }
    $arguments = "/i `"$package`" /qn /norestart /L*v `"$root\wsl-msi.log`""
    # Keep the native process handle. Start-Process + WaitForExit can return
    # a null ExitCode in Windows PowerShell, especially for fast processes.
    $msi = New-Object System.Diagnostics.Process
    $msi.StartInfo.FileName = 'C:\Windows\System32\msiexec.exe'
    $msi.StartInfo.Arguments = $arguments
    $msi.StartInfo.UseShellExecute = $false
    $msi.StartInfo.CreateNoWindow = $true
    try {
        if (!$msi.Start()) { throw 'Could not start Windows Installer.' }
        if (!$msi.WaitForExit(600000)) { throw 'Windows Installer is still running. Do not launch another MSI; inspect wsl-msi.log.' }
        $msiCode = $msi.ExitCode
    } finally { $msi.Dispose() }
    if ($msiCode -eq 3010) { exit 3010 }
    if ($msiCode -ne 0) { throw "WSL MSI failed ($msiCode); see wsl-msi.log." }
}
if (!(Test-Path -LiteralPath $runtime) -or !(Get-Service WslService -ErrorAction SilentlyContinue)) {
    throw 'Machine-wide WSL executable/service is missing after installation.'
}
$version = [version](Get-Item -LiteralPath $runtime).VersionInfo.FileVersion
if ($version -lt [version]'2.4.4.0') { throw "WSL $version is too old for named distributions. Update WSL before retrying; no automatic downgrade/overwrite performed." }
$probe = New-Object System.Diagnostics.Process
$probe.StartInfo.FileName = $runtime
$probe.StartInfo.Arguments = '--version'
$probe.StartInfo.UseShellExecute = $false
$probe.StartInfo.CreateNoWindow = $true
$probe.StartInfo.RedirectStandardOutput = $true
$probe.StartInfo.RedirectStandardError = $true
$probe.StartInfo.StandardOutputEncoding = [Text.Encoding]::UTF8
$probe.StartInfo.StandardErrorEncoding = [Text.Encoding]::UTF8
try {
    if (!$probe.Start()) { throw 'Could not start WSL version probe.' }
    $stdout = $probe.StandardOutput.ReadToEndAsync()
    $stderr = $probe.StandardError.ReadToEndAsync()
    if (!$probe.WaitForExit(30000)) { $probe.Kill(); throw 'Installed WSL did not respond to the version probe within 30 seconds.' }
    $probeCode = $probe.ExitCode
    [IO.File]::WriteAllText("$root\wsl-version.txt", $stdout.Result, [Text.Encoding]::UTF8)
    [IO.File]::WriteAllText("$root\wsl-version-error.txt", $stderr.Result, [Text.Encoding]::UTF8)
    Write-Output $stdout.Result
} finally { $probe.Dispose() }
if ($probeCode -ne 0) { throw "Installed WSL version probe failed ($probeCode); see wsl-version-error.txt." }
Write-Output 'Machine-wide WSL engine verified. Distribution installation remains delegated to the dedicated account.'
