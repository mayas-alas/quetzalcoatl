#requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ManifestSha256,
    [Parameter(Mandatory)][string]$Rootfs,
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$RootfsSha256,
    [string]$Bundle
)

if ([string]::IsNullOrWhiteSpace($Bundle)) {
    $Bundle = if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'manifest.json')) { $PSScriptRoot } else { Join-Path $PSScriptRoot '../../dist' }
}

# Thin presentation only. install.ps1 and gnx-setup.exe remain the policy boundary.
$ErrorActionPreference = 'Stop'

function ConvertTo-ProcessArgument([string]$Value) {
    if ($Value -notmatch '[\s"]') { return $Value }
    return '"' + (($Value -replace '(\\*)"', '$1$1\\"') -replace '(\\+)$', '$1$1') + '"'
}

function Test-Administrator {
    $principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Test-Administrator)) {
    $forward = @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-STA', '-File', (Resolve-Path -LiteralPath $PSCommandPath).Path,
        '-ManifestSha256', $ManifestSha256,
        '-Rootfs', (Resolve-Path -LiteralPath $Rootfs -ErrorAction Stop).Path,
        '-RootfsSha256', $RootfsSha256,
        '-Bundle', (Resolve-Path -LiteralPath $Bundle -ErrorAction Stop).Path
    ) | ForEach-Object { ConvertTo-ProcessArgument ([string]$_) }
    Start-Process -FilePath 'powershell.exe' -Verb RunAs -ArgumentList ($forward -join ' ') | Out-Null
    exit 0
}

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[Windows.Forms.Application]::EnableVisualStyles()

$bundlePath = (Resolve-Path -LiteralPath $Bundle -ErrorAction Stop).Path
$rootfsPath = (Resolve-Path -LiteralPath $Rootfs -ErrorAction Stop).Path
$installer = Join-Path $bundlePath 'install.ps1'
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) { throw 'install.ps1 is missing from the bundle.' }

$form = New-Object Windows.Forms.Form
$form.Text = 'GNX 0.3.1'
$form.ClientSize = New-Object Drawing.Size(560, 250)
$form.StartPosition = 'CenterScreen'
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$form.MinimizeBox = $false
$form.ShowInTaskbar = $true
$icon = Join-Path $bundlePath 'assets/branding-install-logo.ico'
if (Test-Path -LiteralPath $icon -PathType Leaf) { $form.Icon = New-Object Drawing.Icon($icon) }

$logo = New-Object Windows.Forms.PictureBox
$logo.Location = New-Object Drawing.Point(24, 24)
$logo.Size = New-Object Drawing.Size(96, 96)
$logo.SizeMode = 'Zoom'
$logoFile = Join-Path $bundlePath 'assets/branding-install-logo.png'
if (Test-Path -LiteralPath $logoFile -PathType Leaf) { $logo.Image = [Drawing.Image]::FromFile($logoFile) }
$form.Controls.Add($logo)

$title = New-Object Windows.Forms.Label
$title.Location = New-Object Drawing.Point(140, 28)
$title.Size = New-Object Drawing.Size(390, 34)
$title.Font = New-Object Drawing.Font('Segoe UI', 16, [Drawing.FontStyle]::Bold)
$title.Text = 'GNX 0.3.1'
$form.Controls.Add($title)

$status = New-Object Windows.Forms.Label
$status.Location = New-Object Drawing.Point(140, 70)
$status.Size = New-Object Drawing.Size(390, 58)
$status.Font = New-Object Drawing.Font('Segoe UI', 10)
$status.Text = 'Validando e instalando…'
$form.Controls.Add($status)

$progress = New-Object Windows.Forms.ProgressBar
$progress.Location = New-Object Drawing.Point(24, 145)
$progress.Size = New-Object Drawing.Size(506, 20)
$progress.Style = 'Marquee'
$progress.MarqueeAnimationSpeed = 25
$form.Controls.Add($progress)

$close = New-Object Windows.Forms.Button
$close.Location = New-Object Drawing.Point(430, 195)
$close.Size = New-Object Drawing.Size(100, 30)
$close.Text = 'Cerrar'
$close.Enabled = $false
$form.Controls.Add($close)

