'use strict';
// Generat de web/build-pwa.py — nu editați manual.
const CACHE = 'veriga-1.0.2-98e1134b56', SHELL = ["./", "index.html", "pwa.js", "manifest.webmanifest", "icon-192.png", "icon-512.png", "icon-maskable-512.png", "apple-touch-icon.png"];
addEventListener('install', e => e.waitUntil(caches.open(CACHE).then(c => c.addAll(SHELL)).then(() => skipWaiting())));
addEventListener('activate', e => e.waitUntil(caches.keys().then(ks => Promise.all(ks.filter(k => k !== CACHE).map(k => caches.delete(k)))).then(() => clients.claim())));
addEventListener('fetch', e => {
  const u = new URL(e.request.url);
  // doar fișierele aplicației; verificările de linkuri (alte origini) merg direct la rețea
  if (e.request.method !== 'GET' || u.origin !== location.origin) return;
  e.respondWith(caches.match(e.request, { ignoreSearch: true }).then(r => r || fetch(e.request)));
});
