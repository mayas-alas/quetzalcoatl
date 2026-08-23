function Test-RuntimePayloadSource {
    param(
        [Parameter(Mandatory = $true)] [string] $RuntimePayload,
        [Parameter(Mandatory = $true)] [int] $ExpectedPayloadVersion
    )

    $runtimeRoot = (Resolve-Path -LiteralPath $RuntimePayload).Path
    $manifestPath = Join-Path $runtimeRoot 'manifest.toml'
    $lockPath = Join-Path $runtimeRoot 'payload.lock.json'
    foreach ($requiredPath in @($manifestPath, $lockPath)) {
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "Runtime contract file is absent: $requiredPath"
        }
    }

    $manifestText = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8
    $manifestFacts = @{}
    $section = ''
    foreach ($rawLine in $manifestText -split "`r?`n") {
        $line = $rawLine.Trim()
        if (-not $line -or $line.StartsWith('#')) { continue }
        if ($line -match '^\[(?<section>[A-Za-z0-9_.-]+)\]$') {
            $section = $Matches.section
            continue
        }
        if ($line -notmatch '^(?<key>[A-Za-z0-9_.-]+)\s*=\s*(?<value>.+)$') {
            throw "Runtime manifest contains an unsupported line: $rawLine"
        }
        $key = if ($section) { "$section.$($Matches.key)" } else { $Matches.key }
        $value = $Matches.value.Trim()
        if ($value -match '^"(?<text>[^"]*)"$') { $value = $Matches.text }
        elseif ($value -match '^\d+$') { $value = [int] $value }
        else { throw "Runtime manifest contains an unsupported value for $key." }
        if ($manifestFacts.ContainsKey($key)) {
            throw "Runtime manifest contains a duplicate key: $key"
        }
        $manifestFacts[$key] = $value
    }

    if ($manifestFacts.schema_version -ne 1 -or
        $manifestFacts.generation -ne 'proxmox-platform' -or
        $manifestFacts.payload_contract -ne $ExpectedPayloadVersion -or
        $manifestFacts.payload_lock -ne 'payload.lock.json') {
        throw "Runtime manifest differs from generation proxmox-platform payload contract $ExpectedPayloadVersion."
    }
    foreach ($layout in @('commands', 'operations', 'containers', 'services', 'configuration')) {
        if ($manifestFacts["layout.$layout"] -ne $layout -or
            -not (Test-Path -LiteralPath (Join-Path $runtimeRoot $layout) -PathType Container)) {
            throw "Runtime layout is missing or invalid: $layout"
        }
    }
    if (Test-Path -LiteralPath (Join-Path $runtimeRoot 'payload')) {
        throw "Runtime layout must not retain a parallel payload tree."
    }

    $lock = Get-Content -LiteralPath $lockPath -Raw -Encoding utf8 | ConvertFrom-Json
    if ($lock.schema_version -ne 1 -or $lock.payload_version -ne $ExpectedPayloadVersion) {
        throw "Runtime lock contract differs: schema=$($lock.schema_version) payload=$($lock.payload_version)"
    }
    if ($lock.target.os -ne 'linux' -or $lock.target.architecture -ne 'amd64' -or
        $lock.policy.mutable_image_tags_allowed -ne $false -or
        $lock.policy.embedded_secrets_allowed -ne $false) {
        throw "Runtime lock target or security policy differs."
    }

    $lockedPaths = @{}
    foreach ($entry in @($lock.files)) {
        $relative = [string] $entry.path
        if ([string]::IsNullOrWhiteSpace($relative) -or
            [System.IO.Path]::IsPathRooted($relative) -or
            $relative.Contains('..') -or
            $relative.Contains('\') -or
            $relative.StartsWith('/') -or
            $relative.EndsWith('/')) {
            throw "Runtime lock path is not normalized: '$relative'"
        }
        if ($lockedPaths.ContainsKey($relative)) {
            throw "Runtime lock contains a duplicate path: $relative"
        }
        $lockedPaths[$relative] = $true

        $source = Join-Path $runtimeRoot ($relative.Replace('/', '\'))
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Runtime payload file is absent: $relative"
        }
        $actualHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actualHash -ne ([string] $entry.sha256).ToLowerInvariant()) {
            throw "Runtime payload SHA-256 mismatch for $relative"
        }
        $expectedMode = if ($relative.StartsWith('commands/')) { '0755' } else { '0644' }
        if ([string] $entry.mode -ne $expectedMode) {
            throw "Runtime payload mode mismatch for ${relative}: expected $expectedMode, received $($entry.mode)"
        }
    }

    $expectedInstalledPaths = @(
        'commands/gnx-proxmox-entrypoint',
        'commands/gnx-pve-cluster-create',
        'commands/gnx-pve-configure',
        'commands/gnx-runtime-agent',
        'commands/gnx-tailscale-enroll',
        'commands/gnx-tailscale-prepare',
        'commands/gnx-tailscale-rename',
        'configuration/serve.json',
        'containers/gnx-node.pod',
        'containers/proxmox.container',
        'containers/tailscaled.container',
        'services/gnx-tailscale-enroll.service'
    )
    $actualInstalledPaths = @($lockedPaths.Keys | Sort-Object)
    if (($actualInstalledPaths -join "`n") -ne (($expectedInstalledPaths | Sort-Object) -join "`n")) {
        throw "Runtime lock file set differs from the installed payload contract."
    }
    $actualPayloadPaths = @(
        foreach ($layout in @('commands', 'configuration', 'containers', 'services')) {
            $layoutRoot = Join-Path $runtimeRoot $layout
            foreach ($file in Get-ChildItem -LiteralPath $layoutRoot -Recurse -File) {
                $file.FullName.Substring($runtimeRoot.Length).TrimStart('\').Replace('\', '/')
            }
        }
    ) | Sort-Object
    if (($actualPayloadPaths -join "`n") -ne ($actualInstalledPaths -join "`n")) {
        throw "Runtime source contains an installed file outside the payload lock."
    }
    if (@($lockedPaths.Keys | Where-Object { $_.StartsWith('operations/') }).Count -ne 0) {
        throw "Runtime operations must not be installed payload."
    }
}
