/// <reference lib="webworker" />

const sw = self as unknown as ServiceWorkerGlobalScope;

const CACHE_NAME = 'globaltelco-v2';
const WASM_CACHE = 'globaltelco-wasm-v1';
const STATIC_ASSETS = [
	'/',
	'/manifest.json'
];

// Cache-first strategy for static assets and WASM.
// Pre-cache WASM binary on install for V8's compiled code caching
// (near-instant startup from 3rd+ visit).
sw.addEventListener('install', (event) => {
	event.waitUntil(
		Promise.all([
			caches.open(CACHE_NAME).then((cache) => cache.addAll(STATIC_ASSETS))
				.catch(() => { /* offline install — static assets will be cached on first load */ }),
			// Pre-fetch WASM binary into a dedicated cache for streaming compilation
			caches.open(WASM_CACHE).then(async (cache) => {
				const wasmReq = new Request('/wasm/pkg/gt_wasm_bg.wasm');
				const existing = await cache.match(wasmReq);
				if (!existing) {
					try {
						const resp = await fetch(wasmReq);
						if (resp.ok) await cache.put(wasmReq, resp);
					} catch { /* offline install — WASM will be cached on first load */ }
				}
			}),
		])
	);
	sw.skipWaiting();
});

sw.addEventListener('activate', (event) => {
	const KEEP = new Set([CACHE_NAME, WASM_CACHE]);
	event.waitUntil(
		caches.keys().then((names) =>
			Promise.all(
				names
					.filter((name) => !KEEP.has(name))
					.map((name) => caches.delete(name))
			)
		)
	);
	sw.clients.claim();
});

sw.addEventListener('fetch', (event) => {
	const url = new URL(event.request.url);

	// WASM files: serve from dedicated WASM cache (enables V8 compiled code caching)
	if (url.pathname.endsWith('.wasm')) {
		event.respondWith(
			caches.open(WASM_CACHE).then((cache) =>
				cache.match(event.request).then((cached) => {
					if (cached) return cached;
					return fetch(event.request).then((response) => {
						if (response.ok) {
							cache.put(event.request, response.clone());
						}
						return response;
					});
				})
			)
		);
		return;
	}

	// Cache-first for static assets
	if (
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
