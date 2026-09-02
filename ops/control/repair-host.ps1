[CmdletBinding()]
param([string]$Distribution = 'Ubuntu-24.04', [switch]$StayActive)
$ErrorActionPreference = 'Stop'
$state = Join-Path $env:ProgramData 'GNX\control'
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'GNX host repair requires elevation.'
}
& wsl.exe -d $Distribution --user root --exec systemctl start gnx-control.service gnx-console.service gnx-entry.service
if ($LASTEXITCODE -ne 0) { throw 'GNX control services did not start.' }
$address = (& wsl.exe -d $Distribution --exec ip -4 -o addr show dev eth0) -join ' '
if ($address -notmatch 'inet (\d+\.\d+\.\d+\.\d+)/') { throw 'WSL address unavailable.' }
$ip = $Matches[1]
$hostsFile = Join-Path $env:SystemRoot 'System32\drivers\etc\hosts'
$content = [IO.File]::ReadAllText($hostsFile)
$marker = '# GNX control'
$lines = $content -split '\r?\n'
if ($lines | Where-Object { $_ -notmatch '^\s*#' -and $_ -match '\bmesh\.gnx\b' -and -not $_.EndsWith($marker) }) {
    throw 'An unmanaged mesh.gnx hosts entry exists; refusing to replace it.'
}
$backup = Join-Path $state 'hosts.before'
if (-not (Test-Path -LiteralPath $backup)) { Copy-Item -LiteralPath $hostsFile -Destination $backup }
$kept = @($lines | Where-Object { -not $_.EndsWith($marker) })
$updated = ($kept -join "`r`n").TrimEnd("`r", "`n") + "`r`n$ip mesh.gnx $marker`r`n"
if ($updated -ne $content) { [IO.File]::WriteAllText($hostsFile, $updated, [Text.UTF8Encoding]::new($false)) }
Clear-DnsClientCache
$certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new((Join-Path $state 'root.crt'))
if ($certificate.Subject -ne 'CN=GNX Mesh Local Root') { throw 'Unexpected GNX CA subject.' }
if (-not (Test-Path "Cert:\LocalMachine\Root\$($certificate.Thumbprint)")) {
    Import-Certificate -FilePath (Join-Path $state 'root.crt') -CertStoreLocation 'Cert:\LocalMachine\Root' | Out-Null
}
$probe = $null
for ($attempt = 0; $attempt -lt 12; $attempt++) {
    try {
        $probe = Invoke-RestMethod -Uri 'https://mesh.gnx/api/instance' -TimeoutSec 5 -MaximumRedirection 0
        break
    } catch { Start-Sleep -Seconds 3 }
}
if ($null -eq $probe.setup_required) { throw 'Control-plane probe returned an invalid response.' }
'READY host-resolution-and-tls'
if ($StayActive) {
    & wsl.exe -d $Distribution --exec sleep infinity
    throw 'WSL session ended; the scheduled task must repair and restart it.'
}
