param(
    [Parameter(Mandatory)][string]$Archive,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-fA-F]{64}$')][string]$Sha256
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$archivePath = (Resolve-Path -LiteralPath $Archive).Path
if ((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash -ne $Sha256) {
    throw 'Offline input archive SHA256 mismatch'
}
$metadata = cargo metadata --manifest-path (Join-Path $repo 'Cargo.toml') --no-deps --locked --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
$version = ($metadata.packages | Where-Object name -eq 'gnx').version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid package version' }
$payload = Join-Path $repo 'build/payload'
$output = Join-Path $repo "build/releases/v$version"
if ((Test-Path -LiteralPath $payload) -or (Test-Path -LiteralPath $output)) {
    throw 'CI input preparation requires an empty build destination; nothing overwritten'
}
# Exact allowlist: no generic archive extraction, paths, links or executable hooks.
$names = @('rootfs.tar', 'images/mesh.oci.tar', 'images/dns.oci.tar',
    'images/gateway.oci.tar', 'images/compute.oci.tar', 'base-image.digest',
    'config/images.json', 'config/bootc.json')
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($archivePath)
try {
    $entries = @{}
    foreach ($entry in $zip.Entries) {
        if ($entry.FullName -cnotin $names -or $entries.ContainsKey($entry.FullName) -or
            $entry.Length -le 0 -or $entry.Length -gt 8GB) {
            throw 'Unexpected, duplicate, empty or oversized offline input'
        }
        $entries[$entry.FullName] = $entry
    }
    if ($entries.Count -ne $names.Count) { throw 'Offline archive incomplete' }
    if ($entries['base-image.digest'].Length -gt 128 -or
        $entries['config/images.json'].Length -gt 65536 -or
        $entries['config/bootc.json'].Length -gt 65536) { throw 'Offline metadata exceeds limit' }
    $total = ($zip.Entries | Measure-Object Length -Sum).Sum
    if ($total -gt 12GB) { throw 'Offline archive expanded size exceeds limit' }
    foreach ($name in 'config/images.json', 'config/bootc.json') {
        $stream = $entries[$name].Open()
        $sha = [Security.Cryptography.SHA256]::Create()
        try { $hash = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '') }
        finally { $stream.Dispose(); $sha.Dispose() }
        if ($hash -ne (Get-FileHash -LiteralPath (Join-Path $repo $name)).Hash) {
            throw "Offline input provenance differs from checkout: $name"
        }
    }
    $reader = [IO.StreamReader]::new($entries['base-image.digest'].Open())
    try { $digest = $reader.ReadToEnd() } finally { $reader.Dispose() }
    if ($digest.Length -ne 71 -or $digest -cnotmatch '^sha256:[0-9a-f]{64}$') { throw 'Invalid sealed base digest (no whitespace permitted)' }
    New-Item -ItemType Directory -Path $payload, (Join-Path $payload 'images'), $output | Out-Null
    foreach ($name in $names | Where-Object { $_ -ne 'base-image.digest' -and -not $_.StartsWith('config/') }) {
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[$name], (Join-Path $payload $name), $false)
    }
    [IO.File]::WriteAllText((Join-Path $output 'base-image.digest'), $digest, [Text.UTF8Encoding]::new($false))
} finally { $zip.Dispose() }
