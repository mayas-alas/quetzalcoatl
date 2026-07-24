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
$releaseVersion = "0.1.10"
$releaseProductCode = "{51B87559-35D7-4AAC-A311-2313CE8C0647}"
$releaseUpgradeCode = "{47D5BD44-D061-407B-913B-47D17EC3BEA9}"
$releasePackageCode = "{3BE07D30-37EA-4902-ABBF-4EB281CEFCCF}"
$releaseBundleId = "{E49415D1-2A12-411B-9F41-3D863623F159}"
$previousProductCode = "{ABDCB93E-1CC9-44AE-B697-D96BA9D4B21D}"
$previousPackageCode = "{AA1D510C-C890-4C25-87D1-14AF78B6F7EC}"
$previousBundleId = "{B72C8316-76AE-4BEB-97C0-02F9CD00F943}"
$bundleUpgradeCode = "{10B764B2-36AE-4911-A8C8-2F1A2A963769}"
$releaseTimestamp = [DateTime]::SpecifyKind([DateTime] "2026-07-24T00:00:00", [DateTimeKind]::Utc)
$releaseCabDate = [uint16] (((2026 - 1980) -shl 9) -bor (7 -shl 5) -bor 24)
$releaseCabTime = [uint16] 0
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
    if ($bundleNode.GetAttribute('ProviderKey') -ne $bundleUpgradeCode -or
        $bundleNode.GetAttribute('UpgradeCode') -ne $bundleUpgradeCode) {
        throw "Release identity contract: Burn ProviderKey and UpgradeCode must preserve $bundleUpgradeCode."
    }

    $extensionRoot = Join-Path $PSScriptRoot "wixext\Gnx.DeterministicBundle.wixext"
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
        'crates\host-preflight\Cargo.toml'
    )) {
        $manifest = Get-Content -LiteralPath (Join-Path $repoRoot $manifestPath) -Raw -Encoding utf8
        if ($manifest -notmatch "(?m)^version\s*=\s*`"$([regex]::Escape($releaseVersion))`"\s*$") {
            throw "Release identity contract: $manifestPath must use version $releaseVersion."
        }
    }
}

function Test-MsiPayloadCoherence {
    param(
        [Parameter(Mandatory)][string] $MsiPath,
        [Parameter(Mandatory)][string] $ServiceBinary,
        [Parameter(Mandatory)][string] $RuntimePayload
    )

    $verificationRoot = Join-Path $outputRoot "msi-payload-verification"
    $verificationLog = Join-Path $outputRoot "msi-payload-verification.log"
    Remove-Item -LiteralPath $verificationRoot -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $verificationLog -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $verificationRoot | Out-Null

    try {
        # Wait for the full administrative extraction before inspecting the staged tree.
        $msiexecPath = Join-Path $env:SystemRoot "System32\msiexec.exe"
        $msiexecArguments = "/a `"$MsiPath`" /qn /norestart TARGETDIR=`"$verificationRoot`" /L*V `"$verificationLog`""
        $msiexecProcess = Start-Process `
            -FilePath $msiexecPath `
            -ArgumentList $msiexecArguments `
            -Wait `
            -PassThru
        if ($msiexecProcess.ExitCode -ne 0) {
            throw "MSI administrative extraction failed with exit code $($msiexecProcess.ExitCode). Log: $verificationLog"
        }

        $stagedServices = @(Get-ChildItem -LiteralPath $verificationRoot -Recurse -File -Filter "gnx-service.exe")
        if ($stagedServices.Count -ne 1) {
            $stagedFileCount = @(Get-ChildItem -LiteralPath $verificationRoot -Recurse -File).Count
            throw "MSI payload coherence: expected exactly one staged gnx-service.exe; found $($stagedServices.Count) among $stagedFileCount extracted files. Log: $verificationLog"
        }
        $sourceServiceHash = (Get-FileHash -LiteralPath $ServiceBinary -Algorithm SHA256).Hash
        $stagedServiceHash = (Get-FileHash -LiteralPath $stagedServices[0].FullName -Algorithm SHA256).Hash
        if ($sourceServiceHash -ne $stagedServiceHash) {
            throw "MSI payload coherence: staged gnx-service.exe differs from the freshly built binary."
        }

        $stagedManifests = @(Get-ChildItem -LiteralPath $verificationRoot -Recurse -File -Filter "manifest.json" | Where-Object {
            $_.Directory.Name -eq 'runtime'
        })
        if ($stagedManifests.Count -ne 1) {
            throw "MSI payload coherence: expected exactly one staged runtime manifest; found $($stagedManifests.Count)."
        }
        $stagedRuntime = $stagedManifests[0].Directory.FullName

        function Get-TreeHashes([string] $Root) {
            $resolvedRoot = (Resolve-Path -LiteralPath $Root).Path.TrimEnd('\')
            $result = @{}
            foreach ($file in Get-ChildItem -LiteralPath $resolvedRoot -Recurse -File) {
                $relative = $file.FullName.Substring($resolvedRoot.Length).TrimStart('\').Replace('\', '/')
                $result[$relative] = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
            }
            return $result
        }

        $sourceRuntimeHashes = Get-TreeHashes $RuntimePayload
        $stagedRuntimeHashes = Get-TreeHashes $stagedRuntime
        $sourcePaths = @($sourceRuntimeHashes.Keys | Sort-Object)
        $stagedPaths = @($stagedRuntimeHashes.Keys | Sort-Object)
        if (($sourcePaths -join "`n") -ne ($stagedPaths -join "`n")) {
            $missing = @($sourcePaths | Where-Object { -not $stagedRuntimeHashes.ContainsKey($_) })
            $extra = @($stagedPaths | Where-Object { -not $sourceRuntimeHashes.ContainsKey($_) })
            throw "MSI payload coherence: runtime file set differs; missing=$($missing -join ', '); extra=$($extra -join ', ')."
        }
        foreach ($relative in $sourcePaths) {
            if ($sourceRuntimeHashes[$relative] -ne $stagedRuntimeHashes[$relative]) {
                throw "MSI payload coherence: staged runtime file hash differs for $relative."
            }
        }
    } finally {
        Remove-Item -LiteralPath $verificationRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Get-MsiProperty {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][string] $Name
    )

    $installer = New-Object -ComObject WindowsInstaller.Installer
    try {
        $database = $installer.OpenDatabase($Path, 0)
        $query = [string]::Format("SELECT ``Value`` FROM ``Property`` WHERE ``Property``='{0}'", $Name)
        $view = $database.OpenView($query)
        $null = $view.Execute()
        $record = $view.Fetch()
        if (-not $record) { throw "MSI property is missing: $Name" }
        return $record.StringData(1).Trim()
    } finally {
        if ($view) { $null = $view.Close() }
        foreach ($comObject in @($record, $view, $database, $installer)) {
            if ($comObject -and [Runtime.InteropServices.Marshal]::IsComObject($comObject)) {
                $null = [Runtime.InteropServices.Marshal]::FinalReleaseComObject($comObject)
            }
        }
    }
}

function Get-MsiSummaryProperty {
    param(
        [Parameter(Mandatory)][string] $Path,
        [Parameter(Mandatory)][int] $PropertyId
    )

    $installer = New-Object -ComObject WindowsInstaller.Installer
    try {
        $summary = $installer.SummaryInformation($Path, 0)
        return $summary.Property($PropertyId).ToString().Trim()
    } finally {
        foreach ($comObject in @($summary, $installer)) {
            if ($comObject -and [Runtime.InteropServices.Marshal]::IsComObject($comObject)) {
                $null = [Runtime.InteropServices.Marshal]::FinalReleaseComObject($comObject)
            }
        }
    }
}

function Set-MsiDeterministicMetadata {
    param([Parameter(Mandatory)][string] $Path)

    $installer = New-Object -ComObject WindowsInstaller.Installer
    try {
        $summary = $installer.SummaryInformation($Path, 3)
        $summary.Property(9) = $releasePackageCode
        $summary.Property(12) = $releaseTimestamp
        $summary.Property(13) = $releaseTimestamp
        $null = $summary.Persist()
    } finally {
        foreach ($comObject in @($summary, $installer)) {
            if ($comObject -and [Runtime.InteropServices.Marshal]::IsComObject($comObject)) {
                $null = [Runtime.InteropServices.Marshal]::FinalReleaseComObject($comObject)
            }
        }
    }

    $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
    try {
        $header = New-Object byte[] 512
        if ($stream.Read($header, 0, $header.Length) -ne $header.Length) {
            throw "MSI compound-file header is truncated: $Path"
        }
        $compoundSignature = [byte[]] (0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1)
        for ($index = 0; $index -lt $compoundSignature.Length; $index++) {
            if ($header[$index] -ne $compoundSignature[$index]) {
                throw "MSI compound-file signature is invalid: $Path"
            }
        }

        $sectorShift = [BitConverter]::ToUInt16($header, 30)
        $sectorSize = 1 -shl $sectorShift
        $directorySector = [BitConverter]::ToUInt32($header, 48)
        $rootEntryOffset = ([int64] $directorySector + 1) * $sectorSize
        if ($rootEntryOffset + 128 -gt $stream.Length) {
            throw "MSI compound-file root directory entry is outside the file: $Path"
        }

        $null = $stream.Seek($rootEntryOffset + 100, [IO.SeekOrigin]::Begin)
        $zeroTimestamps = New-Object byte[] 16
        $stream.Write($zeroTimestamps, 0, $zeroTimestamps.Length)
        $stream.Flush()
    } finally {
        $stream.Dispose()
    }
}

function Find-FirstCabinetOffset {
    param([Parameter(Mandatory)][IO.FileStream] $Stream)

    $signature = [byte[]] (0x4D, 0x53, 0x43, 0x46)
    $matched = 0
    $null = $Stream.Seek(0, [IO.SeekOrigin]::Begin)
    $scanLimit = [Math]::Min($Stream.Length, 64MB)
    while ($Stream.Position -lt $scanLimit) {
        $value = $Stream.ReadByte()
        if ($value -eq $signature[$matched]) {
            $matched++
            if ($matched -eq $signature.Length) {
                return $Stream.Position - $signature.Length
            }
        } else {
            $matched = if ($value -eq $signature[0]) { 1 } else { 0 }
        }
    }
    throw "Burn bundle does not contain a top-level cabinet in the first 64 MiB."
}

function Set-CabinetDeterministicTimestamps {
    param(
        [Parameter(Mandatory)][IO.FileStream] $Stream,
        [Parameter(Mandatory)][int64] $CabinetOffset
    )

    $header = New-Object byte[] 36
    $null = $Stream.Seek($CabinetOffset, [IO.SeekOrigin]::Begin)
    if ($Stream.Read($header, 0, $header.Length) -ne $header.Length -or
        [Text.Encoding]::ASCII.GetString($header, 0, 4) -ne 'MSCF') {
        throw "Invalid cabinet header at bundle offset $CabinetOffset."
    }

    $cabinetSize = [BitConverter]::ToUInt32($header, 8)
    $filesOffset = [BitConverter]::ToUInt32($header, 16)
    $fileCount = [BitConverter]::ToUInt16($header, 28)
    $cabinetEnd = $CabinetOffset + $cabinetSize
    if ($cabinetSize -lt 36 -or $cabinetEnd -gt $Stream.Length -or $fileCount -eq 0 -or $fileCount -gt 10000) {
        throw "Cabinet bounds or file count are invalid at bundle offset $CabinetOffset."
    }

    $entryOffset = $CabinetOffset + $filesOffset
    for ($index = 0; $index -lt $fileCount; $index++) {
        if ($entryOffset + 17 -gt $cabinetEnd) {
            throw "Cabinet file entry $index is outside its container."
        }

        $null = $Stream.Seek($entryOffset + 10, [IO.SeekOrigin]::Begin)
        $dateBytes = [BitConverter]::GetBytes($releaseCabDate)
        $timeBytes = [BitConverter]::GetBytes($releaseCabTime)
        $Stream.Write($dateBytes, 0, $dateBytes.Length)
        $Stream.Write($timeBytes, 0, $timeBytes.Length)

        $null = $Stream.Seek($entryOffset + 16, [IO.SeekOrigin]::Begin)
        do {
            if ($Stream.Position -ge $cabinetEnd) {
                throw "Cabinet file entry $index has an unterminated name."
            }
            $nameByte = $Stream.ReadByte()
        } while ($nameByte -ne 0)
        $entryOffset = $Stream.Position
    }

    return [int64] $cabinetSize
}

function Set-CabinetDataChecksums {
    param(
        [Parameter(Mandatory)][IO.FileStream] $Stream,
        [Parameter(Mandatory)][int64] $CabinetOffset
    )

    $header = New-Object byte[] 36
    $null = $Stream.Seek($CabinetOffset, [IO.SeekOrigin]::Begin)
    if ($Stream.Read($header, 0, $header.Length) -ne $header.Length -or
        [Text.Encoding]::ASCII.GetString($header, 0, 4) -ne 'MSCF') {
        throw "Invalid cabinet header at bundle offset $CabinetOffset."
    }

    $cabinetSize = [BitConverter]::ToUInt32($header, 8)
    $folderCount = [BitConverter]::ToUInt16($header, 26)
    $flags = [BitConverter]::ToUInt16($header, 30)
    if ($flags -ne 0 -or $folderCount -eq 0 -or $folderCount -gt 1000) {
        throw "Deterministic Burn normalization requires unreserved, single-part cabinets."
    }

    $folderTableOffset = $CabinetOffset + 36
    for ($folderIndex = 0; $folderIndex -lt $folderCount; $folderIndex++) {
        $folder = New-Object byte[] 8
        $null = $Stream.Seek($folderTableOffset + (8 * $folderIndex), [IO.SeekOrigin]::Begin)
        if ($Stream.Read($folder, 0, $folder.Length) -ne $folder.Length) {
            throw "Cabinet folder entry $folderIndex is truncated."
        }

        $dataOffset = $CabinetOffset + [BitConverter]::ToUInt32($folder, 0)
        $dataBlockCount = [BitConverter]::ToUInt16($folder, 4)
        for ($blockIndex = 0; $blockIndex -lt $dataBlockCount; $blockIndex++) {
            if ($dataOffset + 8 -gt $CabinetOffset + $cabinetSize) {
                throw "Cabinet data block $blockIndex is outside its container."
            }

            $dataHeader = New-Object byte[] 8
            $null = $Stream.Seek($dataOffset, [IO.SeekOrigin]::Begin)
            if ($Stream.Read($dataHeader, 0, $dataHeader.Length) -ne $dataHeader.Length) {
                throw "Cabinet data block $blockIndex is truncated."
            }
            $compressedSize = [BitConverter]::ToUInt16($dataHeader, 4)
            $checksumInput = New-Object byte[] (4 + $compressedSize)
            [Array]::Copy($dataHeader, 4, $checksumInput, 0, 4)
            $bytesRead = 0
            while ($bytesRead -lt $compressedSize) {
                $read = $Stream.Read($checksumInput, 4 + $bytesRead, $compressedSize - $bytesRead)
                if ($read -le 0) { throw "Cabinet data block $blockIndex is truncated." }
                $bytesRead += $read
            }

            [uint32] $checksum = 0
            $wholeLength = $checksumInput.Length - ($checksumInput.Length % 4)
            for ($offset = 0; $offset -lt $wholeLength; $offset += 4) {
                $checksum = $checksum -bxor [BitConverter]::ToUInt32($checksumInput, $offset)
            }
            $tailLength = $checksumInput.Length - $wholeLength
            for ($offset = $wholeLength; $offset -lt $checksumInput.Length; $offset++) {
                $tailIndex = $offset - $wholeLength
                $checksum = $checksum -bxor ([uint32] $checksumInput[$offset] -shl (8 * ($tailLength - 1 - $tailIndex)))
            }

            $null = $Stream.Seek($dataOffset, [IO.SeekOrigin]::Begin)
            $checksumBytes = [BitConverter]::GetBytes($checksum)
            $Stream.Write($checksumBytes, 0, $checksumBytes.Length)
            $dataOffset += 8 + $compressedSize
        }
    }
}

function Get-StreamRangeSha512 {
    param(
        [Parameter(Mandatory)][IO.FileStream] $Stream,
        [Parameter(Mandatory)][int64] $Offset,
        [Parameter(Mandatory)][int64] $Length
    )

    $sha512 = [Security.Cryptography.SHA512]::Create()
    try {
        $buffer = New-Object byte[] 1MB
        $remaining = $Length
        $null = $Stream.Seek($Offset, [IO.SeekOrigin]::Begin)
        while ($remaining -gt 0) {
            $requested = [int] [Math]::Min($buffer.Length, $remaining)
            $read = $Stream.Read($buffer, 0, $requested)
            if ($read -le 0) { throw "Bundle ended while hashing its attached container." }
            $null = $sha512.TransformBlock($buffer, 0, $read, $buffer, 0)
            $remaining -= $read
        }
        $empty = New-Object byte[] 0
        $null = $sha512.TransformFinalBlock($empty, 0, 0)
        return [BitConverter]::ToString($sha512.Hash).Replace('-', '')
    } finally {
        $sha512.Dispose()
    }
}

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

    $deterministicExtensionProject = Join-Path $PSScriptRoot "wixext\Gnx.DeterministicBundle.wixext\Gnx.DeterministicBundle.wixext.csproj"
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
    Remove-Item -LiteralPath $releaseBinaries -Force -ErrorAction SilentlyContinue
    $rustBuildStarted = [DateTime]::UtcNow

    foreach ($rustPackage in @('gnx-host-preflight', 'gnx-service', 'gnx-cli')) {
        & cargo rustc --locked --release -p $rustPackage -- -C target-feature=+crt-static
        if ($LASTEXITCODE -ne 0) { throw "Static-CRT Rust release build failed for $rustPackage." }
    }
    foreach ($releaseBinary in $releaseBinaries) {
        if (-not (Test-Path -LiteralPath $releaseBinary)) {
            throw "Fresh Rust release binary is absent: $releaseBinary"
        }
        if ((Get-Item -LiteralPath $releaseBinary).LastWriteTimeUtc -lt $rustBuildStarted.AddSeconds(-2)) {
            throw "Rust release binary was not freshly produced by this build: $releaseBinary"
        }
    }

    & cargo test --locked --release -p gnx-service payload_manifest_matches_all_installed_files
    if ($LASTEXITCODE -ne 0) {
        throw "gnx-service/runtime manifest build contract test failed."
    }
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
        -d "RuntimePayload=$(Join-Path $repoRoot 'runtime\payload-v2')" `
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
        -RuntimePayload (Join-Path $repoRoot 'runtime\payload-v2')

    & dotnet tool run wix -- build `
        (Join-Path $PSScriptRoot "bundle.wxs") `
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
