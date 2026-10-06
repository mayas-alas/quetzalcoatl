// Fast local gate: run the compact installer, runtime, and security contracts
// together instead of starting three separate test cycles.
await import('./installer/test-uninstall.mjs');
await import('./node-runtime/test-runtime.mjs');
await import('./node-runtime/test-compute-upgrade.mjs');
await import('./node-runtime/test-warm-readiness.mjs');
await import('./node-runtime/test-lxc-network.mjs');
await import('./security/test-network-pwa.mjs');
await import('./test-workload-mesh.mjs');
await import('./test-access-services.mjs');
await import('./test-app-auth.mjs');
