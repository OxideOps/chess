import { Game, assessDrill, isMistake, pvMovetext, type PlayResult } from '$lib/chess/wasm';
import { GameStore, type Promotion } from '$lib/chess/game.svelte';
import type { OpponentLike, Search } from '$lib/engine/opponent.svelte';
import type { Drill } from '$lib/generated/Drill';
import type { DrillStatus } from '$lib/generated/DrillStatus';
import type { EngineScore } from '$lib/generated/EngineScore';
import { markCompleted } from './progress';

/** A student move that threw away the drill (or a lot of it). */
export interface Mistake {
	/** The position the student moved from. */
	fen: string;
	/** The student's move, UCI and SAN. */
	played: string;
	playedSan: string;
	/** The engine's line from `fen`, its preferred move first (UCI). */
	better: string[];
	/** The preferred move as numbered SAN, e.g. `3. Qd4`. */
	betterSan: string;
	/** From the student's side: before the move, and after (`null` if the move ended the drill). */
	before: EngineScore;
	after: EngineScore | null;
}

/** What the engine expects in the position the student is about to move from. */
interface Expectation {
	fen: string;
	/** From the student's side. */
	score: EngineScore;
	/** The student's best line from `fen`. */
	pv: string[];
}

/** The same score from the other side (without producing -0). */
const flip = (s: EngineScore): EngineScore => ({ kind: s.kind, value: 0 - s.value || 0 });

/**
 * One drill against the engine: the student moves, chess-core judges where
 * the drill stands (`assessDrill`), and while it's going the engine answers.
 * The engine's own searches also say how good the position is, which is how
 * a student move that throws away the win (or the draw) is spotted.
 */
export class DrillSession {
	readonly game = new GameStore();
	readonly drill: Drill;
	status: DrillStatus = $state({ state: 'going', moves_left: 0 });
	/** The engine is thinking (choosing its move, or sizing up the start). */
	thinking = $state(false);
	error: string | null = $state(null);
	/** The student's latest mistake, if they made one. */
	mistake: Mistake | null = $state(null);

	readonly #opponent: OpponentLike;
	/** Bumped on restart so a late engine answer can't land in a new attempt. */
	#generation = 0;
	#expect: Expectation | null = null;

	constructor(drill: Drill, opponent: OpponentLike) {
		this.drill = drill;
		this.#opponent = opponent;
	}

	get canMove(): boolean {
		const view = this.game.view;
		return (
			this.status.state === 'going' &&
			!this.thinking &&
			!view.viewingHistory &&
			view.turn === this.drill.student
		);
	}

	/** Set the board up; the engine sizes up the start, or moves first. */
	async start(): Promise<void> {
		const generation = ++this.#generation;
		this.error = null;
		this.mistake = null;
		this.#expect = null;
		this.thinking = false;
		this.game.replace(Game.fromFen(this.drill.fen));
		this.#assess();
		if (this.game.view.turn !== this.drill.student) return this.#answer(null);
		// The student moves first: find out what the engine makes of the start.
		this.thinking = true;
		const search = await this.#opponent.search(this.drill.fen, []);
		if (generation !== this.#generation) return;
		this.thinking = false;
		if (search?.score) {
			this.#expect = { fen: this.game.view.fen, score: search.score, pv: search.pv };
		}
	}

	restart(): Promise<void> {
		return this.start();
	}

	/** Board callback. */
	tryMove(from: string, to: string, promotion?: Promotion): PlayResult {
		if (!this.canMove) return 'illegal';
		const expected = this.#expect;
		const result = this.game.play(from, to, promotion);
		if (result === 'ok') {
			const move = this.game.view.moves.at(-1)!;
			const judged = { expected, played: move.uci, playedSan: move.san };
			this.#assess();
			if (this.status.state === 'going') void this.#answer(judged);
			else if (this.status.state === 'lost') this.#noteMistake(judged, null);
		}
		return result;
	}

	dispose(): void {
		this.#generation++;
		this.#opponent.dispose();
		this.game.dispose();
	}

	#moves(): string[] {
		return this.game.view.moves.map((m) => m.uci);
	}

	#assess(): void {
		this.status = assessDrill(this.drill.id, this.#moves());
		if (this.status.state === 'won') markCompleted(this.drill.id);
	}

	#noteMistake(
		judged: { expected: Expectation | null; played: string; playedSan: string },
		after: EngineScore | null
	): void {
		const { expected, played, playedSan } = judged;
		if (!expected || expected.pv.length === 0 || expected.pv[0] === played) return;
		if (after !== null && !isMistake(expected.score, after)) return;
		this.mistake = {
			fen: expected.fen,
			played,
			playedSan,
			better: expected.pv,
			betterSan: pvMovetext(expected.fen, [expected.pv[0]]),
			before: expected.score,
			after
		};
	}

	/** Let the engine move; judge the student's move by what its search says. */
	async #answer(
		judged: { expected: Expectation | null; played: string; playedSan: string } | null
	): Promise<void> {
		const generation = this.#generation;
		this.thinking = true;
		const search: Search | null = await this.#opponent.search(this.drill.fen, this.#moves());
		if (generation !== this.#generation) return;
		this.thinking = false;
		if (search === null || search.best === null) {
			this.error = 'The engine could not move.';
			return;
		}
		// The search was from the engine's side, after the student's move.
		const forStudent = search.score ? flip(search.score) : null;
		if (judged && forStudent) this.#noteMistake(judged, forStudent);
		this.game.playUci(search.best);
		this.#assess();
		this.#expect =
			forStudent && search.pv.length > 1
				? { fen: this.game.view.fen, score: forStudent, pv: search.pv.slice(1) }
				: null;
	}
}
