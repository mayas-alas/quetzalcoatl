# Network/PWA validation

Run the reproducible checks from the repository root:

```sh
node tests/node-runtime/test-runtime.mjs
node tests/security/test-network-pwa.mjs
```

These checks validate the reviewed contracts: private hostnames, explicit gateway TLS files, backend CA validation for `compute.gnx`, no insecure TLS bypass, ingress restricted to loopback and `tailscale0`, protected secret-file inputs, CLI-only onboarding through the named pipe, and an asset-only offline cache with no credential persistence. GitHub configuration uses its HTTPS API directly with a one-operation token; the in-memory tests verify permission denial, private repo creation, workflow injection, partial failure and origin gating.

The following require an authorized tailnet and cannot be proven by static tests:

- Configure Split DNS for `gnx` to the node’s Tailscale address, then resolve `app.gnx` and `compute.gnx` from a tailnet client.
- Install/trust the operator-managed gateway CA on that client and verify HTTPS certificate validation for both hostnames.
- From a tailnet client, verify the PWA loads, installs, and opens its cached shell after disconnecting; verify `compute.gnx` is never served from the PWA cache.
- Confirm a non-tailnet interface cannot reach DNS/HTTPS and that the compute service remains reachable only through the gateway.
- Confirm `install.run` retry/cancel/reboot cleanup leaves no mesh enrollment file in `/run/gnx-node`, while the protected Proxmox file remains mode `0600`.
- Confirm `verify.run` JSONL contains only sanitized Tailscale/Pi-hole addresses and the Split DNS plus gateway-CA next actions.
- On `app.gnx`, select the reviewed `payload/node/actions/gnx-ephemeral-exit-node.yml`, enter a classic GitHub token with `repo, workflow`, name a new test repository and explicitly confirm creation. Verify one private repository and one workflow, no `gnx://github` navigation/UAC, and no token in storage or logs. Add the YAML's required secrets in GitHub; this operation does not execute Actions.

## Read-only commands, no temporary files

From the host user WSL (do not use the dedicated runtime identity):

```sh
cd /mnt/c/Users/mayas/quetzalcoatl
hf auth whoami
git fetch origin
git rev-list --left-right --count HEAD...origin/main
node tests/test-static-contracts.mjs
```

In the authorized Proxmox node shell opened from `https://compute.gnx/#v1:0:18:4:::::::`:

```sh
pveversion
systemctl is-active pveproxy pvedaemon
qm list
pct list
# For the running LXC id shown by pct list; substitute <CTID>:
pct exec <CTID> -- systemctl --failed --no-pager
pct exec <CTID> -- systemctl is-active rustfs workload-mesh workload-mesh-guard
pct exec <CTID> -- test -c /dev/net/tun
pct exec <CTID> -- podman ps --format '{{.Names}} {{.Status}}'
```

These are inspection commands, not evidence of results until executed. Do not dump
secret files, tokens, full container environments or credentials. No installer,
restart, workflow dispatch or VM/LXC mutation is needed for these checks.