$reboot = New-Object Windows.Forms.Button
$reboot.Location = New-Object Drawing.Point(310, 195)
$reboot.Size = New-Object Drawing.Size(110, 30)
$reboot.Text = 'Reiniciar ahora'
$reboot.Visible = $false
$form.Controls.Add($reboot)

$script:terminal = $false
$script:completedEvent = $null
$script:child = $null
$script:badOutput = $false

function Set-UiState([string]$Text, [bool]$Done) {
    $status.Text = $Text
    if ($Done) {
        $progress.Style = 'Continuous'
        $progress.Value = 100
        $close.Enabled = $true
    }
}

function Receive-Progress([string]$Line) {
    if ([string]::IsNullOrWhiteSpace($Line)) { return }
    try { $event = $Line | ConvertFrom-Json -ErrorAction Stop } catch {
        $script:badOutput = $true
        Set-UiState 'Respuesta de instalación no válida.' $true
        return
    }
    if ($event.schema -ne 1 -or $event.operation -notin @('setup-check', 'setup-provision') -or $event.phase -notin @('started', 'completed') -or $event.apply_available -ne $false) {
        $script:badOutput = $true
        Set-UiState 'Respuesta de instalación no compatible.' $true
        return
    }
    if ($event.phase -eq 'completed') {
        $script:completedEvent = $event
        $script:terminal = $true
        $state = [string]$event.state
        if ($state -eq 'READY') { Set-UiState 'Instalación completada y verificada.' $true }
        elseif ($state -eq 'ACTION_REQUIRED') {
            if ([string]$event.code -match 'REBOOT') {
                Set-UiState 'Se requiere reiniciar Windows para continuar.' $true
                $reboot.Visible = $true
            } else {
                Set-UiState 'Instalación preparada. Continúa con bootstrap y verificación.' $true
            }
        }
        else { Set-UiState 'La instalación no pudo completarse.' $true }
    }
}

$psi = New-Object Diagnostics.ProcessStartInfo
$psi.FileName = 'powershell.exe'
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$childArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $installer, '-ManifestSha256', $ManifestSha256, '-Rootfs', $rootfsPath, '-RootfsSha256', $RootfsSha256, '-Bundle', $bundlePath)
if ($null -ne $psi.PSObject.Properties['ArgumentList']) {
    foreach ($arg in $childArgs) { [void]$psi.ArgumentList.Add([string]$arg) }
} else {
    $psi.Arguments = (($childArgs | ForEach-Object { ConvertTo-ProcessArgument ([string]$_) }) -join ' ')
}

$child = New-Object Diagnostics.Process
$child.StartInfo = $psi
$child.EnableRaisingEvents = $true
$child.add_OutputDataReceived({ param($sender, $args) if ($args.Data) { $line = [string]$args.Data; $form.BeginInvoke([Action]{ Receive-Progress $line }) | Out-Null } })
$child.add_ErrorDataReceived({ param($sender, $args) if ($args.Data) { $form.BeginInvoke([Action]{ if (-not $script:terminal) { Set-UiState 'La instalación fue bloqueada.' $true } }) | Out-Null } })
$child.add_Exited({ param($sender, $args) $form.BeginInvoke([Action]{
    if (-not $script:terminal -and -not $script:badOutput) { Set-UiState 'La instalación terminó sin un resultado válido.' $true }
}) | Out-Null })
if (-not $child.Start()) { throw 'Unable to start install.ps1.' }
$script:child = $child
$child.BeginOutputReadLine()
$child.BeginErrorReadLine()

$close.Add_Click({ $form.Close() })
$reboot.Add_Click({
    $answer = [Windows.Forms.MessageBox]::Show('Windows se reiniciará ahora.', 'GNX', 'OKCancel', 'Warning')
    if ($answer -eq 'OK') { Start-Process -FilePath 'shutdown.exe' -ArgumentList '/r /t 5 /c "GNX installation restart"' -WindowStyle Hidden; $form.Close() }
})
$form.Add_FormClosing({ param($sender, $args)
    if ($script:child -and -not $script:child.HasExited -and -not $script:terminal) {
        $args.Cancel = $true
        [Windows.Forms.MessageBox]::Show('La instalación sigue en curso.', 'GNX', 'OK', 'Information') | Out-Null
    }
})
[void][Windows.Forms.Application]::Run($form)
