// Who the browser is signed in as. Loaded once in the root layout from
// `GET /api/me` and kept in sync by the auth calls below, so the nav and the
// pages read `session.user` and never poll.
import type { User } from '$lib/generated/User';
import { refusal } from './refusal';

export { AuthError } from './refusal';

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

export class Session {
	/** `null` until `load()` has run, and when there is no session. */
	user: User | null = $state(null);
	readonly #fetch: FetchLike;
	/** The in-flight `load()`, so `ensure()` can wait for it. */
	#loading: Promise<void> | null = null;

	constructor(fetchImpl?: FetchLike) {
		this.#fetch = fetchImpl ?? ((input, init) => fetch(input, init));
	}

	/** A registered account, as opposed to a guest or nobody. */
	get registered(): boolean {
		return this.user !== null && !this.user.is_guest;
	}

	/** What to call the signed-in user; `null` when there is no session. */
	get displayName(): string | null {
		if (this.user === null) return null;
		return this.user.username ?? 'Guest';
	}

	/** Find out who the cookie belongs to. Never throws: no server, no session. */
	load(): Promise<void> {
		this.#loading = (async () => {
			try {
				const response = await this.#fetch('/api/me');
				this.user = response.ok ? ((await response.json()) as User) : null;
			} catch {
				this.user = null;
			}
		})();
		return this.#loading;
	}

	/**
	 * The current user, creating a guest if there is nobody yet.
	 *
	 * Waits for the first `load()` before deciding. Without that, a click
	 * in the moment between the page appearing and `GET /api/me` answering
	 * would make a guest and *replace the signed-in session with it* —
	 * logging someone out by being quick.
	 */
	async ensure(): Promise<User> {
		if (this.#loading !== null) await this.#loading;
		return this.user ?? this.#post('/api/auth/guest');
	}

	/** Register. A guest is upgraded in place and keeps their games. */
	signup(username: string, password: string): Promise<User> {
		return this.#post('/api/auth/signup', { username, password });
	}

	login(username: string, password: string): Promise<User> {
		return this.#post('/api/auth/login', { username, password });
	}

	async logout(): Promise<void> {
		const response = await this.#fetch('/api/auth/logout', { method: 'POST' });
		if (!response.ok) throw await refusal(response);
		this.user = null;
	}

	async #post(path: string, body?: object): Promise<User> {
		const response = await this.#fetch(path, {
			method: 'POST',
			headers: body ? { 'content-type': 'application/json' } : undefined,
			body: body ? JSON.stringify(body) : undefined
		});
		if (!response.ok) throw await refusal(response);
		this.user = (await response.json()) as User;
		return this.user;
	}
}

export const session = new Session();
