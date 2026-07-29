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
    if (-not $Artifact.authenticode) {
        throw "Locked artifact $($Artifact.id) omits Authenticode policy."
    }
    switch ([string] $Artifact.authenticode.status) {
        'valid' {
            $signature = Get-AuthenticodeSignature -LiteralPath $destination
            if ($signature.Status -ne 'Valid' -or
                $signature.SignerCertificate.Thumbprint -ne $Artifact.authenticode.thumbprint) {
                throw "Authenticode identity mismatch for locked artifact $($Artifact.id): status=$($signature.Status)."
            }
        }
        'not_signed' {
            $signature = Get-AuthenticodeSignature -LiteralPath $destination
            if ($signature.Status -ne 'NotSigned' -or
                [string]::IsNullOrWhiteSpace([string] $Artifact.authenticode.reason)) {
                throw "Unsigned exception is invalid for locked artifact $($Artifact.id)."
            }
        }
        'not_applicable' {
            if ([string]::IsNullOrWhiteSpace([string] $Artifact.authenticode.reason)) {
                throw "Non-Authenticode exception is invalid for locked artifact $($Artifact.id)."
            }
        }
        default {
            throw "Unsupported Authenticode policy for locked artifact $($Artifact.id)."
        }
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

