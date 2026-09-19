# Linux artifact and lab-rootfs acceptance evidence

This is lab evidence for commit `99b2dfc` (2026-09-18). It is not a release
approval and does not establish `READY`.

## Scope and safety

- No secrets, update URLs, installer invocation, host legacy mounts, or
  `gnx-node` filesystem access were used.
- The existing WSL distro `gnx-node` was observed running and was left
  untouched. A separate disposable distro name was used for the fixture.
- The fixture digest below is explicitly `LAB_ONLY`; it is an integrity
  observation for this run, not a production trust-channel signature or
  authenticated release input.

## Observations

| Check | Evidence | Result |
| --- | --- | --- |
| Candidate | `git log -1 --oneline` | `99b2dfc docs: record dockur image pull and boot blocker` |
| WSL | `wsl --status`; `wsl --list --verbose` | WSL 2 is available; default distro `gnx-node`, running, version 2 |
| Podman | `wsl bash -lc 'podman info'` | Podman 4.9.3, Linux amd64; command completed |
| Disposable container | `podman create/start -a ... caddy version` | `v2.11.4 ...`; `EPHEMERAL_OK`; container removed |
| WSL build toolchain | `wsl bash -lc 'command -v cargo; command -v rustup'` | Neither `cargo` nor `rustup` is installed in WSL |
| Host Linux target | `cargo build --release --target x86_64-unknown-linux-gnu` | Failed: target `x86_64-unknown-linux-gnu` is not installed; host linker also resolves to non-MSVC `link.exe` |

The Linux bundle cannot be built reproducibly in this checkout with the
available toolchain. `packaging/linux/build.sh` requires `dist/gnx-linux`, but
there is no declared/pinned GNX build image or WSL Linux Rust toolchain from
which to produce that input. The Podman smoke test proves only that disposable
containers can start; it does not prove a GNX build or release provenance.

## Disposable LAB_ONLY rootfs fixture

A temporary rootfs was exported from the already available local
`docker.io/library/caddy:latest` image, imported as
`GNX-LAB-ARTIFACT-20260918`, and verified with `/usr/bin/caddy version`:

```text
v2.11.4 h1:XKxkMTgNSizEvKG6QHue6cAsFOteU2qA61w2tKkCWi0=
LAB_ONLY_ROOTFS_SHA256=320fa2678773aaa8025a8b7c3a2abc099ac2e8b8d770e1e375b34f5dbedca86a
```

The distro was unregistered and its temporary export directory removed. The
post-cleanup WSL list contained only `gnx-node`; Podman containers created for
the check were removed, and the pulled `caddy:latest` tag was removed while the
pre-existing dangling caddy image was preserved. This fixture was never passed
to the GNX installer and must not be treated as a GNX release rootfs.

## Remaining blockers

1. Provide a pinned, reproducible Linux build environment (Rust toolchain,
   target, linker, and declared container/build inputs) and build `gnx-linux`.
2. Produce `gnx-linux-bundle.tar` and `gnx-linux.run` from that binary, then
   authenticate their manifest through the approved release trust channel.
3. Produce and authenticate a GNX-compatible WSL rootfs; the caddy-derived
   `LAB_ONLY` fixture is only proof that isolated import/cleanup mechanics work.
4. Run acceptance on a clean host without adopting or modifying `gnx-node` or
   any legacy path. No gate is promoted by this report.
