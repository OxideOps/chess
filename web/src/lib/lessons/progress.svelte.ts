// The lessons the visitor has finished, from whichever source is theirs: a
// guest's live in this browser (`progress.ts`), an account's on the server
// (`/api/lessons/completed`), so they follow it to every device.
//
// Finishing a lesson always writes the browser copy first, then (for an
// account) sends everything the browser holds and forgets what the server
// took. So a write that fails costs nothing — the browser still has it and
// the next load sends it again — and what a guest finished before signing
// up or signing in is merged into the account the first time they open the
// lessons. The merge is a union on the server: neither side loses anything.
import { SvelteSet } from 'svelte/reactivity';
import { session as defaultSession } from '$lib/auth/session.svelte';
import type { LessonProgress as Progress } from '$lib/generated/LessonProgress';
import type { RecordLessons } from '$lib/generated/RecordLessons';
import type { User } from '$lib/generated/User';
import { completed as local, forgetCompleted, markCompleted } from './progress';

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

/** Who is signed in; `Session` in the app. */
export interface Account {
	readonly user: User | null;
}

const PATH = '/api/lessons/completed';

export class LessonProgress {
	/** Ids of the finished lessons. */
	readonly done = new SvelteSet<string>();
	readonly #fetch: FetchLike;
	readonly #account: Account;
	/** The account `done` was last loaded for (`null`: this browser's guest progress). */
	#owner: string | null = null;

	constructor(options: { fetch?: FetchLike; account?: Account } = {}) {
		this.#fetch = options.fetch ?? ((input, init) => fetch(input, init));
		this.#account = options.account ?? defaultSession;
	}

	/** The signed-in account's id, or `null` for guests and nobody. */
	get #accountId(): string | null {
		const user = this.#account.user;
		return user && !user.is_guest ? user.id : null;
	}

	/**
	 * Read the progress from its source. For an account, whatever this browser
	 * holds is merged into it on the way. Never throws: if the server can't be
	 * reached, what we already knew plus the browser's copy is shown.
	 */
	async load(): Promise<void> {
		const account = this.#accountId;
		const held = local();
		// Another account's (or the guest's) progress isn't this one's.
		if (this.#owner !== account || account === null) this.done.clear();
		this.#owner = account;
		this.#show(held);
		if (account === null) return;

		const body: RecordLessons = { completed: [...held] };
		try {
			const response = await this.#fetch(PATH, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify(body)
			});
			if (!response.ok) return;
			const progress = (await response.json()) as Progress;
			forgetCompleted(held);
			// Signed out (or in as someone else) while we waited: not theirs to show.
			if (this.#accountId !== account) return;
			this.done.clear();
			this.#show(progress.completed);
			this.#show(local());
		} catch {
			// Offline or no server: the browser copy still has it; the next load retries.
		}
	}

	#show(ids: Iterable<string>): void {
		for (const id of ids) this.done.add(id);
	}

	/**
	 * Record a finished lesson: shown at once, kept by the browser, and sent to
	 * the account in the background. Never throws.
	 */
	complete(id: string): Promise<void> {
		markCompleted(id);
		return this.load();
	}
}

export const lessonProgress = new LessonProgress();
