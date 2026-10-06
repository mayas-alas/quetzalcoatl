// Retire the former offline shell. Authentication is checked on every request;
// never serve an old cached PWA after logout, expiry, or loss of connectivity.
self.addEventListener('install', event => event.waitUntil(self.skipWaiting()));
self.addEventListener('activate', event => event.waitUntil((async () => {
  const keys = await caches.keys();
  await Promise.all(keys.filter(key => key.startsWith('gnx-app-')).map(key => caches.delete(key)));
  await self.clients.claim();
  await self.registration.unregister();
  const windows = await self.clients.matchAll({ type: 'window' });
  await Promise.all(windows.map(client => client.navigate(client.url)));
})()));
// Deliberately no fetch listener, cache writes, or offline fallback.
