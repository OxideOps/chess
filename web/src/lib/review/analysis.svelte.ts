import { GameStore } from '$lib/chess/game.svelte';
import { Opponent, type OpponentLike } from '$lib/engine/opponent.svelte';
import type { PositionEval } from '$lib/generated/PositionEval';

/**
 * How deep the engine looks at each position of a game under review. Shallow
 * on purpose: a whole game is dozens of positions on the player's own
 * battery, and a big swing is big enough to see at this depth.
 */
export const REVIEW_DEPTH = 12;

/** How many moves of each engine line are kept: enough to show the idea. */
const LINE_MOVES = 8;

/** One position of the game: its FEN, and whether the game is over there. */
export interface ReviewPosition {
	fen: string;
	over: boolean;
}

/** Every position of a game's main line, from the start. */
export function positionsOf(pgn: string): ReviewPosition[] {
	const game = GameStore.fromPgn(pgn);
	try {
		const out: ReviewPosition[] = [];
		for (let ply = 0; ply <= game.view.plyCount; ply++) {
			game.goToPly(ply);
			out.push({ fen: game.view.fen, over: game.view.gameOver });
		}
		return out;
	} finally {
		game.dispose();
	}
}

export type AnalysisStatus = 'idle' | 'running' | 'stopped' | 'done' | 'failed';

/** Where finished work is kept; `localStorage` unless a test says otherwise. */
export interface EvalCache {
	load(key: string): (PositionEval | null)[] | null;
	save(key: string, evals: (PositionEval | null)[]): void;
}

export interface GameAnalysisOptions {
	depth?: number;
	cache?: EvalCache;
	/** Injected in tests; defaults to Stockfish searching to `depth`. */
	createSearcher?: () => OpponentLike;
}

/**
 * The engine's pass over every position of a finished game, one position at
 * a time in its Web Worker, so the page stays responsive and can show how
 * far it has got. `stop()` ends it at once (the worker is terminated, so the
 * CPU is given back); `start()` carries on from where it stopped. Each
 * position is saved as it is done, so reopening the review — or coming back
 * after leaving half way — doesn't search anything twice.
 */
export class GameAnalysis {
	/** One per position done so far, from the start; `null` where there was nothing to search. */
	evals: (PositionEval | null)[] = $state([]);
	status: AnalysisStatus = $state('idle');
	error: string | null = $state(null);

	readonly #key: string;
	readonly #positions: ReviewPosition[];
	readonly #cache: EvalCache;
	readonly #create: () => OpponentLike;
	#searcher: OpponentLike | null = null;
	/** Bumped by `stop()`, so a search that finishes afterwards is dropped. */
	#run = 0;

	constructor(key: string, positions: ReviewPosition[], options: GameAnalysisOptions = {}) {
		const depth = options.depth ?? REVIEW_DEPTH;
		this.#key = `${key}:d${depth}`;
		this.#positions = positions;
		this.#cache = options.cache ?? localCache;
		this.#create = options.createSearcher ?? (() => new Opponent({ depth }));
		const saved = this.#cache.load(this.#key);
		if (saved && saved.length <= positions.length) {
			this.evals = saved;
			if (saved.length === positions.length) this.status = 'done';
		}
	}

	/** Positions in the game (the start included). */
	get total(): number {
		return this.#positions.length;
	}

	get done(): number {
		return this.evals.length;
	}

	/** Search every position not yet done. Does nothing if already running or done. */
	start(): void {
		if (this.status === 'running' || this.status === 'done') return;
		void this.#go(++this.#run);
	}

	/** Stop searching now; what is done so far is kept. */
	stop(): void {
		if (this.status !== 'running') return;
		this.#run++;
		this.#release();
		this.status = 'stopped';
	}

	dispose(): void {
		this.#run++;
		this.#release();
	}

	#release(): void {
		this.#searcher?.dispose();
		this.#searcher = null;
	}

	async #go(run: number): Promise<void> {
		this.status = 'running';
		this.error = null;
		while (this.evals.length < this.#positions.length) {
			const position = this.#positions[this.evals.length];
			let result: PositionEval | null = null;
			if (!position.over) {
				this.#searcher ??= this.#create();
				const search = await this.#searcher.search(position.fen, []);
				if (run !== this.#run) return;
				if (search === null) {
					this.#release();
					this.status = 'failed';
					this.error = 'The engine stopped working.';
					return;
				}
				if (search.score !== null) {
					result = { score: search.score, pv: search.pv.slice(0, LINE_MOVES) };
				}
			}
			this.evals = [...this.evals, result];
			this.#cache.save(this.#key, this.evals);
		}
		this.#release();
		this.status = 'done';
	}
}

/** Per browser, like lesson progress: missing storage just means no cache. */
const localCache: EvalCache = {
	load(key) {
		try {
			const raw = localStorage.getItem(`chess.review.${key}`);
			return raw ? (JSON.parse(raw) as (PositionEval | null)[]) : null;
		} catch {
			return null;
		}
	},
	save(key, evals) {
		try {
			localStorage.setItem(`chess.review.${key}`, JSON.stringify(evals));
		} catch {
			// storage unavailable or full: the review still works, it just isn't kept
		}
	}
};
