[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('folder', 'zip')]
    [string]$Kind
)

$ErrorActionPreference = 'Stop'

function Write-Json([object]$Value) {
    [Console]::Out.WriteLine(($Value | ConvertTo-Json -Compress -Depth 5))
}

function Invoke-Dialog {
    try {
        Add-Type -AssemblyName System.Windows.Forms -ErrorAction Stop | Out-Null
        [System.Windows.Forms.Application]::EnableVisualStyles()
        if ($Kind -eq 'folder') {
            $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
            try {
                $dialog.Description = 'Select a GnX workload bundle folder'
                $dialog.ShowNewFolderButton = $false
                $result = $dialog.ShowDialog()
                if ($result -ne [System.Windows.Forms.DialogResult]::OK -or [string]::IsNullOrWhiteSpace($dialog.SelectedPath)) {
                    Write-Json ([ordered]@{ source_path = $null; cancelled = $true }); return
                }
                $path = [IO.Path]::GetFullPath($dialog.SelectedPath)
            } finally { $dialog.Dispose() }
        } else {
            $dialog = New-Object System.Windows.Forms.OpenFileDialog
            try {
                $dialog.Title = 'Select a GnX workload ZIP'
                $dialog.Filter = 'ZIP workload bundles (*.zip)|*.zip'
                $dialog.CheckFileExists = $true
                $dialog.Multiselect = $false
                $result = $dialog.ShowDialog()
                if ($result -ne [System.Windows.Forms.DialogResult]::OK -or [string]::IsNullOrWhiteSpace($dialog.FileName)) {
                    Write-Json ([ordered]@{ source_path = $null; cancelled = $true }); return
                }
                $path = [IO.Path]::GetFullPath($dialog.FileName)
            } finally { $dialog.Dispose() }
        }
        if ($path.StartsWith('\\', [StringComparison]::Ordinal) -or -not [IO.Path]::IsPathRooted($path)) { throw 'LOCAL_PATH_REQUIRED' }
        Write-Json ([ordered]@{ source_path = $path; cancelled = $false })
    } catch {
        [Console]::Error.WriteLine((([ordered]@{ error = [ordered]@{ code = 'SELECTOR_FAILED'; message = 'SELECTOR_FAILED' } }) | ConvertTo-Json -Compress -Depth 5))
        exit 1
    }
}

if ([Threading.Thread]::CurrentThread.ApartmentState -ne [Threading.ApartmentState]::STA) {
    $powershell = Join-Path $PSHome 'powershell.exe'
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = $powershell
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Arguments = '-NoLogo -NoProfile -STA -ExecutionPolicy Bypass -File "' + $PSCommandPath + '" -Kind ' + $Kind
    $child = New-Object Diagnostics.Process
    $child.StartInfo = $psi
    [void]$child.Start()
    $stdout = $child.StandardOutput.ReadToEnd()
    $stderr = $child.StandardError.ReadToEnd()
    $child.WaitForExit()
    [Console]::Out.Write($stdout)
    [Console]::Error.Write($stderr)
    exit $child.ExitCode
}

Invoke-Dialog
