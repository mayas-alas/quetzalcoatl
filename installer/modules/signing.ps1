function Resolve-CodeSigningCertificate {
    param([Parameter(Mandatory)][string] $Thumbprint)

    $normalized = $Thumbprint.Replace(' ', '').ToUpperInvariant()
    if ($normalized -notmatch '^[0-9A-F]{40}$') {
        throw 'Signing certificate thumbprint must contain exactly 40 hexadecimal characters.'
    }
    $matches = @()
    foreach ($store in @(
        @{ Path = 'Cert:\CurrentUser\My'; Machine = $false },
        @{ Path = 'Cert:\LocalMachine\My'; Machine = $true }
    )) {
        $certificate = Get-ChildItem -LiteralPath $store.Path -ErrorAction SilentlyContinue |
            Where-Object Thumbprint -eq $normalized
        foreach ($item in $certificate) {
            $matches += [pscustomobject]@{
                Certificate = $item
                MachineStore = $store.Machine
            }
        }
    }
    if ($matches.Count -ne 1) {
        throw "Expected exactly one signing certificate with thumbprint $normalized; found $($matches.Count)."
    }
    $match = $matches[0]
    $certificate = $match.Certificate
    if (-not $certificate.HasPrivateKey) {
        throw "Signing certificate $normalized has no private key."
    }
    if ($certificate.NotBefore.ToUniversalTime() -gt [DateTime]::UtcNow -or
        $certificate.NotAfter.ToUniversalTime() -le [DateTime]::UtcNow) {
        throw "Signing certificate $normalized is not currently valid."
    }
    if (-not ($certificate.EnhancedKeyUsageList.ObjectId -contains '1.3.6.1.5.5.7.3.3')) {
        throw "Certificate $normalized is not authorized for code signing."
    }
    [pscustomobject]@{
        Certificate = $certificate
        MachineStore = $match.MachineStore
        SelfSigned = $certificate.Subject -eq $certificate.Issuer
    }
}

function Test-CodeSigningCertificateTrust {
    param(
        [Parameter(Mandatory)] $Certificate,
        [Parameter(Mandatory)][bool] $RequireAuthRoot
    )

    if ($Certificate.PublicKey.Oid.Value -ne '1.2.840.113549.1.1.1') {
        throw "Smart App Control release signing requires an RSA certificate: $($Certificate.Subject)"
    }

    $chain = [Security.Cryptography.X509Certificates.X509Chain]::new()
    try {
        $chain.ChainPolicy.RevocationMode = [Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
        # Authenticode's Valid status already evaluates the RFC 3161 timestamp.
        # Build only the publisher chain here, including timestamp-valid artifacts
        # whose leaf certificate has since expired.
        $chain.ChainPolicy.VerificationFlags = [Security.Cryptography.X509Certificates.X509VerificationFlags]::IgnoreNotTimeValid
        if (-not $chain.Build($Certificate)) {
            $failures = @($chain.ChainStatus | ForEach-Object Status) -join ', '
            throw "Signing certificate does not build a trusted Windows chain: $failures"
        }
        $elements = @($chain.ChainElements | ForEach-Object Certificate)
        if ($elements.Count -lt 2) {
            throw 'Smart App Control release signing rejects a self-signed certificate chain.'
        }
        $root = $elements[-1]
        $trustedProgramRoots = @(
            Get-ChildItem -LiteralPath 'Cert:\LocalMachine\AuthRoot' -ErrorAction SilentlyContinue
            Get-ChildItem -LiteralPath 'Cert:\CurrentUser\AuthRoot' -ErrorAction SilentlyContinue
        )
        if ($RequireAuthRoot -and
            -not ($trustedProgramRoots | Where-Object Thumbprint -eq $root.Thumbprint)) {
            throw "Signing chain root is not present in the Windows AuthRoot store: $($root.Subject)"
        }
    } finally {
        $chain.Dispose()
    }
}

function Test-TrustedAuthenticodeArtifact {
    param([Parameter(Mandatory)][string] $Path)

    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or -not $signature.SignerCertificate) {
        throw "Trusted Authenticode verification failed for ${Path}: status=$($signature.Status)."
    }
    if (-not $signature.TimeStamperCertificate) {
        throw "Trusted Authenticode artifact has no timestamp: $Path"
    }
    Test-CodeSigningCertificateTrust `
        -Certificate $signature.SignerCertificate `
        -RequireAuthRoot $false
}

