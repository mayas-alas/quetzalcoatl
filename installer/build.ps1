[CmdletBinding()]
param(
    [string] $RebootContractBundlePath,
    [string] $RebootContractBundleXml,
    [switch] $TestRebootContractOnly
)

$ErrorActionPreference = "Stop"
$installerRoot = $PSScriptRoot
$repoRoot = (Resolve-Path (Join-Path $installerRoot "..")).Path
$cacheRoot = Join-Path $repoRoot "target\installer-cache"
$outputRoot = Join-Path $repoRoot "target\installer"
$lockPath = Join-Path $installerRoot "dependencies.lock.json"
$dotnetToolManifest = Join-Path $repoRoot ".config\dotnet-tools.json"
$releaseVersion = "0.1.14"
$releaseProductCode = "{ACFA43DA-DDE5-501B-A773-C50BED15F59F}"
$releaseUpgradeCode = "{47D5BD44-D061-407B-913B-47D17EC3BEA9}"
$releasePackageCode = "{ACE7E7A7-7411-5444-8DD3-3DBF7F2DCAD2}"
$releaseBundleId = "{C7F7AE72-0CA0-5D2E-96B4-E91C50C294B9}"
$previousProductCode = "{56E3CF39-864C-51F8-BE28-86C9ADE58118}"
$previousPackageCode = "{96520581-4D5C-53CA-80F8-8329F919CA69}"
$previousBundleId = "{8C9449BC-368E-516A-BEEF-CFA0D3C243E7}"
$bundleUpgradeCode = "{10B764B2-36AE-4911-A8C8-2F1A2A963769}"
$releaseTimestamp = [DateTime]::SpecifyKind([DateTime] "2026-07-24T00:00:00", [DateTimeKind]::Utc)
$releaseCabDate = [uint16] (((2026 - 1980) -shl 9) -bor (7 -shl 5) -bor 24)
$releaseCabTime = [uint16] 0
$dependencyLock = Get-Content -LiteralPath $lockPath -Raw -Encoding utf8 | ConvertFrom-Json

if ($dependencyLock.schema_version -ne 1) {
    throw "Unsupported installer dependency lock schema."
}

New-Item -ItemType Directory -Force -Path $cacheRoot, $outputRoot | Out-Null


$moduleRoot = Join-Path $installerRoot "modules"
foreach ($module in @('dependencies.ps1', 'contracts.ps1', 'runtime.ps1', 'rust.ps1', 'msi.ps1', 'bundle.ps1')) {
    . (Join-Path $moduleRoot $module)
}

if (-not $TestRebootContractOnly -and ($RebootContractBundlePath -or $RebootContractBundleXml)) {
    throw "RebootContractBundlePath and RebootContractBundleXml are only permitted with -TestRebootContractOnly."
}

$contractBundlePath = if ($TestRebootContractOnly -and $RebootContractBundlePath) { $RebootContractBundlePath } else { Join-Path $installerRoot "bundle.wxs" }
$contractBundleXml = if ($TestRebootContractOnly) { $RebootContractBundleXml } else { $null }
Test-RebootContract -BundlePath $contractBundlePath -BundleXml $contractBundleXml
if ($TestRebootContractOnly) { return }
Test-ReleaseIdentityContract
Test-RuntimePayloadSource -RuntimePayload (Join-Path $repoRoot 'runtime\payload') -ExpectedPayloadVersion 4

if (-not (Test-Path -LiteralPath $dotnetToolManifest -PathType Leaf)) {
    throw "Pinned .NET tool manifest is absent: $dotnetToolManifest"
}
Unblock-File -LiteralPath $dotnetToolManifest -ErrorAction Stop

$artifacts = @{}
foreach ($artifact in $dependencyLock.artifacts) {
    $artifacts[$artifact.id] = Get-LockedArtifact -Artifact $artifact
}

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

    $deterministicExtensionProject = Join-Path $installerRoot "wixext\Gnx.DeterministicBundle.wixext\Gnx.DeterministicBundle.wixext.csproj"
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

    $hostPreflight = Join-Path $repoRoot "target\release\gnx-host-preflight.exe"
    $gnxService = Join-Path $repoRoot "target\release\gnx-service.exe"
    $gnxCli = Join-Path $repoRoot "target\release\gnx.exe"
    $releaseBinaries = @($hostPreflight, $gnxService, $gnxCli)

    Build-RustReleaseArtifacts `
        -Packages @('gnx-host-preflight', 'gnx-service', 'gnx-cli') `
        -ReleaseBinaries $releaseBinaries

    Test-StaticCrtRustArtifacts -Artifacts @(
        @{ Name = 'gnx-host-preflight'; Path = $hostPreflight },
        @{ Name = 'gnx-service'; Path = $gnxService },
        @{ Name = 'gnx'; Path = $gnxCli }
    )

    $productMsi = Join-Path $outputRoot "Quetzalcoatl.msi"
    $setupExe = Join-Path $outputRoot "QuetzalcoatlSetup.exe"

    & dotnet tool run wix -- build `
        (Join-Path $installerRoot "package.wxs") `
        -arch x64 `
        -d "GnxCli=$gnxCli" `
        -d "GnxService=$gnxService" `
        -d "WinSW=$($artifacts.winsw)" `
        -d "ServiceConfig=$(Join-Path $installerRoot 'Quetzalcoatl.Service.xml')" `
        -d "WinSWLicense=$(Join-Path $installerRoot 'licenses\WinSW.txt')" `
        -d "PodmanMachineImage=$($artifacts.podman_machine)" `
        -d "RuntimePayload=$(Join-Path $repoRoot 'runtime\payload')" `
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
        -RuntimePayload (Join-Path $repoRoot 'runtime\payload')

    & dotnet tool run wix -- build `
        (Join-Path $installerRoot "bundle.wxs") `
        -arch x64 `
        -dcl none `
        -ext $balExtension `
        -ext $deterministicBundleExtension `
        -d "HostPreflight=$hostPreflight" `
        -d "WslMsi=$($artifacts.wsl)" `
        -d "PodmanMsi=$($artifacts.podman)" `
        -d "ProductMsi=$productMsi" `
        -out $setupExe
    if ($LASTEXITCODE -ne 0) { throw "Bundle build failed." }
    Set-BurnDeterministicMetadata -Path $setupExe
    Test-BundleIdentityAndPayload -BundlePath $setupExe -ProductMsiPath $productMsi

    Get-FileHash -Algorithm SHA256 -LiteralPath $productMsi, $setupExe
} finally {
    Pop-Location
}
