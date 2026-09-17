#requires -Version 5.1
# Build/test/package with a Windows Rust toolchain. No administrator rights required.
# GNU uses LLVM's linker/import librarian because Rust's minimal bundled GNU tools
# are insufficient for the graphical dependency graph. Install llvm-tools-preview first.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location (Join-Path $PSScriptRoot '..')
$oldFlags = $env:CARGO_ENCODED_RUSTFLAGS
try {
    function Invoke-Checked([string]$Program, [string[]]$Arguments) {
        & $Program @Arguments
        if ($LASTEXITCODE -ne 0) { throw "$Program failed ($LASTEXITCODE)" }
    }
    $hostInfo = (& rustc -vV) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'Rust is not available.' }
    if ($hostInfo -match 'host: x86_64-pc-windows-gnu') {
        $sysroot = (& rustc --print sysroot).Trim()
        $bin = Join-Path $sysroot 'lib\rustlib\x86_64-pc-windows-gnu\bin'
        foreach ($tool in @('llvm-ar.exe', 'llvm-readobj.exe', 'rust-lld.exe')) {
            if (!(Test-Path (Join-Path $bin $tool))) { throw 'Run: rustup component add llvm-tools-preview' }
        }
        $support = New-Item -ItemType Directory -Force 'target\build-support'
        $dlltool = Join-Path $support.FullName 'llvm-dlltool.exe'
        # LLVM's llvm-ar dispatches to its dlltool implementation by executable name.
        Copy-Item (Join-Path $bin 'llvm-ar.exe') $dlltool -Force
        $exports = & (Join-Path $bin 'llvm-readobj.exe') --coff-exports "$env:SystemRoot\System32\shlwapi.dll"
        if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect Windows shlwapi exports.' }
        $names = @($exports | ForEach-Object { if ($_ -match '^  Name: (\S+)$') { $Matches[1] } })
        if ($names.Count -eq 0) { throw 'Missing Windows shlwapi exports.' }
        $def = Join-Path $support.FullName 'shlwapi.def'
        @('LIBRARY shlwapi.dll', 'EXPORTS') + $names | Set-Content -Encoding ASCII $def
        Invoke-Checked $dlltool @('-m', 'i386:x86-64', '-d', $def, '-l', (Join-Path $support.FullName 'libshlwapi.a'))
        $flags = @('-C', "dlltool=$dlltool", '-C', "linker=$(Join-Path $bin 'rust-lld.exe')", '-C', 'linker-flavor=ld.lld', '-L', "native=$($support.FullName)")
        $env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31
    } elseif ($hostInfo -notmatch 'host: x86_64-pc-windows-msvc') {
        throw 'Use a Windows x64 Rust toolchain.'
    }
    Invoke-Checked cargo @('test', '--locked')
    Invoke-Checked cargo @('build', '--release', '--locked')
    # Loading the real GUI binary catches PE/linker problems absent from unit tests.
    $probe = Start-Process (Resolve-Path 'target\release\quetzalcoatl-gnx-setup.exe') -ArgumentList '--help' -Wait -PassThru
    if ($probe.ExitCode -ne 0) { throw "Setup executable smoke test failed: $($probe.ExitCode)" }
    New-Item -ItemType Directory -Force 'dist\quetzalcoatl-gnx' | Out-Null
    Copy-Item 'target\release\quetzalcoatl-gnx.exe', 'target\release\quetzalcoatl-gnx-setup.exe', 'target\release\quetzalcoatl-gnx-tray.exe' 'dist\quetzalcoatl-gnx\' -Force
    Write-Host 'Built, tested and packaged in dist\quetzalcoatl-gnx. No installation was started.'
} finally {
    $env:CARGO_ENCODED_RUSTFLAGS = $oldFlags
    Pop-Location
}
