function Test-RebootContract {
    param(
        [string] $BundlePath = (Join-Path $installerRoot "bundle.wxs"),
        [string] $BundleXml
    )

    $exitCodesPath = Join-Path $repoRoot "crates\gnx-host-preflight\src\exit_codes.rs"
    $exitCodes = Get-Content -LiteralPath $exitCodesPath -Raw -Encoding utf8
    $rebootValues = @{}
    foreach ($constantName in @('REBOOT_PENDING', 'REBOOT_REQUIRED')) {
        $constantMatch = [regex]::Match(
            $exitCodes,
            "(?m)^\s*pub\s+const\s+$constantName\s*:\s*i32\s*=\s*(?<value>\d+)\s*;"
        )
        if (-not $constantMatch.Success) {
            throw "Installer reboot contract: $constantName was not found in $exitCodesPath."
        }
        $rebootValues[$constantName] = $constantMatch.Groups['value'].Value
    }
    $rebootPending = $rebootValues['REBOOT_PENDING']
    $rebootRequired = $rebootValues['REBOOT_REQUIRED']

    $hasBundleXml = -not [string]::IsNullOrEmpty($BundleXml)
    $bundleSource = if ($hasBundleXml) { '<provided XML>' } else { $BundlePath }
    $bundleText = if ($hasBundleXml) { $BundleXml } else { Get-Content -LiteralPath $BundlePath -Raw -Encoding utf8 }
    $bundle = [xml] $bundleText
    $exePackages = @($bundle.SelectNodes('//*[local-name()="ExePackage"]'))
    $expectedMappings = @{
        PrepareWsl = @{
            '0' = 'success'
            $rebootPending = 'forceReboot'
            $rebootRequired = 'forceReboot'
        }
        ValidateHost = @{
            '0' = 'success'
            $rebootPending = 'forceReboot'
        }
    }

    foreach ($authorizedId in $expectedMappings.Keys) {
        $matches = @($exePackages | Where-Object { $_.GetAttribute('Id') -eq $authorizedId })
        if ($matches.Count -ne 1) {
            throw "Installer reboot contract: expected exactly one ExePackage Id=$authorizedId in $bundleSource; found $($matches.Count)."
        }

        $packageExitCodes = @($matches[0].SelectNodes('./*[local-name()="ExitCode"]'))
        $valuedExitCodes = @($packageExitCodes | Where-Object { $_.HasAttribute('Value') })
        $catchAll = @($packageExitCodes | Where-Object {
            -not $_.HasAttribute('Value') -and $_.GetAttribute('Behavior') -eq 'error'
        })
        if ($catchAll.Count -ne 1 -or $packageExitCodes.Count -ne ($expectedMappings[$authorizedId].Count + 1)) {
            throw "Installer reboot contract: $authorizedId must contain exactly its valued mappings and one catch-all ExitCode Behavior=error."
        }
        if ($packageExitCodes[$packageExitCodes.Count - 1] -ne $catchAll[0]) {
            throw "Installer reboot contract: $authorizedId catch-all ExitCode Behavior=error must be the last ExitCode child, after all valued mappings."
        }

        $actualValues = @($valuedExitCodes | ForEach-Object { $_.GetAttribute('Value') })
        if ($actualValues.Count -ne ($actualValues | Select-Object -Unique).Count) {
            throw "Installer reboot contract: $authorizedId must not contain duplicate valued ExitCode mappings."
        }
        foreach ($value in $actualValues) {
            if (-not $expectedMappings[$authorizedId].ContainsKey($value)) {
                throw "Installer reboot contract: $authorizedId must not contain ExitCode Value=$value."
            }
        }
        foreach ($value in $expectedMappings[$authorizedId].Keys) {
            $mapping = @($valuedExitCodes | Where-Object { $_.GetAttribute('Value') -eq $value })
            if ($mapping.Count -ne 1 -or $mapping[0].GetAttribute('Behavior') -ne $expectedMappings[$authorizedId][$value]) {
                throw "Installer reboot contract: $authorizedId must map ExitCode Value=$value to Behavior=$($expectedMappings[$authorizedId][$value]) exactly once."
            }
        }
    }

    foreach ($exePackage in $exePackages) {
        if ($expectedMappings.ContainsKey($exePackage.GetAttribute('Id'))) { continue }
        $hasUnauthorizedMapping = @($exePackage.SelectNodes('./*[local-name()="ExitCode"]') | Where-Object {
            $_.GetAttribute('Value') -eq $rebootPending -and $_.GetAttribute('Behavior') -eq 'forceReboot'
        }).Count -gt 0
        if ($hasUnauthorizedMapping) {
            throw "Installer reboot contract: ExePackage Id=$($exePackage.GetAttribute('Id')) must not map Rust REBOOT_PENDING=$rebootPending to Behavior=forceReboot."
        }
    }
}

