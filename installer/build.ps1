[CmdletBinding()]
param(
    [string] $RebootContractBundlePath,
    [string] $RebootContractBundleXml,
    [switch] $TestRebootContractOnly,
    [string] $SigningCertificateThumbprint = $env:GNX_SIGNING_CERTIFICATE_THUMBPRINT,
    [string] $TimestampUrl = 'http://timestamp.digicert.com',
    [switch] $AllowUnsigned,
    [switch] $AllowSelfSigned
)

$ErrorActionPreference = "Stop"
$installerRoot = $PSScriptRoot
$repoRoot = (Resolve-Path (Join-Path $installerRoot "..")).Path
$moduleRoot = Join-Path $installerRoot "modules"
. (Join-Path $moduleRoot 'release.ps1')

$releaseManifestPath = Join-Path $repoRoot 'release\manifest.toml'
$releaseFacts = Read-ReleaseManifest -Path $releaseManifestPath
$releaseVersion = [string] (Get-ReleaseFact $releaseFacts 'version')
$releaseProductCode = [string] (Get-ReleaseFact $releaseFacts 'identities.product_code')
$releaseUpgradeCode = [string] (Get-ReleaseFact $releaseFacts 'identities.upgrade_code')
$releasePackageCode = [string] (Get-ReleaseFact $releaseFacts 'identities.package_code')
$releaseBundleId = [string] (Get-ReleaseFact $releaseFacts 'identities.bundle_id')
$previousProductCode = [string] (Get-ReleaseFact $releaseFacts 'identities.previous_product_code')
$previousPackageCode = [string] (Get-ReleaseFact $releaseFacts 'identities.previous_package_code')
$previousBundleId = [string] (Get-ReleaseFact $releaseFacts 'identities.previous_bundle_id')
$bundleUpgradeCode = [string] (Get-ReleaseFact $releaseFacts 'identities.bundle_upgrade_code')
$runtimePayloadContract = [int] (Get-ReleaseFact $releaseFacts 'contracts.payload_contract')
$releaseTimestamp = [DateTime]::Parse(
    [string] (Get-ReleaseFact $releaseFacts 'release_timestamp_utc'),
    [Globalization.CultureInfo]::InvariantCulture,
    [Globalization.DateTimeStyles]::AssumeUniversal
).ToUniversalTime()
$releaseCabDate = [uint16] ((($releaseTimestamp.Year - 1980) -shl 9) -bor ($releaseTimestamp.Month -shl 5) -bor $releaseTimestamp.Day)
$releaseCabTime = [uint16] (($releaseTimestamp.Hour -shl 11) -bor ($releaseTimestamp.Minute -shl 5) -bor [Math]::Floor($releaseTimestamp.Second / 2))

$cacheRoot = Join-Path $repoRoot "target\installer-cache"
$outputRoot = Join-Path $repoRoot "target\installer"
$lockPath = Join-Path $installerRoot "dependencies.lock.json"
$dotnetToolManifest = Join-Path $repoRoot ".config\dotnet-tools.json"
$dependencyLock = Get-Content -LiteralPath $lockPath -Raw -Encoding utf8 | ConvertFrom-Json
$runtimeLockPath = Join-Path $repoRoot 'runtime\payload.lock.json'
$runtimeLock = Get-Content -LiteralPath $runtimeLockPath -Raw -Encoding utf8 | ConvertFrom-Json
if ($dependencyLock.schema_version -ne 1) {
    throw "Unsupported installer dependency lock schema."
}
New-Item -ItemType Directory -Force -Path $cacheRoot, $outputRoot | Out-Null

foreach ($module in @('dependencies.ps1', 'contracts.ps1', 'runtime.ps1', 'rust.ps1', 'msi.ps1', 'bundle.ps1', 'signing.ps1')) {
    . (Join-Path $moduleRoot $module)
}

if (-not $TestRebootContractOnly -and ($RebootContractBundlePath -or $RebootContractBundleXml)) {
    throw "RebootContractBundlePath and RebootContractBundleXml are only permitted with -TestRebootContractOnly."
}

