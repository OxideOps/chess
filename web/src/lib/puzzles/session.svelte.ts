import { Game, judgePuzzle, type PlayResult } from '$lib/chess/wasm';
import { GameStore, type Promotion } from '$lib/chess/game.svelte';
import type { AttemptResult } from '$lib/generated/AttemptResult';
import type { PuzzleData } from '$lib/generated/PuzzleData';
import type { Side } from '$lib/generated/Side';

export type PuzzlePhase =
	| 'loading'
	/** Showing the opponent's move that sets the puzzle up. */
	| 'setup'
	| 'solving'
	| 'solved'
	| 'failed'
	/** The server has no puzzle for us (none imported, or all tried). */
	| 'empty'
	| 'error';

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

export interface PuzzleSessionOptions {
	fetch?: FetchLike;
	/** Pauses between moves the computer plays; injectable so tests don't wait. */
	delay?: (ms: number) => Promise<void>;
}

/** How long the setup move and each reply take to appear. */
export const MOVE_DELAY_MS = 450;

/**
 * One puzzle at a time: load it from the server, play the setup move, judge
 * each of the solver's moves with chess-core (via `judgePuzzle`), answer
 * with the opponent's replies, and report the first try to the server,
 * which rates it.
 */
export class PuzzleSession {
	readonly game = new GameStore();
	phase: PuzzlePhase = $state('loading');
	puzzle: PuzzleData | null = $state(null);
	/** The side the solver plays (the side to move after the setup). */
	solver: Side = $state('white');
	/** After a wrong move: the move that was wanted, in SAN. */
	expected: string | null = $state(null);
	/** What the server made of this puzzle, once reported. */
	result: AttemptResult | null = $state(null);
	error: string | null = $state(null);

	readonly #fetch: FetchLike;
	readonly #delay: (ms: number) => Promise<void>;
	/** Every move since the setup: the solver's and the replies. */
	#played: string[] = [];
	/** Bumped per puzzle so a slow reply can't land on the next one. */
	#generation = 0;

	constructor({ fetch: fetchImpl, delay }: PuzzleSessionOptions = {}) {
		this.#fetch = fetchImpl ?? ((input, init) => fetch(input, init));
		this.#delay = delay ?? ((ms) => new Promise((resolve) => setTimeout(resolve, ms)));
	}

	/** Moves the solver can make now. */
	get canMove(): boolean {
		return this.phase === 'solving' && !this.game.view.viewingHistory;
	}

	/** Load the next puzzle and play its setup move. */
	async next(): Promise<void> {
		const generation = ++this.#generation;
		this.phase = 'loading';
		this.expected = null;
		this.result = null;
		this.error = null;
		this.#played = [];
		let puzzle: PuzzleData;
		try {
			const response = await this.#fetch('/api/puzzles/next');
			if (response.status === 404) {
				if (generation === this.#generation) this.phase = 'empty';
				return;
			}
			if (!response.ok) throw new Error(`the server said ${response.status}`);
			puzzle = (await response.json()) as PuzzleData;
		} catch (e) {
			if (generation === this.#generation) {
				this.error = e instanceof Error ? e.message : String(e);
				this.phase = 'error';
			}
			return;
		}
		if (generation !== this.#generation) return;
		this.puzzle = puzzle;
		this.game.replace(Game.fromFen(puzzle.fen));
		// The solver answers the side that plays the setup move.
		this.solver = this.game.view.turn === 'white' ? 'black' : 'white';
		this.phase = 'setup';
		await this.#delay(MOVE_DELAY_MS);
		if (generation !== this.#generation) return;
		this.game.playUci(puzzle.moves[0]);
		this.phase = 'solving';
	}

	/** Board callback: play the solver's move and judge it. */
	tryMove(from: string, to: string, promotion?: Promotion): PlayResult {
		if (!this.canMove || !this.puzzle) return 'illegal';
		const played = this.game.play(from, to, promotion);
		if (played !== 'ok') return played;
		const uci = this.game.view.moves.at(-1)!.uci;
		this.#played.push(uci);
		const puzzle = this.puzzle;
		const verdict = judgePuzzle(puzzle.fen, puzzle.moves, this.#played);
		switch (verdict.kind) {
			case 'correct':
				void this.#reply(verdict.reply);
				break;
			case 'solved':
				this.phase = 'solved';
				void this.#report(true);
				break;
			case 'wrong':
				this.phase = 'failed';
				this.expected = verdict.expected_san;
				void this.#report(false);
				break;
		}
		return 'ok';
	}

	/** After a wrong move: take it back and play the rest of the solution. */
	async showSolution(): Promise<void> {
		const puzzle = this.puzzle;
		if (this.phase !== 'failed' || !puzzle) return;
		const generation = this.#generation;
		this.phase = 'solved';
		this.game.goBack();
		this.#played.pop();
		for (const uci of puzzle.moves.slice(1 + this.#played.length)) {
			await this.#delay(MOVE_DELAY_MS);
			if (generation !== this.#generation) return;
			this.game.playHereUci(uci);
			this.#played.push(uci);
		}
	}

	dispose(): void {
		this.#generation++;
		this.game.dispose();
	}

	async #reply(uci: string): Promise<void> {
		const generation = this.#generation;
		this.phase = 'setup'; // the opponent is "thinking": no moves meanwhile
		await this.#delay(MOVE_DELAY_MS);
		if (generation !== this.#generation) return;
		this.game.playUci(uci);
		this.#played.push(uci);
		this.phase = 'solving';
	}

	async #report(solved: boolean): Promise<void> {
		const puzzle = this.puzzle;
		if (!puzzle) return;
		const generation = this.#generation;
		try {
			const response = await this.#fetch(`/api/puzzles/${encodeURIComponent(puzzle.id)}/attempt`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ solved })
			});
			if (!response.ok) throw new Error(`the server said ${response.status}`);
			const result = (await response.json()) as AttemptResult;
			if (generation === this.#generation) this.result = result;
		} catch (e) {
			// The puzzle still counts as done here; only the rating update is lost.
			if (generation === this.#generation) {
				this.error = `could not record the result: ${e instanceof Error ? e.message : String(e)}`;
			}
		}
	}
}
