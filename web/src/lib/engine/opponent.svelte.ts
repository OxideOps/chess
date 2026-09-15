import { parseEngineMessage } from '$lib/chess/wasm';
import type { EngineScore } from '$lib/generated/EngineScore';
import { Engine, type EngineEvent, type EngineLike } from './worker';

/** How long the engine thinks per move when it plays against you. */
export const MOVE_TIME_MS = 400;

export interface OpponentOptions {
	movetime?: number;
	/** Injected in tests; defaults to the Stockfish worker. */
	createEngine?: (onEvent: (event: EngineEvent) => void) => EngineLike;
}

/** What one search found: its move, and how it rates the position it searched. */
export interface Search {
	/** The move to play (UCI); `null` if there is none (mate or stalemate). */
	best: string | null;
	/** For the side to move in the searched position; `null` if none was reported. */
	score: EngineScore | null;
	/** The expected line from the searched position, `best` first. */
	pv: string[];
	/** How deep that line was searched, when the engine said. */
	depth?: number;
}

/** What a drill needs from an opponent; `Opponent` is the real one. */
export interface OpponentLike {
	/** Search the position after `moves` from `fen`; `null` if replaced or failed. */
	search(fen: string, moves: string[]): Promise<Search | null>;
	dispose(): void;
}

/**
 * Stockfish playing a side: asked about a position, it thinks for `movetime`
 * and reports its best move with its evaluation and line. One request at a
 * time; a new request stops the previous search and that one resolves `null`.
 */
export class Opponent implements OpponentLike {
	status: 'loading' | 'ready' | 'failed' = $state('loading');
	error: string | null = $state(null);

	readonly #engine: EngineLike;
	readonly #movetime: number;
	readonly #ready: Promise<boolean>;
	#markReady!: (ok: boolean) => void;
	#pending: ((search: Search | null) => void) | null = null;
	/** The deepest main line seen in the current search. */
	#line: { score: EngineScore; pv: string[]; depth: number } | null = null;

	constructor({ movetime = MOVE_TIME_MS, createEngine }: OpponentOptions = {}) {
		this.#movetime = movetime;
		this.#ready = new Promise((resolve) => (this.#markReady = resolve));
		const create = createEngine ?? ((onEvent) => new Engine(onEvent));
		this.#engine = create((event) => this.#onEvent(event));
		this.#engine.send('uci');
	}

	async search(fen: string, moves: string[]): Promise<Search | null> {
		if (!(await this.#ready)) return null;
		// A newer request replaces an older one: it resolves null, and `stop`
		// ends its search (its bestmove then arrives with nobody waiting).
		this.#settle(null);
		const position =
			moves.length > 0 ? `position fen ${fen} moves ${moves.join(' ')}` : `position fen ${fen}`;
		return new Promise((resolve) => {
			this.#pending = resolve;
			this.#line = null;
			this.#engine.send(position);
			this.#engine.send(`go movetime ${this.#movetime}`);
		});
	}

	dispose(): void {
		this.#settle(null);
		this.#engine.terminate();
	}

	#settle(search: Search | null): void {
		const pending = this.#pending;
		this.#pending = null;
		if (pending) {
			if (search === null) this.#engine.send('stop');
			pending(search);
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
			case 'info':
				if (this.#pending && msg.line.multipv === 1) {
					this.#line = { score: msg.line.score, pv: msg.line.pv, depth: msg.line.depth };
				}
				break;
			case 'best_move': {
				const pending = this.#pending;
				this.#pending = null;
				pending?.({
					best: msg.best,
					score: this.#line?.score ?? null,
					pv: this.#line?.pv ?? (msg.best ? [msg.best] : []),
					depth: this.#line?.depth
				});
				break;
			}
			default:
				break;
		}
	}
}
