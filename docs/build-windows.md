# Build on Windows — 0.3.1

This source bundle was prepared statically. Build and tests must be executed on your Windows host before promoting the release.

GNX produces two binaries from the same source:

- **Windows `gnx.exe`** — bridge/CLI.
- **Linux `gnx`** — runtime executed inside the dedicated WSL distribution named `GNX`.

0.3.1 does not provision that distribution. It assumes the `GNX` runtime already exists when live Windows commands are used.

## 1. Windows prerequisites

Open an elevated PowerShell.

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Rustlang.Rustup -e
```

Close and reopen PowerShell after installation, then verify:

```powershell
rustup default stable-msvc
rustup target add x86_64-pc-windows-msvc
rustc --version
cargo --version
```

The package declares a minimum Rust version of 1.85. Use a newer stable toolchain if available.

## 2. Build the Windows bridge

From the extracted source directory:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

Expected artifact:

```text
target\x86_64-pc-windows-msvc\release\gnx.exe
```

The 0.3.1 source ZIP intentionally does not carry forward the 0.3.0 lockfile. The first successful Cargo operation creates a lockfile for this source release. Preserve that generated `Cargo.lock` with the build evidence before promotion.

## 3. Static checks and tests

Run these on the host after the first build:

```powershell
rustup component add rustfmt clippy
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

Do not call the PoC accepted only because these pass; runtime acceptance remains in [`poc.md`](poc.md).

## 4. Build the Linux runtime from Windows

The Linux binary must be built for Linux, not copied from the Windows target. With the source available inside the dedicated `GNX` distribution and its Rust toolchain installed:

```powershell
wsl.exe --distribution GNX --exec bash -lc "cd /path/to/quetzalcoatl-0.3.1 && cargo build --release"
```

Expected Linux artifact inside that distribution:

```text
target/release/gnx
```

Install it as the runtime binary only after your local build/tests succeed:

```powershell
wsl.exe --distribution GNX --user root --exec install -m 0755 /path/to/quetzalcoatl-0.3.1/target/release/gnx /usr/local/bin/gnx
```

## 5. Smoke contract

Create intent and inspect the plan before any live apply:

```powershell
.\target\x86_64-pc-windows-msvc\release\gnx.exe init
.\target\x86_64-pc-windows-msvc\release\gnx.exe plan
```

Live commands require the dedicated `GNX` distribution and its Linux runtime:

```powershell
.\target\x86_64-pc-windows-msvc\release\gnx.exe doctor
.\target\x86_64-pc-windows-msvc\release\gnx.exe status
```

If your distribution has a different temporary name during development, use `--distribution <name>` explicitly. The release default remains `GNX`.
