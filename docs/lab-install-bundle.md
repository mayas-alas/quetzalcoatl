# GNX 0.3.1 LAB_ONLY installation bundle

This is a disposable lab acceptance bundle, not a production release. The
manifest and SHA-256 values below provide lab integrity inputs only; they are
not release signatures and do not establish production trust. Apply was not
executed.

## Provenance

- Source checkout: GitHub `main`, commit `5d2feb76a90fe0a4fb5036ceeaf9f435dc068127`.
- Host: Windows worktree with WSL 2 distro `gnx-node` (running) and Podman
  4.9.3. The existing `gnx-windows-lab` container and
  `gnx-windows-lab-storage` volume were not inspected or changed.
- Linux compiler image: `docker.io/library/rust@sha256:af306cfa71d987911a781c37b59d7d67d934f49684058f96cf72079c3626bfe0`
  (`rust:1.88-bookworm`, Rust 1.88.0). `cargo test --locked --all-targets`
  passed (7 passed, 1 intentionally ignored), followed by
  `cargo build --release --locked --bin gnx` and `packaging/linux/build.sh`.
- Windows binaries: built from the same checkout and `Cargo.lock` in the same
  ephemeral Rust container, target `x86_64-pc-windows-gnu`, with Debian
  `gcc-mingw-w64-x86-64` 12.2.0-14+25.2. Native host MSVC/GNU linking was
  unavailable (`link.exe` was not a usable MSVC linker), so this is explicitly
  a GNU lab build.
- Rootfs source: `docker.io/library/ubuntu@sha256:008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3`
  (`ubuntu:24.04`, amd64). The disposable container was augmented with
  `systemd`, `systemd-sysv`, `dbus`, `iproute2`, `ca-certificates`, `curl`,
  `tar`, and `xz-utils`, then exported. WSL still must provide the kernel,
  import mechanism, and host Podman integration; those are not claimed by the
  tar alone.

## Strict six-artifact manifest

`dist/manifest.json` contains exactly these six artifact keys:

| Artifact | SHA-256 |
| --- | --- |
| `gnx.exe` | `cec7eb847acb76912cc84fac52e1d5381c133f229b360c3090656633f78177e2` |
| `gnx-service.exe` | `a47665c586519c9918f199612e20ed437ef973810795ffb27da6bd024a4acad9` |
| `gnx-setup.exe` | `5aa95f53ab4387eaccf42996c7a8e79aa81764986c91670610042e368260054b` |
| `gnx-linux` | `58f23387dc4ded857ab96a7ab72e0666457b9211ba153b85fc254a19bb6c1a2c` |
| `gnx-linux-bundle.tar` | `98722a7592fb5a3802bd4d03d6a4e2b3c80e55120d0d0c58fa40b63b8121e88d` |
| `gnx-linux.run` | `0801f7fe23c315242ed546acd3c76cc3a966b937e410c891feafc8f367d9534b` |

Manifest SHA-256: `08c44f3e0ba69b62239a50512ef27d782c6524619d94d7977d7d440f34aa3cff`.

The separate GNX-compatible rootfs input is `gnx-ubuntu-rootfs.tar`, with
SHA-256 `e3857d212ef0095cb03162aa06419941952de88a83aa4e9b60b4babd0b256ee7`.

## Retained guest staging result

The retained share
`C:\Users\mayas\AppData\Local\Temp\gnx-windows-lab-staging` now contains
all installer inputs: the six manifest artifacts, `manifest.json`, and the
rootfs tar. It also contains `lab-apply.ps1`, a sanitized, non-executed command
file with the two lab hashes. Existing staging files were retained. No
installer apply, WSL import, service enablement, or legacy-path operation was
performed.

Conclusion: the retained guest has every installer input for a LAB_ONLY
acceptance attempt. It does not have a production-trusted release, and clean
elevated Windows/WSL apply and post-install acceptance remain unverified.
