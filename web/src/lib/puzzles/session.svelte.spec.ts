import { beforeAll, describe, expect, it } from 'vitest';
import { initChess } from '$lib/chess/wasm';
import { PuzzleSession } from './session.svelte';
import type { PuzzleData } from '$lib/generated/PuzzleData';

// Lichess puzzle 00008 (CC0): after ...Bxg3, White plays Rxe7, Nc1, Qxc1.
const PUZZLE: PuzzleData = {
	id: '00008',
	fen: 'r6k/pp2r2p/4Rp1Q/3p4/8/1N1P2R1/PqP2bPP/7K b - - 0 24',
	moves: ['f2g3', 'e6e7', 'b2b1', 'b3c1', 'b1c1', 'h6c1'],
	rating: 1797,
	themes: ['crushing', 'hangingPiece'],
	your_rating: { value: 1500, provisional: true },
	streak: { current: 2, best: 5 },
	tried: false
};

/** A fake server: serves the puzzle and records what was asked for. */
function server() {
	const attempts: { url: string; body: unknown }[] = [];
	const asked: string[] = [];
	const fetchImpl = async (url: string, init?: RequestInit) => {
		if (!init) asked.push(url);
		if (url.startsWith('/api/puzzles/next')) return new Response(JSON.stringify(PUZZLE));
		if (url.startsWith('/api/puzzles/daily')) {
			return new Response(
				JSON.stringify({ date: '2026-09-22', puzzle: { ...PUZZLE, tried: true } })
			);
		}
		const body = JSON.parse(init!.body as string) as { solved: boolean };
		attempts.push({ url, body });
		const diff = body.solved ? 12 : -9;
		const streak = body.solved ? { current: 3, best: 5 } : { current: 0, best: 5 };
		return new Response(
			JSON.stringify({
				rating: { value: 1500 + diff, provisional: true },
				diff,
				counted: true,
				streak
			})
		);
	};
	return { attempts, asked, fetchImpl };
}

/** Waits for the session's queued work (replies, reports) to finish. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
const immediately = async () => {};

beforeAll(() => initChess());

describe('PuzzleSession', () => {
	it('plays the setup move and the replies, and reports a solve', async () => {
		const { attempts, fetchImpl } = server();
		const s = new PuzzleSession({ fetch: fetchImpl, delay: immediately });
		await s.next();
		expect(s.phase).toBe('solving');
		expect(s.solver).toBe('white');
		expect(s.game.view.moves.map((m) => m.uci)).toEqual(['f2g3']);

		expect(s.tryMove('e6', 'e7')).toBe('ok');
		await settle();
		expect(s.game.view.moves.map((m) => m.uci)).toEqual(['f2g3', 'e6e7', 'b2b1']);
		expect(s.phase).toBe('solving');
		s.tryMove('b3', 'c1');
		await settle();
		s.tryMove('h6', 'c1');
		await settle();
		expect(s.phase).toBe('solved');
		expect(attempts).toEqual([{ url: '/api/puzzles/00008/attempt', body: { solved: true } }]);
		await expect
			.poll(() => s.result)
			.toEqual({
				rating: { value: 1512, provisional: true },
				diff: 12,
				counted: true,
				streak: { current: 3, best: 5 }
			});
		expect(s.streak).toEqual({ current: 3, best: 5 });
		// No more moves once it's over.
		expect(s.tryMove('c1', 'c2')).toBe('illegal');
		s.dispose();
	});

	it('fails a wrong move, names the right one, and can show the rest', async () => {
		const { attempts, fetchImpl } = server();
		const s = new PuzzleSession({ fetch: fetchImpl, delay: immediately });
		await s.next();
		s.tryMove('h6', 'h7');
		await settle();
		expect(s.phase).toBe('failed');
		expect(s.expected).toBe('Rxe7');
		expect(attempts).toEqual([{ url: '/api/puzzles/00008/attempt', body: { solved: false } }]);
		await expect.poll(() => s.result?.diff).toBe(-9);
		expect(s.streak).toEqual({ current: 0, best: 5 });

		await s.showSolution();
		expect(s.game.view.moves.map((m) => m.uci)).toEqual(PUZZLE.moves);
		expect(s.phase).toBe('solved');
		expect(attempts).toHaveLength(1); // showing the answer isn't another try
		s.dispose();
	});

	it('asks for the chosen theme, and shows the streak the server keeps', async () => {
		const { asked, fetchImpl } = server();
		const s = new PuzzleSession({ fetch: fetchImpl, delay: immediately });
		await s.next();
		expect(s.streak).toEqual({ current: 2, best: 5 });
		s.theme = 'mateIn2';
		await s.next();
		expect(asked).toEqual(['/api/puzzles/next', '/api/puzzles/next?theme=mateIn2']);
		expect(s.date).toBeNull();
		s.dispose();
	});

	it("loads the daily puzzle, today's or a given day's", async () => {
		const { asked, fetchImpl } = server();
		const s = new PuzzleSession({ fetch: fetchImpl, delay: immediately });
		s.theme = 'fork'; // the daily puzzle is everyone's: no theme applies
		await s.daily();
		await s.daily('2026-09-01');
		expect(asked).toEqual(['/api/puzzles/daily', '/api/puzzles/daily/2026-09-01']);
		expect(s.date).toBe('2026-09-22');
		expect(s.puzzle?.tried).toBe(true);
		expect(s.phase).toBe('solving');
		s.dispose();
	});

	it('says when there is nothing to serve, or the server fails', async () => {
		const empty = new PuzzleSession({
			fetch: async () => new Response('{}', { status: 404 }),
			delay: immediately
		});
		await empty.next();
		expect(empty.phase).toBe('empty');
		const broken = new PuzzleSession({
			fetch: async () => new Response('', { status: 503 }),
			delay: immediately
		});
		await broken.next();
		expect(broken.phase).toBe('error');
		expect(broken.error).toMatch(/503/);
	});
});
