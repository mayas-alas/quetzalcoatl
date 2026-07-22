[CmdletBinding()]
param(
    [string] $RebootContractBundlePath,
    [string] $RebootContractBundleXml,
    [switch] $TestRebootContractOnly
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$cacheRoot = Join-Path $repoRoot "target\installer-cache"
$outputRoot = Join-Path $repoRoot "target\installer"
$lockPath = Join-Path $PSScriptRoot "dependencies.lock.json"
$releaseVersion = "0.1.3"
$releaseProductCode = "{2A1C371C-EDE5-48DE-A297-1EE70F18CD1C}"
$releaseUpgradeCode = "{47D5BD44-D061-407B-913B-47D17EC3BEA9}"
$dependencyLock = Get-Content -LiteralPath $lockPath -Raw -Encoding utf8 | ConvertFrom-Json

if ($dependencyLock.schema_version -ne 1) {
    throw "Unsupported installer dependency lock schema."
}

New-Item -ItemType Directory -Force -Path $cacheRoot, $outputRoot | Out-Null

function Get-LockedArtifact {
    param([Parameter(Mandatory)] $Artifact)

    $destination = Join-Path $cacheRoot $Artifact.file_name
    if (-not (Test-Path -LiteralPath $destination)) {
        $partial = "$destination.download"
        Remove-Item -LiteralPath $partial -ErrorAction SilentlyContinue
        & curl.exe --fail --location --retry 3 --output $partial $Artifact.url
        if ($LASTEXITCODE -ne 0) {
            throw "Download failed for $($Artifact.id)."
        }
        Move-Item -LiteralPath $partial -Destination $destination
    }

    $file = Get-Item -LiteralPath $destination
    $hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
    if ($file.Length -ne $Artifact.size -or $hash -ne $Artifact.sha256) {
        throw "Locked artifact mismatch for $($Artifact.id): $destination"
    }
    return $destination
}

function Get-PeImportedDllNames {
    param([Parameter(Mandatory)][string] $Path)

    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $peOffset = [BitConverter]::ToInt32($bytes, 0x3c)
    if ([BitConverter]::ToUInt32($bytes, $peOffset) -ne 0x00004550) {
        throw "Not a PE file: $Path"
    }

    $sectionCount = [BitConverter]::ToUInt16($bytes, $peOffset + 6)
    $optionalOffset = $peOffset + 24
    $magic = [BitConverter]::ToUInt16($bytes, $optionalOffset)
    if ($magic -ne 0x20b) {
        throw "Expected a PE32+ executable: $Path"
    }

    $importRva = [BitConverter]::ToUInt32($bytes, $optionalOffset + 112 + 8)
    if ($importRva -eq 0) { return @() }
    $optionalSize = [BitConverter]::ToUInt16($bytes, $peOffset + 20)
    $sectionOffset = $optionalOffset + $optionalSize

    function Convert-RvaToOffset([uint32] $Rva) {
        for ($index = 0; $index -lt $sectionCount; $index++) {
            $offset = $sectionOffset + (40 * $index)
            $virtualSize = [BitConverter]::ToUInt32($bytes, $offset + 8)
            $virtualAddress = [BitConverter]::ToUInt32($bytes, $offset + 12)
            $rawSize = [BitConverter]::ToUInt32($bytes, $offset + 16)
            if ($Rva -ge $virtualAddress -and $Rva -lt ($virtualAddress + [Math]::Max($virtualSize, $rawSize))) {
                return [int] ($Rva - $virtualAddress + [BitConverter]::ToUInt32($bytes, $offset + 20))
            }
        }
        throw "PE RVA 0x{0:X8} is outside all sections: $Path" -f $Rva
    }

    $imports = [System.Collections.Generic.List[string]]::new()
    for ($offset = Convert-RvaToOffset $importRva; [BitConverter]::ToUInt32($bytes, $offset + 12) -ne 0; $offset += 20) {
        $nameOffset = Convert-RvaToOffset ([BitConverter]::ToUInt32($bytes, $offset + 12))
        $end = $nameOffset
        while ($bytes[$end] -ne 0) { $end++ }
        $imports.Add([System.Text.Encoding]::ASCII.GetString($bytes, $nameOffset, $end - $nameOffset))
    }
    return $imports | Sort-Object -Unique
}

function Test-RebootContract {
    param(
        [string] $BundlePath = (Join-Path $PSScriptRoot "bundle.wxs"),
        [string] $BundleXml
    )

    $exitCodesPath = Join-Path $repoRoot "crates\host-preflight\src\exit_codes.rs"
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
    $packagePath = Join-Path $PSScriptRoot "package.wxs"
    $bundlePath = Join-Path $PSScriptRoot "bundle.wxs"
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
        throw "Release identity contract: package ProductCode must be the explicit 0.1.3 identity $releaseProductCode."
    }
    if ($packageNode.GetAttribute('UpgradeCode') -ne $releaseUpgradeCode) {
        throw "Release identity contract: package UpgradeCode must remain $releaseUpgradeCode."
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
        'crates\host-preflight\Cargo.toml'
    )) {
        $manifest = Get-Content -LiteralPath (Join-Path $repoRoot $manifestPath) -Raw -Encoding utf8
        if ($manifest -notmatch "(?m)^version\s*=\s*`"$([regex]::Escape($releaseVersion))`"\s*$") {
            throw "Release identity contract: $manifestPath must use version $releaseVersion."
        }
    }
}

function Get-MsiProperty {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $Name
    )

    $installer = New-Object -ComObject WindowsInstaller.Installer
    $database = $installer.OpenDatabase($Path, 0)
    $query = [string]::Format("SELECT ``Value`` FROM ``Property`` WHERE ``Property``='{0}'", $Name)
    $view = $database.OpenView($query)
    $null = $view.Execute()
    $record = $view.Fetch()
    if (-not $record) { throw "MSI property is missing: $Name" }
    $value = $record.StringData(1).Trim()
    $null = $view.Close()
    return $value
}

if (-not $TestRebootContractOnly -and ($RebootContractBundlePath -or $RebootContractBundleXml)) {
    throw "RebootContractBundlePath and RebootContractBundleXml are only permitted with -TestRebootContractOnly."
}

$contractBundlePath = if ($TestRebootContractOnly -and $RebootContractBundlePath) { $RebootContractBundlePath } else { Join-Path $PSScriptRoot "bundle.wxs" }
$contractBundleXml = if ($TestRebootContractOnly) { $RebootContractBundleXml } else { $null }
Test-RebootContract -BundlePath $contractBundlePath -BundleXml $contractBundleXml
if ($TestRebootContractOnly) { return }
Test-ReleaseIdentityContract

$artifacts = @{}
foreach ($artifact in $dependencyLock.artifacts) {
    $artifacts[$artifact.id] = Get-LockedArtifact -Artifact $artifact
}

Push-Location $repoRoot
try {
    & dotnet tool restore
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

    foreach ($rustPackage in @('gnx-host-preflight', 'gnx-service', 'gnx-cli')) {
        & cargo rustc --release -p $rustPackage -- -C target-feature=+crt-static
        if ($LASTEXITCODE -ne 0) { throw "Static-CRT Rust release build failed for $rustPackage." }
    }

    $hostPreflight = Join-Path $repoRoot "target\release\gnx-host-preflight.exe"
    $gnxService = Join-Path $repoRoot "target\release\gnx-service.exe"
    $gnxCli = Join-Path $repoRoot "target\release\gnx.exe"
    $productMsi = Join-Path $outputRoot "Quetzalcoatl.msi"
    $setupExe = Join-Path $outputRoot "QuetzalcoatlSetup.exe"

    foreach ($rustBinary in @(
        @{ Name = 'gnx-host-preflight'; Path = $hostPreflight },
        @{ Name = 'gnx-service'; Path = $gnxService },
        @{ Name = 'gnx'; Path = $gnxCli }
    )) {
        $prohibitedCrtImports = Get-PeImportedDllNames -Path $rustBinary.Path |
            Where-Object { $_ -match '(?i)^(?:api-ms-win-crt-.+|vcruntime[0-9].*|msvcp[0-9].*|msvcr[0-9].*|concrt[0-9].*|vcomp[0-9].*|ucrtbase)\.dll$' }
        if ($prohibitedCrtImports) {
            throw "$($rustBinary.Name) must not dynamically import a Visual C++ runtime DLL: $($prohibitedCrtImports -join ', ')"
        }
    }

    & dotnet tool run wix -- build `
        (Join-Path $PSScriptRoot "package.wxs") `
        -arch x64 `
        -d "GnxCli=$gnxCli" `
        -d "GnxService=$gnxService" `
        -d "WinSW=$($artifacts.winsw)" `
        -d "ServiceConfig=$(Join-Path $PSScriptRoot 'Quetzalcoatl.Service.xml')" `
        -d "WinSWLicense=$(Join-Path $PSScriptRoot 'licenses\WinSW.txt')" `
        -d "PodmanMachineImage=$($artifacts.podman_machine)" `
        -d "RuntimePayload=$(Join-Path $repoRoot 'runtime\payload-v1')" `
        -out $productMsi
    if ($LASTEXITCODE -ne 0) { throw "MSI build failed." }

    $actualProductVersion = Get-MsiProperty -Path $productMsi -Name 'ProductVersion'
    $actualProductCode = Get-MsiProperty -Path $productMsi -Name 'ProductCode'
    $actualUpgradeCode = Get-MsiProperty -Path $productMsi -Name 'UpgradeCode'
    if ($actualProductVersion -ne $releaseVersion -or
        $actualProductCode -ne $releaseProductCode -or
        $actualUpgradeCode -ne $releaseUpgradeCode) {
        throw "Built MSI identity mismatch: version=$actualProductVersion ProductCode=$actualProductCode UpgradeCode=$actualUpgradeCode"
    }

    & dotnet tool run wix -- build `
        (Join-Path $PSScriptRoot "bundle.wxs") `
        -arch x64 `
        -dcl none `
        -ext $balExtension `
        -d "HostPreflight=$hostPreflight" `
        -d "WslMsi=$($artifacts.wsl)" `
        -d "PodmanMsi=$($artifacts.podman)" `
        -d "ProductMsi=$productMsi" `
        -out $setupExe
    if ($LASTEXITCODE -ne 0) { throw "Bundle build failed." }

    Get-FileHash -Algorithm SHA256 -LiteralPath $productMsi, $setupExe
} finally {
    Pop-Location
}
