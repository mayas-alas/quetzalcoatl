# Quetzalcoatl — Windows 11 Build Host

## Purpose

This document prepares a **clean Windows 11 host** to build the Quetzalcoatl
installer and native Rust workspace from this repository.

The intended host starts with no development toolchain installed.

## Required host tooling

| Tool | Required | Purpose |
|---|---:|---|
| Visual Studio 2022 Build Tools | Yes | MSVC linker/compiler and Windows native build prerequisites |
| Python 3.13 | Yes | Repository/build helper scripts |
| .NET SDK 8 | Yes | WiX tool execution and installer tooling |
| Rustup | Yes | Rust toolchain management |
| Rust 1.96.1 | Yes | Pinned project Rust toolchain |
| cargo-audit 0.22.2 | Yes | Dependency/security audit |
| WiX 5.0.2 | Repository-managed | Restored through `.config/dotnet-tools.json` |

### Deliberately NOT required as global host dependencies

Do not manually install these just to build the repository:

- WiX globally
- Tailscale
- Podman
- WSL
- WinSW

The installer/build workflow manages the required packaged/runtime artifacts
where applicable.

---

## 1. Open PowerShell

Use a normal PowerShell window for most commands.

For Visual Studio Build Tools installation, allow the installer to request
administrator privileges.

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force
```

## 2. Install the Windows build prerequisites

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Python.Python.3.13 --exact
winget install --id Microsoft.DotNet.SDK.8 --exact
winget install --id Rustlang.Rustup --exact
```

Close and reopen PowerShell after installation.

Then refresh the current process PATH:

```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
            [System.Environment]::GetEnvironmentVariable("Path","User")
```

## 3. Install the pinned Rust toolchain

Quetzalcoatl uses Rust **1.96.1**.

```powershell
rustup toolchain install 1.96.1
rustup default 1.96.1
rustup target add x86_64-pc-windows-msvc
```

Install the pinned audit tool:

```powershell
cargo install cargo-audit --version 0.22.2 --locked
```

## 4. Verify the host

```powershell
rustc --version
cargo --version
cargo audit --version
python --version
dotnet --version
cl.exe
msbuild -version
```

Expected important versions:

```text
rustc 1.96.1
cargo-audit 0.22.2
Python 3.13.x
.NET 8.x
```

The exact patch version of Python/.NET/Visual Studio may vary within the
supported major version.

## 5. Enter the repository

Example:

```powershell
cd C:\path\to\quetzalcoatl
```

Confirm the repository is correct:

```powershell
git status
```

## 6. Restore repository-managed tooling

The repository pins WiX through the .NET tool manifest.

```powershell
dotnet tool restore --tool-manifest .config\dotnet-tools.json
dotnet tool run wix -- --version
```

Expected:

```text
5.0.2
```

Do not install a separate global WiX version unless the repository explicitly
requires it.

## 7. Rust validation before installer build

Run:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

Then audit dependencies:

```powershell
cargo audit
```

If these checks fail, fix the failure before treating the installer build as
successful.

## 8. Build an unsigned installer

For a clean development/QA build without a signing certificate:

```powershell
.\installer\build.ps1 -AllowUnsigned
```

Expected artifacts are under:

```text
target\installer\
```

Look for the generated MSI and bootstrapper executable, for example:

```text
target\installer\Quetzalcoatl.msi
target\installer\QuetzalcoatlSetup.exe
```

The exact artifact names should be taken from the build output rather than
assumed by automation.

## 9. Repository validation

Where available, run the repository check script:

```powershell
.\tools\check.ps1
```

For source-only validation when signing material is unavailable:

```powershell
.\tools\check.ps1 -SourceOnly
```

Then build the unsigned installer:

```powershell
.\installer\build.ps1 -AllowUnsigned
```

## 10. One-shot bootstrap script

The following can be saved as:

```text
bootstrap-build-host.ps1
```

and executed from an elevated PowerShell session.

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force
$ErrorActionPreference = "Stop"

Write-Host "== Quetzalcoatl Windows Build Host =="

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

Write-Host ""
Write-Host "== Installed versions =="

rustc --version
cargo --version
cargo audit --version
python --version
dotnet --version

Write-Host ""
Write-Host "== Host bootstrap complete =="
Write-Host "Close/reopen PowerShell if PATH changes are not visible."
```

After the bootstrap completes, from the repository root:

```powershell
dotnet tool restore --tool-manifest .config\dotnet-tools.json
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo audit
.\installer\build.ps1 -AllowUnsigned
```

## 11. Signing

An unsigned build is intended for development/QA.

Production release signing requires the repository's configured signing
certificate/material and should be performed through the project's release
workflow. Do not embed a private signing key in this repository or bootstrap
script.

## 12. Important host assumptions

This guide assumes:

- Windows 11 x64
- Internet access during initial dependency installation
- `winget` available
- Visual Studio Build Tools can install the MSVC workload
- the repository's pinned dependencies remain accessible
- no Rust runtime is preinstalled

The Rust compiler is intentionally installed here because this document is
for the **actual Windows build host**. This is separate from any restricted
agent environment where Rust execution is prohibited.

## Quick path

For a fresh machine:

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force

winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Python.Python.3.13 --exact
winget install --id Microsoft.DotNet.SDK.8 --exact
winget install --id Rustlang.Rustup --exact
```

Reopen PowerShell:

```powershell
rustup toolchain install 1.96.1
rustup default 1.96.1
rustup target add x86_64-pc-windows-msvc
cargo install cargo-audit --version 0.22.2 --locked

dotnet tool restore --tool-manifest .config\dotnet-tools.json
.\installer\build.ps1 -AllowUnsigned
```