$contractBundlePath = if ($TestRebootContractOnly -and $RebootContractBundlePath) { $RebootContractBundlePath } else { Join-Path $installerRoot "source\bundle.wxs" }
$contractBundleXml = if ($TestRebootContractOnly) { $RebootContractBundleXml } else { $null }
Test-RebootContract -BundlePath $contractBundlePath -BundleXml $contractBundleXml
if ($TestRebootContractOnly) { return }
Test-ReleaseIdentityContract
Test-DependencyStagingContract
Test-MaintenanceContract
Test-RuntimePayloadSource -RuntimePayload (Join-Path $repoRoot 'runtime') -ExpectedPayloadVersion $runtimePayloadContract
$signingIdentity = $null
if ($AllowUnsigned -and $AllowSelfSigned) {
    throw 'AllowUnsigned and AllowSelfSigned are mutually exclusive.'
}
if ([string]::IsNullOrWhiteSpace($SigningCertificateThumbprint)) {
    if ($AllowSelfSigned) {
        throw 'AllowSelfSigned requires an explicit SigningCertificateThumbprint.'
    }
    if (-not $AllowUnsigned) {
        throw 'Production release requires -SigningCertificateThumbprint or GNX_SIGNING_CERTIFICATE_THUMBPRINT.'
    }
    Write-Warning 'Building an unsigned development artifact; it is not releasable.'
} else {
    if ($AllowUnsigned) {
        throw 'AllowUnsigned must not be combined with a signing certificate.'
    }
    $signingIdentity = Resolve-CodeSigningCertificate -Thumbprint $SigningCertificateThumbprint
    if ($signingIdentity.SelfSigned) {
        if (-not $AllowSelfSigned) {
            throw 'Production release rejects self-signed certificates. Use -AllowSelfSigned only for local QA.'
        }
        Write-Warning 'Building a self-signed development artifact; it is trusted only by explicitly configured test machines and is not releasable.'
    } elseif ($AllowSelfSigned) {
        throw 'AllowSelfSigned is valid only for a self-signed development certificate.'
    }
}

if (-not (Test-Path -LiteralPath $dotnetToolManifest -PathType Leaf)) {
    throw "Pinned .NET tool manifest is absent: $dotnetToolManifest"
}
Unblock-File -LiteralPath $dotnetToolManifest -ErrorAction Stop

$artifacts = @{}
foreach ($artifact in $dependencyLock.artifacts) {
    $artifacts[$artifact.id] = Get-LockedArtifact -Artifact $artifact
}
$machineComponent = @($runtimeLock.components | Where-Object { $_.id -eq 'podman-machine-os' })
if ($machineComponent.Count -ne 1) {
    throw "Runtime lock must contain exactly one podman-machine-os component."
}
$machineArtifact = [pscustomobject] @{
    id = 'podman_machine'
    version = $machineComponent[0].version
    file_name = $machineComponent[0].artifact
    size = $machineComponent[0].artifact_size
    sha256 = ([string] $machineComponent[0].layer_digest).Replace('sha256:', '').ToUpperInvariant()
    url = $machineComponent[0].artifact_url
    authenticode = [pscustomobject] @{
        status = 'not_applicable'
        reason = 'Compressed Linux machine image; SHA-256 layer digest is authoritative.'
    }
}
$artifacts.podman_machine = Get-LockedArtifact -Artifact $machineArtifact

