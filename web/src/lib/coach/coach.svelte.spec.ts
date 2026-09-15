import { describe, expect, it } from 'vitest';
import { Coach, mistakeKey } from './coach.svelte';
import type { EngineLine } from '$lib/generated/EngineLine';

const FEN = 'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1';
const LINES: EngineLine[] = [
	{ depth: 20, multipv: 1, score: { kind: 'cp', value: -30 }, pv: ['c7c5', 'g1f3'] }
];

/** An explanation as the server sends it: the text, and its parts. */
const reply = (text: string) =>
	new Response(JSON.stringify({ text, parts: [{ kind: 'text', text }] }));

function server(...replies: Response[]) {
	const calls: { url: string; body: unknown }[] = [];
	const fetchImpl = async (url: string, init?: RequestInit) => {
		calls.push({ url, body: init?.body ? JSON.parse(init.body as string) : null });
		return replies.shift() ?? new Response('{}', { status: 500 });
	};
	return { calls, fetchImpl };
}

describe('Coach', () => {
	it('knows whether the server has a coach', async () => {
		const on = new Coach(server(new Response('{"available":true}')).fetchImpl);
		await on.load();
		expect(on.available).toBe(true);
		const off = new Coach(async () => {
			throw new Error('offline');
		});
		await off.load();
		expect(off.available).toBe(false);
	});

	it('sends the position, the last move and the lines, and remembers the answer', async () => {
		const { calls, fetchImpl } = server(reply('Black fights for d4.'));
		const coach = new Coach(fetchImpl);
		await coach.explain(FEN, 'e4', LINES);
		expect(coach.answer?.text).toBe('Black fights for d4.');
		expect(coach.busy).toBe(false);
		expect(calls[0]).toEqual({
			url: '/api/coach/explain',
			body: {
				fen: FEN,
				last_move: 'e4',
				lines: [{ depth: 20, score: { kind: 'cp', value: -30 }, pv: ['c7c5', 'g1f3'] }]
			}
		});
		// Asked again: no request.
		await coach.explain(FEN, 'e4', LINES);
		expect(calls).toHaveLength(1);
		expect(coach.answerFor(FEN)?.text).toBe('Black fights for d4.');
	});

	it("shows the server's reason for a refusal", async () => {
		const coach = new Coach(
			server(new Response('{"error":"the coach is for accounts"}', { status: 403 })).fetchImpl
		);
		await coach.explain(FEN, null, LINES);
		expect(coach.answer).toBeNull();
		expect(coach.error).toBe('the coach is for accounts');
	});

	it('asks about a drill mistake and keeps the answer under its own key', async () => {
		const { calls, fetchImpl } = server(reply('The queen was hanging.'));
		const coach = new Coach(fetchImpl);
		const mistake = {
			fen: '8/8/8/3k4/8/8/7Q/4K3 w - - 0 1',
			played: 'h2e5',
			playedSan: 'Qe5+',
			better: ['h2e2', 'd5d4'],
			betterSan: '1. Qe2',
			before: { kind: 'mate' as const, value: 8 },
			after: { kind: 'cp' as const, value: 0 },
			reply: ['d5e5']
		};
		await coach.explainMistake(mistake, 'queen-mate');
		expect(coach.answer?.text).toBe('The queen was hanging.');
		expect(calls[0]).toEqual({
			url: '/api/coach/mistake',
			body: {
				fen: mistake.fen,
				played: 'h2e5',
				better: ['h2e2', 'd5d4'],
				before: { kind: 'mate', value: 8 },
				after: { kind: 'cp', value: 0 },
				drill: 'queen-mate',
				reply: ['d5e5']
			}
		});
		expect(coach.answerFor(mistakeKey(mistake))?.text).toBe('The queen was hanging.');
		// Not mixed up with an explanation of the same position.
		expect(coach.answerFor(mistake.fen)).toBeNull();
	});
});
