import { initChess } from '$lib/chess/wasm';
import { session } from '$lib/auth/session.svelte';

// Single-page app: no server rendering (the chess-core WASM only runs in the
// browser), but every route is prerendered as an HTML shell so a static host
// can serve it directly.
export const ssr = false;
export const prerender = true;

// Nothing renders before chess-core is loaded and we know who is signed in,
// so pages use the WASM synchronously and the nav shows the right name at once.
export async function load() {
	await Promise.all([initChess(), session.load()]);
}
