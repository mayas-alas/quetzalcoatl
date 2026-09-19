# Compatibility entry point; credentials are entered only in GNX's hidden prompt.
[CmdletBinding()]
param()
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
& (Join-Path $repo 'target\release\gnx.exe') access configure --config (Join-Path $repo 'runtime\access\access.toml')
exit $LASTEXITCODE
