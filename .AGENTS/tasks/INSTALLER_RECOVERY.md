# Task: stable dependency installation

## Required path

`Burn ancillary payload → hash/size validation → ProgramData stable cache → msiexec → registry/binary post-validation`

## Required failure behavior

- Missing ancillary payload: stop before `msiexec`.
- Invalid size/hash: quarantine invalid staged copy and stop or restage from a valid source.
- MSI error: retain stable MSI and verbose log.
- Reboot: persist phase and resume without repeating completed work.
- More than three attempts in the same phase: stop with `INSTALL_RESUME_LIMIT_REACHED`.

No dependency download or caller-supplied path is allowed.
