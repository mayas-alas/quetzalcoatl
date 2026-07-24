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
        [Parameter(Mandatory)][string] $ProductMsiPath
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
    } finally {
        if (Test-Path -LiteralPath $resolvedVerificationRoot) {
            Remove-Item -LiteralPath $resolvedVerificationRoot -Recurse -Force
        }
    }
}

