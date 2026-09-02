[CmdletBinding()]
param([Parameter(Mandatory)][ValidatePattern('^[D-Zd-z]$')][string]$DriveLetter)
$ErrorActionPreference = 'Stop'
$backups = Join-Path $env:LOCALAPPDATA 'GNX\backups'
$statusPath = Join-Path $backups 'latest.json'
$status = Get-Content -LiteralPath $statusPath -Raw | ConvertFrom-Json
if ($status.result -ne 'READY' -or -not $status.roundtrip_verified) { throw 'No verified backup is available.' }
$archive = [IO.Path]::GetFullPath($status.archive)
if ([IO.Path]::GetDirectoryName($archive) -ne $backups) { throw 'Backup is outside the protected source directory.' }
$disk = @(Get-Partition -DriveLetter $DriveLetter | Get-Disk)
if ($disk.Count -ne 1 -or $disk[0].BusType -ne 'USB' -or $disk[0].IsReadOnly) { throw 'Destination must be one writable USB disk.' }
$volume = Get-Volume -DriveLetter $DriveLetter
if ($volume.SizeRemaining -lt ((Get-Item -LiteralPath $archive).Length + 1048576)) { throw 'USB does not have enough free space.' }
$destination = "$($DriveLetter.ToUpperInvariant()):\GNX-backups"
New-Item -ItemType Directory -Path $destination -Force | Out-Null
if ((Get-Item -LiteralPath $destination).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Refusing a redirected destination folder.' }
foreach ($source in @($archive, [IO.Path]::ChangeExtension($archive, 'json'))) {
    $target = Join-Path $destination ([IO.Path]::GetFileName($source))
    $expected = (Get-FileHash -Algorithm SHA256 -LiteralPath $source).Hash
    if (-not (Test-Path -LiteralPath $target)) {
        $partial = "$target.partial"
        $sourceStream = [IO.File]::OpenRead($source)
        try {
            $destinationStream = [IO.File]::Open($partial, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
            try { $sourceStream.CopyTo($destinationStream); $destinationStream.Flush($true) }
            finally { $destinationStream.Dispose() }
        } finally { $sourceStream.Dispose() }
        if ((Get-FileHash -Algorithm SHA256 -LiteralPath $partial).Hash -ne $expected) { throw 'USB verification failed; local backup is unchanged.' }
        [IO.File]::Move($partial, $target)
    }
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $target).Hash -ne $expected) { throw 'A different destination file exists; refusing to overwrite it.' }
}
$status.external_copy = $true
$status | Add-Member -NotePropertyName external_archive -NotePropertyValue (Join-Path $destination ([IO.Path]::GetFileName($archive))) -Force
$status | ConvertTo-Json | Set-Content -LiteralPath $statusPath
'READY verified-usb-backup'
