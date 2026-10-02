# Private GitHub Actions adapter

These are fixed, reviewable assets for a private repository. Copy
`gnx-ephemeral-exit-node.yml` to that repository's `.github/workflows/` directory
and commit it; do not edit the workflow in a generated checkout. It supports both
`workflow_call` and `workflow_dispatch`. The workflow checks out the private
repository, installs Tailscale `1.102.4` from the pinned archive and SHA-256,
advertises an ephemeral exit node, and never permits a lease over 350 minutes.

## One-time repository setup

1. Create a repository secret named `TAILSCALE_AUTHKEY`. The pre-auth key must be
   **ephemeral**, restricted to the intended tag (for example `tag:gnx-exit`), and
   permitted by the tailnet ACL to advertise an exit node. Do not put the key in
   this directory, workflow inputs, logs, URLs, or command arguments.
2. Copy `config.env.example` to an operator-owned path and set the private
   `OWNER/REPOSITORY`, workflow filename, and ref. The config contains no secret.
3. Store a GitHub fine-grained token in an operator-owned regular file owned by the
   invoking user (or root), mode `0400` or `0600`; grant only Actions read/write
   permission for that repository. The adapter never accepts a token argument or
   environment variable.

## Adapter operations

`gnx-actions.sh` emits one JSON object per invocation and exits nonzero for an API
or validation failure. `dispatch` starts a lease, `status` reads one run (or the
latest dispatch runs when `--run-id` is omitted), and `cancel` requests cancellation
of a numeric run ID. `--lease-minutes` is validated to `1..350`; all refs, IDs,
repository names, and workflow names are allowlisted before constructing a URL.

```sh
# Dispatch; token is read from a protected file, never argv.
./gnx-actions.sh --config /run/gnx/actions.env \
  --token-file /run/gnx/github-token dispatch --lease-minutes 60 --request-id node-001

# Read a run and cancel it.
./gnx-actions.sh --config /run/gnx/actions.env \
  --token-file /run/gnx/github-token status --run-id 123456789
./gnx-actions.sh --config /run/gnx/actions.env \
  --token-file /run/gnx/github-token cancel --run-id 123456789

# A protected pipe is also supported; the value is read exactly once.
cat /run/gnx/github-token | ./gnx-actions.sh --config /run/gnx/actions.env --token-stdin status
```

The adapter sends authorization headers to curl through stdin rather than placing
credentials in a process argument. The workflow writes the protected GitHub secret
to a mode-`0600` runner file and invokes `tailscale up --auth-key=file:...`; the
file and in-memory Tailscale state are removed on exit. The JSON result includes
`ok`, `state`, `run_id`, `request_id`, `lease_minutes`, and (when ready)
`tailscale_ip`, `issued_at`, and `expires_at`; consume it as data, not shell code.

The workflow is intentionally not installed under the product's `.github/`
directory: this payload is the adapter bundle and its only repository changes are
under `payload/node/actions/`.
