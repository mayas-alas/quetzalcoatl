[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this explicit trust action from an elevated PowerShell.'
}
$source = 'C:\ProgramData\GNX\public\root.crt'
if (-not (Test-Path -LiteralPath $source)) { throw 'FAILED CA_MISSING' }
$certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new($source)
if ($certificate.Subject -ne 'CN=GNX Autonomous Root' -or $certificate.HasPrivateKey) {
    $certificate.Dispose()
    throw 'FAILED CA_IDENTITY'
}
$store = [Security.Cryptography.X509Certificates.X509Store]::new('Root', 'LocalMachine')
try {
    $store.Open('ReadWrite')
    $store.Add($certificate)
} finally {
    $store.Close()
    $certificate.Dispose()
}
Write-Output 'READY autonomous CA trusted explicitly on this Windows host'
