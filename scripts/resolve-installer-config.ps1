param(
    [Parameter(Mandatory)][ValidateSet('push', 'workflow_dispatch')][string]$EventName,
    [Parameter(Mandatory)][string]$Ref,
    [string]$CliMode,
    [string]$PayloadRelease,
    [string]$PayloadSha256,
    [switch]$PublishRelease
)
$ErrorActionPreference = 'Stop'
# Resolve the event before considering manual inputs. A push can never publish,
# even if another caller passes publication inputs to the reusable workflow.
if ($EventName -eq 'push') {
    if ($Ref -cne 'refs/heads/main') { throw 'Automatic installers require a push to main' }
    $pin = Get-Content (Join-Path $PSScriptRoot '../config/ci-offline-inputs.json') -Raw | ConvertFrom-Json
    if ($pin.asset -cne 'gnx-offline-inputs.zip' -or $pin.bytes -le 0) { throw 'Invalid offline input pin' }
    $CliMode = 'preview'
    $PayloadRelease = $pin.release
    $PayloadSha256 = $pin.sha256
    $PublishRelease = $false
    $bytes = $pin.bytes
} else {
    if ($CliMode -cnotin @('versioned', 'source')) { throw 'Select manual CLI mode: versioned or source' }
    if ($PublishRelease -and $Ref -cnotmatch '^refs/tags/v\d+\.\d+\.\d+$') {
        throw 'Publication requires workflow_dispatch on the matching version tag'
    }
    $bytes = 0 # Manual input releases are verified by their explicitly supplied hash.
}
if ($PayloadRelease -cnotmatch '^[a-zA-Z0-9][a-zA-Z0-9._-]*$') { throw 'Invalid input release tag' }
if ($PayloadSha256 -cnotmatch '^[0-9a-fA-F]{64}$') { throw 'Invalid archive SHA256' }
[pscustomobject]@{
    CliMode = $CliMode
    Preview = ($CliMode -eq 'preview')
    OutputRoot = $(if ($CliMode -eq 'preview') { 'build/previews' } else { 'build/releases' })
    PayloadRelease = $PayloadRelease
    PayloadSha256 = $PayloadSha256.ToLowerInvariant()
    PayloadBytes = $bytes
    PublishRelease = [bool]$PublishRelease
}
