# Evidence

The source tree must satisfy all of the following:

- workspace format, Clippy and tests pass;
- every shell payload parses with `sh -n` and passes ShellCheck;
- runtime manifest hashes and the Rust payload allowlist agree;
- installer static contracts pass;
- the generated source ZIP is produced from the validated tree.

Hosted CI is static and build evidence. It does not replace physical Windows, nested virtualization, Tailscale direct-path or multi-host Corosync acceptance.
