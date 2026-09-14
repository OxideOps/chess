import { parseEngineMessage } from '$lib/chess/wasm';
import type { EngineLine } from '$lib/generated/EngineLine';
import type { Side } from '$lib/generated/Side';
import { Engine, type EngineEvent, type EngineLike } from './worker';

/**
 * How far the engine searches before stopping on its own. Deep enough to be
 * useful, shallow enough that an idle tab stops burning CPU.
 */
export const MAX_DEPTH = 24;

export type EngineStatus = 'loading' | 'ready' | 'failed';

export interface AnalyserOptions {
	multipv?: number;
	/** Injected in tests; defaults to the Stockfish worker. */
	createEngine?: (onEvent: (event: EngineEvent) => void) => EngineLike;
}

/**
 * Keeps an engine pointed at whatever position the UI wants analysed and
 * exposes its principal variations as reactive state.
 *
 * UCI has no request ids, so a new position is never sent while a search is
 * running: we send `stop`, wait for the `bestmove` that ends the old search,
 * and only then start the new one. That guarantees every line in `lines`
 * belongs to `fen`.
 */
export class Analyser {
	status: EngineStatus = $state('loading');
	error: string | null = $state(null);
	name: string | null = $state(null);
	/** Search threads in use, known once the engine is created. */
	threads = $state(1);
	/** The position the lines are for; `null` when idle. */
	fen: string | null = $state(null);
	turn: Side = $state('white');
	/** Best lines first (by multipv), each the deepest seen for its slot. */
	lines: EngineLine[] = $state([]);
	searching = $state(false);

	readonly #engine: EngineLike;
	readonly #multipv: number;
	#ready = false;
	/** The position to search once the engine is free, if any. */
	#pending: string | null = null;

	constructor({ multipv = 3, createEngine }: AnalyserOptions = {}) {
		this.#multipv = multipv;
		const create = createEngine ?? ((onEvent) => new Engine(onEvent));
		this.#engine = create((event) => this.#onEvent(event));
		this.threads = this.#engine.threads;
		this.#engine.send('uci');
	}

	get best(): EngineLine | undefined {
		return this.lines[0];
	}

	get depth(): number | undefined {
		return this.best?.depth;
	}

	/** Ask for `target` to be analysed; `null` pauses the engine. */
	request(target: string | null): void {
		if (target === null) {
			this.#pending = null;
			if (this.searching) this.#engine.send('stop');
			this.fen = null;
			this.lines = [];
			this.searching = false;
			return;
		}
		if (this.fen === target && this.#pending === null) return;
		if (this.#ready && !this.searching) {
			this.#start(target);
		} else {
			this.#pending = target;
			if (this.searching) this.#engine.send('stop');
		}
	}

	/** Terminate the engine. The analyser is unusable afterwards. */
	dispose(): void {
		this.#engine.terminate();
	}

	#start(fen: string): void {
		this.#engine.send(`position fen ${fen}`);
		this.#engine.send(`go depth ${MAX_DEPTH}`);
		this.turn = fen.split(/\s+/)[1] === 'b' ? 'black' : 'white';
		this.fen = fen;
		this.lines = [];
		this.searching = true;
	}

	#onEvent(event: EngineEvent): void {
		if (event.type === 'error') {
			this.status = 'failed';
			this.error = event.message;
			this.searching = false;
			this.lines = [];
			return;
		}
		const msg = parseEngineMessage(event.text);
		switch (msg.type) {
			case 'uci_ok':
				this.#engine.send(`setoption name MultiPV value ${this.#multipv}`);
				if (this.threads > 1) {
					this.#engine.send(`setoption name Threads value ${this.threads}`);
				}
				this.#engine.send('isready');
				break;
			case 'ready_ok':
				if (!this.#ready) {
					this.#ready = true;
					this.status = 'ready';
					if (this.#pending !== null) {
						const fen = this.#pending;
						this.#pending = null;
						this.#start(fen);
					}
				}
				break;
			case 'info':
				// Lines that arrive after a `stop` belong to a search we no longer show.
				if (this.searching && this.fen !== null) {
					const line = msg.line;
					const i = this.lines.findIndex((l) => l.multipv === line.multipv);
					const lines = i >= 0 ? this.lines.with(i, line) : [...this.lines, line];
					lines.sort((a, b) => a.multipv - b.multipv);
					this.lines = lines;
				}
				break;
			case 'best_move':
				this.searching = false;
				if (this.#pending !== null) {
					const fen = this.#pending;
					this.#pending = null;
					this.#start(fen);
				}
				break;
			case 'other':
				if (msg.text.startsWith('id name ')) this.name = msg.text.slice('id name '.length);
				break;
		}
	}
}
