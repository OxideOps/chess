import { initChess } from '$lib/chess/wasm';

// Single-page app: no server rendering (the chess-core WASM only runs in the
// browser), but every route is prerendered as an HTML shell so a static host
// can serve it directly.
export const ssr = false;
export const prerender = true;

// Nothing renders before chess-core is loaded, so pages can use it synchronously.
export async function load() {
	await initChess();
}
