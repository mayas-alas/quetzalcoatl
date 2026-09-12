# GNX release authority

`trusted-release.pub` is the Ed25519 public key pinned into `gnx-install.exe`.
Its SHA-256 key identity is
`7720127012478b755554071a15ba57b4874cb99152be37c1fa11620a5f649a63`.

The corresponding private 32-byte seed is external release-authority state. It
must be a lowercase 64-character hex file protected for the release operator;
it must never enter this repository, a release archive, logs or acceptance
evidence. Pass its path to `packaging/windows/build.ps1 -SigningKey`.

Rotating this authority requires an explicitly reviewed release that pins the
new public key. A manifest signed by any other key is rejected before installer
elevation or artifact execution.

Every signed manifest also carries the positive monotonic `release_serial` from
`Cargo.toml`. Normal updates may only increase it; applying a lower serial is an
explicit rollback operation.
