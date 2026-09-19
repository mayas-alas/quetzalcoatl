$ErrorActionPreference = 'Stop'
$files = @('build.ps1', 'install.ps1', 'setup-ui.ps1', 'uninstall.ps1') | ForEach-Object { Join-Path $PSScriptRoot $_ }
$assetHashes = @{
    'branding-install-logo.ico' = '1f1e54880c65036a057169a32af3b114d9a7cfe2ab28840f8a389606556ab3a5'
    'branding-install-logo.png' = 'ac4b4ea86c58d1a2e61521ff1f31c50a2faec306c2990c6e148ac5a7ff408bad'
    'banner-install-side.png' = 'd1f12c614f608440d7f43aac2d53c1f54720ed8771216ac221557ce7f0f2f0e0'
    'bg-installer-banner.png' = 'dac8ba6b71216482f4f97ab78cc739125cf067070cda4a647956581ebc03647f'
    'tray-icon.ico' = 'b0aa0d235a77b722fb54962077725ea20b86176f321ecd98c6940775ec9a1579'
    'tray-icon.png' = 'a259230f9ed86e0b4a1d220530531909460a615e88417f126d8d722858199195'
}
foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Required packaging script missing: $file" }
    $tokens = $null; $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile($file, [ref]$tokens, [ref]$errors) | Out-Null
    if ($errors.Count) { throw "PowerShell parse failed: $file" }
}
$ui = Get-Content (Join-Path $PSScriptRoot 'setup-ui.ps1') -Raw
foreach ($guard in @('Malformed setup progress event', 'Unsupported setup progress event', 'Invalid started event', 'Invalid terminal state', 'Apply must remain unavailable', 'Setup transport failed', 'Setup exit state mismatch')) {
    if ($ui -notmatch [regex]::Escape($guard)) { throw "UI lacks observable transport rejection: $guard" }
}
if ($ui -notmatch 'READY\s*=\s*0;\s*FAILED\s*=\s*1;\s*ACTION_REQUIRED\s*=\s*2') { throw 'UI exit/state contract changed; review acceptance mapping.' }
foreach ($state in @('RUNNING', 'READY', 'ACTION_REQUIRED', 'FAILED')) {
    if ($ui -notmatch [regex]::Escape($state)) { throw "Missing finite UI state: $state" }
}
foreach ($forbidden in @('PSCredential', 'Start-Service', 'New-Service', 'icacls')) {
    if ($ui -match [regex]::Escape($forbidden)) { throw "UI duplicates privileged setup policy: $forbidden" }
}
$installer = Get-Content (Join-Path $PSScriptRoot 'install.ps1') -Raw
foreach ($name in @('gnx.exe', 'gnx-service.exe', 'gnx-setup.exe', 'gnx-linux', 'gnx-linux-bundle.tar', 'gnx-linux.run')) {
    if ($installer -notmatch [regex]::Escape($name)) { throw "Installer omits release artifact: $name" }
}
$uninstaller = Get-Content (Join-Path $PSScriptRoot 'uninstall.ps1') -Raw
foreach ($owned in @('C:\Program Files\GNX-0.3.1', 'C:\ProgramData\GNX-0.3.1', 'C:\ProgramData\GNX-Setup-0.3.1', 'GNXRuntime', 'gnx-runtime')) {
    if ($uninstaller -notmatch [regex]::Escape($owned)) { throw "Uninstaller omits owned object: $owned" }
}
foreach ($guard in @('Assert-PlainTree', 'ReparsePoint', 'PathName', 'StartName', 'Description')) {
    if ($uninstaller -notmatch [regex]::Escape($guard)) { throw "Uninstaller lacks ownership guard: $guard" }
}
$build = Get-Content (Join-Path $PSScriptRoot 'build.ps1') -Raw
foreach ($packaged in @('packaging/windows/uninstall.ps1', 'packaging/windows/uninstall-checklist.md')) {
    if ($build -notmatch [regex]::Escape($packaged)) { throw "Build does not package uninstall file: $packaged" }
}
$provenance = Get-Content (Join-Path $PSScriptRoot 'assets/PROVENANCE.md') -Raw
foreach ($asset in $assetHashes.Keys) {
    $path = Join-Path $PSScriptRoot "assets/$asset"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing Windows branding asset: $asset" }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $assetHashes[$asset]) { throw "Windows branding asset hash changed: $asset" }
    if ($build -notmatch [regex]::Escape("packaging/windows/assets/$asset")) { throw "Build does not package Windows branding asset: $asset" }
    if ($provenance -notmatch [regex]::Escape("origin/legacy:assets/$asset") -or $provenance -notmatch $assetHashes[$asset]) { throw "Missing provenance for Windows branding asset: $asset" }
}
$checklist = Get-Content (Join-Path $PSScriptRoot 'uninstall-checklist.md') -Raw
foreach ($required in @('GNXRuntime', 'gnx-runtime', 'C:\Program Files\GNX-0.3.1', 'C:\ProgramData\GNX-Setup-0.3.1', 'GNX-0.3.1', 'Path', 'Run')) {
    if ($checklist -notmatch [regex]::Escape($required)) { throw "Uninstall checklist omits evidence item: $required" }
}
if ($checklist -notmatch 'No GNX-owned tray app exists') { throw 'Tray gap is not recorded.' }
$audit = Get-Content (Join-Path $PSScriptRoot '../../docs/AUDIT-INSTALL-REBOOT.md') -Raw
foreach ($gate in @('service', 'account-sid', 'wsl-distro', 'journal-lock', 'acl', 'path-autorun', 'residue', 'runtime-ready')) {
    if ($audit -notmatch [regex]::Escape("Gate '$gate'")) { throw "Post-reboot audit gate missing: $gate" }
}
foreach ($contract in @('POST_REBOOT_READY', 'doctor_failed', 'status_failed', 'setup_lock_held', 'unexpected_setup_residue', 'BLOCKED', 'SETUP_PROVISIONED')) {
    if ($audit -notmatch [regex]::Escape($contract)) { throw "Post-reboot audit contract missing: $contract" }
}
$setupSource = Get-Content (Join-Path $PSScriptRoot '../../src/adapter/windows/setup.rs') -Raw
foreach ($phase in @('PREFLIGHT', 'STAGING', 'PUBLISHING', 'REGISTERING', 'SECURING', 'PROVISIONED')) {
    if ($setupSource -notmatch ('"' + $phase + '"')) { throw "Documented provisioning phase missing from implementation: $phase" }
    if ($checklist -notmatch ('`' + $phase + '`')) { throw "Missing lifecycle phase definition: $phase" }
}
foreach ($guard in @('ACCOUNT_SID_CONFLICT', 'ACCOUNT_REFERENCED', 'WSL_RUNTIME_CONTEXT_REQUIRED', 'WSL_RUNTIME_CONTEXT_UNVERIFIED', 'SERVICE_DELETE_PENDING', 'RESOURCES_REMAIN', 'PATH_REMAINS', 'ROOT_REMAINS', 'NATIVE_OPERATION_FAILED')) {
    if ($uninstaller -notmatch [regex]::Escape($guard)) { throw "Uninstaller lacks observable failure guard: $guard" }
}
if ($uninstaller -notmatch 'if\s*\(\$LASTEXITCODE\s*-ne\s*0\)') { throw 'Uninstaller must reject native command failure.' }
if ($uninstaller -notmatch "result='BLOCKED'" -or $uninstaller -notmatch 'exit 1') { throw 'Uninstaller must report failure with nonzero exit.' }
foreach ($phase in @('DRAINING', 'UNREGISTERING', 'CLEANING', 'REMOVED', 'BLOCKED', 'RECOVERY_REQUIRED')) {
    if ($checklist -notmatch ('`' + $phase + '`')) { throw "Missing removal/recovery classification: $phase" }
}
foreach ($evidence in @('Failure and retry matrix', 'Evidence procedure', 'BasePath', 'SID', 'ReparsePoint', 'reboot', 'foreign', 'exit code', 'residue', 'SETUP_PROVISIONED')) {
    if ($checklist -notmatch [regex]::Escape($evidence)) { throw "Missing lifecycle acceptance requirement: $evidence" }
}
Write-Output 'Windows setup packaging static checks passed; host lifecycle acceptance remains a separate gate.'
