import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { drills, initChess } from '$lib/chess/wasm';
import type { OpponentLike } from '$lib/engine/opponent.svelte';
import { DrillSession } from './drill.svelte';
import { completed } from './progress';

/** An opponent that plays the moves it is given, and records what it was asked. */
function scripted(...replies: string[]) {
	const asked: string[][] = [];
	const opponent: OpponentLike = {
		move: async (_fen, moves) => {
			asked.push([...moves]);
			return replies.shift() ?? null;
		},
		dispose: () => {}
	};
	return { asked, opponent };
}

const drill = (id: string) => drills().find((d) => d.id === id)!;
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeAll(() => initChess());
beforeEach(() => localStorage.clear());

describe('DrillSession', () => {
	it('wins the back-rank mate in one and remembers it', async () => {
		const { asked, opponent } = scripted();
		const s = new DrillSession(drill('back-rank-mate'), opponent);
		await s.start();
		expect(s.status).toEqual({ state: 'going', moves_left: 1 });
		expect(s.canMove).toBe(true);
		expect(s.tryMove('a1', 'a8')).toBe('ok');
		expect(s.status).toEqual({ state: 'won', reason: 'Checkmate!' });
		expect(asked).toEqual([]); // no reply needed after mate
		expect(completed().has('back-rank-mate')).toBe(true);
		expect(s.tryMove('a8', 'b8')).toBe('illegal');
		s.dispose();
	});

	it('lets the engine move first when the student defends, and answers each move', async () => {
		const { asked, opponent } = scripted('e4d4', 'd4c4');
		const s = new DrillSession(drill('hold-the-draw'), opponent);
		await s.start();
		expect(asked).toEqual([[]]);
		expect(s.game.view.moves.map((m) => m.uci)).toEqual(['e4d4']);
		expect(s.status).toEqual({ state: 'going', moves_left: 20 });

		expect(s.tryMove('e6', 'd6')).toBe('ok');
		expect(s.thinking).toBe(true);
		expect(s.canMove).toBe(false);
		await settle();
		expect(asked[1]).toEqual(['e4d4', 'e6d6']);
		expect(s.game.view.moves.map((m) => m.uci)).toEqual(['e4d4', 'e6d6', 'd4c4']);
		expect(s.status).toEqual({ state: 'going', moves_left: 19 });
		expect(completed().size).toBe(0);
		s.dispose();
	});

	it('loses on a wasted move, restarts cleanly, and reports an engine failure', async () => {
		const { opponent } = scripted(); // no replies: the engine "fails"
		const s = new DrillSession(drill('back-rank-mate'), opponent);
		await s.start();
		s.tryMove('a1', 'a7');
		expect(s.status.state).toBe('lost');
		await s.restart();
		expect(s.status).toEqual({ state: 'going', moves_left: 1 });
		expect(s.game.view.plyCount).toBe(0);

		const two = new DrillSession(drill('two-rooks'), opponent);
		await two.start();
		two.tryMove('a1', 'a5');
		await settle();
		expect(two.error).toBe('The engine could not move.');
		two.dispose();
		s.dispose();
	});
});
