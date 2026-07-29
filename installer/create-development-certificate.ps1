[CmdletBinding()]
param(
    [switch] $TrustForLocalMachine
)

$ErrorActionPreference = 'Stop'
$subject = 'CN=GNX Labs'
$friendlyName = 'GNX Labs Quetzalcoatl Development Code Signing'
$personalStore = 'Cert:\CurrentUser\My'
$rootStore = 'Cert:\CurrentUser\Root'
$codeSigningEku = '1.3.6.1.5.5.7.3.3'

if ($TrustForLocalMachine) {
    $principal = [Security.Principal.WindowsPrincipal]::new(
        [Security.Principal.WindowsIdentity]::GetCurrent()
    )
    if (-not $principal.IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator
    )) {
        throw 'TrustForLocalMachine requires an elevated administrator process.'
    }
}

$matches = @(
    Get-ChildItem -LiteralPath $personalStore |
        Where-Object {
            $_.Subject -eq $subject -and
            $_.Issuer -eq $subject -and
            $_.FriendlyName -eq $friendlyName -and
            $_.HasPrivateKey -and
            $_.NotAfter.ToUniversalTime() -gt [DateTime]::UtcNow -and
            $_.EnhancedKeyUsageList.ObjectId -contains $codeSigningEku
        }
)
if ($matches.Count -gt 1) {
    throw "Expected at most one active GNX Labs development certificate; found $($matches.Count)."
}

if ($matches.Count -eq 1) {
    $certificate = $matches[0]
} else {
    $certificate = New-SelfSignedCertificate `
        -Type CodeSigningCert `
        -Subject $subject `
        -FriendlyName $friendlyName `
        -CertStoreLocation $personalStore `
        -KeyAlgorithm RSA `
        -KeyLength 3072 `
        -HashAlgorithm SHA256 `
        -KeyExportPolicy NonExportable `
        -NotAfter (Get-Date).AddYears(1)
}

$trusted = Get-ChildItem -LiteralPath $rootStore |
    Where-Object Thumbprint -eq $certificate.Thumbprint
if (-not $trusted) {
    $store = [Security.Cryptography.X509Certificates.X509Store]::new(
        'Root',
        [Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
    )
    try {
        $store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
        $store.Add($certificate)
    } finally {
        $store.Dispose()
    }
}

$trusted = @(
    Get-ChildItem -LiteralPath $rootStore |
        Where-Object Thumbprint -eq $certificate.Thumbprint
)
if ($trusted.Count -lt 1) {
    throw 'GNX Labs development certificate was not trusted for the current user.'
}

$machineTrustStores = @()
if ($TrustForLocalMachine) {
    $publicCertificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new(
        $certificate.RawData
    )
    foreach ($storeName in @('Root', 'TrustedPublisher')) {
        $store = [Security.Cryptography.X509Certificates.X509Store]::new(
            $storeName,
            [Security.Cryptography.X509Certificates.StoreLocation]::LocalMachine
        )
        try {
            $store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
            $existing = @(
                $store.Certificates |
                    Where-Object Thumbprint -eq $certificate.Thumbprint
            )
            if ($existing.Count -eq 0) {
                $store.Add($publicCertificate)
            } elseif ($existing.Count -gt 1) {
                throw "GNX Labs development certificate appears more than once in LocalMachine\\$storeName."
            } elseif ($existing[0].HasPrivateKey) {
                $store.Remove($existing[0])
                $store.Add($publicCertificate)
            }
        } finally {
            $store.Dispose()
        }

        $installed = @(
            Get-ChildItem -LiteralPath "Cert:\LocalMachine\$storeName" |
                Where-Object Thumbprint -eq $certificate.Thumbprint
        )
        if ($installed.Count -ne 1 -or $installed[0].HasPrivateKey) {
            throw "GNX Labs public certificate was not trusted exactly once without a private key in LocalMachine\\$storeName."
        }
        $machineTrustStores += "Cert:\LocalMachine\$storeName"
    }
}

[pscustomobject]@{
    Subject = $certificate.Subject
    Thumbprint = $certificate.Thumbprint
    NotAfter = $certificate.NotAfter
    PrivateKeyStore = $personalStore
    TrustStore = $rootStore
    MachineTrustStores = $machineTrustStores
    Exportable = $false
    Purpose = 'DevelopmentOnly'
}
