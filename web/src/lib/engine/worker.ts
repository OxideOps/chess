// Stockfish.js (GPLv3, see static/engine/COPYING.txt) running in a Web
// Worker as a separate program; we talk UCI text to it over postMessage.
// The loader finds its `.wasm` from the URL fragment; the multi-threaded
// build spawns its own thread workers from the same script.
import { base } from '$app/paths';
import { currentEnvironment, pickBuild, type EngineBuild } from './build';

export type EngineEvent = { type: 'line'; text: string } | { type: 'error'; message: string };

/** What the analyser needs from an engine; `Engine` is the real one. */
export interface EngineLike {
	send(command: string): void;
	terminate(): void;
	/** Search threads this engine can use (1 unless multi-threaded). */
	readonly threads: number;
}

export class Engine implements EngineLike {
	#worker: Worker;
	readonly build: EngineBuild;

	constructor(onEvent: (event: EngineEvent) => void) {
		this.build = pickBuild(currentEnvironment());
		const js = `${base}/engine/${this.build.stem}.js`;
		const wasm = `${base}/engine/${this.build.stem}.wasm`;
		this.#worker = new Worker(`${js}#${wasm}`);
		this.#worker.onmessage = (event: MessageEvent) => {
			if (typeof event.data === 'string') onEvent({ type: 'line', text: event.data });
		};
		this.#worker.onerror = (event: ErrorEvent) => {
			onEvent({ type: 'error', message: event.message || 'engine worker failed' });
		};
	}

	get threads(): number {
		return this.build.threads;
	}

	send(command: string): void {
		this.#worker.postMessage(command);
	}

	terminate(): void {
		this.#worker.terminate();
	}
}
