// Stockfish.js (GPLv3, see static/engine/COPYING.txt) running in a Web
// Worker as a separate program; we talk UCI text to it over postMessage.
// The loader finds its `.wasm` from the URL fragment.
import { base } from '$app/paths';

export type EngineEvent = { type: 'line'; text: string } | { type: 'error'; message: string };

/** What the analyser needs from an engine; `Engine` is the real one. */
export interface EngineLike {
	send(command: string): void;
	terminate(): void;
}

export class Engine implements EngineLike {
	#worker: Worker;

	constructor(onEvent: (event: EngineEvent) => void) {
		const js = `${base}/engine/stockfish-18-lite-single.js`;
		const wasm = `${base}/engine/stockfish-18-lite-single.wasm`;
		this.#worker = new Worker(`${js}#${wasm}`);
		this.#worker.onmessage = (event: MessageEvent) => {
			if (typeof event.data === 'string') onEvent({ type: 'line', text: event.data });
		};
		this.#worker.onerror = (event: ErrorEvent) => {
			onEvent({ type: 'error', message: event.message || 'engine worker failed' });
		};
	}

	send(command: string): void {
		this.#worker.postMessage(command);
	}

	terminate(): void {
		this.#worker.terminate();
	}
}
