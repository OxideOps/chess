import { beforeEach, describe, expect, it } from 'vitest';
import type { User } from '$lib/generated/User';
import { LessonProgress } from './progress.svelte';
import { completed, markCompleted } from './progress';

const alice: User = { id: 'u1', username: 'alice', is_guest: false };
const guest: User = { id: 'g1', username: null, is_guest: true };

/**
 * A fake server holding one account's progress: a POST adds to it (a union,
 * as the real one does) and answers with all of it. `down` makes it fail.
 */
function server(...stored: string[]) {
	const state = { stored: new Set(stored), down: false as false | 'refuse' | 'unreachable' };
	const posts: string[][] = [];
	const fetchImpl = async (path: string, init?: RequestInit) => {
		expect(path).toBe('/api/lessons/completed');
		if (state.down === 'unreachable') throw new TypeError('Failed to fetch');
		if (state.down === 'refuse') return new Response(null, { status: 503 });
		const { completed } = JSON.parse(init!.body as string) as { completed: string[] };
		posts.push(completed);
		for (const id of completed) state.stored.add(id);
		return new Response(JSON.stringify({ completed: [...state.stored] }));
	};
	return { state, posts, fetchImpl };
}

const as = (user: User | null): { user: User | null } => ({ user });
const sorted = (s: ReadonlySet<string>) => [...s].sort();

beforeEach(() => localStorage.clear());

describe('LessonProgress', () => {
	it("keeps a guest's progress in this browser and never calls the server", async () => {
		for (const who of [null, guest]) {
			localStorage.clear();
			const { posts, fetchImpl } = server('queen-mate');
			const calls: string[] = [];
			const progress = new LessonProgress({
				account: as(who),
				fetch: (path, init) => {
					calls.push(path);
					return fetchImpl(path, init);
				}
			});
			markCompleted('back-rank-mate');
			await progress.load();
			expect(sorted(progress.done)).toEqual(['back-rank-mate']);
			await progress.complete('two-rooks');
			expect(sorted(progress.done)).toEqual(['back-rank-mate', 'two-rooks']);
			// Exactly as before accounts kept it: in localStorage, under the same key.
			expect(sorted(completed())).toEqual(['back-rank-mate', 'two-rooks']);
			expect(JSON.parse(localStorage.getItem('chess.lessons.done')!)).toHaveLength(2);
			expect(calls).toEqual([]);
			expect(posts).toEqual([]);
		}
	});

	it("shows an account's progress from the server", async () => {
		const { posts, fetchImpl } = server('queen-mate', 'rook-mate');
		const progress = new LessonProgress({ account: as(alice), fetch: fetchImpl });
		await progress.load();
		expect(sorted(progress.done)).toEqual(['queen-mate', 'rook-mate']);
		expect(posts).toEqual([[]]);

		await progress.complete('two-rooks');
		expect(sorted(progress.done)).toEqual(['queen-mate', 'rook-mate', 'two-rooks']);
		expect(posts[1]).toEqual(['two-rooks']);
		// The server took it, so the browser no longer holds it.
		expect(completed().size).toBe(0);
	});

	it('merges what a guest finished into the account on sign-up or sign-in', async () => {
		// Finished as a guest...
		const account = as(guest);
		const { state, posts, fetchImpl } = server('queen-mate', 'rook-mate');
		const progress = new LessonProgress({ account, fetch: fetchImpl });
		await progress.complete('back-rank-mate');
		await progress.complete('queen-mate');
		expect(posts).toEqual([]);

		// ...then signed in to an account that has progress of its own.
		account.user = alice;
		await progress.load();
		expect(posts).toEqual([['back-rank-mate', 'queen-mate']]);
		// A union: the account keeps its own, and gains the guest's.
		expect(sorted(state.stored)).toEqual(['back-rank-mate', 'queen-mate', 'rook-mate']);
		expect(sorted(progress.done)).toEqual(['back-rank-mate', 'queen-mate', 'rook-mate']);
		expect(completed().size).toBe(0);

		// Signed out, the browser is a fresh guest's again: nothing of Alice's shows.
		account.user = null;
		await progress.load();
		expect(progress.done.size).toBe(0);
	});

	it("doesn't lose a completion the server couldn't take, and sends it next time", async () => {
		const { state, posts, fetchImpl } = server('queen-mate');
		const progress = new LessonProgress({ account: as(alice), fetch: fetchImpl });
		await progress.load();

		state.down = 'unreachable';
		await expect(progress.complete('two-rooks')).resolves.toBeUndefined();
		expect(sorted(progress.done)).toEqual(['queen-mate', 'two-rooks']);
		state.down = 'refuse';
		await progress.complete('rook-mate');
		expect(sorted(progress.done)).toEqual(['queen-mate', 'rook-mate', 'two-rooks']);
		expect(sorted(completed())).toEqual(['rook-mate', 'two-rooks']);

		state.down = false;
		await progress.load();
		expect(posts.at(-1)!.sort()).toEqual(['rook-mate', 'two-rooks']);
		expect(sorted(state.stored)).toEqual(['queen-mate', 'rook-mate', 'two-rooks']);
		expect(completed().size).toBe(0);
	});
});
