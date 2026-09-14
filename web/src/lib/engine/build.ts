// Which Stockfish.js build to run. The multi-threaded one needs
// `SharedArrayBuffer`, which browsers only give to cross-origin isolated
// pages (the Rust server and the Vite dev server send the COOP/COEP
// headers); anywhere else the single-threaded build is the fallback.

export interface EngineBuild {
	/** File stem under `/engine/`, e.g. `stockfish-18-lite`. */
	stem: string;
	/** How many search threads to ask for; 1 for the single-threaded build. */
	threads: number;
}

export interface BuildEnvironment {
	crossOriginIsolated: boolean;
	hardwareConcurrency: number;
}

/** Leave a core for the UI and the WASM main thread; cap so the lite net's
 * small hash table doesn't drown in threads on big machines. */
const MAX_THREADS = 8;

export function pickBuild(env: BuildEnvironment): EngineBuild {
	if (!env.crossOriginIsolated) {
		return { stem: 'stockfish-18-lite-single', threads: 1 };
	}
	const cores = Math.max(1, Math.floor(env.hardwareConcurrency) || 1);
	const threads = Math.min(MAX_THREADS, Math.max(1, cores - 1));
	return { stem: 'stockfish-18-lite', threads };
}

/** What the current browser offers. */
export function currentEnvironment(): BuildEnvironment {
	return {
		crossOriginIsolated:
			typeof crossOriginIsolated !== 'undefined' &&
			crossOriginIsolated &&
			typeof SharedArrayBuffer !== 'undefined',
		hardwareConcurrency: navigator.hardwareConcurrency ?? 1
	};
}
