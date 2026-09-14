// Who the browser is signed in as. Loaded once in the root layout from
// `GET /api/me` and kept in sync by the auth calls below, so the nav and the
// pages read `session.user` and never poll.
import type { User } from '$lib/generated/User';

/** A refusal from the server, with its message (`{ "error": "..." }`). */
export class AuthError extends Error {
	constructor(
		message: string,
		readonly status: number
	) {
		super(message);
		this.name = 'AuthError';
	}
}

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

export class Session {
	/** `null` until `load()` has run, and when there is no session. */
	user: User | null = $state(null);
	readonly #fetch: FetchLike;

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
	async load(): Promise<void> {
		try {
			const response = await this.#fetch('/api/me');
			this.user = response.ok ? ((await response.json()) as User) : null;
		} catch {
			this.user = null;
		}
	}

	/** The current user, creating a guest if there is nobody yet. */
	async ensure(): Promise<User> {
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

async function refusal(response: Response): Promise<AuthError> {
	let message = `the server said ${response.status}`;
	try {
		const body = (await response.json()) as { error?: string };
		if (typeof body.error === 'string') message = body.error;
	} catch {
		// not JSON; keep the status text
	}
	return new AuthError(message, response.status);
}

export const session = new Session();
