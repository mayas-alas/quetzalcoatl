function Set-BurnAttachedContainerHash {
    param(
        [Parameter(Mandatory)][IO.FileStream] $Stream,
        [Parameter(Mandatory)][int64] $UxCabinetOffset,
        [Parameter(Mandatory)][int64] $UxCabinetSize,
        [Parameter(Mandatory)][string] $AttachedContainerHash
    )

    if ($UxCabinetSize -gt 16MB) {
        throw "Burn UX cabinet is unexpectedly large for deterministic manifest normalization."
    }
    $uxCabinet = New-Object byte[] ([int] $UxCabinetSize)
    $null = $Stream.Seek($UxCabinetOffset, [IO.SeekOrigin]::Begin)
    if ($Stream.Read($uxCabinet, 0, $uxCabinet.Length) -ne $uxCabinet.Length) {
        throw "Burn UX cabinet is truncated."
    }

    $uxText = [Text.Encoding]::ASCII.GetString($uxCabinet)
    $hashMatches = [regex]::Matches(
        $uxText,
        '<Container Id="WixAttachedContainer" FileSize="\d+" Hash="(?<hash>[0-9A-F]{128})" FilePath="QuetzalcoatlSetup\.exe"'
    )
    if ($hashMatches.Count -ne 1 -or $hashMatches[0].Groups['hash'].Length -ne $AttachedContainerHash.Length) {
        throw "Burn manifest attached-container hash field was not found exactly once."
    }

    $hashBytes = [Text.Encoding]::ASCII.GetBytes($AttachedContainerHash)
    $null = $Stream.Seek($UxCabinetOffset + $hashMatches[0].Groups['hash'].Index, [IO.SeekOrigin]::Begin)
    $Stream.Write($hashBytes, 0, $hashBytes.Length)
}

function Set-BurnDeterministicMetadata {
    param([Parameter(Mandatory)][string] $Path)

    $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
    try {
        $firstCabinetOffset = Find-FirstCabinetOffset -Stream $stream
        $firstCabinetSize = Set-CabinetDeterministicTimestamps -Stream $stream -CabinetOffset $firstCabinetOffset
        $secondCabinetOffset = $firstCabinetOffset + $firstCabinetSize
        $secondCabinetSize = Set-CabinetDeterministicTimestamps -Stream $stream -CabinetOffset $secondCabinetOffset
        if ($secondCabinetOffset + $secondCabinetSize -ne $stream.Length) {
            throw "Unsigned Burn bundle must end exactly after its two top-level cabinets."
        }

        $attachedContainerHash = Get-StreamRangeSha512 -Stream $stream -Offset $secondCabinetOffset -Length $secondCabinetSize
        Set-BurnAttachedContainerHash `
            -Stream $stream `
            -UxCabinetOffset $firstCabinetOffset `
            -UxCabinetSize $firstCabinetSize `
            -AttachedContainerHash $attachedContainerHash
        Set-CabinetDataChecksums -Stream $stream -CabinetOffset $firstCabinetOffset
        $stream.Flush()
    } finally {
        $stream.Dispose()
    }
}

