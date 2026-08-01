[CmdletBinding()]
param(
    [string] $SigningCertificateThumbprint
)

$ErrorActionPreference = 'Stop'
$rootSubject = 'CN=GNX Labs QA Root'
$rootFriendlyName = 'GNX Labs Quetzalcoatl QA Root'
$publisherSubject = 'CN=GNX Labs QA Publisher'
$publisherFriendlyName = 'GNX Labs Quetzalcoatl QA Code Signing'
$personalStore = 'Cert:\CurrentUser\My'
$codeSigningEku = '1.3.6.1.5.5.7.3.3'
$rsaOid = '1.2.840.113549.1.1.1'
$now = [DateTime]::UtcNow

function Test-QaRootCertificate {
    param([Parameter(Mandatory)] $Certificate)

    $constraints = @(
        $Certificate.Extensions |
            Where-Object { $_ -is [Security.Cryptography.X509Certificates.X509BasicConstraintsExtension] }
    )
    $keyUsage = @(
        $Certificate.Extensions |
            Where-Object { $_ -is [Security.Cryptography.X509Certificates.X509KeyUsageExtension] }
    )
    $Certificate.Subject -eq $rootSubject -and
        $Certificate.Issuer -eq $rootSubject -and
        $Certificate.FriendlyName -eq $rootFriendlyName -and
        $Certificate.HasPrivateKey -and
        $Certificate.PublicKey.Oid.Value -eq $rsaOid -and
        $Certificate.NotAfter.ToUniversalTime() -gt $now.AddYears(1) -and
        $constraints.Count -eq 1 -and
        $constraints[0].CertificateAuthority -and
        $constraints[0].HasPathLengthConstraint -and
        $constraints[0].PathLengthConstraint -eq 0 -and
        $keyUsage.Count -eq 1 -and
        (($keyUsage[0].KeyUsages -band [Security.Cryptography.X509Certificates.X509KeyUsageFlags]::KeyCertSign) -ne 0)
}

function Test-QaPublisherCertificate {
    param(
        [Parameter(Mandatory)] $Certificate,
        [Parameter(Mandatory)] $RootCertificate
    )

    $keyUsage = @(
        $Certificate.Extensions |
            Where-Object { $_ -is [Security.Cryptography.X509Certificates.X509KeyUsageExtension] }
    )
    $Certificate.Subject -eq $publisherSubject -and
        $Certificate.Issuer -eq $RootCertificate.Subject -and
        $Certificate.FriendlyName -eq $publisherFriendlyName -and
        $Certificate.HasPrivateKey -and
        $Certificate.PublicKey.Oid.Value -eq $rsaOid -and
        $Certificate.NotAfter.ToUniversalTime() -gt $now.AddDays(120) -and
        ($Certificate.EnhancedKeyUsageList.ObjectId -contains $codeSigningEku) -and
        $keyUsage.Count -eq 1 -and
        (($keyUsage[0].KeyUsages -band [Security.Cryptography.X509Certificates.X509KeyUsageFlags]::DigitalSignature) -ne 0)
}

