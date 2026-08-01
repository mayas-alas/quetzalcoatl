[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$platformRoot = Join-Path $repoRoot 'platform'
$lockPath = Join-Path $platformRoot 'platform.lock.json'

$files = @(
    Get-ChildItem -LiteralPath $platformRoot -Recurse -Force -File |
        Where-Object FullName -ne $lockPath |
        ForEach-Object {
            $relative = $_.FullName.Substring($platformRoot.Length + 1).Replace('\', '/')
            if ($relative -cne $relative.ToLowerInvariant()) {
                throw "Platform paths must be lowercase: $relative"
            }
            $mode = if ($relative -in @(
                    'operations/deploy',
                    'operations/forgejo-admin',
                    'operations/discover-releases.py',
                    'operations/lxc-host',
                    'operations/lxc-service',
                    'operations/reconcile',
                    'operations/verify-release.py',
                    'tofu/foundation/entrypoint',
                    'tofu/service/entrypoint'
                )) { '0755' } else { '0644' }
            [ordered]@{
                path = $relative
                mode = $mode
                sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            }
        } |
        Sort-Object path
)

$lock = [ordered]@{
    schema_version = 1
    bundle_contract = 1
    policy = [ordered]@{
        mutable_image_tags_allowed = $false
        embedded_secrets_allowed = $false
        repository_commands_allowed = $false
    }
    files = $files
}

$json = $lock | ConvertTo-Json -Depth 6
[System.IO.File]::WriteAllText($lockPath, "$json`n", [System.Text.UTF8Encoding]::new($false))
Write-Host "Generated $lockPath with $($files.Count) files."