function Test-BundleIdentityAndPayload {
    param(
        [Parameter(Mandatory)][string] $BundlePath,
        [Parameter(Mandatory)][string] $ProductMsiPath,
        [Parameter(Mandatory)][string] $BootstrapPath,
        [Parameter(Mandatory)][string] $WslMsiPath,
        [Parameter(Mandatory)][string] $PodmanMsiPath,
        [Parameter(Mandatory)][string] $ExpectedProductVersion,
        [Parameter(Mandatory)][bool] $ExpectSigned,
        [AllowNull()][string] $ExpectedSignerThumbprint,
        [Parameter(Mandatory)][bool] $QaTrustEnabled,
        [AllowNull()][string] $QaRootCertificatePath,
        [AllowNull()][string] $QaPublisherCertificatePath
    )

    $verificationRoot = Join-Path $outputRoot ("verify-" + [guid]::NewGuid().ToString('N'))
    $resolvedOutputRoot = [IO.Path]::GetFullPath($outputRoot).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $resolvedVerificationRoot = [IO.Path]::GetFullPath($verificationRoot)
    if (-not $resolvedVerificationRoot.StartsWith($resolvedOutputRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Bundle verification directory is outside the installer output root."
    }

    $baRoot = Join-Path $verificationRoot "ba"
    $payloadRoot = Join-Path $verificationRoot "payload"
    New-Item -ItemType Directory -Force -Path $baRoot, $payloadRoot | Out-Null
    try {
        & dotnet tool run wix -- burn extract $BundlePath -oba $baRoot -o $payloadRoot
        if ($LASTEXITCODE -ne 0) { throw "Bundle extraction verification failed." }

        $manifestPath = Join-Path $baRoot "manifest.xml"
        $manifest = [xml] (Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8)
        $registration = $manifest.SelectSingleNode('//*[local-name()="Registration"]')
        if (-not $registration -or $registration.GetAttribute('Id') -ne $releaseBundleId) {
            $actualBundleId = if ($registration) { $registration.GetAttribute('Id') } else { '<missing>' }
            throw "Built Burn registration identity mismatch: $actualBundleId"
        }
        if ($registration.GetAttribute('Version') -ne $releaseVersion -or
            $registration.GetAttribute('ProviderKey') -ne $bundleUpgradeCode) {
            throw "Built Burn registration version or ProviderKey does not match the release upgrade contract."
        }
        $relatedUpgrade = @($manifest.SelectNodes('//*[local-name()="RelatedBundle"]') | Where-Object {
            $_.GetAttribute('Id') -eq $bundleUpgradeCode -and $_.GetAttribute('Action') -eq 'Upgrade'
        })
        if ($relatedUpgrade.Count -ne 1) {
            throw "Built Burn manifest must contain exactly one preserved upgrade relation."
        }

        $embeddedProduct = @(Get-ChildItem -LiteralPath $payloadRoot -Recurse -File -Filter "Quetzalcoatl.msi")
        if ($embeddedProduct.Count -ne 1) {
            throw "Bundle must contain exactly one Quetzalcoatl.msi payload; found $($embeddedProduct.Count)."
        }
        $sourceHash = (Get-FileHash -LiteralPath $ProductMsiPath -Algorithm SHA256).Hash
        $embeddedHash = (Get-FileHash -LiteralPath $embeddedProduct[0].FullName -Algorithm SHA256).Hash
        if ($sourceHash -ne $embeddedHash) {
            throw "Embedded MSI does not match the deterministic product MSI."
        }
        Test-ReleaseArtifactSet `
            -Artifacts @(@{ Name = 'embedded-product-msi'; Path = $embeddedProduct[0].FullName }) `
            -ExpectSigned $ExpectSigned `
            -ExpectedThumbprint $ExpectedSignerThumbprint

        $expectedBootstrapNames = @(
            'gnx-bootstrap-install-podman.exe',
            'gnx-bootstrap-install-wsl.exe',
            'gnx-bootstrap-prepare.exe',
            $(if ($QaTrustEnabled) { 'gnx-bootstrap-prepare-qa-trust.exe' }),
            'gnx-bootstrap-validate.exe'
        ) | Where-Object { $_ }
        $embeddedBootstraps = @(Get-ChildItem -LiteralPath $payloadRoot -Recurse -File -Filter 'gnx-bootstrap-*.exe')
        $actualBootstrapNames = @($embeddedBootstraps | ForEach-Object Name | Sort-Object)
        if (($actualBootstrapNames -join "`n") -ne ($expectedBootstrapNames -join "`n")) {
            throw "Bundle bootstrap executable inventory differs: $($actualBootstrapNames -join ', ')"
        }
        $bootstrapHash = (Get-FileHash -LiteralPath $BootstrapPath -Algorithm SHA256).Hash
        foreach ($embeddedBootstrap in $embeddedBootstraps) {
            if ((Get-FileHash -LiteralPath $embeddedBootstrap.FullName -Algorithm SHA256).Hash -ne $bootstrapHash) {
                throw "Embedded bootstrap payload differs from the signed release input: $($embeddedBootstrap.Name)"
            }
        }
        Test-ReleaseArtifactSet `
            -Artifacts @($embeddedBootstraps | ForEach-Object {
                @{ Name = $_.BaseName; Path = $_.FullName; ExpectedVersion = $ExpectedProductVersion }
            }) `
            -ExpectSigned $ExpectSigned `
            -ExpectedThumbprint $ExpectedSignerThumbprint

        $embeddedQaCertificates = @(Get-ChildItem -LiteralPath $payloadRoot -Recurse -File -Filter 'gnx-qa-*.cer')
        if ($QaTrustEnabled) {
            if ([string]::IsNullOrWhiteSpace($QaRootCertificatePath) -or
                [string]::IsNullOrWhiteSpace($QaPublisherCertificatePath)) {
                throw 'QA Bundle verification requires both public certificate sources.'
            }
            $expectedQaCertificates = @(
                @{ Name = 'gnx-qa-root.cer'; Source = $QaRootCertificatePath },
                @{ Name = 'gnx-qa-publisher.cer'; Source = $QaPublisherCertificatePath }
            )
            $actualQaCertificateNames = @($embeddedQaCertificates | ForEach-Object Name | Sort-Object)
            if (($actualQaCertificateNames -join "`n") -ne (($expectedQaCertificates.Name | Sort-Object) -join "`n")) {
                throw "QA Bundle certificate inventory differs: $($actualQaCertificateNames -join ', ')"
            }
            foreach ($expectedQaCertificate in $expectedQaCertificates) {
                $embedded = @($embeddedQaCertificates | Where-Object Name -eq $expectedQaCertificate.Name)
                if ($embedded.Count -ne 1 -or
                    (Get-FileHash -LiteralPath $embedded[0].FullName -Algorithm SHA256).Hash -ne
                    (Get-FileHash -LiteralPath $expectedQaCertificate.Source -Algorithm SHA256).Hash) {
                    throw "Embedded QA certificate differs from its locked public source: $($expectedQaCertificate.Name)"
                }
                $certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new(
                    [IO.File]::ReadAllBytes($embedded[0].FullName)
                )
                try {
                    if ($certificate.HasPrivateKey) {
                        throw "Embedded QA certificate contains a private key: $($expectedQaCertificate.Name)"
                    }
                } finally {
                    $certificate.Dispose()
                }
            }
        } elseif ($embeddedQaCertificates.Count -ne 0) {
            throw 'Production Bundle must not contain QA trust certificates.'
        }

        $bootstrapperApplication = @(Get-ChildItem -LiteralPath $baRoot -Recurse -File -Filter 'wixstdba.exe')
        if ($bootstrapperApplication.Count -ne 1) {
            throw "Bundle must contain exactly one WiX bootstrapper application; found $($bootstrapperApplication.Count)."
        }
        Test-TrustedAuthenticodeArtifact -Path $bootstrapperApplication[0].FullName

        foreach ($dependency in @(
            @{ Name = 'wsl.2.7.10.0.x64.msi'; Source = $WslMsiPath },
            @{ Name = 'podman-installer-windows-amd64.msi'; Source = $PodmanMsiPath }
        )) {
            $embeddedDependency = @(Get-ChildItem -LiteralPath $payloadRoot -Recurse -File -Filter $dependency.Name)
            if ($embeddedDependency.Count -ne 1) {
                throw "Bundle must contain exactly one $($dependency.Name) ancillary payload; found $($embeddedDependency.Count)."
            }
            $expectedDependencyHash = (Get-FileHash -LiteralPath $dependency.Source -Algorithm SHA256).Hash
            $embeddedDependencyHash = (Get-FileHash -LiteralPath $embeddedDependency[0].FullName -Algorithm SHA256).Hash
            if ($expectedDependencyHash -ne $embeddedDependencyHash) {
                throw "Embedded dependency payload does not match its pinned source: $($dependency.Name)"
            }
            Test-TrustedAuthenticodeArtifact -Path $embeddedDependency[0].FullName
        }
    } finally {
        if (Test-Path -LiteralPath $resolvedVerificationRoot) {
            Remove-Item -LiteralPath $resolvedVerificationRoot -Recurse -Force
        }
    }
}

