[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Validate', 'Import')]
    [string]$Mode,

    [Parameter(Mandatory = $true)]
    [string]$SourcePath,

    [Parameter(Mandatory = $true)]
    [string]$NodePayload,

    [Parameter(Mandatory = $true)]
    [string]$StateDir,

    [Parameter(Mandatory = $false)]
    [string]$ExpectedWorkload
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$script:PartialRevision = $null
$script:ReceiptTemp = $null
$script:Policy = $null
$script:InstalledRoot = $null
$script:InstalledIndex = $null
$script:WorkloadName = $null
$script:NetworkFileName = $null

function Fail([string]$Code) {
    throw [System.InvalidOperationException]::new("GNX_BUNDLE:$Code")
}

function Get-FullPath([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) { Fail 'PATH_REQUIRED' }
    try { return [System.IO.Path]::GetFullPath($Path) } catch { Fail 'PATH_INVALID' }
}

function Assert-LocalPath([string]$Path) {
    if ($Path.StartsWith('\\', [System.StringComparison]::Ordinal)) { Fail 'LOCAL_PATH_REQUIRED' }
    if (-not [System.IO.Path]::IsPathRooted($Path)) { Fail 'LOCAL_PATH_REQUIRED' }
}

function Test-Reparse([System.IO.FileSystemInfo]$Item) {
    return (($Item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)
}

function Get-RelativePath([string]$Root, [string]$Path) {
    $rootFull = (Get-FullPath $Root).TrimEnd('\')
    $pathFull = Get-FullPath $Path
    if ($pathFull.Equals($rootFull, [System.StringComparison]::OrdinalIgnoreCase)) { return '' }
    $prefix = $rootFull + '\'
    if (-not $pathFull.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) { Fail 'SOURCE_ESCAPE' }
    return $pathFull.Substring($prefix.Length).Replace('\', '/')
}

function Assert-SafeRelativePath([string]$RelativePath) {
    if ([string]::IsNullOrWhiteSpace($RelativePath) -or $RelativePath.Contains('\') -or
        $RelativePath.StartsWith('/') -or $RelativePath.Contains(':') -or
        $RelativePath.Contains([char]0)) { Fail 'PATH_INVALID' }
    $parts = $RelativePath.Split('/')
    foreach ($part in $parts) {
        if ([string]::IsNullOrWhiteSpace($part) -or $part -eq '.' -or $part -eq '..') { Fail 'PATH_TRAVERSAL' }
        if ($part -match '[<>:"|?*]' -or $part -match '[\x00-\x1f]') { Fail 'PATH_INVALID' }
    }
}

function Get-CanonicalJson([object]$Value) {
    if ($null -eq $Value) { return 'null' }
    if ($Value -is [string]) { return ($Value | ConvertTo-Json -Compress) }
    if ($Value -is [bool]) { if ($Value) { return 'true' } else { return 'false' } }
    if ($Value -is [byte] -or $Value -is [sbyte] -or $Value -is [int16] -or $Value -is [uint16] -or
        $Value -is [int32] -or $Value -is [uint32] -or $Value -is [int64] -or $Value -is [uint64] -or
        $Value -is [single] -or $Value -is [double] -or $Value -is [decimal]) {
        return [System.Convert]::ToString($Value, [Globalization.CultureInfo]::InvariantCulture)
    }
    if ($Value -is [System.Collections.IDictionary]) {
        $parts = @()
        $keys = @($Value.Keys | ForEach-Object { [string]$_ })
        [Array]::Sort($keys, [StringComparer]::Ordinal)
        foreach ($key in $keys) {
            $parts += ((($key | ConvertTo-Json -Compress)) + ':' + (Get-CanonicalJson $Value[$key]))
        }
        return '{' + ($parts -join ',') + '}'
    }
    if (($Value -is [System.Collections.IEnumerable]) -and -not ($Value -is [string])) {
        $parts = @()
        foreach ($item in $Value) { $parts += (Get-CanonicalJson $item) }
        return '[' + ($parts -join ',') + ']'
    }
    $propertyNames = @($Value.PSObject.Properties | ForEach-Object { [string]$_.Name })
    [Array]::Sort($propertyNames, [StringComparer]::Ordinal)
    if ($propertyNames.Count -eq 0) { return ($Value | ConvertTo-Json -Compress) }
    $parts = @()
    foreach ($propertyName in $propertyNames) {
        $parts += ((($propertyName | ConvertTo-Json -Compress)) + ':' + (Get-CanonicalJson $Value.PSObject.Properties[$propertyName].Value))
    }
    return '{' + ($parts -join ',') + '}'
}

function Get-StrictUtf8([byte[]]$Bytes) {
    try {
        $encoding = New-Object System.Text.UTF8Encoding($false, $true)
        $text = $encoding.GetString($Bytes)
        if ($text.Length -gt 0 -and $text.StartsWith(([string][char]0xfeff), [System.StringComparison]::Ordinal)) { $text = $text.Substring(1) }
        if ($text.Contains([char]0)) { Fail 'TEXT_INVALID' }
        return $text
    } catch [System.Text.DecoderFallbackException] { Fail 'TEXT_INVALID' }
}

function Get-NormalizedTextBytes([byte[]]$Bytes) {
    $text = Get-StrictUtf8 $Bytes
    $text = [regex]::Replace($text, "`r`n?", "`n")
    $text = [regex]::Replace($text, "`n+$", '') + "`n"
    $encoding = New-Object System.Text.UTF8Encoding($false)
    return ,$encoding.GetBytes($text)
}

function Get-Sha256Bytes([byte[]]$Bytes) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return (-join ($sha.ComputeHash($Bytes) | ForEach-Object { $_.ToString('x2') })) }
    finally { $sha.Dispose() }
}

function Get-Sha256File([string]$Path) {
    $sha = [Security.Cryptography.SHA256]::Create()
    $stream = $null
    try {
        $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        return (-join ($sha.ComputeHash($stream) | ForEach-Object { $_.ToString('x2') }))
    } catch { Fail 'SOURCE_READ_FAILED' }
    finally {
        if ($null -ne $stream) { $stream.Dispose() }
        $sha.Dispose()
    }
}

function Read-FileBytes([string]$Path, [int64]$Limit = 2097152) {
    try {
        $info = Get-Item -LiteralPath $Path -Force
        if ($info.Length -gt $Limit) { Fail 'CONFIG_SIZE_LIMIT' }
        return ,([IO.File]::ReadAllBytes($Path))
    } catch [System.InvalidOperationException] { throw }
    catch { Fail 'SOURCE_READ_FAILED' }
}

function Assert-NoAlternateDataStream([string]$Path) {
    try {
        $streams = @(Get-Item -LiteralPath $Path -Stream * -ErrorAction SilentlyContinue | Where-Object {
            $null -ne $_.Stream -and $_.Stream -ne ':$DATA'
        })
        if ($streams.Count -gt 0) { Fail 'ADS_NOT_ALLOWED' }
    } catch [System.InvalidOperationException] { throw }
    catch { Fail 'SOURCE_READ_FAILED' }
}

function Read-ZipEntryBytes([System.IO.Compression.ZipArchiveEntry]$Entry, [int64]$Limit = 2097152) {
    if ($Entry.Length -gt $Limit) { Fail 'CONFIG_SIZE_LIMIT' }
    $input = $null
    $output = New-Object IO.MemoryStream
    $buffer = New-Object byte[] 81920
    $total = 0L
    try {
        $input = $Entry.Open()
        while (($read = $input.Read($buffer, 0, $buffer.Length)) -gt 0) {
            $total += $read
            if ($total -gt $Limit) { Fail 'CONFIG_SIZE_LIMIT' }
            $output.Write($buffer, 0, $read)
        }
        return ,$output.ToArray()
    } catch [System.InvalidOperationException] { throw }
    catch { Fail 'SOURCE_READ_FAILED' }
    finally {
        if ($null -ne $input) { $input.Dispose() }
        $output.Dispose()
    }
}

function Get-Sha256ZipEntry([System.IO.Compression.ZipArchiveEntry]$Entry, [int64]$Limit) {
    if ($Entry.Length -gt $Limit) { Fail 'SEALED_INPUT_SIZE_MISMATCH' }
    $sha = [Security.Cryptography.SHA256]::Create()
    $stream = $null
    $buffer = New-Object byte[] 81920
    $total = 0L
    try {
        $stream = $Entry.Open()
        while (($read = $stream.Read($buffer, 0, $buffer.Length)) -gt 0) {
            $total += $read
            if ($total -gt $Limit) { Fail 'SEALED_INPUT_SIZE_MISMATCH' }
            $sha.TransformBlock($buffer, 0, $read, $buffer, 0) | Out-Null
        }
        $empty = [byte[]]@()
        $sha.TransformFinalBlock($empty, 0, 0) | Out-Null
        if ($total -ne $Entry.Length) { Fail 'SEALED_INPUT_SIZE_MISMATCH' }
        return (-join ($sha.Hash | ForEach-Object { $_.ToString('x2') }))
    } catch [System.InvalidOperationException] { throw }
    catch { Fail 'SOURCE_READ_FAILED' }
    finally {
        if ($null -ne $stream) { $stream.Dispose() }
        $sha.Dispose()
    }
}

function Get-InstalledFile([string]$RelativePath) {
    $key = $RelativePath.ToLowerInvariant()
    if (-not $script:InstalledIndex.ContainsKey($key)) { return $null }
    return $script:InstalledIndex[$key]
}

function Get-RelativeInstalledFiles() {
    $items = @(Get-ChildItem -LiteralPath $script:InstalledRoot -Force -Recurse)
    $result = @{}
    foreach ($item in $items) {
        if (Test-Reparse $item) { Fail 'INSTALLED_REPARSE_POINT' }
        if ($item.PSIsContainer) { continue }
        $relative = Get-RelativePath $script:InstalledRoot $item.FullName
        Assert-SafeRelativePath $relative
        $result[$relative.ToLowerInvariant()] = $item.FullName
    }
    return $result
}

function Get-ProfileField([object]$Profile, [string]$Name) {
    $property = $Profile.PSObject.Properties[$Name]
    if ($null -eq $property) { Fail 'PROFILE_FIELD_MISSING' }
    return $property.Value
}

function Assert-ProfileName([object]$Profile) {
    $name = [string](Get-ProfileField $Profile 'name')
    if ($name -ne [string]$script:WorkloadName -or [string](Get-ProfileField $Profile 'kind') -ne 'gnx.machine.profile') { Fail 'WORKLOAD_NOT_ALLOWED' }
    if (-not [string]::IsNullOrWhiteSpace($ExpectedWorkload) -and $ExpectedWorkload -ne [string]$script:WorkloadName) { Fail 'WORKLOAD_MISMATCH' }
}

function Get-ComparableProfile([object]$Profile) {
    $mutable = @($script:Policy.profile.mutable_fields)
    $map = [ordered]@{}
    foreach ($property in ($Profile.PSObject.Properties | Sort-Object Name)) {
        if ($mutable -notcontains $property.Name) { $map[$property.Name] = $property.Value }
    }
    return $map
}

function Assert-ResourceBounds([object]$Profile) {
    foreach ($field in @('cores', 'memory_mib', 'rootfs_gib')) {
        $property = $Profile.PSObject.Properties[$field]
        if ($null -eq $property -or $property.Value -is [bool]) { Fail 'RESOURCE_INVALID' }
        $value = 0L
        try { $value = [int64]$property.Value } catch { Fail 'RESOURCE_INVALID' }
        $bound = $script:Policy.profile.resource_bounds.$field
        if ($value -lt [int64]$bound.min -or $value -gt [int64]$bound.max) { Fail 'RESOURCE_OUT_OF_RANGE' }
        if ($value -ne [double]$property.Value) { Fail 'RESOURCE_INVALID' }
    }
    $release = [string](Get-ProfileField $Profile 'release')
    if ($release -notmatch [string]$script:Policy.profile.release_pattern) { Fail 'RELEASE_INVALID' }
}

function Get-InstalledImages([object]$Images) {
    $refs = @()
    $imageMap = $Images.PSObject.Properties['images']
    if ($null -eq $imageMap -or $null -eq $imageMap.Value) { Fail 'IMAGES_INVALID' }
    foreach ($image in $imageMap.Value.PSObject.Properties) {
        $refProperty = $image.Value.PSObject.Properties['image']
        if ($null -eq $refProperty -or [string]$refProperty.Value -notmatch '@sha256:[0-9a-f]{64}$') { Fail 'IMAGES_INVALID' }
        $refs += [string]$refProperty.Value
    }
    return $refs
}

function Get-InstalledNormalized([string]$Path) {
    $bytes = Read-FileBytes $Path 16777216
    return ,(Get-NormalizedTextBytes $bytes)
}

function Test-ExactOrNormalized([byte[]]$SourceBytes, [string]$InstalledPath) {
    $installedBytes = Read-FileBytes $InstalledPath 16777216
    if ((Get-Sha256Bytes $SourceBytes) -eq (Get-Sha256Bytes $installedBytes)) { return $true }
    try {
        return ((Get-Sha256Bytes (Get-NormalizedTextBytes $SourceBytes)) -eq (Get-Sha256Bytes (Get-NormalizedTextBytes $installedBytes)))
    } catch { return $false }
}

function Get-FileClassification([string]$RelativePath) {
    Assert-SafeRelativePath $RelativePath
    $parts = $RelativePath.Split('/')
    if ($parts.Count -eq 1) {
        $name = $parts[0]
        $lower = $name.ToLowerInvariant()
        if (@($script:Policy.required_files | ForEach-Object { ([string]$_).ToLowerInvariant() }) -contains $lower) {
            return [pscustomobject]@{ Kind = 'accepted'; Path = $lower }
        }
        if ($lower -eq 'offline.tar.gz') {
            return [pscustomobject]@{ Kind = 'sealed'; Path = 'offline.tar.gz' }
        }
        if ($lower.EndsWith('.container')) {
            $installed = Get-InstalledFile $RelativePath
            if ($null -eq $installed -or @($script:Policy.quadlets) -notcontains $lower) { Fail 'QUADLET_NOT_ALLOWED' }
            return [pscustomobject]@{ Kind = 'accepted'; Path = [IO.Path]::GetFileName($installed) }
        }
        foreach ($extension in @($script:Policy.ignored_exact_extensions)) {
            if ($lower.EndsWith([string]$extension)) {
                $installed = Get-InstalledFile $RelativePath
                if ($null -eq $installed) { Fail 'SCRIPT_NOT_TRUSTED' }
                return [pscustomobject]@{ Kind = 'ignored'; Path = [IO.Path]::GetFileName($installed) }
            }
        }
        Fail 'FILE_NOT_ALLOWED'
    }
    Fail 'FILE_NOT_ALLOWED'
}

function New-Entry([string]$RelativePath, [string]$FullPath, [byte[]]$RawBytes, [string]$Kind, [string]$CanonicalPath, [object]$ZipEntry) {
    if ($Kind -eq 'ignored') {
        $installed = Get-InstalledFile $RelativePath
        if ($null -eq $installed -or -not (Test-ExactOrNormalized $RawBytes $installed)) { Fail 'SCRIPT_NOT_TRUSTED' }
        return [pscustomobject]@{ Kind = 'ignored'; Path = $CanonicalPath; Bytes = $null; RawLength = [int64]$RawBytes.Length; FullPath = $FullPath; ZipEntry = $ZipEntry }
    }
    if ($Kind -eq 'sealed') {
        $installedArchive = Get-InstalledFile 'offline.tar.gz'
        if ($null -eq $installedArchive) { Fail 'SEALED_INPUT_MISSING' }
        $installedLength = [int64](Get-Item -LiteralPath $installedArchive -Force).Length
        $sourceLength = if ($null -ne $FullPath) { [int64](Get-Item -LiteralPath $FullPath -Force).Length } else { [int64]$ZipEntry.Length }
        if ($sourceLength -ne $installedLength) { Fail 'SEALED_INPUT_SIZE_MISMATCH' }
        $installedHash = Get-Sha256File $installedArchive
        $manifest = Get-InstalledFile 'offline.sha256'
        $manifestText = [Text.Encoding]::UTF8.GetString((Read-FileBytes $manifest 65536))
        if ($manifestText -notmatch "(?im)^$installedHash\s+offline\.tar\.gz\s*$") { Fail 'SEALED_INPUT_INVALID' }
        $sourceHash = if ($null -ne $FullPath) { Get-Sha256File $FullPath } else {
            Get-Sha256ZipEntry $ZipEntry $installedLength
        }
        if ($sourceHash -ne $installedHash) { Fail 'SEALED_INPUT_MISMATCH' }
        return [pscustomobject]@{ Kind = 'sealed'; Path = 'offline.tar.gz'; Bytes = $null; RawLength = 0L; FullPath = $FullPath; ZipEntry = $ZipEntry }
    }
    if ($null -eq $RawBytes) { Fail 'SOURCE_READ_FAILED' }
    if ($RawBytes.Length -gt 2097152) { Fail 'CONFIG_SIZE_LIMIT' }
    if ($CanonicalPath -in @('profile.json', 'images.json')) {
        try { $json = (Get-StrictUtf8 $RawBytes | ConvertFrom-Json) } catch { Fail 'JSON_INVALID' }
        return [pscustomobject]@{ Kind = 'accepted'; Path = $CanonicalPath; Bytes = [Text.Encoding]::UTF8.GetBytes((Get-CanonicalJson $json) + "`n"); RawLength = [int64]$RawBytes.Length; FullPath = $FullPath; ZipEntry = $ZipEntry; Json = $json }
    }
    return [pscustomobject]@{ Kind = 'accepted'; Path = $CanonicalPath; Bytes = (Get-NormalizedTextBytes $RawBytes); RawLength = [int64]$RawBytes.Length; FullPath = $FullPath; ZipEntry = $ZipEntry }
}

function Get-SourceFolderEntries([string]$Root) {
    $rootItem = Get-Item -LiteralPath $Root -Force
    if (-not $rootItem.PSIsContainer -or (Test-Reparse $rootItem)) { Fail 'SOURCE_FOLDER_INVALID' }
    $count = 0
    $directory = New-Object IO.DirectoryInfo($Root)
    $seen = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    $entries = @()
    foreach ($info in $directory.EnumerateFileSystemInfos()) {
        $count++
        if ($count -gt [int64]$script:Policy.limits.max_entries) { Fail 'ENTRY_COUNT_LIMIT' }
        $item = Get-Item -LiteralPath $info.FullName -Force
        if (Test-Reparse $item) { Fail 'REPARSE_POINT_NOT_ALLOWED' }
        $relative = Get-RelativePath $Root $item.FullName
        if ($item.PSIsContainer) {
            Fail 'DIRECTORY_NOT_ALLOWED'
        }
        if ($item.Attributes -band [IO.FileAttributes]::Device) { Fail 'NONREGULAR_ENTRY' }
        Assert-SafeRelativePath $relative
        Assert-NoAlternateDataStream $item.FullName
        if (-not $seen.Add($relative)) { Fail 'DUPLICATE_ENTRY' }
        $classification = Get-FileClassification $relative
        $installedPath = Get-InstalledFile $relative
        $readLimit = if ($classification.Kind -eq 'sealed') { 0L } elseif ($classification.Kind -eq 'ignored' -and $null -ne $installedPath) { [int64](Get-Item -LiteralPath $installedPath -Force).Length } else { [int64]$script:Policy.limits.max_config_bytes }
        $raw = if ($classification.Kind -eq 'sealed') { $null } else { Read-FileBytes $item.FullName $readLimit }
        $entry = New-Entry $relative $item.FullName $raw $classification.Kind $classification.Path $null
        $entries += $entry
    }
    return ,$entries
}

function Test-ZipSymlink([System.IO.Compression.ZipArchiveEntry]$Entry) {
    $mode = ([int64]$Entry.ExternalAttributes -shr 16) -band 0xffff
    return (($mode -band 0xf000) -eq 0xa000)
}

function Test-ZipDirectory([System.IO.Compression.ZipArchiveEntry]$Entry) {
    if ($Entry.FullName.EndsWith('/')) { return $true }
    return (([int64]$Entry.ExternalAttributes -band 0x10) -ne 0 -and $Entry.Length -eq 0)
}

function Assert-ZipRegularEntry([System.IO.Compression.ZipArchiveEntry]$Entry) {
    $mode = ([int64]$Entry.ExternalAttributes -shr 16) -band 0xffff
    $type = $mode -band 0xf000
    if ($type -ne 0 -and $type -ne 0x8000) { Fail 'ZIP_NONREGULAR_ENTRY' }
}

function Get-SourceZipEntries([string]$ZipPath) {
    try { Add-Type -AssemblyName System.IO.Compression,System.IO.Compression.FileSystem -ErrorAction Stop | Out-Null } catch { Fail 'ZIP_UNSUPPORTED' }
    $stream = $null; $archive = $null
    try {
        $stream = [IO.File]::Open($ZipPath, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        $archive = New-Object System.IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Read, $false)
        if ($archive.Entries.Count -gt [int64]$script:Policy.limits.max_entries) { Fail 'ENTRY_COUNT_LIMIT' }
        $seen = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
        $entries = @()
        foreach ($zipEntry in $archive.Entries) {
            $name = $zipEntry.FullName
            if ([string]::IsNullOrWhiteSpace($name) -or $name.Contains('\')) { Fail 'PATH_INVALID' }
            $isDirectory = Test-ZipDirectory $zipEntry
            $relative = $name.TrimEnd('/')
            Assert-SafeRelativePath $relative
            if (-not $seen.Add($relative)) { Fail 'DUPLICATE_ENTRY' }
            if (Test-ZipSymlink $zipEntry) { Fail 'ZIP_LINK_NOT_ALLOWED' }
            if ($isDirectory) {
                Fail 'DIRECTORY_NOT_ALLOWED'
            }
            Assert-ZipRegularEntry $zipEntry
            $classification = Get-FileClassification $relative
            if ($classification.Kind -eq 'sealed') {
                $entry = New-Entry $relative $null $null $classification.Kind $classification.Path $zipEntry
            } else {
                $installedPath = Get-InstalledFile $relative
                $readLimit = if ($classification.Kind -eq 'ignored' -and $null -ne $installedPath) { [int64](Get-Item -LiteralPath $installedPath -Force).Length } else { [int64]$script:Policy.limits.max_config_bytes }
                $raw = Read-ZipEntryBytes $zipEntry $readLimit
                $entry = New-Entry $relative $null $raw $classification.Kind $classification.Path $zipEntry
            }
            $entries += $entry
        }
        return ,$entries
    } catch [System.InvalidOperationException] { throw }
    catch { Fail 'ZIP_INVALID' }
    finally {
        if ($null -ne $archive) { $archive.Dispose() }
        if ($null -ne $stream) { $stream.Dispose() }
    }
}

function Assert-ExactShippedQuadlet([string]$Name, [byte[]]$Bytes, [string]$InstalledPath) {
    $installedBytes = Get-InstalledNormalized $InstalledPath
    if ((Get-Sha256Bytes $Bytes) -ne (Get-Sha256Bytes $installedBytes)) { Fail 'QUADLET_NOT_EXACT' }
}

function Assert-Configuration([object[]]$Entries, [object]$InstalledProfile, [object]$InstalledImages) {
    $installedQuadlets = @($InstalledProfile.quadlets | ForEach-Object { ([string]$_).ToLowerInvariant() } | Sort-Object)
    $policyQuadlets = @($script:Policy.quadlets | ForEach-Object { ([string]$_).ToLowerInvariant() } | Sort-Object)
    if ((@($installedQuadlets) -join '|') -ne (@($policyQuadlets) -join '|')) { Fail 'POLICY_INVALID' }
    $byPath = @{}
    foreach ($entry in $Entries) { if ($entry.Kind -eq 'accepted') { $byPath[$entry.Path.ToLowerInvariant()] = $entry } }
    foreach ($required in @($script:Policy.required_files)) { if (-not $byPath.ContainsKey(([string]$required).ToLowerInvariant())) { Fail 'REQUIRED_FILE_MISSING' } }
    foreach ($quadlet in @($InstalledProfile.quadlets)) {
        if (-not $byPath.ContainsKey(([string]$quadlet).ToLowerInvariant())) { Fail 'QUADLET_MISSING' }
    }
    $profileEntry = $byPath['profile.json']; $imagesEntry = $byPath['images.json']
    if ($null -eq $profileEntry.Json) { try { $profileEntry.Json = (Get-StrictUtf8 $profileEntry.Bytes | ConvertFrom-Json) } catch { Fail 'JSON_INVALID' } }
    if ($null -eq $imagesEntry.Json) { try { $imagesEntry.Json = (Get-StrictUtf8 $imagesEntry.Bytes | ConvertFrom-Json) } catch { Fail 'JSON_INVALID' } }
    Assert-ProfileName $profileEntry.Json
    Assert-ResourceBounds $profileEntry.Json
    if ((Get-CanonicalJson (Get-ComparableProfile $profileEntry.Json)) -ne (Get-CanonicalJson (Get-ComparableProfile $InstalledProfile))) { Fail 'PROFILE_IMMUTABLE_FIELD' }
    if ((Get-CanonicalJson $imagesEntry.Json) -ne (Get-CanonicalJson $InstalledImages)) { Fail 'IMAGES_IMMUTABLE' }
    $network = $byPath[[string]$script:NetworkFileName.ToLowerInvariant()]
    if ($null -eq $network) { Fail 'REQUIRED_FILE_MISSING' }
    $installedNetwork = Get-InstalledFile $script:NetworkFileName
    if ((Get-Sha256Bytes $network.Bytes) -ne (Get-Sha256Bytes (Get-InstalledNormalized $installedNetwork))) { Fail 'NETWORK_IMMUTABLE' }
    $offline = $byPath['offline.sha256']
    $installedOffline = Get-InstalledFile 'offline.sha256'
    if ((Get-Sha256Bytes $offline.Bytes) -ne (Get-Sha256Bytes (Get-InstalledNormalized $installedOffline))) { Fail 'OFFLINE_MANIFEST_IMMUTABLE' }
    foreach ($quadlet in @($InstalledProfile.quadlets)) {
        $entry = $byPath[[string]$quadlet.ToLowerInvariant()]
        Assert-ExactShippedQuadlet ([string]$quadlet) $entry.Bytes (Get-InstalledFile ([string]$quadlet))
    }
}

function Get-Revision([object[]]$Entries) {
    $hash = [Security.Cryptography.SHA256]::Create()
    $stream = New-Object IO.MemoryStream
    try {
        foreach ($entry in @($Entries | Where-Object { $_.Kind -eq 'accepted' } | Sort-Object Path)) {
            $line = [Text.Encoding]::UTF8.GetBytes($entry.Path + "`0" + $entry.Bytes.Length.ToString([Globalization.CultureInfo]::InvariantCulture) + "`0")
            $stream.Write($line, 0, $line.Length)
            $stream.Write($entry.Bytes, 0, $entry.Bytes.Length)
        }
        return (-join ($hash.ComputeHash($stream.ToArray()) | ForEach-Object { $_.ToString('x2') }))
    } finally { $stream.Dispose(); $hash.Dispose() }
}

function Get-ManagedAccountSid {
    try {
        $account = New-Object System.Security.Principal.NTAccount("$env:COMPUTERNAME\gnxnodesvc")
        return $account.Translate([System.Security.Principal.SecurityIdentifier])
    } catch { Fail 'ACL_PRINCIPAL_INVALID' }
}

function New-ExactSecurity([bool]$Directory) {
    $security = if ($Directory) { New-Object System.Security.AccessControl.DirectorySecurity } else { New-Object System.Security.AccessControl.FileSecurity }
    $security.SetAccessRuleProtection($true, $false)
    $system = New-Object System.Security.Principal.SecurityIdentifier('S-1-5-18')
    $admins = New-Object System.Security.Principal.SecurityIdentifier('S-1-5-32-544')
    $managed = Get-ManagedAccountSid
    $inheritance = if ($Directory) { [System.Security.AccessControl.InheritanceFlags]::ContainerInherit -bor [System.Security.AccessControl.InheritanceFlags]::ObjectInherit } else { [System.Security.AccessControl.InheritanceFlags]::None }
    $propagation = [System.Security.AccessControl.PropagationFlags]::None
    $full = [System.Security.AccessControl.FileSystemRights]::FullControl
    $readExecute = [System.Security.AccessControl.FileSystemRights]::ReadAndExecute
    foreach ($sid in @($system, $admins)) {
        $security.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule -ArgumentList @($sid, $full, $inheritance, $propagation, [System.Security.AccessControl.AccessControlType]::Allow)))
    }
    $security.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule -ArgumentList @($managed, $readExecute, $inheritance, $propagation, [System.Security.AccessControl.AccessControlType]::Allow)))
    return $security
}

function Set-ExactAcl([string]$Path, [bool]$Directory) {
    try {
        $item = Get-Item -LiteralPath $Path -Force
        if (($item.PSIsContainer -ne $Directory) -or (Test-Reparse $item)) { Fail 'STATE_PATH_INVALID' }
        $security = Get-Acl -LiteralPath $Path
        $security.SetAccessRuleProtection($true, $false)
        foreach ($rule in @($security.Access)) { [void]$security.RemoveAccessRuleSpecific($rule) }
        $exact = New-ExactSecurity $Directory
        foreach ($rule in @($exact.Access)) { [void]$security.AddAccessRule($rule) }
        Set-Acl -LiteralPath $Path -AclObject $security
    } catch [System.InvalidOperationException] { throw }
    catch { Fail 'ACL_UPDATE_FAILED' }
}

function Ensure-SecureDirectory([string]$Path) {
    if (Test-Path -LiteralPath $Path) {
        Set-ExactAcl $Path $true
        return
    }
    try {
        $parent = Split-Path -Parent $Path
        if (-not (Test-Path -LiteralPath $parent)) { Fail 'STATE_PATH_INVALID' }
        $parentItem = Get-Item -LiteralPath $parent -Force
        if (-not $parentItem.PSIsContainer -or (Test-Reparse $parentItem)) { Fail 'STATE_PATH_INVALID' }
        $directory = New-Object System.IO.DirectoryInfo($Path)
        $directory.Create((New-ExactSecurity $true))
    } catch [System.InvalidOperationException] { throw }
    catch { Fail 'STATE_WRITE_FAILED' }
    Set-ExactAcl $Path $true
}

function Protect-Tree([string]$Root) {
    Set-ExactAcl $Root $true
    foreach ($item in @(Get-ChildItem -LiteralPath $Root -Force -Recurse)) {
        if (Test-Reparse $item) { Fail 'REPARSE_POINT_NOT_ALLOWED' }
        Set-ExactAcl $item.FullName ([bool]$item.PSIsContainer)
    }
}

function Write-ReceiptAtomic([string]$Path, [object]$Receipt, [string]$Parent) {
    $script:ReceiptTemp = Join-Path $Parent ('.current.' + [Guid]::NewGuid().ToString('N') + '.json')
    try {
        $json = $Receipt | ConvertTo-Json -Compress -Depth 20
        [IO.File]::WriteAllText($script:ReceiptTemp, $json + "`n", (New-Object System.Text.UTF8Encoding($false)))
        Set-ExactAcl $script:ReceiptTemp $false
        if (Test-Path -LiteralPath $Path) {
            [IO.File]::Replace($script:ReceiptTemp, $Path, [NullString]::Value)
        } else {
            [IO.File]::Move($script:ReceiptTemp, $Path)
        }
        $script:ReceiptTemp = $null
    } catch [System.InvalidOperationException] { throw }
    catch { Fail 'RECEIPT_WRITE_FAILED' }
}

function Import-Bundle([object[]]$Entries, [string]$Revision, [int64]$Bytes, [string[]]$Files) {
    $bundleRoot = Join-Path $StateDir 'workload-bundles'
    $workloadRoot = Join-Path $bundleRoot $script:WorkloadName
    $revisionDir = Join-Path $workloadRoot $Revision
    Ensure-SecureDirectory $bundleRoot
    Ensure-SecureDirectory $workloadRoot
    if (Test-Path -LiteralPath $revisionDir) {
        $existing = Get-Item -LiteralPath $revisionDir -Force
        if (-not $existing.PSIsContainer -or (Test-Reparse $existing)) { Fail 'REVISION_CONFLICT' }
        $existingFiles = @(Get-ChildItem -LiteralPath $revisionDir -File -Force -Recurse | ForEach-Object { Get-RelativePath $revisionDir $_.FullName })
        $expectedFiles = @($Files) + @('receipt.json')
        if ((@($existingFiles | Sort-Object) -join '|') -ne (@($expectedFiles | Sort-Object) -join '|')) { Fail 'REVISION_CONFLICT' }
        foreach ($entry in @($Entries | Where-Object { $_.Kind -eq 'accepted' })) {
            $target = Join-Path $revisionDir ($entry.Path.Replace('/', '\'))
            if ((Get-Sha256File $target) -ne (Get-Sha256Bytes $entry.Bytes)) { Fail 'REVISION_CONFLICT' }
        }
        $revisionReceipt = Join-Path $revisionDir 'receipt.json'
        try { $storedReceipt = Get-Content -LiteralPath $revisionReceipt -Raw | ConvertFrom-Json } catch { Fail 'REVISION_CONFLICT' }
        if ($storedReceipt.schema -ne 1 -or $storedReceipt.workload -ne $script:WorkloadName -or $storedReceipt.revision -ne $Revision) { Fail 'REVISION_CONFLICT' }
        foreach ($storedFile in @($storedReceipt.files)) {
            if ($Files -notcontains [string]$storedFile.path) { Fail 'REVISION_CONFLICT' }
            $storedTarget = Join-Path $revisionDir ([string]$storedFile.path).Replace('/', '\')
            if ((Get-Sha256File $storedTarget) -ne [string]$storedFile.sha256 -or (Get-Item -LiteralPath $storedTarget).Length -ne [int64]$storedFile.bytes) { Fail 'REVISION_CONFLICT' }
        }
        Protect-Tree $revisionDir
    } else {
        Ensure-SecureDirectory $revisionDir
        $script:PartialRevision = $revisionDir
        foreach ($entry in @($Entries | Where-Object { $_.Kind -eq 'accepted' })) {
            $target = Join-Path $revisionDir ($entry.Path.Replace('/', '\'))
            $parent = Split-Path -Parent $target
            if ($parent -ne $revisionDir) { Ensure-SecureDirectory $parent }
            try { [IO.File]::WriteAllBytes($target, [byte[]]$entry.Bytes) } catch { Fail 'BUNDLE_WRITE_FAILED' }
        }
        $receiptFiles = @($Entries | Where-Object { $_.Kind -eq 'accepted' } | Sort-Object Path | ForEach-Object {
            [ordered]@{ path = [string]$_.Path; sha256 = Get-Sha256Bytes $_.Bytes; bytes = [int64]$_.Bytes.Length }
        })
        $revisionReceipt = [ordered]@{ schema = 1; workload = $script:WorkloadName; revision = $Revision; files = $receiptFiles }
        try {
            [IO.File]::WriteAllText((Join-Path $revisionDir 'receipt.json'), (($revisionReceipt | ConvertTo-Json -Compress -Depth 10) + "`n"), (New-Object System.Text.UTF8Encoding($false)))
        } catch { Fail 'BUNDLE_WRITE_FAILED' }
        Protect-Tree $revisionDir
        $script:PartialRevision = $null
    }
    $receipt = [ordered]@{ workload = $script:WorkloadName; revision = $Revision; bundle_dir = (Get-FullPath $revisionDir); files = @($Files); bytes = $Bytes }
    Write-ReceiptAtomic (Join-Path $workloadRoot 'current.json') $receipt $workloadRoot
    return $receipt
}

try {
    $sourceFull = Get-FullPath $SourcePath
    $nodeFull = Get-FullPath $NodePayload
    $stateFull = Get-FullPath $StateDir
    $StateDir = $stateFull
    Assert-LocalPath $sourceFull; Assert-LocalPath $nodeFull; Assert-LocalPath $stateFull
    $policyPath = Join-Path $PSScriptRoot 'workload-import-policy.json'
    try { $script:Policy = (Get-Content -LiteralPath $policyPath -Raw | ConvertFrom-Json) } catch { Fail 'POLICY_INVALID' }
    if ([int]$script:Policy.schema -ne 1 -or [string]$script:Policy.profile.name -ne 'ai') { Fail 'POLICY_INVALID' }
    $script:WorkloadName = [string]$script:Policy.profile.name
    $payloadItem = Get-Item -LiteralPath $nodeFull -Force
    if (-not $payloadItem.PSIsContainer -or (Test-Reparse $payloadItem)) { Fail 'NODE_PAYLOAD_INVALID' }
    $script:InstalledRoot = Join-Path (Join-Path $nodeFull 'workloads') $script:WorkloadName
    $installedItem = Get-Item -LiteralPath $script:InstalledRoot -Force
    if (-not $installedItem.PSIsContainer -or (Test-Reparse $installedItem)) { Fail 'INSTALLED_PROFILE_MISSING' }
    $script:InstalledIndex = Get-RelativeInstalledFiles
    $installedProfilePath = Get-InstalledFile 'profile.json'
    $installedImagesPath = Get-InstalledFile 'images.json'
    if ($null -eq $installedProfilePath -or $null -eq $installedImagesPath) { Fail 'INSTALLED_PROFILE_MISSING' }
    try { $installedProfile = (Get-StrictUtf8 (Read-FileBytes $installedProfilePath) | ConvertFrom-Json) } catch { Fail 'INSTALLED_PROFILE_INVALID' }
    try { $installedImages = (Get-StrictUtf8 (Read-FileBytes $installedImagesPath) | ConvertFrom-Json) } catch { Fail 'INSTALLED_IMAGES_INVALID' }
    Assert-ProfileName $installedProfile
    $networkProperty = $installedProfile.PSObject.Properties['network']
    if ($null -eq $networkProperty -or $null -eq $networkProperty.Value.PSObject.Properties['quadlet']) { Fail 'INSTALLED_PROFILE_INVALID' }
    $script:NetworkFileName = [string]$networkProperty.Value.quadlet
    if ($script:NetworkFileName -notmatch '^[a-z0-9][a-z0-9._-]{0,63}$' -or
        @($script:Policy.required_files | ForEach-Object { ([string]$_).ToLowerInvariant() }) -notcontains $script:NetworkFileName.ToLowerInvariant()) { Fail 'POLICY_INVALID' }
    [void](Get-InstalledImages $installedImages)
    $sourceItem = Get-Item -LiteralPath $sourceFull -Force
    if (Test-Reparse $sourceItem) { Fail 'SOURCE_REPARSE_POINT' }
    $entries = if ($sourceItem.PSIsContainer) { Get-SourceFolderEntries $sourceFull } elseif ($sourceFull.ToLowerInvariant().EndsWith('.zip')) { Get-SourceZipEntries $sourceFull } else { Fail 'SOURCE_KIND_INVALID' }
    if ($entries.Count -gt [int64]$script:Policy.limits.max_entries) { Fail 'ENTRY_COUNT_LIMIT' }
    $accepted = @($entries | Where-Object { $_.Kind -eq 'accepted' })
    $configBytes = [int64](($accepted | ForEach-Object { [int64]$_.Bytes.Length } | Measure-Object -Sum).Sum)
    if ($configBytes -gt [int64]$script:Policy.limits.max_config_bytes) { Fail 'CONFIG_SIZE_LIMIT' }
    Assert-Configuration $entries $installedProfile $installedImages
    $revision = Get-Revision $entries
    $files = @($accepted | Sort-Object Path | ForEach-Object { [string]$_.Path })
    $bundleRoot = Join-Path (Join-Path $stateFull 'workload-bundles') $script:WorkloadName
    $revisionDir = Join-Path $bundleRoot $revision
    if ($Mode -eq 'Import') {
        $result = Import-Bundle $entries $revision $configBytes $files
    } else {
        $result = [ordered]@{ workload = [string]$script:WorkloadName; revision = $revision; bundle_dir = (Get-FullPath $revisionDir); files = @($files); bytes = $configBytes }
    }
    Write-Output ($result | ConvertTo-Json -Compress -Depth 20)
    exit 0
} catch {
    if ($null -ne $script:ReceiptTemp -and (Test-Path -LiteralPath $script:ReceiptTemp)) { Remove-Item -LiteralPath $script:ReceiptTemp -Force -ErrorAction SilentlyContinue }
    if ($null -ne $script:PartialRevision -and (Test-Path -LiteralPath $script:PartialRevision)) { Remove-Item -LiteralPath $script:PartialRevision -Recurse -Force -ErrorAction SilentlyContinue }
    $code = 'ADMISSION_FAILED'
    if ($_.Exception.Message -match '^GNX_BUNDLE:([A-Z0-9_]+)$') { $code = $matches[1] }
    $errorObject = [ordered]@{ error = [ordered]@{ code = $code; message = $code } }
    [Console]::Error.WriteLine(($errorObject | ConvertTo-Json -Compress -Depth 5))
    exit 1
}
