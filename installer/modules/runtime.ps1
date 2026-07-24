function Test-RuntimePayloadSource {
    param(
        [Parameter(Mandatory = $true)] [string] $RuntimePayload,
        [Parameter(Mandatory = $true)] [int] $ExpectedPayloadVersion
    )

    $payloadRoot = (Resolve-Path -LiteralPath $RuntimePayload).Path
    $manifestPath = Join-Path $payloadRoot 'manifest.json'
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Runtime manifest is absent: $manifestPath"
    }

    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
    if ($manifest.schema_version -ne 1 -or $manifest.payload_version -ne $ExpectedPayloadVersion) {
        throw "Unexpected runtime manifest contract: schema=$($manifest.schema_version) payload=$($manifest.payload_version)"
    }

    $manifestFiles = @{}
    foreach ($entry in $manifest.files) {
        $relative = [string] $entry.path
        if ([string]::IsNullOrWhiteSpace($relative) -or
            [System.IO.Path]::IsPathRooted($relative) -or
            $relative.Contains('..') -or
            $relative.Contains('\')) {
            throw "Runtime manifest path is not a normalized relative path: '$relative'"
        }
        if ($manifestFiles.ContainsKey($relative)) {
            throw "Runtime manifest contains a duplicate path: $relative"
        }
        $manifestFiles[$relative] = $entry

        $source = Join-Path $payloadRoot ($relative.Replace('/', '\'))
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Runtime payload file is absent: $relative"
        }
        $actualHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actualHash -ne ([string] $entry.sha256).ToLowerInvariant()) {
            throw "Runtime payload SHA-256 mismatch for $relative"
        }
        $expectedMode = if ($relative.StartsWith('bin/')) { '0755' } else { '0644' }
        if ([string] $entry.mode -ne $expectedMode) {
            throw "Runtime payload mode mismatch for ${relative}: expected $expectedMode, received $($entry.mode)"
        }
    }

    $physicalFiles = @{}
    foreach ($file in (Get-ChildItem -LiteralPath $payloadRoot -File -Recurse)) {
        if ($file.FullName -eq $manifestPath) { continue }
        $relative = $file.FullName.Substring($payloadRoot.Length).TrimStart([char[]] @('\', '/')).Replace('\', '/')
        $physicalFiles[$relative] = $true
    }

    $missing = @($manifestFiles.Keys | Where-Object { -not $physicalFiles.ContainsKey($_) } | Sort-Object)
    $extra = @($physicalFiles.Keys | Where-Object { -not $manifestFiles.ContainsKey($_) } | Sort-Object)
    if ($missing.Count -ne 0 -or $extra.Count -ne 0) {
        throw "Runtime payload file set differs from manifest: missing=$($missing -join ',') extra=$($extra -join ',')"
    }
}
