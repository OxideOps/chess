// The coach: asks the server to explain the analysed position in plain
// language (Claude, grounded in Stockfish's lines). Explanations are kept per
// position for the page's lifetime, so going back and forth costs nothing.
import { SvelteMap } from 'svelte/reactivity';
import type { CoachLine } from '$lib/generated/CoachLine';
import type { EngineLine } from '$lib/generated/EngineLine';
import type { ExplainRequest } from '$lib/generated/ExplainRequest';
import type { Explanation } from '$lib/generated/Explanation';

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

/** Lines shallower than this aren't worth explaining yet. */
export const MIN_DEPTH = 10;

export class Coach {
	/** Whether this server has a coach; `null` until asked. */
	available: boolean | null = $state(null);
	/** The position being explained or last explained. */
	fen: string | null = $state(null);
	text: string | null = $state(null);
	error: string | null = $state(null);
	busy = $state(false);

	readonly #fetch: FetchLike;
	/** Reactive, so views of other positions update when an answer lands. */
	readonly #answers = new SvelteMap<string, string>();

	constructor(fetchImpl?: FetchLike) {
		this.#fetch = fetchImpl ?? ((input, init) => fetch(input, init));
	}

	async load(): Promise<void> {
		try {
			const response = await this.#fetch('/api/coach');
			this.available = response.ok && ((await response.json()) as { available: boolean }).available;
		} catch {
			this.available = false;
		}
	}

	/** What has been said about `fen`, if anything. */
	answerFor(fen: string): string | null {
		return this.#answers.get(fen) ?? null;
	}

	async explain(fen: string, lastMove: string | null, lines: EngineLine[]): Promise<void> {
		this.fen = fen;
		this.error = null;
		const known = this.#answers.get(fen);
		if (known) {
			this.text = known;
			return;
		}
		this.text = null;
		this.busy = true;
		const request: ExplainRequest = {
			fen,
			last_move: lastMove,
			lines: lines.map((l): CoachLine => ({ depth: l.depth, score: l.score, pv: l.pv }))
		};
		try {
			const response = await this.#fetch('/api/coach/explain', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify(request)
			});
			if (!response.ok) {
				let message = `the server said ${response.status}`;
				try {
					const body = (await response.json()) as { error?: string };
					if (body.error) message = body.error;
				} catch {
					// not JSON
				}
				throw new Error(message);
			}
			const { text } = (await response.json()) as Explanation;
			this.#answers.set(fen, text);
			if (this.fen === fen) this.text = text;
		} catch (e) {
			if (this.fen === fen) this.error = e instanceof Error ? e.message : String(e);
		} finally {
			if (this.fen === fen) this.busy = false;
		}
	}
}
