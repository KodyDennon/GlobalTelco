/// <reference lib="webworker" />

const sw = self as unknown as ServiceWorkerGlobalScope;

const CACHE_NAME = 'globaltelco-v1';
const STATIC_ASSETS = [
	'/',
	'/manifest.json'
];

// Cache-first strategy for static assets and WASM
sw.addEventListener('install', (event) => {
	event.waitUntil(
		caches.open(CACHE_NAME).then((cache) => cache.addAll(STATIC_ASSETS))
	);
	sw.skipWaiting();
});

sw.addEventListener('activate', (event) => {
	event.waitUntil(
		caches.keys().then((names) =>
			Promise.all(
				names
					.filter((name) => name !== CACHE_NAME)
					.map((name) => caches.delete(name))
			)
		)
	);
	sw.clients.claim();
});

sw.addEventListener('fetch', (event) => {
	const url = new URL(event.request.url);

	// Cache-first for WASM and static assets
	if (
		url.pathname.endsWith('.wasm') ||
		url.pathname.endsWith('.js') ||
		url.pathname.endsWith('.css') ||
		url.pathname.endsWith('.png') ||
		url.pathname.endsWith('.svg') ||
		url.pathname.endsWith('.ogg') ||
		url.pathname.startsWith('/assets/')
	) {
		event.respondWith(
			caches.match(event.request).then((cached) => {
				if (cached) return cached;
				return fetch(event.request).then((response) => {
					if (response.ok) {
						const clone = response.clone();
						caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
					}
					return response;
				});
			})
		);
		return;
	}

	// Network-first for API calls
	if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/ws')) {
		event.respondWith(fetch(event.request));
		return;
	}

	// Network-first with cache fallback for pages
	event.respondWith(
		fetch(event.request)
			.then((response) => {
				if (response.ok) {
					const clone = response.clone();
					caches.open(CACHE_NAME).then((cache) => cache.put(event.request, clone));
				}
				return response;
			})
			.catch(() => caches.match(event.request).then((cached) => cached || new Response('Offline', { status: 503 })))
	);
});
