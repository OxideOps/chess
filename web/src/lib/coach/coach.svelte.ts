// The coach: asks the server to explain things in plain language (Claude,
// grounded in Stockfish's lines): an analysed position, or a mistake in a
// drill. Answers are kept per question for the page's lifetime, so going
// back and forth costs nothing.
import { SvelteMap } from 'svelte/reactivity';
import type { CoachLine } from '$lib/generated/CoachLine';
import type { EngineLine } from '$lib/generated/EngineLine';
import type { ExplainRequest } from '$lib/generated/ExplainRequest';
import type { Explanation } from '$lib/generated/Explanation';
import type { MistakeRequest } from '$lib/generated/MistakeRequest';
import type { Mistake } from '$lib/lessons/drill.svelte';

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

/** Lines shallower than this aren't worth explaining yet. */
export const MIN_DEPTH = 10;

/** The key a mistake's explanation is kept under. */
export const mistakeKey = (m: Pick<Mistake, 'fen' | 'played'>) => `mistake:${m.fen}:${m.played}`;

export class Coach {
	/** Whether this server has a coach; `null` until asked. */
	available: boolean | null = $state(null);
	/** What is being (or was last) asked about: a FEN, or a `mistakeKey`. */
	key: string | null = $state(null);
	text: string | null = $state(null);
	error: string | null = $state(null);
	busy = $state(false);

	readonly #fetch: FetchLike;
	/** Reactive, so views of other questions update when an answer lands. */
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

	/** What has been said about `key`, if anything. */
	answerFor(key: string): string | null {
		return this.#answers.get(key) ?? null;
	}

	/** Explain the position `fen` from the engine's lines. */
	explain(fen: string, lastMove: string | null, lines: EngineLine[]): Promise<void> {
		const request: ExplainRequest = {
			fen,
			last_move: lastMove,
			lines: lines.map((l): CoachLine => ({ depth: l.depth, score: l.score, pv: l.pv }))
		};
		return this.#ask(fen, '/api/coach/explain', request);
	}

	/** Explain why a drill move was a mistake. */
	explainMistake(mistake: Mistake, drill: string): Promise<void> {
		const request: MistakeRequest = {
			fen: mistake.fen,
			played: mistake.played,
			better: mistake.better,
			before: mistake.before,
			after: mistake.after,
			drill
		};
		return this.#ask(mistakeKey(mistake), '/api/coach/mistake', request);
	}

	async #ask(key: string, url: string, request: object): Promise<void> {
		this.key = key;
		this.error = null;
		const known = this.#answers.get(key);
		if (known) {
			this.text = known;
			return;
		}
		this.text = null;
		this.busy = true;
		try {
			const response = await this.#fetch(url, {
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
			this.#answers.set(key, text);
			if (this.key === key) this.text = text;
		} catch (e) {
			if (this.key === key) this.error = e instanceof Error ? e.message : String(e);
		} finally {
			if (this.key === key) this.busy = false;
		}
	}
}
