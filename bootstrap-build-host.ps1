Set-ExecutionPolicy -Scope Process Bypass -Force
$ErrorActionPreference = "Stop"

winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Python.Python.3.13 --exact
winget install --id Microsoft.DotNet.SDK.8 --exact
winget install --id Rustlang.Rustup --exact

$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
            [System.Environment]::GetEnvironmentVariable("Path","User")

rustup toolchain install 1.96.1
rustup default 1.96.1
rustup target add x86_64-pc-windows-msvc
cargo install cargo-audit --version 0.22.2 --locked

Write-Host "Build host bootstrap complete. Reopen PowerShell if required."
