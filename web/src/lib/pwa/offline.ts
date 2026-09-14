// Ask the service worker to keep files for offline use. The engine build is
// only known in the page (it depends on cross-origin isolation), and on a
// first visit the engine loads before the service worker takes control, so
// the page names the files once the worker is ready.

export interface CacheRequest {
	type: 'cache-engine';
	urls: string[];
}

export function keepOffline(urls: string[]): void {
	if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return;
	const message: CacheRequest = { type: 'cache-engine', urls };
	navigator.serviceWorker.ready
		.then((registration) => registration.active?.postMessage(message))
		.catch(() => {
			// No service worker (unsupported, blocked, or not registered): nothing to keep.
		});
}
