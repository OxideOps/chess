import { describe, expect, it, vi } from 'vitest';
import { Coach, mistakeKey, type FollowUpStatus } from './coach.svelte';
import type { EngineLine } from '$lib/generated/EngineLine';

const FEN = 'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1';
const LINES: EngineLine[] = [
	{ depth: 20, multipv: 1, score: { kind: 'cp', value: -30 }, pv: ['c7c5', 'g1f3'] }
];

/** An explanation as the server sends it: the text, its parts, and a thread. */
const reply = (text: string, thread: string | null = 't1') =>
	new Response(JSON.stringify({ text, parts: [{ kind: 'text', text }], thread }));

/** A follow-up answer as the server sends it. */
const followUpReply = (text: string, left: number) =>
	new Response(
		JSON.stringify({
			kind: 'answer',
			left,
			answer: { text, parts: [{ kind: 'text', text }], thread: 't1' }
		})
	);

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

	it("asks follow-ups on the answer's thread, one at a time", async () => {
		const { calls, fetchImpl } = server(
			reply('Black fights for d4.'),
			followUpReply('Because it hits d4.', 4)
		);
		const coach = new Coach(fetchImpl);
		const probe = vi.fn();
		await coach.explain(FEN, 'e4', LINES);
		expect(coach.leftFor(FEN)).toBe(5);
		await coach.followUp(FEN, 'Why c5?', probe);
		expect(calls[1]).toEqual({
			url: '/api/coach/followup',
			body: { thread: 't1', question: 'Why c5?', probe: null }
		});
		expect(coach.followUps(FEN)).toEqual([
			{
				question: 'Why c5?',
				answer: expect.objectContaining({ text: 'Because it hits d4.' }),
				error: null
			}
		]);
		expect(coach.leftFor(FEN)).toBe(4);
		expect(coach.following).toBeNull();
		expect(probe).not.toHaveBeenCalled();
		// No answer, no thread: nothing to follow up.
		await coach.followUp('other', 'Why?', probe);
		expect(calls).toHaveLength(2);
	});

	it('gets Stockfish on a move the question names, then asks again with it', async () => {
		const { calls, fetchImpl } = server(
			reply('Black fights for d4.'),
			new Response(JSON.stringify({ kind: 'probe', fen: FEN, uci: 'g8f6', san: 'Nf6' })),
			followUpReply('Nf6 is fine too.', 4)
		);
		const coach = new Coach(fetchImpl);
		await coach.explain(FEN, 'e4', LINES);
		const line = { depth: 18, score: { kind: 'cp' as const, value: 40 }, pv: ['e4e5'] };
		let seen: FollowUpStatus | null = null;
		const probe = vi.fn(async () => {
			seen = coach.following;
			return line;
		});
		await coach.followUp(FEN, 'Why not Nf6?', probe);
		expect(probe).toHaveBeenCalledWith(FEN, 'g8f6');
		expect(seen).toEqual({ key: FEN, stage: 'engine', san: 'Nf6' });
		expect(calls[2].body).toEqual({
			thread: 't1',
			question: 'Why not Nf6?',
			probe: { uci: 'g8f6', line }
		});
		expect(coach.followUps(FEN)[0].answer?.text).toBe('Nf6 is fine too.');
	});

	it("says so when Stockfish can't look, or the server refuses", async () => {
		const { fetchImpl } = server(
			reply('Black fights for d4.'),
			new Response(JSON.stringify({ kind: 'probe', fen: FEN, uci: 'g8f6', san: 'Nf6' })),
			new Response('{"error":"that\'s all the questions about this answer"}', { status: 429 })
		);
		const coach = new Coach(fetchImpl);
		await coach.explain(FEN, 'e4', LINES);
		await coach.followUp(FEN, 'Why not Nf6?', async () => null);
		expect(coach.followUps(FEN)[0].error).toBe("Stockfish couldn't look at Nf6");
		await coach.followUp(FEN, 'And?', async () => null);
		expect(coach.followUps(FEN)[1].error).toBe("that's all the questions about this answer");
	});
});
