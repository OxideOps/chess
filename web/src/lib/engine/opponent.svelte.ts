import { parseEngineMessage } from '$lib/chess/wasm';
import { Engine, type EngineEvent, type EngineLike } from './worker';

/** How long the engine thinks per move when it plays against you. */
export const MOVE_TIME_MS = 400;

export interface OpponentOptions {
	movetime?: number;
	/** Injected in tests; defaults to the Stockfish worker. */
	createEngine?: (onEvent: (event: EngineEvent) => void) => EngineLike;
}

/** What a drill needs from an opponent; `Opponent` is the real one. */
export interface OpponentLike {
	/** The engine's move (UCI) after `moves` from `fen`; `null` if it has none or failed. */
	move(fen: string, moves: string[]): Promise<string | null>;
	dispose(): void;
}

/**
 * Stockfish playing a side: asked for a move, it thinks for `movetime` and
 * answers with its best one. One request at a time; a new request stops the
 * previous search and that one resolves `null`.
 */
export class Opponent implements OpponentLike {
	status: 'loading' | 'ready' | 'failed' = $state('loading');
	error: string | null = $state(null);

	readonly #engine: EngineLike;
	readonly #movetime: number;
	readonly #ready: Promise<boolean>;
	#markReady!: (ok: boolean) => void;
	#pending: ((uci: string | null) => void) | null = null;

	constructor({ movetime = MOVE_TIME_MS, createEngine }: OpponentOptions = {}) {
		this.#movetime = movetime;
		this.#ready = new Promise((resolve) => (this.#markReady = resolve));
		const create = createEngine ?? ((onEvent) => new Engine(onEvent));
		this.#engine = create((event) => this.#onEvent(event));
		this.#engine.send('uci');
	}

	async move(fen: string, moves: string[]): Promise<string | null> {
		if (!(await this.#ready)) return null;
		// A newer request replaces an older one: it resolves null, and `stop`
		// ends its search (its bestmove then arrives with nobody waiting).
		this.#settle(null);
		const position =
			moves.length > 0 ? `position fen ${fen} moves ${moves.join(' ')}` : `position fen ${fen}`;
		return new Promise((resolve) => {
			this.#pending = resolve;
			this.#engine.send(position);
			this.#engine.send(`go movetime ${this.#movetime}`);
		});
	}

	dispose(): void {
		this.#settle(null);
		this.#engine.terminate();
	}

	#settle(uci: string | null): void {
		const pending = this.#pending;
		this.#pending = null;
		if (pending) {
			if (uci === null) this.#engine.send('stop');
			pending(uci);
		}
	}

	#onEvent(event: EngineEvent): void {
		if (event.type === 'error') {
			this.status = 'failed';
			this.error = event.message;
			this.#markReady(false);
			this.#settle(null);
			return;
		}
		const msg = parseEngineMessage(event.text);
		switch (msg.type) {
			case 'uci_ok':
				this.#engine.send('isready');
				break;
			case 'ready_ok':
				this.status = 'ready';
				this.#markReady(true);
				break;
			case 'best_move': {
				const pending = this.#pending;
				this.#pending = null;
				pending?.(msg.best);
				break;
			}
			default:
				break;
		}
	}
}
