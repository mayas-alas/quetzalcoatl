function Test-MsiPayloadCoherence {
    param(
        [Parameter(Mandatory)][string] $MsiPath,
        [Parameter(Mandatory)][string] $ServiceBinary,
        [Parameter(Mandatory)][string] $CliBinary,
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

        $stagedClis = @(Get-ChildItem -LiteralPath $verificationRoot -Recurse -File -Filter "gnx.exe")
        if ($stagedClis.Count -ne 1) {
            throw "MSI payload coherence: expected exactly one staged gnx.exe; found $($stagedClis.Count)."
        }
        $sourceCliHash = (Get-FileHash -LiteralPath $CliBinary -Algorithm SHA256).Hash
        $stagedCliHash = (Get-FileHash -LiteralPath $stagedClis[0].FullName -Algorithm SHA256).Hash
        if ($sourceCliHash -ne $stagedCliHash) {
            throw "MSI payload coherence: staged gnx.exe differs from the freshly built binary."
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

