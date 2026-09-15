/// <reference types="@sveltejs/kit" />
/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" />

// Makes a second visit instant and the app usable offline (local play and
// analysis; online play needs the server, of course).
//
// - The app shell (built JS/CSS, the prerendered pages, icons, pieces) is
//   cached on install, per build: a new deploy is a new cache.
// - The Stockfish builds (7 MB each) are cached the first time they are
//   used, in a cache that outlives deploys, so only the build this browser
//   runs is ever downloaded.
// - `/api` is never touched: sessions, games, sockets and OAuth are live.
// - Other navigations (e.g. /game/<id>) go to the network and fall back to
//   the cached `404.html` when offline: the SPA shell with absolute asset
//   paths (the prerendered pages use relative ones, so they only work at
//   their own URL), which renders whatever route the URL names.
//
// Cached responses keep the headers they were served with, including the
// cross-origin isolation headers, so the multi-threaded engine still runs.
import { build, files, prerendered, version } from '$service-worker';
import type { CacheRequest } from '$lib/pwa/offline';
import type { SkipWaiting } from '$lib/pwa/updates.svelte';

const sw = self as unknown as ServiceWorkerGlobalScope;

const APP_CACHE = `app-${version}`;
const ENGINE_CACHE = 'engine';
const isEngineFile = (path: string) => /^\/engine\/.+\.(js|wasm)$/.test(path);

const SHELL = [...build, ...prerendered, ...files.filter((f) => !isEngineFile(f))];
const SHELL_SET = new Set(SHELL);
/** Written by adapter-static after the build, so not in `build` or `files`. */
const FALLBACK = '/404.html';
const ENGINE_FILES = new Set(files.filter(isEngineFile));

sw.addEventListener('install', (event) => {
	event.waitUntil(
		(async () => {
			const cache = await caches.open(APP_CACHE);
			await cache.addAll(SHELL);
			// Absent under `vite dev`, which is fine: there's no offline story there.
			await cache.add(FALLBACK).catch(() => {});
		})()
	);
});

sw.addEventListener('activate', (event) => {
	event.waitUntil(
		(async () => {
			for (const key of await caches.keys()) {
				if (key !== APP_CACHE && key !== ENGINE_CACHE) await caches.delete(key);
			}
			// Drop engine builds this version no longer ships.
			const engine = await caches.open(ENGINE_CACHE);
			for (const request of await engine.keys()) {
				if (!ENGINE_FILES.has(new URL(request.url).pathname)) await engine.delete(request);
			}
			// Control pages that opened before this worker existed (the first
			// visit), so the engine they load next is cached too.
			await sw.clients.claim();
		})()
	);
});

async function cacheFirst(cacheName: string, request: Request): Promise<Response> {
	const cache = await caches.open(cacheName);
	const hit = await cache.match(request);
	if (hit) return hit;
	const response = await fetch(request);
	if (response.status === 200) await cache.put(request, response.clone());
	return response;
}

/**
 * The engine's loader reads its URL fragment (`#<wasm>` for the engine,
 * `#<wasm>,worker` for each of its search threads) to know what it is. A
 * worker's URL is its response's URL, and a cached or fetched response has
 * the fragment stripped, so every thread would think it was a new engine
 * and spawn threads of its own, forever. A newly constructed response has
 * no URL, which makes the browser use the request's, fragment included.
 */
async function engineFile(request: Request): Promise<Response> {
	const response = await cacheFirst(ENGINE_CACHE, request);
	return new Response(response.body, {
		status: response.status,
		statusText: response.statusText,
		headers: response.headers
	});
}

async function networkThenShell(request: Request): Promise<Response> {
	try {
		return await fetch(request);
	} catch {
		const cache = await caches.open(APP_CACHE);
		const shell = (await cache.match(request)) ?? (await cache.match(FALLBACK));
		if (shell) return shell;
		throw new Error('offline and no cached shell');
	}
}

// The page names the engine build it runs (see `keepOffline`); fetch what
// isn't cached yet so the next visit, online or not, has it.
sw.addEventListener('message', (event) => {
	const data = event.data as CacheRequest | SkipWaiting | undefined;
	// A new version waits until every tab has closed, so a page never runs
	// half-old, half-new code. The page asks it to take over when the user
	// chooses to reload (see `src/lib/pwa/updates.svelte.ts`).
	if (data?.type === 'skip-waiting') {
		void sw.skipWaiting();
		return;
	}
	if (data?.type !== 'cache-engine') return;
	event.waitUntil(
		(async () => {
			const cache = await caches.open(ENGINE_CACHE);
			for (const url of data.urls) {
				const path = new URL(url, sw.location.origin).pathname;
				if (!ENGINE_FILES.has(path) || (await cache.match(path))) continue;
				const response = await fetch(path);
				if (response.status === 200) await cache.put(path, response);
			}
		})()
	);
});

sw.addEventListener('fetch', (event) => {
	const { request } = event;
	if (request.method !== 'GET') return;
	const url = new URL(request.url);
	if (url.origin !== sw.location.origin || url.pathname.startsWith('/api/')) return;

	if (ENGINE_FILES.has(url.pathname)) {
		event.respondWith(engineFile(request));
	} else if (SHELL_SET.has(url.pathname)) {
		event.respondWith(cacheFirst(APP_CACHE, request));
	} else if (request.mode === 'navigate') {
		event.respondWith(networkThenShell(request));
	}
});