function Test-ReleaseArtifactSet {
    param(
        [Parameter(Mandatory)][hashtable[]] $Artifacts,
        [Parameter(Mandatory)][bool] $ExpectSigned,
        [AllowNull()][string] $ExpectedThumbprint
    )

    if ($Artifacts.Count -eq 0) {
        throw 'Release artifact signature inventory must not be empty.'
    }
    if ($ExpectSigned -and $ExpectedThumbprint -notmatch '^[0-9A-Fa-f]{40}$') {
        throw 'Signed release artifact inventory requires one signer thumbprint.'
    }

    $names = @($Artifacts | ForEach-Object { [string] $_.Name })
    if ($names -contains '' -or @($names | Sort-Object -Unique).Count -ne $names.Count) {
        throw 'Release artifact signature inventory contains an absent or duplicate semantic name.'
    }
    $paths = @($Artifacts | ForEach-Object { [IO.Path]::GetFullPath([string] $_.Path) })
    if (@($paths | Sort-Object -Unique).Count -ne $paths.Count) {
        throw 'Release artifact signature inventory contains a duplicate path.'
    }

    foreach ($artifact in $Artifacts) {
        $name = [string] $artifact.Name
        $path = [IO.Path]::GetFullPath([string] $artifact.Path)
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Release artifact is absent: $name ($path)"
        }
        if ($artifact.ContainsKey('ExpectedVersion')) {
            $expectedVersion = [string] $artifact.ExpectedVersion
            $version = (Get-Item -LiteralPath $path).VersionInfo
            if ($version.FileVersion -ne $expectedVersion -or
                $version.ProductVersion -ne $expectedVersion) {
                throw "Release artifact version mismatch for ${name}: file=$($version.FileVersion) product=$($version.ProductVersion) expected=$expectedVersion"
            }
        }

        if ($ExpectSigned) {
            Test-AuthenticodeArtifact -Path $path -ExpectedThumbprint $ExpectedThumbprint
        } else {
            $signature = Get-AuthenticodeSignature -LiteralPath $path
            if ($signature.Status -ne 'NotSigned') {
                throw "Unsigned QA artifact inventory unexpectedly contains a signature for ${name}: status=$($signature.Status)."
            }
        }
    }
}

function Get-SignToolPath {
    $command = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $candidate = Get-ChildItem -LiteralPath $kits -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue |
        Where-Object FullName -Match '\\x64\\signtool\.exe$' |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $candidate) {
        throw 'Windows SDK signtool.exe is unavailable.'
    }
    $candidate.FullName
}

function Invoke-AuthenticodeSign {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)] $SigningIdentity,
        [Parameter(Mandatory)][string] $TimestampUrl
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Cannot sign absent artifact: $Path"
    }
    if ($TimestampUrl -ne 'http://timestamp.digicert.com' -and
        $TimestampUrl -notmatch '^https://') {
        throw 'The Authenticode timestamp URL must use HTTPS or the pinned DigiCert RFC 3161 endpoint.'
    }
    $signTool = Get-SignToolPath
    $arguments = @(
        'sign',
        '/fd', 'SHA256',
        '/td', 'SHA256',
        '/tr', $TimestampUrl,
        '/s', 'My',
        '/sha1', $SigningIdentity.Certificate.Thumbprint
    )
    if ($SigningIdentity.MachineStore) {
        $arguments += '/sm'
    }
    $arguments += $Path
    & $signTool @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Authenticode signing failed for $Path."
    }
    Test-AuthenticodeArtifact -Path $Path -ExpectedThumbprint $SigningIdentity.Certificate.Thumbprint
}

function Test-AuthenticodeArtifact {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $ExpectedThumbprint
    )

    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or
        $signature.SignerCertificate.Thumbprint -ne $ExpectedThumbprint) {
        throw "Authenticode verification failed for ${Path}: status=$($signature.Status)."
    }
    if (-not $signature.TimeStamperCertificate) {
        throw "Authenticode signature for $Path has no trusted timestamp."
    }
}

function Invoke-BurnAuthenticodeSign {
    param(
        [Parameter(Mandatory)][string] $BundlePath,
        [Parameter(Mandatory)] $SigningIdentity,
        [Parameter(Mandatory)][string] $TimestampUrl,
        [Parameter(Mandatory)][string] $WorkingDirectory
    )

    $engine = Join-Path $WorkingDirectory 'QuetzalcoatlSetup.engine.exe'
    $reattached = Join-Path $WorkingDirectory 'QuetzalcoatlSetup.reattached.exe'
    foreach ($temporary in @($engine, $reattached)) {
        if (Test-Path -LiteralPath $temporary) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
    & dotnet tool run wix -- burn detach $BundlePath -engine $engine
    if ($LASTEXITCODE -ne 0) {
        throw 'Burn engine detach failed.'
    }
    Invoke-AuthenticodeSign -Path $engine -SigningIdentity $SigningIdentity -TimestampUrl $TimestampUrl
    & dotnet tool run wix -- burn reattach $BundlePath -engine $engine -out $reattached
    if ($LASTEXITCODE -ne 0) {
        throw 'Signed Burn engine reattach failed.'
    }
    Invoke-AuthenticodeSign -Path $reattached -SigningIdentity $SigningIdentity -TimestampUrl $TimestampUrl
    Move-Item -LiteralPath $reattached -Destination $BundlePath -Force
    Remove-Item -LiteralPath $engine -Force
    Test-AuthenticodeArtifact `
        -Path $BundlePath `
        -ExpectedThumbprint $SigningIdentity.Certificate.Thumbprint
}