$runtimePackage = Join-Path $outputRoot 'runtime-payload'
if (Test-Path -LiteralPath $runtimePackage) {
    Remove-Item -LiteralPath $runtimePackage -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $runtimePackage | Out-Null
Copy-Item -LiteralPath $runtimeLockPath -Destination $runtimePackage
foreach ($directory in @('commands', 'configuration', 'containers', 'services')) {
    Copy-Item -LiteralPath (Join-Path $repoRoot "runtime\$directory") -Destination $runtimePackage -Recurse
}

$previousBundleIdEnvironment = $env:GNX_RELEASE_BUNDLE_ID
$env:GNX_RELEASE_BUNDLE_ID = $releaseBundleId
Push-Location $repoRoot
try {
    & dotnet tool restore --tool-manifest $dotnetToolManifest
    if ($LASTEXITCODE -ne 0) { throw "WiX tool restore failed." }

    $wixVersion = (& dotnet tool run wix -- --version).Trim()
    $expectedWix = [regex]::Escape($dependencyLock.wix.version)
    if ($LASTEXITCODE -ne 0 -or $wixVersion -notmatch "^$expectedWix(?:\+[0-9A-Za-z.-]+)?$") {
        throw "Expected WiX $($dependencyLock.wix.version), received '$wixVersion'."
    }

    & dotnet tool run wix -- extension add "WixToolset.Bal.wixext/$($dependencyLock.wix.version)"
    if ($LASTEXITCODE -ne 0) { throw "WiX Bal extension restore failed." }
    $balExtension = Join-Path $repoRoot ".wix\extensions\WixToolset.Bal.wixext\$($dependencyLock.wix.version)\wixext5\WixToolset.BootstrapperApplications.wixext.dll"
    if (-not (Test-Path -LiteralPath $balExtension)) {
        throw "Pinned WiX Bal extension DLL is absent: $balExtension"
    }
    & dotnet tool run wix -- extension add "WixToolset.Util.wixext/$($dependencyLock.wix.version)"
    if ($LASTEXITCODE -ne 0) { throw "WiX Util extension restore failed." }
    $utilExtension = Join-Path $repoRoot ".wix\extensions\WixToolset.Util.wixext\$($dependencyLock.wix.version)\wixext5\WixToolset.Util.wixext.dll"
    if (-not (Test-Path -LiteralPath $utilExtension)) {
        throw "Pinned WiX Util extension DLL is absent: $utilExtension"
    }

    $deterministicExtensionProject = Join-Path $installerRoot "extensions\deterministic-bundle\DeterministicBundle.wixext.csproj"
    $deterministicExtensionOutput = Join-Path $repoRoot "target\wixext\bin"
    $deterministicExtensionIntermediate = Join-Path $repoRoot "target\wixext\obj"
    & dotnet restore $deterministicExtensionProject `
        --locked-mode `
        --nologo `
        "-p:BaseIntermediateOutputPath=$deterministicExtensionIntermediate\"
    if ($LASTEXITCODE -ne 0) { throw "Deterministic Burn extension restore failed." }

    & dotnet build $deterministicExtensionProject `
        --configuration Release `
        --no-restore `
        --nologo `
        "-p:BaseOutputPath=$deterministicExtensionOutput\" `
        "-p:BaseIntermediateOutputPath=$deterministicExtensionIntermediate\"
    if ($LASTEXITCODE -ne 0) { throw "Deterministic Burn extension build failed." }
    $deterministicBundleExtension = Join-Path $deterministicExtensionOutput "Release\netstandard2.0\Gnx.DeterministicBundle.wixext.dll"
    if (-not (Test-Path -LiteralPath $deterministicBundleExtension)) {
        throw "Deterministic Burn extension DLL is absent: $deterministicBundleExtension"
    }

    $gnxBootstrap = Join-Path $repoRoot "target\release\gnx-bootstrap.exe"
    $gnxService = Join-Path $repoRoot "target\release\gnx-service.exe"
    $gnxCli = Join-Path $repoRoot "target\release\gnx.exe"
    $gnxTray = Join-Path $repoRoot "target\release\gnx-tray.exe"
    $releaseBinaries = @($gnxBootstrap, $gnxService, $gnxCli, $gnxTray)

    Build-RustReleaseArtifacts `
        -Packages @('gnx-bootstrap', 'gnx-service', 'gnx') `
        -ReleaseBinaries $releaseBinaries

    Test-StaticCrtRustArtifacts -Artifacts @(
        @{ Name = 'gnx-bootstrap'; Path = $gnxBootstrap },
        @{ Name = 'gnx-service'; Path = $gnxService },
        @{ Name = 'gnx'; Path = $gnxCli },
        @{ Name = 'gnx-tray'; Path = $gnxTray }
    )
    if ($signingIdentity) {
        foreach ($releaseBinary in $releaseBinaries) {
            Invoke-AuthenticodeSign `
                -Path $releaseBinary `
                -SigningIdentity $signingIdentity `
                -TimestampUrl $TimestampUrl
        }
    }
    $generatedInputs = @($releaseBinaries) + @(
        Get-ChildItem -LiteralPath $runtimePackage -Recurse -File |
            ForEach-Object FullName
    )
    foreach ($generatedInput in $generatedInputs) {
        [IO.File]::SetLastWriteTimeUtc([string] $generatedInput, $releaseTimestamp)
    }

    $productMsi = Join-Path $outputRoot "Quetzalcoatl.msi"
    $setupExe = Join-Path $outputRoot "QuetzalcoatlSetup.exe"

    & dotnet tool run wix -- build `
        (Join-Path $installerRoot "source\product.wxs") `
        -arch x64 `
        -ext $utilExtension `
        -d "ProductVersion=$releaseVersion" `
        -d "ProductCode=$releaseProductCode" `
        -d "UpgradeCode=$releaseUpgradeCode" `
        -d "GnxCli=$gnxCli" `
        -d "GnxTray=$gnxTray" `
        -d "GnxService=$gnxService" `
        -d "BrandIcon=$(Join-Path $installerRoot 'assets\branding\icon.ico')" `
        -d "WinSW=$($artifacts.winsw)" `
        -d "ServiceConfig=$(Join-Path $installerRoot 'source\Quetzalcoatl.Service.xml')" `
        -d "WinSWLicense=$(Join-Path $installerRoot 'assets\licenses\WinSW.txt')" `
        -d "WiXLicense=$(Join-Path $installerRoot 'assets\licenses\WiX.txt')" `
        -d "ProductLicense=$(Join-Path $repoRoot 'LICENSE')" `
        -d "ProductNotice=$(Join-Path $repoRoot 'NOTICE')" `
        -d "ThirdPartyNotices=$(Join-Path $repoRoot 'THIRD_PARTY_NOTICES.md')" `
        -d "PodmanMachineImage=$($artifacts.podman_machine)" `
        -d "PodmanMachineImageName=$($machineArtifact.file_name)" `
        -d "RuntimePayload=$runtimePackage" `
        -out $productMsi
    if ($LASTEXITCODE -ne 0) { throw "MSI build failed." }
    Set-MsiDeterministicMetadata -Path $productMsi

    $actualProductVersion = Get-MsiProperty -Path $productMsi -Name 'ProductVersion'
    $actualProductCode = Get-MsiProperty -Path $productMsi -Name 'ProductCode'
    $actualUpgradeCode = Get-MsiProperty -Path $productMsi -Name 'UpgradeCode'
    $actualPackageCode = Get-MsiSummaryProperty -Path $productMsi -PropertyId 9
    if ($actualProductVersion -ne $releaseVersion -or
        $actualProductCode -ne $releaseProductCode -or
        $actualUpgradeCode -ne $releaseUpgradeCode -or
        $actualPackageCode -ne $releasePackageCode) {
        throw "Built MSI identity mismatch: version=$actualProductVersion ProductCode=$actualProductCode UpgradeCode=$actualUpgradeCode PackageCode=$actualPackageCode"
    }
    Test-MsiPayloadCoherence `
        -MsiPath $productMsi `
        -ServiceBinary $gnxService `
        -CliBinary $gnxCli `
        -TrayBinary $gnxTray `
        -MachineImage $artifacts.podman_machine `
        -RuntimePayload $runtimePackage
    if ($signingIdentity) {
        Invoke-AuthenticodeSign `
            -Path $productMsi `
            -SigningIdentity $signingIdentity `
            -TimestampUrl $TimestampUrl
    }
    Test-InstalledMsiIdentity -MsiPath $productMsi -ProductCode $releaseProductCode

    & dotnet tool run wix -- build `
        (Join-Path $installerRoot "source\bundle.wxs") `
        -arch x64 `
        -dcl none `
        -ext $balExtension `
        -ext $deterministicBundleExtension `
        -d "ProductVersion=$releaseVersion" `
        -d "BundleUpgradeCode=$bundleUpgradeCode" `
        -d "GnxBootstrap=$gnxBootstrap" `
        -d "WslMsi=$($artifacts.wsl)" `
        -d "PodmanMsi=$($artifacts.podman)" `
        -d "ProductMsi=$productMsi" `
        -d "BrandIcon=$(Join-Path $installerRoot 'assets\branding\icon.ico')" `
        -d "BrandLogo=$(Join-Path $installerRoot 'assets\wixstdba-logo.png')" `
        -d "BrandSide=$(Join-Path $installerRoot 'assets\wixstdba-side.png')" `
        -d "BrandTheme=$(Join-Path $installerRoot 'assets\wixstdba-theme.xml')" `
        -out $setupExe
    if ($LASTEXITCODE -ne 0) { throw "Bundle build failed." }
    Set-BurnDeterministicMetadata -Path $setupExe
    Test-BundleIdentityAndPayload `
        -BundlePath $setupExe `
        -ProductMsiPath $productMsi `
        -WslMsiPath $artifacts.wsl `
        -PodmanMsiPath $artifacts.podman
    if ($signingIdentity) {
        Invoke-BurnAuthenticodeSign `
            -BundlePath $setupExe `
            -SigningIdentity $signingIdentity `
            -TimestampUrl $TimestampUrl `
            -WorkingDirectory $outputRoot
        Test-BundleIdentityAndPayload `
            -BundlePath $setupExe `
            -ProductMsiPath $productMsi `
            -WslMsiPath $artifacts.wsl `
            -PodmanMsiPath $artifacts.podman
    }

    Get-FileHash -Algorithm SHA256 -LiteralPath $productMsi, $setupExe
 } finally {
    Pop-Location
    $env:GNX_RELEASE_BUNDLE_ID = $previousBundleIdEnvironment
}
