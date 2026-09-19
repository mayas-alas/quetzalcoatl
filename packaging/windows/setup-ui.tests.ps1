$ErrorActionPreference = 'Stop'
$files = @('build.ps1', 'install.ps1', 'setup-ui.ps1') | ForEach-Object { Join-Path $PSScriptRoot $_ }
foreach ($file in $files) {
    $tokens = $null; $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile($file, [ref]$tokens, [ref]$errors) | Out-Null
    if ($errors.Count) { throw "PowerShell parse failed: $file" }
}
$ui = Get-Content (Join-Path $PSScriptRoot 'setup-ui.ps1') -Raw
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
Write-Output 'Windows setup packaging checks passed.'
