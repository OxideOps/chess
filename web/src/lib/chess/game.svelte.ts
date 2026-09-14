import { Game, type PlayResult } from './wasm';
import type { GameView } from '$lib/generated/GameView';

export type Promotion = 'queen' | 'rook' | 'bishop' | 'knight';

/**
 * A chess game as reactive state. Wraps the WASM `Game` and refreshes a
 * `GameView` snapshot after every change, so components only ever read
 * `view`. All chess logic lives in chess-core; nothing here decides what is
 * legal.
 */
export class GameStore {
	#game: Game;
	view: GameView = $state(emptyView());

	constructor(game: Game = new Game()) {
		this.#game = game;
		this.refresh();
	}

	/** Throws with the parser's message on invalid input. */
	static fromFen(fen: string): GameStore {
		return new GameStore(Game.fromFen(fen));
	}

	/** Throws with the parser's message on invalid input. */
	static fromPgn(pgn: string): GameStore {
		return new GameStore(Game.fromPgn(pgn));
	}

	/** Replace the game (e.g. after an import) and drop the old one. */
	replace(game: Game): void {
		this.#game.free();
		this.#game = game;
		this.refresh();
	}

	/** Release the WASM memory. The store is unusable afterwards. */
	dispose(): void {
		this.#game.free();
	}

	legalDestinations(from: string): string[] {
		return this.#game.legalDestinations(from);
	}

	/** Play at the end of the game. */
	play(from: string, to: string, promotion?: Promotion): PlayResult {
		return this.#mutate(() => this.#game.playFromTo(from, to, promotion));
	}

	/**
	 * Play from the viewed position: a move already played there is
	 * followed, a new one starts a variation (see `chess_core::Game`).
	 */
	playHere(from: string, to: string, promotion?: Promotion): PlayResult {
		return this.#mutate(() => this.#game.playHereFromTo(from, to, promotion));
	}

	playUci(uci: string): PlayResult {
		return this.#mutate(() => this.#game.playUci(uci));
	}

	playHereUci(uci: string): PlayResult {
		return this.#mutate(() => this.#game.playHereUci(uci));
	}

	/** View a move anywhere in the tree; its line becomes current. */
	goToNode(id: number): void {
		this.#mutate(() => this.#game.goToNode(id));
	}

	/** The next (`1`) or previous (`-1`) alternative to the move at the cursor. */
	switchVariation(step: 1 | -1): void {
		this.#mutate(() => this.#game.switchVariation(step));
	}

	/** Promote the variation the cursor is in by one level. */
	promoteVariation(): boolean {
		return this.#mutate(() => this.#game.promote(this.view.node));
	}

	/** Delete the move at the cursor and everything after it. */
	deleteFromHere(): boolean {
		return this.#mutate(() => this.#game.deleteFrom(this.view.node));
	}

	goToPly(ply: number): void {
		this.#mutate(() => this.#game.goToPly(ply));
	}

	goBack(): void {
		this.#mutate(() => this.#game.goBack());
	}

	goForward(): void {
		this.#mutate(() => this.#game.goForward());
	}

	goToStart(): void {
		this.#mutate(() => this.#game.goToStart());
	}

	goToEnd(): void {
		this.#mutate(() => this.#game.goToEnd());
	}

	pgn(): string {
		return this.#game.pgn();
	}

	movetext(): string {
		return this.#game.movetext();
	}

	#mutate<T>(f: () => T): T {
		const result = f();
		this.refresh();
		return result;
	}

	private refresh(): void {
		this.view = this.#game.view() as GameView;
	}
}

function emptyView(): GameView {
	return {
		fen: '',
		turn: 'white',
		status: 'ongoing',
		winner: null,
		gameOver: false,
		cursor: 0,
		plyCount: 0,
		viewingHistory: false,
		lastMove: null,
		checkSquare: null,
		pieces: [],
		moves: [],
		tree: [],
		node: 0,
		mainLine: true,
		movetext: '',
		pgn: '*',
		startTurn: 'white',
		startFullmove: 1
	};
}
