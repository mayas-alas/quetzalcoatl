$ErrorActionPreference = 'Stop'
$root = '@ROOT@'
$sid = '@SID@'
$task = '@TASK@'
$trayTask = 'QuetzalcoatlGNX-Tray'
$uninstallKey = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\QuetzalcoatlGNX'
if ($root -ne 'C:\ProgramData\QuetzalcoatlGNX-Setup') { throw 'Invalid fixed cleanup root.' }
if (!$sid) { $sid = (Get-CimInstance Win32_UserProfile | Where-Object { $_.LocalPath -eq 'C:\Users\svc_quetzalcoatl_gnx' } | Select-Object -First 1 -ExpandProperty SID) }
if ($sid -notmatch '^S-1-5-21-(\d+-){3}\d+$') { throw 'Dedicated profile SID was not found; staging preserved for diagnosis.' }
# Wait for the installer EXE that created this task. The helper itself is PowerShell.
$installer = $null
for ($i = 0; $i -lt 120; $i++) {
    $installer = @(Get-CimInstance Win32_Process | Where-Object {
        $_.Name -eq 'quetzalcoatl-gnx-setup.exe' -and $_.CommandLine -like "*$root*"
    })
    if ($installer.Count -eq 0) { break }
    Start-Sleep -Seconds 1
}
if ($installer.Count -ne 0) { throw 'Installer did not exit; staging preserved for diagnosis.' }
if (Get-LocalUser -SID $sid -ErrorAction SilentlyContinue) { throw 'Dedicated account still exists; refusing profile cleanup.' }
$profile = $null
for ($i = 0; $i -lt 300; $i++) {
    $profile = Get-CimInstance Win32_UserProfile | Where-Object { $_.SID -eq $sid }
    if (!$profile -or !$profile.Loaded) { break }
    # The deleted service account can leave only its NTUSER hive mounted.
    # Do not stop the global WSL service or touch another user profile.
    try { & reg.exe unload "HKU\$sid" 2>$null | Out-Null } catch { }
    Start-Sleep -Seconds 1
}
if ($profile -and $profile.Loaded) { throw 'Dedicated Windows profile is still loaded after five minutes; staging preserved for retry.' }
if ($profile) { $profile | Remove-CimInstance }
# Remove the task before deleting its script; task process remains alive.
Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
Unregister-ScheduledTask -TaskName $trayTask -Confirm:$false -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $uninstallKey -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $root -Recurse -Force
