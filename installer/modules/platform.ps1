function Test-PlatformPayloadSource {
    param(
        [Parameter(Mandatory = $true)] [string] $PlatformPayload
    )

    $root = (Resolve-Path -LiteralPath $PlatformPayload).Path
    $lockPath = Join-Path $root 'platform.lock.json'
    if (-not (Test-Path -LiteralPath $lockPath -PathType Leaf)) {
        throw "Platform lock is absent: $lockPath"
    }
    $lock = Get-Content -LiteralPath $lockPath -Raw -Encoding utf8 | ConvertFrom-Json
    if ($lock.schema_version -ne 1 -or
        $lock.bundle_contract -ne 1 -or
        $lock.policy.mutable_image_tags_allowed -ne $false -or
        $lock.policy.embedded_secrets_allowed -ne $false -or
        $lock.policy.repository_commands_allowed -ne $false) {
        throw 'Platform lock contract or policy differs.'
    }

    $locked = @{}
    foreach ($entry in @($lock.files)) {
        $relative = [string] $entry.path
        if ([string]::IsNullOrWhiteSpace($relative) -or
            $relative -cne $relative.ToLowerInvariant() -or
            [IO.Path]::IsPathRooted($relative) -or
            $relative.Contains('\') -or
            $relative.Contains('..') -or
            $locked.ContainsKey($relative)) {
            throw "Platform lock path is invalid: $relative"
        }
        if ([string] $entry.mode -notin @('0644', '0755')) {
            throw "Platform lock mode is invalid: $relative"
        }
        $source = Join-Path $root $relative.Replace('/', '\')
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Locked platform file is absent: $relative"
        }
        $actual = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -ne ([string] $entry.sha256).ToLowerInvariant()) {
            throw "Platform payload SHA-256 mismatch: $relative"
        }
        $locked[$relative] = $true
    }

    $actualPaths = @(
        Get-ChildItem -LiteralPath $root -Recurse -Force -File |
            Where-Object Name -ne 'platform.lock.json' |
            ForEach-Object {
                $_.FullName.Substring($root.Length + 1).Replace('\', '/')
            }
    )
    if ((@($locked.Keys | Sort-Object) -join "`n") -ne (@($actualPaths | Sort-Object) -join "`n")) {
        throw 'Platform source and lock inventories differ.'
    }

    $expectedDirectories = @{}
    foreach ($relative in $locked.Keys) {
        $parts = $relative.Split('/')
        for ($index = 1; $index -lt $parts.Count; $index++) {
            $expectedDirectories[($parts[0..($index - 1)] -join '/')] = $true
        }
    }
    $actualDirectories = @(
        Get-ChildItem -LiteralPath $root -Recurse -Force -Directory |
            ForEach-Object {
                $_.FullName.Substring($root.Length + 1).Replace('\', '/')
            }
    )
    if ((@($expectedDirectories.Keys | Sort-Object) -join "`n") -ne (@($actualDirectories | Sort-Object) -join "`n")) {
        throw 'Platform source contains an empty or unlocked directory.'
    }
}

function Copy-PlatformPayload {
    param(
        [Parameter(Mandatory = $true)] [string] $SourceRoot,
        [Parameter(Mandatory = $true)] [string] $DestinationRoot
    )

    Test-PlatformPayloadSource -PlatformPayload $SourceRoot
    $source = (Resolve-Path -LiteralPath $SourceRoot).Path
    $lockPath = Join-Path $source 'platform.lock.json'
    $lock = Get-Content -LiteralPath $lockPath -Raw -Encoding utf8 | ConvertFrom-Json

    if (Test-Path -LiteralPath $DestinationRoot) {
        Remove-Item -LiteralPath $DestinationRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $DestinationRoot | Out-Null
    Copy-Item -LiteralPath $lockPath -Destination $DestinationRoot

    foreach ($entry in @($lock.files)) {
        $relative = [string] $entry.path
        $sourcePath = Join-Path $source $relative.Replace('/', '\')
        $destinationPath = Join-Path $DestinationRoot $relative.Replace('/', '\')
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destinationPath) | Out-Null
        Copy-Item -LiteralPath $sourcePath -Destination $destinationPath
    }

    Test-PlatformPayloadSource -PlatformPayload $DestinationRoot
}
