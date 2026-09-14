// Stockfish.js (GPLv3, see static/engine/COPYING.txt) running in a Web
// Worker as a separate program; we talk UCI text to it over postMessage.
// The loader finds its `.wasm` from the URL fragment; the multi-threaded
// build spawns its own thread workers from the same script.
import { base } from '$app/paths';
import { keepOffline } from '$lib/pwa/offline';
import {
	currentEnvironment,
	pickBuild,
	SINGLE_THREADED,
	type BuildEnvironment,
	type EngineBuild
} from './build';

export type EngineEvent = { type: 'line'; text: string } | { type: 'error'; message: string };

/** What the analyser needs from an engine; `Engine` is the real one. */
export interface EngineLike {
	send(command: string): void;
	terminate(): void;
	/** Search threads this engine can use (1 unless multi-threaded). Final once it has answered. */
	readonly threads: number;
}

/** The part of `Worker` the engine uses; injectable for tests. */
export interface WorkerLike {
	postMessage(message: string): void;
	terminate(): void;
	onmessage: ((event: { data: unknown }) => void) | null;
	onerror: ((event: { message?: string }) => void) | null;
}

export interface EngineOptions {
	environment?: BuildEnvironment;
	spawn?: (url: string) => WorkerLike;
}

/**
 * The engine in a worker. If the multi-threaded build fails before it has
 * said anything (a browser that can't give it the shared memory it asks
 * for, which is the worry on phones), it is replaced by the single-threaded
 * build and the commands sent so far are replayed, so the caller never
 * sees the failure.
 */
export class Engine implements EngineLike {
	#build: EngineBuild;
	#worker!: WorkerLike;
	/** Whether the current worker has answered yet. */
	#heard = false;
	/** Commands sent before the first answer, to replay on a fallback. */
	#early: string[] = [];
	readonly #onEvent: (event: EngineEvent) => void;
	readonly #spawn: (url: string) => WorkerLike;

	constructor(onEvent: (event: EngineEvent) => void, options: EngineOptions = {}) {
		this.#onEvent = onEvent;
		this.#spawn = options.spawn ?? ((url) => new Worker(url) as unknown as WorkerLike);
		this.#build = pickBuild(options.environment ?? currentEnvironment());
		this.#start();
	}

	get build(): EngineBuild {
		return this.#build;
	}

	get threads(): number {
		return this.#build.threads;
	}

	send(command: string): void {
		if (!this.#heard) this.#early.push(command);
		this.#worker.postMessage(command);
	}

	terminate(): void {
		this.#worker.terminate();
	}

	#start(): void {
		const js = `${base}/engine/${this.#build.stem}.js`;
		const wasm = `${base}/engine/${this.#build.stem}.wasm`;
		const worker = this.#spawn(`${js}#${wasm}`);
		this.#worker = worker;
		this.#heard = false;
		keepOffline([js, wasm]);
		worker.onmessage = (event) => {
			if (worker !== this.#worker || typeof event.data !== 'string') return;
			this.#heard = true;
			this.#early = [];
			this.#onEvent({ type: 'line', text: event.data });
		};
		worker.onerror = (event) => {
			if (worker !== this.#worker) return;
			const message = event.message || 'engine worker failed';
			if (!this.#heard && this.#build.threads > 1) this.#fallBack(message);
			else this.#onEvent({ type: 'error', message });
		};
	}

	#fallBack(reason: string): void {
		console.warn(
			`multi-threaded engine failed to start (${reason}); using the single-threaded one`
		);
		this.#worker.terminate();
		this.#build = SINGLE_THREADED;
		const replay = this.#early;
		this.#early = [];
		this.#start();
		for (const command of replay) this.send(command);
	}
}
