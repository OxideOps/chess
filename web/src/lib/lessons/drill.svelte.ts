import { Game, assessDrill, type PlayResult } from '$lib/chess/wasm';
import { GameStore, type Promotion } from '$lib/chess/game.svelte';
import type { OpponentLike } from '$lib/engine/opponent.svelte';
import type { Drill } from '$lib/generated/Drill';
import type { DrillStatus } from '$lib/generated/DrillStatus';
import { markCompleted } from './progress';

/**
 * One drill against the engine: the student moves, chess-core judges where
 * the drill stands (`assessDrill`), and while it's going the engine answers.
 */
export class DrillSession {
	readonly game = new GameStore();
	readonly drill: Drill;
	status: DrillStatus = $state({ state: 'going', moves_left: 0 });
	/** The engine is choosing its move. */
	thinking = $state(false);
	error: string | null = $state(null);

	readonly #opponent: OpponentLike;
	/** Bumped on restart so a late engine move can't land in a new attempt. */
	#generation = 0;

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

	/** Set the board up; if the engine moves first, let it. */
	start(): Promise<void> {
		this.#generation++;
		this.error = null;
		this.thinking = false;
		this.game.replace(Game.fromFen(this.drill.fen));
		this.#assess();
		return this.game.view.turn === this.drill.student ? Promise.resolve() : this.#answer();
	}

	restart(): Promise<void> {
		return this.start();
	}

	/** Board callback. */
	tryMove(from: string, to: string, promotion?: Promotion): PlayResult {
		if (!this.canMove) return 'illegal';
		const result = this.game.play(from, to, promotion);
		if (result === 'ok') {
			this.#assess();
			if (this.status.state === 'going') void this.#answer();
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

	async #answer(): Promise<void> {
		const generation = this.#generation;
		this.thinking = true;
		const uci = await this.#opponent.move(this.drill.fen, this.#moves());
		if (generation !== this.#generation) return;
		this.thinking = false;
		if (uci === null) {
			this.error = 'The engine could not move.';
			return;
		}
		this.game.playUci(uci);
		this.#assess();
	}
}
