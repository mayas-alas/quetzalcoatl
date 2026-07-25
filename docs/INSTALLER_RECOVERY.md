# Installer recovery — 0.1.15

## Problem boundary

Burn may cache and verify a dependency payload but a later Windows Installer process can still fail to open that cache path. Quetzalcoatl therefore no longer executes the pinned WSL and Podman MSIs directly as Burn `MsiPackage` entries.

## Implemented path

```text
Burn ExePackage
  ├─ gnx-host-preflight.exe
  └─ pinned ancillary MSI
        ↓ fixed-name, size and SHA-256 validation
C:\ProgramData\Quetzalcoatl\Installer\cache\<dependency>\<version>
        ↓ .partial + flush + activation + revalidation
msiexec /qn /L*V
        ↓
registry and binary post-validation
```

The helper accepts no caller-supplied path, URL, version, size or hash. Only `install-wsl` and `install-podman` select compiled dependency specifications that are checked against `installer/dependencies.lock.json` during the release build.

## Recovery journal

`install-state.json` records schema, product version, current phase, attempt and the last typed error. Completed phases reset their attempt counter. Re-entering the same incomplete phase more than three times stops rather than creating a reboot/retry loop.

## Stable evidence

```text
C:\ProgramData\Quetzalcoatl\Installer\
├─ install-state.json
├─ cache\
└─ logs\
   ├─ wsl-2.7.10.0-install.log
   └─ podman-6.0.1-install.log
```

An MSI failure retains the staged package and log for diagnosis. The implementation does not modify ACLs on the global Burn Package Cache and does not download dependencies during setup.

## Certification boundary

Source validation proves wiring and pin coherence. Only a Windows execution can certify Burn ancillary-payload layout, reboot resume, Windows Installer access and WSL/Podman behavior on the target host.