function Test-ReleaseIdentityContract {
    $packagePath = Join-Path $installerRoot "package.wxs"
    $bundlePath = Join-Path $installerRoot "bundle.wxs"
    $package = [xml] (Get-Content -LiteralPath $packagePath -Raw -Encoding utf8)
    $bundle = [xml] (Get-Content -LiteralPath $bundlePath -Raw -Encoding utf8)
    $packageNode = $package.SelectSingleNode('/*[local-name()="Wix"]/*[local-name()="Package"]')
    $bundleNode = $bundle.SelectSingleNode('/*[local-name()="Wix"]/*[local-name()="Bundle"]')

    if (-not $packageNode -or -not $bundleNode) {
        throw "Release identity contract: package or bundle root is missing."
    }
    if ($packageNode.GetAttribute('Version') -ne $releaseVersion -or $bundleNode.GetAttribute('Version') -ne $releaseVersion) {
        throw "Release identity contract: package and bundle must both use version $releaseVersion."
    }
    if ($packageNode.GetAttribute('ProductCode') -ne $releaseProductCode) {
        throw "Release identity contract: package ProductCode must be the explicit $releaseVersion identity $releaseProductCode."
    }
    if ($packageNode.GetAttribute('UpgradeCode') -ne $releaseUpgradeCode) {
        throw "Release identity contract: package UpgradeCode must remain $releaseUpgradeCode."
    }
    if ($releaseProductCode -eq $previousProductCode -or
        $releasePackageCode -eq $previousPackageCode -or
        $releaseBundleId -eq $previousBundleId) {
        throw "Release identity contract: $releaseVersion must not reuse the previous release identity."
    }
    $majorUpgrade = @($packageNode.SelectNodes('./*[local-name()="MajorUpgrade"]'))
    if ($majorUpgrade.Count -ne 1 -or
        $majorUpgrade[0].GetAttribute('Schedule') -ne 'afterInstallInitialize') {
        throw "Release identity contract: MSI major upgrade must preserve rollback-safe scheduling."
    }
    $serviceBinaryComponents = @($package.SelectNodes('//*[local-name()="Component" and @Id="GnxServiceBinaryComponent"]'))
    if ($serviceBinaryComponents.Count -ne 1) {
        throw "Release identity contract: gnx-service.exe must have exactly one dedicated MSI component."
    }
    $serviceBinaryFiles = @($serviceBinaryComponents[0].SelectNodes('./*[local-name()="File" and @Id="GnxService"]'))
    if ($serviceBinaryFiles.Count -ne 1 -or
        $serviceBinaryFiles[0].GetAttribute('KeyPath') -ne 'yes' -or
        $serviceBinaryFiles[0].GetAttribute('Name') -ne 'gnx-service.exe') {
        throw "Release identity contract: gnx-service.exe must be the key path of GnxServiceBinaryComponent."
    }
    $legacyGroupedServiceFiles = @($package.SelectNodes('//*[local-name()="Component" and @Id="ServiceComponent"]/*[local-name()="File" and @Id="GnxService"]'))
    if ($legacyGroupedServiceFiles.Count -ne 0) {
        throw "Release identity contract: gnx-service.exe must not remain grouped behind the WinSW key path."
    }
    $serviceBinaryRefs = @($package.SelectNodes('//*[local-name()="Feature"]/*[local-name()="ComponentRef" and @Id="GnxServiceBinaryComponent"]'))
    if ($serviceBinaryRefs.Count -ne 1) {
        throw "Release identity contract: ProductFeature must install GnxServiceBinaryComponent exactly once."
    }
    $cliComponents = @($package.SelectNodes('//*[local-name()="Component" and @Id="CliComponent"]'))
    if ($cliComponents.Count -ne 1) {
        throw "Release identity contract: gnx.exe must have exactly one CLI component."
    }
    $cliFiles = @($cliComponents[0].SelectNodes('./*[local-name()="File" and @Id="GnxCli"]'))
    if ($cliFiles.Count -ne 1 -or
        $cliFiles[0].GetAttribute('KeyPath') -ne 'yes' -or
        $cliFiles[0].GetAttribute('Name') -ne 'gnx.exe') {
        throw "Release identity contract: gnx.exe must be the key path of CliComponent."
    }
    $cliPathEntries = @($cliComponents[0].SelectNodes('./*[local-name()="Environment" and @Id="SystemPath"]'))
    if ($cliPathEntries.Count -ne 1 -or
        $cliPathEntries[0].GetAttribute('Name') -ne 'PATH' -or
        $cliPathEntries[0].GetAttribute('Value') -ne '[INSTALLFOLDER]' -or
        $cliPathEntries[0].GetAttribute('Action') -ne 'set' -or
        $cliPathEntries[0].GetAttribute('Part') -ne 'last' -or
        $cliPathEntries[0].GetAttribute('System') -ne 'yes' -or
        $cliPathEntries[0].GetAttribute('Permanent') -ne 'no') {
        throw "Release identity contract: gnx.exe system PATH registration differs."
    }
    $cliRefs = @($package.SelectNodes('//*[local-name()="Feature"]/*[local-name()="ComponentRef" and @Id="CliComponent"]'))
    if ($cliRefs.Count -ne 1) {
        throw "Release identity contract: ProductFeature must install CliComponent exactly once."
    }
    if ($bundleNode.GetAttribute('ProviderKey') -ne $bundleUpgradeCode -or
        $bundleNode.GetAttribute('UpgradeCode') -ne $bundleUpgradeCode) {
        throw "Release identity contract: Burn ProviderKey and UpgradeCode must preserve $bundleUpgradeCode."
    }

    $extensionRoot = Join-Path $installerRoot "wixext\Gnx.DeterministicBundle.wixext"
    $extensionSourcePath = Join-Path $extensionRoot "DeterministicBundleExtension.cs"
    $extensionProjectPath = Join-Path $extensionRoot "Gnx.DeterministicBundle.wixext.csproj"
    $extensionLockPath = Join-Path $extensionRoot "packages.lock.json"
    $extensionSource = Get-Content -LiteralPath $extensionSourcePath -Raw -Encoding utf8
    if ($extensionSource -notmatch "public\s+const\s+string\s+BundleId\s*=\s*`"$([regex]::Escape($releaseBundleId))`"\s*;") {
        throw "Release identity contract: deterministic binder BundleId must remain $releaseBundleId."
    }

    $extensionProject = [xml] (Get-Content -LiteralPath $extensionProjectPath -Raw -Encoding utf8)
    $extensionReference = @($extensionProject.Project.ItemGroup.PackageReference | Where-Object {
        $_.Include -eq 'WixToolset.Extensibility'
    })
    if ($extensionReference.Count -ne 1 -or $extensionReference[0].Version -ne $dependencyLock.wix.version) {
        throw "Release identity contract: deterministic binder must use WixToolset.Extensibility $($dependencyLock.wix.version)."
    }

    $extensionLock = Get-Content -LiteralPath $extensionLockPath -Raw -Encoding utf8 | ConvertFrom-Json
    $extensionLockTarget = $extensionLock.dependencies.PSObject.Properties.Value | Select-Object -First 1
    $lockedExtension = $extensionLockTarget.'WixToolset.Extensibility'
    if (-not $lockedExtension -or $lockedExtension.type -ne 'Direct' -or $lockedExtension.resolved -ne $dependencyLock.wix.version) {
        throw "Release identity contract: deterministic binder package lock must resolve WixToolset.Extensibility $($dependencyLock.wix.version)."
    }

    foreach ($packageId in @('PrepareWsl', 'ValidateHost')) {
        $exePackage = @($bundle.SelectNodes('//*[local-name()="ExePackage"]') | Where-Object {
            $_.GetAttribute('Id') -eq $packageId
        })
        if ($exePackage.Count -ne 1 -or $exePackage[0].GetAttribute('CacheId') -notmatch "-$([regex]::Escape($releaseVersion))$") {
            throw "Release identity contract: ExePackage Id=$packageId must have a CacheId ending in -$releaseVersion."
        }
    }

    foreach ($manifestPath in @(
        'crates\gnx-cli\Cargo.toml',
        'crates\gnx-protocol\Cargo.toml',
        'crates\gnx-service\Cargo.toml',
        'crates\gnx-host-preflight\Cargo.toml'
    )) {
        $manifest = Get-Content -LiteralPath (Join-Path $repoRoot $manifestPath) -Raw -Encoding utf8
        if ($manifest -notmatch "(?m)^version\s*=\s*`"$([regex]::Escape($releaseVersion))`"\s*$") {
            throw "Release identity contract: $manifestPath must use version $releaseVersion."
        }
    }
}