$roots = @(
    Get-ChildItem -LiteralPath $personalStore |
        Where-Object { Test-QaRootCertificate -Certificate $_ }
)
if ($roots.Count -gt 1) {
    throw "Expected at most one active GNX Labs QA root with a private key; found $($roots.Count)."
}
if ($roots.Count -eq 1) {
    $root = $roots[0]
} else {
    $root = New-SelfSignedCertificate `
        -Type Custom `
        -Subject $rootSubject `
        -FriendlyName $rootFriendlyName `
        -CertStoreLocation $personalStore `
        -KeyAlgorithm RSA `
        -KeyLength 4096 `
        -HashAlgorithm SHA256 `
        -KeyExportPolicy NonExportable `
        -KeyUsage CertSign, CRLSign, DigitalSignature `
        -TextExtension @('2.5.29.19={critical}{text}ca=true&pathlength=0') `
        -NotAfter (Get-Date).AddYears(10)
    if (-not (Test-QaRootCertificate -Certificate $root)) {
        throw 'Generated GNX Labs QA root does not satisfy the closed CA contract.'
    }
}

$publicRoot = [Security.Cryptography.X509Certificates.X509Certificate2]::new($root.RawData)
$rootStore = [Security.Cryptography.X509Certificates.X509Store]::new(
    'Root',
    [Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
)
try {
    $rootStore.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
    if (-not ($rootStore.Certificates | Where-Object Thumbprint -eq $root.Thumbprint)) {
        $rootStore.Add($publicRoot)
    }
} finally {
    $rootStore.Dispose()
    $publicRoot.Dispose()
}

$requestedThumbprint = ([string] $SigningCertificateThumbprint).Replace(' ', '').ToUpperInvariant()
if ($requestedThumbprint -and $requestedThumbprint -notmatch '^[0-9A-F]{40}$') {
    throw 'SigningCertificateThumbprint must contain exactly 40 hexadecimal characters.'
}
$publishers = @(
    Get-ChildItem -LiteralPath $personalStore |
        Where-Object {
            (Test-QaPublisherCertificate -Certificate $_ -RootCertificate $root) -and
            (-not $requestedThumbprint -or $_.Thumbprint -eq $requestedThumbprint)
        } |
        Sort-Object NotAfter -Descending
)
if ($requestedThumbprint -and $publishers.Count -ne 1) {
    throw "The requested QA publisher $requestedThumbprint is absent or does not satisfy the closed QA certificate contract."
}
if ($publishers.Count -gt 0) {
    $publisher = $publishers[0]
} else {
    $publisherNotAfter = (Get-Date).AddYears(2)
    if ($publisherNotAfter -ge $root.NotAfter) {
        $publisherNotAfter = $root.NotAfter.AddDays(-1)
    }
    $publisher = New-SelfSignedCertificate `
        -Type Custom `
        -Subject $publisherSubject `
        -FriendlyName $publisherFriendlyName `
        -Signer $root `
        -CertStoreLocation $personalStore `
        -KeyAlgorithm RSA `
        -KeyLength 3072 `
        -HashAlgorithm SHA256 `
        -KeyExportPolicy NonExportable `
        -KeyUsage DigitalSignature `
        -TextExtension @("2.5.29.37={critical}{text}$codeSigningEku") `
        -NotAfter $publisherNotAfter
    if (-not (Test-QaPublisherCertificate -Certificate $publisher -RootCertificate $root)) {
        throw 'Generated GNX Labs QA publisher does not satisfy the closed code-signing contract.'
    }
}

$chain = [Security.Cryptography.X509Certificates.X509Chain]::new()
try {
    $chain.ChainPolicy.RevocationMode = [Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
    if (-not $chain.Build($publisher)) {
        $failures = @($chain.ChainStatus | ForEach-Object Status) -join ', '
        throw "GNX Labs QA publisher does not build to the QA root: $failures"
    }
    $elements = @($chain.ChainElements | ForEach-Object Certificate)
    if ($elements.Count -ne 2 -or $elements[-1].Thumbprint -ne $root.Thumbprint) {
        throw 'GNX Labs QA publisher chain does not contain exactly the expected leaf and QA root.'
    }
} finally {
    $chain.Dispose()
}

$publicPublisher = [Security.Cryptography.X509Certificates.X509Certificate2]::new($publisher.RawData)
$publisherStore = [Security.Cryptography.X509Certificates.X509Store]::new(
    'TrustedPublisher',
    [Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
)
try {
    $publisherStore.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
    if (-not ($publisherStore.Certificates | Where-Object Thumbprint -eq $publisher.Thumbprint)) {
        $publisherStore.Add($publicPublisher)
    }
} finally {
    $publisherStore.Dispose()
    $publicPublisher.Dispose()
}

$outputRoot = Join-Path (Split-Path -Parent $PSScriptRoot) 'target\qa-signing'
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
$rootPath = Join-Path $outputRoot 'gnx-qa-root.cer'
$publisherPath = Join-Path $outputRoot 'gnx-qa-publisher.cer'
Export-Certificate -Cert $root -FilePath $rootPath -Type CERT -Force | Out-Null
Export-Certificate -Cert $publisher -FilePath $publisherPath -Type CERT -Force | Out-Null

$exportedRoot = [Security.Cryptography.X509Certificates.X509Certificate2]::new(
    [IO.File]::ReadAllBytes($rootPath)
)
$exportedPublisher = [Security.Cryptography.X509Certificates.X509Certificate2]::new(
    [IO.File]::ReadAllBytes($publisherPath)
)
try {
    if ($exportedRoot.HasPrivateKey -or $exportedPublisher.HasPrivateKey) {
        throw 'QA public certificate export unexpectedly contains a private key.'
    }
} finally {
    $exportedRoot.Dispose()
    $exportedPublisher.Dispose()
}

[pscustomobject]@{
    Purpose = 'QaOnly'
    RootSubject = $root.Subject
    RootThumbprint = $root.Thumbprint
    RootNotAfter = $root.NotAfter
    RootCertificatePath = $rootPath
    RootSha256 = (Get-FileHash -LiteralPath $rootPath -Algorithm SHA256).Hash
    PublisherSubject = $publisher.Subject
    PublisherThumbprint = $publisher.Thumbprint
    PublisherNotAfter = $publisher.NotAfter
    PublisherCertificatePath = $publisherPath
    PublisherSha256 = (Get-FileHash -LiteralPath $publisherPath -Algorithm SHA256).Hash
    PrivateKeyStore = $personalStore
    Exportable = $false
}
