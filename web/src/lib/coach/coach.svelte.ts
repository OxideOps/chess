// The coach: asks the server to explain things in plain language (Claude,
// grounded in Stockfish's lines): an analysed position, or a mistake in a
// drill, and follow-up questions about an answer. Answers are kept per
// question for the page's lifetime, so going back and forth costs nothing.
import { SvelteMap } from 'svelte/reactivity';
import type { Probe } from './probe';
import type { CoachLine } from '$lib/generated/CoachLine';
import type { EngineLine } from '$lib/generated/EngineLine';
import type { ExplainRequest } from '$lib/generated/ExplainRequest';
import type { Explanation } from '$lib/generated/Explanation';
import type { FollowUpReply } from '$lib/generated/FollowUpReply';
import type { FollowUpRequest } from '$lib/generated/FollowUpRequest';
import type { MistakeRequest } from '$lib/generated/MistakeRequest';
import type { Mistake } from '$lib/lessons/drill.svelte';

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

/**
 * Lines shallower than this aren't worth explaining yet: the coach treats the
 * engine's lines as the truth, so they should be deep enough to be.
 */
export const MIN_DEPTH = 16;

/** Follow-up questions per answer (the server's `MAX_FOLLOW_UPS`). */
export const MAX_FOLLOW_UPS = 5;

/** A follow-up question and, once it lands, its answer (or why not). */
export interface FollowUp {
	question: string;
	answer: Explanation | null;
	error: string | null;
}

/** What the follow-up being asked is waiting for. */
export interface FollowUpStatus {
	key: string;
	/** Stockfish looking at the move the question names, or the coach answering. */
	stage: 'engine' | 'coach';
	san?: string;
}

/** The key a mistake's explanation is kept under. */
export const mistakeKey = (m: Pick<Mistake, 'fen' | 'played'>) => `mistake:${m.fen}:${m.played}`;

export class Coach {
	/** Whether this server has a coach; `null` until asked. */
	available: boolean | null = $state(null);
	/** What is being (or was last) asked about: a FEN, or a `mistakeKey`. */
	key: string | null = $state(null);
	/** The answer to `key`, once it has one. */
	answer: Explanation | null = $state(null);
	error: string | null = $state(null);
	busy = $state(false);

	readonly #fetch: FetchLike;
	/** Reactive, so views of other questions update when an answer lands. */
	readonly #answers = new SvelteMap<string, Explanation>();
	readonly #followUps = new SvelteMap<string, FollowUp[]>();
	readonly #left = new SvelteMap<string, number>();
	/** The follow-up being asked, if any: one at a time. */
	following: FollowUpStatus | null = $state(null);

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
	answerFor(key: string): Explanation | null {
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
			drill,
			reply: mistake.reply
		};
		return this.#ask(mistakeKey(mistake), '/api/coach/mistake', request);
	}

	/** The follow-ups asked about the answer to `key`, oldest first. */
	followUps(key: string): FollowUp[] {
		return this.#followUps.get(key) ?? [];
	}

	/** How many more follow-ups the answer to `key` takes. */
	leftFor(key: string): number {
		return this.#left.get(key) ?? MAX_FOLLOW_UPS;
	}

	/**
	 * Ask a follow-up about the answer to `key`. When the question names a
	 * move the engine's lines didn't cover, the server asks for Stockfish's
	 * look at it first: `probe` provides that, and the question goes again.
	 */
	async followUp(key: string, question: string, probe: Probe): Promise<void> {
		const thread = this.#answers.get(key)?.thread;
		if (!thread || this.following) return;
		const index = this.followUps(key).length;
		this.#followUps.set(key, [...this.followUps(key), { question, answer: null, error: null }]);
		const update = (patch: Partial<FollowUp>) =>
			this.#followUps.set(
				key,
				this.followUps(key).map((t, i) => (i === index ? { ...t, ...patch } : t))
			);
		this.following = { key, stage: 'coach' };
		try {
			const request: FollowUpRequest = { thread, question, probe: null };
			let reply = await this.#post<FollowUpReply>('/api/coach/followup', request);
			if (reply.kind === 'probe') {
				this.following = { key, stage: 'engine', san: reply.san };
				const line = await probe(reply.fen, reply.uci);
				if (!line) throw new Error(`Stockfish couldn't look at ${reply.san}`);
				this.following = { key, stage: 'coach' };
				reply = await this.#post<FollowUpReply>('/api/coach/followup', {
					...request,
					probe: { uci: reply.uci, line }
				});
			}
			if (reply.kind !== 'answer') throw new Error('the coach had no answer');
			update({ answer: reply.answer });
			this.#left.set(key, reply.left);
		} catch (e) {
			update({ error: e instanceof Error ? e.message : String(e) });
		} finally {
			this.following = null;
		}
	}

	/** POST `body` as JSON; the reply, or an error with the server's reason. */
	async #post<T>(url: string, body: object): Promise<T> {
		const response = await this.#fetch(url, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body)
		});
		if (!response.ok) {
			let message = `the server said ${response.status}`;
			try {
				const reply = (await response.json()) as { error?: string };
				if (reply.error) message = reply.error;
			} catch {
				// not JSON
			}
			throw new Error(message);
		}
		return (await response.json()) as T;
	}

	async #ask(key: string, url: string, request: object): Promise<void> {
		this.key = key;
		this.error = null;
		const known = this.#answers.get(key);
		if (known) {
			this.answer = known;
			return;
		}
		this.answer = null;
		this.busy = true;
		try {
			const answer = await this.#post<Explanation>(url, request);
			this.#answers.set(key, answer);
			if (this.key === key) this.answer = answer;
		} catch (e) {
			if (this.key === key) this.error = e instanceof Error ? e.message : String(e);
		} finally {
			if (this.key === key) this.busy = false;
		}
	}
}
