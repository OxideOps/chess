import { describe, expect, it } from 'vitest';
import { AuthError, Session } from './session.svelte';
import type { User } from '$lib/generated/User';

const alice: User = { id: 'u1', username: 'alice', is_guest: false };
const guest: User = { id: 'g1', username: null, is_guest: true };

/** A fake server: one canned reply per call, recording what was asked. */
function fakeFetch(...replies: Array<{ status: number; body?: unknown } | Error>) {
	const calls: Array<{ path: string; init?: RequestInit }> = [];
	const fetchImpl = async (path: string, init?: RequestInit) => {
		calls.push({ path, init });
		const reply = replies.shift();
		if (reply === undefined) throw new Error(`unexpected request to ${path}`);
		if (reply instanceof Error) throw reply;
		return new Response(reply.body === undefined ? null : JSON.stringify(reply.body), {
			status: reply.status
		});
	};
	return { calls, fetchImpl };
}

describe('Session', () => {
	it('loads the current user, or nobody', async () => {
		const { fetchImpl, calls } = fakeFetch({ status: 200, body: alice });
		const session = new Session(fetchImpl);
		expect(session.user).toBeNull();
		await session.load();
		expect(session.user).toEqual(alice);
		expect(session.registered).toBe(true);
		expect(session.displayName).toBe('alice');
		expect(calls[0].path).toBe('/api/me');

		const noSession = new Session(fakeFetch({ status: 401 }).fetchImpl);
		await noSession.load();
		expect(noSession.user).toBeNull();
		expect(noSession.displayName).toBeNull();

		const noServer = new Session(fakeFetch(new Error('connection refused')).fetchImpl);
		await noServer.load();
		expect(noServer.user).toBeNull();
	});

	it('ensure() creates a guest only when there is nobody', async () => {
		const { fetchImpl, calls } = fakeFetch({ status: 200, body: guest });
		const session = new Session(fetchImpl);
		expect(await session.ensure()).toEqual(guest);
		expect(await session.ensure()).toEqual(guest);
		expect(calls.map((c) => c.path)).toEqual(['/api/auth/guest']);
		expect(session.registered).toBe(false);
		expect(session.displayName).toBe('Guest');
	});

	it('signup and login post credentials and keep the user; logout forgets', async () => {
		const { fetchImpl, calls } = fakeFetch(
			{ status: 201, body: alice },
			{ status: 204 },
			{ status: 200, body: alice }
		);
		const session = new Session(fetchImpl);
		await session.signup('alice', 'correct horse');
		expect(session.user).toEqual(alice);
		expect(calls[0].init?.method).toBe('POST');
		expect(JSON.parse(calls[0].init?.body as string)).toEqual({
			username: 'alice',
			password: 'correct horse'
		});
		await session.logout();
		expect(session.user).toBeNull();
		await session.login('alice', 'correct horse');
		expect(session.user).toEqual(alice);
		expect(calls.map((c) => c.path)).toEqual([
			'/api/auth/signup',
			'/api/auth/logout',
			'/api/auth/login'
		]);
	});

	it("surfaces the server's reason for a refusal", async () => {
		const session = new Session(
			fakeFetch({ status: 409, body: { error: 'that username is taken' } }, { status: 502 })
				.fetchImpl
		);
		await expect(session.signup('alice', 'correct horse')).rejects.toMatchObject({
			name: 'AuthError',
			message: 'that username is taken',
			status: 409
		});
		expect(session.user).toBeNull();
		const err = await session.login('alice', 'x').catch((e: unknown) => e);
		expect(err).toBeInstanceOf(AuthError);
		expect((err as AuthError).message).toBe('the server said 502');
	});

	it('does not make a guest while it is still finding out who we are', async () => {
		// `GET /api/me` is slow; the user clicks something in the meantime.
		let answer: (response: Response) => void = () => {};
		const pending = new Promise<Response>((resolve) => (answer = resolve));
		const calls: string[] = [];
		const session = new Session((input) => {
			calls.push(input);
			if (input === '/api/me') return pending;
			return Promise.resolve(
				new Response(JSON.stringify({ id: 'g1', username: null, is_guest: true }), {
					status: 200
				})
			);
		});

		const loading = session.load();
		const ensuring = session.ensure();
		answer(
			new Response(JSON.stringify({ id: 'u1', username: 'dana', is_guest: false }), {
				status: 200
			})
		);
		await loading;
		const user = await ensuring;

		// The account that was already signed in, not a new guest over the top.
		expect(user.username).toBe('dana');
		expect(session.registered).toBe(true);
		expect(calls).toEqual(['/api/me']);
	});
});
