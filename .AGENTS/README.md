# Project README

## Overview
Quetzalcoatl is a Windows-managed v1.wp-managed platform that installs and orchestrates Fedora Podman Machines for controller and member node management. Developed by GNX Labs, this implementation ensures secure service deployment without exposing service-specific infrastructure to Rust.

## Directory Structure
- `.AGENTS/`: Active delivery contract with:
  - `SCOPE.md`: Overview, outcomes, invariants
  - `TRACKER.md`: Current status, dependencies, blockers
  - `WORKSTREAMS.md`: Three ownership lanes (protocol, bundle, registry)
  - `EVIDENCE.md`: Verifiable artifacts

## Development Workflow
1. **Git Setup**: Configured with [EMAIL] and "maya" identity
2. **API Testing**: Embeddings API `/v1/embeddings` returns 500 error (expected in dev)
3. **Commit History**: Initial `chore(agents)` commit tracks foundation stabilization

## Key Requirements
- Use distributive workstreams for protocol, bundle, and registry tasks
- Maintain strict separation between foundation operations and service-specific implementations
- Validate through</think>