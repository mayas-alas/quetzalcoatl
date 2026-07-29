# Quetzalcoatl 0.1.17 buildfix-02

## Symptom

`TAILSCALE_SERVE_APPLY_FAILED` reported `/config/serve.json: No such file or directory` after Proxmox was already ready.

## Root cause

The service invoked `podman machine ssh` with a nested `sh -c` and input redirection. Podman Machine transports the remote command as a shell command string, so the `< /config/serve.json` redirection was evaluated by the Fedora machine shell rather than by the shell inside `gnx-tailscaled`.

## Fix

- Build the fixed Serve JSON in Rust from the committed local hostname and configured tailnet.
- Pass the JSON through stdin to `podman exec -i gnx-tailscaled tailscale serve set-raw`.
- Remove `sh -c` and remote redirection from this path.
- Strengthen `validate_remote_execution.py` so multiline Rust argv containing `sh`, `-c` is rejected.
- Add a unit test covering the generated fixed PVE route.

No discovery, role, cluster-join, installer, schema, payload-version, or port contract was changed.
