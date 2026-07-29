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
    $match
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
    if ($TimestampUrl -notmatch '^https://') {
        throw 'The Authenticode timestamp URL must use HTTPS.'
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
