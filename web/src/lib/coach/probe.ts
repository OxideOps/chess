// Stockfish's look at a move a follow-up question asks about, for the coach
// (`FollowUpReply` kind `probe`). Its own engine, started on first use, so it
// doesn't disturb the analysis or a drill's opponent.
import type { CoachLine } from '$lib/generated/CoachLine';
import { Opponent, type OpponentLike } from '$lib/engine/opponent.svelte';

/** Long enough for a line worth explaining; the question waits for it. */
export const PROBE_TIME_MS = 1500;

export type Probe = (fen: string, uci: string) => Promise<CoachLine | null>;

export class Prober {
	#engine: OpponentLike | null = null;
	readonly #create: () => OpponentLike;

	constructor(create: () => OpponentLike = () => new Opponent({ movetime: PROBE_TIME_MS })) {
		this.#create = create;
	}

	/** The engine's line from the position after `uci`; `null` if it has none. */
	probe: Probe = async (fen, uci) => {
		this.#engine ??= this.#create();
		const search = await this.#engine.search(fen, [uci]);
		if (!search?.score) return null;
		return { depth: search.depth ?? 0, score: search.score, pv: search.pv };
	};

	dispose(): void {
		this.#engine?.dispose();
		this.#engine = null;
	}
}
