function Test-RebootContract {
    param(
        [string] $BundlePath = (Join-Path $installerRoot "source\bundle.wxs"),
        [string] $BundleXml
    )

    $exitCodesPath = Join-Path $repoRoot "apps\gnx-bootstrap\src\exit_codes.rs"
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
        PrepareQaTrust = @{
            '0' = 'success'
        }
        PrepareWsl = @{
            '0' = 'success'
            $rebootPending = 'forceReboot'
            $rebootRequired = 'forceReboot'
        }
        InstallWsl = @{
            '0' = 'success'
            '1641' = 'forceReboot'
            $rebootRequired = 'forceReboot'
        }
        InstallPodman = @{
            '0' = 'success'
            '1641' = 'forceReboot'
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
    $packagePath = Join-Path $installerRoot "source\product.wxs"
    $bundlePath = Join-Path $installerRoot "source\bundle.wxs"
    $package = [xml] (Get-Content -LiteralPath $packagePath -Raw -Encoding utf8)
    $bundle = [xml] (Get-Content -LiteralPath $bundlePath -Raw -Encoding utf8)
    $packageNode = $package.SelectSingleNode('/*[local-name()="Wix"]/*[local-name()="Package"]')
    $bundleNode = $bundle.SelectSingleNode('/*[local-name()="Wix"]/*[local-name()="Bundle"]')

    if (-not $packageNode -or -not $bundleNode) {
        throw "Release identity contract: package or bundle root is missing."
    }
    if ($packageNode.GetAttribute('Version') -ne '$(var.ProductVersion)' -or
        $bundleNode.GetAttribute('Version') -ne '$(var.ProductVersion)') {
        throw "Release identity contract: package and bundle must both consume ProductVersion."
    }
    if ($packageNode.GetAttribute('ProductCode') -ne '$(var.ProductCode)') {
        throw "Release identity contract: package must consume the release ProductCode."
    }
    if ($packageNode.GetAttribute('UpgradeCode') -ne '$(var.UpgradeCode)') {
        throw "Release identity contract: package must consume the stable UpgradeCode."
    }
    if ($releaseProductCode -eq $previousProductCode -or
        $releasePackageCode -eq $previousPackageCode -or
        $releaseBundleId -eq $previousBundleId) {
        throw "Release identity contract: $releaseVersion must not reuse the previous release identity."
    }
    if (-not (Test-MsiIdentityCollision `
        -CandidatePackageCode '{00000000-0000-0000-0000-000000000001}' `
        -CandidateHash ('A' * 64) `
        -InstalledPackageCode '{00000000-0000-0000-0000-000000000001}' `
        -InstalledHash ('B' * 64)) -or
        (Test-MsiIdentityCollision `
            -CandidatePackageCode '{00000000-0000-0000-0000-000000000001}' `
            -CandidateHash ('A' * 64) `
            -InstalledPackageCode '{00000000-0000-0000-0000-000000000001}' `
            -InstalledHash ('A' * 64))) {
        throw "Release identity contract: MSI identity collision detection is not fail-closed."
    }
    $majorUpgrade = @($packageNode.SelectNodes('./*[local-name()="MajorUpgrade"]'))
    if ($majorUpgrade.Count -ne 1 -or
        $majorUpgrade[0].GetAttribute('Schedule') -ne 'afterInstallInitialize' -or
        $majorUpgrade[0].GetAttribute('AllowSameVersionUpgrades') -ne 'yes') {
        throw "Release identity contract: MSI must preserve rollback-safe same-version major upgrades."
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
    $groupedServiceFiles = @($package.SelectNodes('//*[local-name()="Component" and @Id="ServiceComponent"]/*[local-name()="File" and @Id="GnxService"]'))
    if ($groupedServiceFiles.Count -ne 0) {
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
    if ($bundleNode.GetAttribute('ProviderKey') -ne '$(var.BundleUpgradeCode)' -or
        $bundleNode.GetAttribute('UpgradeCode') -ne '$(var.BundleUpgradeCode)') {
        throw "Release identity contract: Burn ProviderKey and UpgradeCode must consume the stable bundle upgrade code."
    }

    $extensionRoot = Join-Path $installerRoot "extensions\deterministic-bundle"
    $extensionSourcePath = Join-Path $extensionRoot "DeterministicBundleExtension.cs"
    $extensionProjectPath = Join-Path $extensionRoot "DeterministicBundle.wixext.csproj"
    $extensionLockPath = Join-Path $extensionRoot "packages.lock.json"
    $extensionSource = Get-Content -LiteralPath $extensionSourcePath -Raw -Encoding utf8
    if ($extensionSource -notmatch 'Environment\.GetEnvironmentVariable\("GNX_RELEASE_BUNDLE_ID"\)' -or
        $extensionSource -notmatch 'bundle\.BundleId\s*=\s*BundleId\s*;') {
        throw "Release identity contract: deterministic binder must consume the release BundleId from GNX_RELEASE_BUNDLE_ID."
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

    foreach ($packageId in @('PrepareQaTrust', 'PrepareWsl', 'InstallWsl', 'InstallPodman', 'ValidateHost')) {
        $exePackage = @($bundle.SelectNodes('//*[local-name()="ExePackage"]') | Where-Object {
            $_.GetAttribute('Id') -eq $packageId
        })
        if ($exePackage.Count -ne 1 -or $exePackage[0].GetAttribute('CacheId') -notmatch '-\$\(var\.ProductVersion\)$') {
            throw "Release identity contract: ExePackage Id=$packageId must bind CacheId to ProductVersion."
        }
    }

    $workspaceManifest = Get-Content -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -Raw -Encoding utf8
    if ($workspaceManifest -notmatch "(?ms)\[workspace\.package\].*?^version\s*=\s*`"$([regex]::Escape($releaseVersion))`"\s*$") {
        throw "Release identity contract: workspace package version must be $releaseVersion."
    }
    foreach ($manifestPath in @(
        'apps\gnx\Cargo.toml',
        'apps\gnx-service\Cargo.toml',
        'apps\gnx-bootstrap\Cargo.toml',
        'crates\gnx-contracts\Cargo.toml'
    )) {
        $manifest = Get-Content -LiteralPath (Join-Path $repoRoot $manifestPath) -Raw -Encoding utf8
        if ($manifest -notmatch '(?m)^version\.workspace\s*=\s*true\s*$') {
            throw "Release identity contract: $manifestPath must inherit the workspace version."
        }
    }
}


function Test-DependencyStagingContract {
    $bundlePath = Join-Path $installerRoot "source\bundle.wxs"
    $bundle = [xml] (Get-Content -LiteralPath $bundlePath -Raw -Encoding utf8)
    $dependencySourcePath = Join-Path $repoRoot "apps\gnx-bootstrap\src\dependencies\mod.rs"
    $dependencySource = Get-Content -LiteralPath $dependencySourcePath -Raw -Encoding utf8
    $normalizedDependencySource = $dependencySource -replace '_', ''

    $contracts = @(
        @{
            ArtifactId = 'wsl'
            PackageId = 'InstallWsl'
            PayloadId = 'WslMsiPayload'
            SourceVariable = '$(var.WslMsi)'
            InstallArguments = 'install-wsl --operation install --format json'
            RepairArguments = 'install-wsl --operation repair --format json'
        },
        @{
            ArtifactId = 'podman'
            PackageId = 'InstallPodman'
            PayloadId = 'PodmanMsiPayload'
            SourceVariable = '$(var.PodmanMsi)'
            InstallArguments = 'install-podman --operation install --format json'
            RepairArguments = 'install-podman --operation repair --format json'
        }
    )

    foreach ($contract in $contracts) {
        $artifact = @($dependencyLock.artifacts | Where-Object { $_.id -eq $contract.ArtifactId })
        if ($artifact.Count -ne 1) {
            throw "Dependency staging contract: lock entry $($contract.ArtifactId) is not unique."
        }
        $artifact = $artifact[0]
        foreach ($value in @(
            [string] $artifact.version,
            [string] $artifact.file_name,
            [string] $artifact.sha256,
            [string] $artifact.size
        )) {
            if ($normalizedDependencySource -notlike "*$value*") {
                throw "Dependency staging contract: helper constants differ for $($contract.ArtifactId): $value"
            }
        }

        $legacyMsi = @($bundle.SelectNodes('//*[local-name()="MsiPackage"]') | Where-Object {
            $_.GetAttribute('Id') -in @('Wsl', 'Podman')
        })
        if ($legacyMsi.Count -ne 0) {
            throw "Dependency staging contract: WSL and Podman must not execute directly as Burn MsiPackage entries."
        }

        $packages = @($bundle.SelectNodes('//*[local-name()="ExePackage"]') | Where-Object {
            $_.GetAttribute('Id') -eq $contract.PackageId
        })
        if ($packages.Count -ne 1) {
            throw "Dependency staging contract: expected exactly one $($contract.PackageId) helper."
        }
        $package = $packages[0]
        if ($package.GetAttribute('SourceFile') -ne '$(var.GnxBootstrap)' -or
            $package.GetAttribute('InstallArguments') -ne $contract.InstallArguments -or
            $package.GetAttribute('RepairArguments') -ne $contract.RepairArguments -or
            $package.GetAttribute('RepairCondition') -ne '1' -or
            $package.GetAttribute('PerMachine') -ne 'yes' -or
            $package.GetAttribute('Vital') -ne 'yes') {
            throw "Dependency staging contract: $($contract.PackageId) execution policy differs."
        }
        $payloads = @($package.SelectNodes('./*[local-name()="Payload"]') | Where-Object {
            $_.GetAttribute('Id') -eq $contract.PayloadId
        })
        if ($payloads.Count -ne 1 -or
            $payloads[0].GetAttribute('SourceFile') -ne $contract.SourceVariable -or
            $payloads[0].GetAttribute('Name') -ne $artifact.file_name -or
            $payloads[0].GetAttribute('Compressed') -ne 'yes') {
            throw "Dependency staging contract: $($contract.PackageId) payload differs from the lock."
        }
    }
}

function Test-MaintenanceContract {
    $bundlePath = Join-Path $installerRoot "source\bundle.wxs"
    $packagePath = Join-Path $installerRoot "source\product.wxs"
    $themePath = Join-Path $installerRoot "assets\wixstdba-theme.xml"
    $serviceConfigPath = Join-Path $installerRoot "source\Quetzalcoatl.Service.xml"
    $bundle = [xml] (Get-Content -LiteralPath $bundlePath -Raw -Encoding utf8)
    $package = [xml] (Get-Content -LiteralPath $packagePath -Raw -Encoding utf8)
    $theme = [xml] (Get-Content -LiteralPath $themePath -Raw -Encoding utf8)
    $serviceConfig = [xml] (Get-Content -LiteralPath $serviceConfigPath -Raw -Encoding utf8)

    $expected = @{
        PrepareQaTrust = @{
            Install = 'prepare-qa-trust --root-certificate "[WixBundleExecutePackageCacheFolder]\gnx-qa-root.cer" --root-sha256 $(var.QaRootSha256) --publisher-certificate "[WixBundleExecutePackageCacheFolder]\gnx-qa-publisher.cer" --publisher-sha256 $(var.QaPublisherSha256) --operation install --format json'
            Repair = 'prepare-qa-trust --root-certificate "[WixBundleExecutePackageCacheFolder]\gnx-qa-root.cer" --root-sha256 $(var.QaRootSha256) --publisher-certificate "[WixBundleExecutePackageCacheFolder]\gnx-qa-publisher.cer" --publisher-sha256 $(var.QaPublisherSha256) --operation repair --format json'
        }
        PrepareWsl = @{
            Install = 'prepare-wsl --operation install --format json'
            Repair = 'prepare-wsl --operation repair --format json'
        }
        InstallWsl = @{
            Install = 'install-wsl --operation install --format json'
            Repair = 'install-wsl --operation repair --format json'
        }
        InstallPodman = @{
            Install = 'install-podman --operation install --format json'
            Repair = 'install-podman --operation repair --format json'
        }
        ValidateHost = @{
            Install = '--operation install --format json'
            Repair = '--operation repair --format json'
        }
    }

    foreach ($packageId in $expected.Keys) {
        $nodes = @($bundle.SelectNodes('//*[local-name()="ExePackage"]') | Where-Object {
            $_.GetAttribute('Id') -eq $packageId
        })
        if ($nodes.Count -ne 1) {
            throw "Maintenance contract: expected exactly one ExePackage Id=$packageId."
        }
        $node = $nodes[0]
        if ($node.GetAttribute('InstallArguments') -ne $expected[$packageId].Install -or
            $node.GetAttribute('RepairArguments') -ne $expected[$packageId].Repair -or
            $node.GetAttribute('RepairCondition') -ne '1') {
            throw "Maintenance contract: $packageId must expose closed install and repair operations."
        }
    }

    $msiPackages = @($bundle.SelectNodes('//*[local-name()="MsiPackage" and @Id="QuetzalcoatlProduct"]'))
    if ($msiPackages.Count -ne 1) {
        throw "Maintenance contract: the product MSI must remain in the bundle chain."
    }
    if ($msiPackages[0].GetAttribute('Visible') -ne 'no') {
        throw "Maintenance contract: Setup must be the sole Programs and Features entry."
    }
    $repairButtons = @($theme.SelectNodes('//*[local-name()="Button" and @Name="RepairButton"]'))
    if ($repairButtons.Count -ne 1) {
        throw "Maintenance contract: the bootstrapper theme must expose exactly one repair action."
    }
    $majorUpgrade = @($package.SelectNodes('//*[local-name()="MajorUpgrade"]'))
    if ($majorUpgrade.Count -ne 1 -or
        $majorUpgrade[0].GetAttribute('Schedule') -ne 'afterInstallInitialize') {
        throw "Maintenance contract: MSI upgrade scheduling must preserve rollback-safe replacement."
    }
    $serviceControls = @($package.SelectNodes('//*[local-name()="ServiceControl" and @Id="QuetzalcoatlServiceControl"]'))
    if ($serviceControls.Count -ne 1 -or
        $serviceControls[0].GetAttribute('Start') -ne 'install' -or
        $serviceControls[0].GetAttribute('Stop') -ne 'both' -or
        $serviceControls[0].GetAttribute('Remove') -ne 'uninstall' -or
        $serviceControls[0].GetAttribute('Wait') -ne 'yes') {
        throw "Maintenance contract: upgrade and repair must stop and restart the service deterministically."
    }
    if ($serviceConfig.service.stopexecutable -ne '%BASE%\gnx-service.exe' -or
        $serviceConfig.service.stoparguments -ne '--stop-managed-machine' -or
        $null -eq $serviceConfig.service.startarguments) {
        throw "Maintenance contract: service stop must preserve but release the managed Podman machine."
    }
    $closeApplications = @($package.SelectNodes('//*[local-name()="CloseApplication" and @Id="CloseQuetzalcoatlTray"]'))
    if ($closeApplications.Count -ne 1 -or
        $closeApplications[0].GetAttribute('Target') -ne 'gnx-tray.exe' -or
        $closeApplications[0].GetAttribute('CloseMessage') -ne 'yes' -or
        $closeApplications[0].GetAttribute('ElevatedCloseMessage') -ne 'yes' -or
        $closeApplications[0].GetAttribute('TerminateProcess') -ne '0' -or
        $closeApplications[0].GetAttribute('RebootPrompt') -ne 'no') {
        throw "Maintenance contract: uninstall must close and, when required, terminate the tray without a reboot prompt."
    }
    $closeApplicationSequence = @($package.SelectNodes('//*[local-name()="InstallExecuteSequence"]/*[local-name()="Custom" and @Action="override Wix4CloseApplications_X64"]'))
    if ($closeApplicationSequence.Count -ne 1 -or
        $closeApplicationSequence[0].GetAttribute('Before') -ne 'RemoveFiles') {
        throw "Maintenance contract: elevated tray shutdown must precede product file and directory removal."
    }
    $trayLaunchers = @($package.SelectNodes('//*[local-name()="CustomAction" and @Id="LaunchQuetzalcoatlTray"]'))
    if ($trayLaunchers.Count -ne 1 -or
        $trayLaunchers[0].GetAttribute('FileRef') -ne 'GnxTray' -or
        $trayLaunchers[0].GetAttribute('ExeCommand') -ne '--launch-detached' -or
        $trayLaunchers[0].GetAttribute('Execute') -ne 'immediate' -or
        $trayLaunchers[0].GetAttribute('Impersonate') -ne 'yes' -or
        $trayLaunchers[0].GetAttribute('Return') -ne 'ignore' -or
        $trayLaunchers[0].HasAttribute('Directory')) {
        throw "Maintenance contract: tray launch must be detached and non-vital to installation."
    }
    $payloadValidators = @($package.SelectNodes('//*[local-name()="CustomAction" and @Id="ValidateInstalledPayload"]'))
    if ($payloadValidators.Count -ne 1 -or
        $payloadValidators[0].GetAttribute('FileRef') -ne 'GnxService' -or
        $payloadValidators[0].GetAttribute('ExeCommand') -ne '--validate-installation' -or
        $payloadValidators[0].GetAttribute('Execute') -ne 'deferred' -or
        $payloadValidators[0].GetAttribute('Impersonate') -ne 'no' -or
        $payloadValidators[0].GetAttribute('Return') -ne 'check') {
        throw "Maintenance contract: installed payload validation must be a checked elevated gnx-service operation."
    }
    $payloadValidatorSequence = @($package.SelectNodes('//*[local-name()="InstallExecuteSequence"]/*[local-name()="Custom" and @Action="ValidateInstalledPayload"]'))
    if ($payloadValidatorSequence.Count -ne 1 -or
        $payloadValidatorSequence[0].GetAttribute('After') -ne 'InstallFiles' -or
        $payloadValidatorSequence[0].GetAttribute('Condition') -ne 'NOT (REMOVE = "ALL")') {
        throw "Maintenance contract: installed payload validation must run after files and before service start."
    }
}
