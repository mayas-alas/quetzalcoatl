# Legacy tracker — retired

Active progress moved to `../.AGENTS/TRACKER.md`; accepted proof moved to `../.AGENTS/EVIDENCE.md`. This file remains only as a stable route for old links.

Historical conclusions retained for regression context:

- A prior single-controller candidate reached `READY` and survived a real reboot.
- Prior Dockur guests proved Windows/KVM reachability but used DERP at roughly 64–73 ms, so they did not satisfy Corosync acceptance.
- The member path and the real three-host quorum gate were not closed by the legacy tracker.

These facts award no current-cycle points until reproduced against the integrated candidate. The complete retired tracker is recoverable from Git revision `4c4cfd5bf5757480bdd19020bae10db6b2169c21` as `docs/TRACKING.md`; its SHA-256 was `98A68F2A32E85C49CDF30FE87E4B07D5FAAE2B998041D34D47ABC1612E4A896B`.
