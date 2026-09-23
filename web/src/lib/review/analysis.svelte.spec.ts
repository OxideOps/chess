import { beforeAll, describe, expect, it } from 'vitest';
import { initChess, reviewGame } from '$lib/chess/wasm';
import type { OpponentLike, Search } from '$lib/engine/opponent.svelte';
import type { PositionEval } from '$lib/generated/PositionEval';
import { GameAnalysis, positionsOf, type EvalCache } from './analysis.svelte';

beforeAll(() => initChess());

const SCHOLARS_MATE = '1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7#';

/** A searcher that answers when told to, one search at a time. */
class FakeSearcher implements OpponentLike {
	asked: string[] = [];
	disposed = false;
	#pending: ((s: Search | null) => void) | null = null;
	search(fen: string): Promise<Search | null> {
		this.asked.push(fen);
		return new Promise((resolve) => (this.#pending = resolve));
	}
	answer(search: Search | null) {
		const pending = this.#pending;
		this.#pending = null;
		pending?.(search);
	}
	dispose() {
		this.disposed = true;
		this.answer(null);
	}
}

function memoryCache(): EvalCache & { data: Map<string, (PositionEval | null)[]> } {
	const data = new Map<string, (PositionEval | null)[]>();
	return {
		data,
		load: (key) => data.get(key) ?? null,
		save: (key, evals) => void data.set(key, evals)
	};
}

const tick = () => new Promise((r) => setTimeout(r, 0));

function cp(value: number, ...pv: string[]): Search {
	return { best: pv[0] ?? null, score: { kind: 'cp', value }, pv };
}

describe('positionsOf', () => {
	it('lists every position from the start, marking where the game is over', () => {
		const positions = positionsOf(SCHOLARS_MATE);
		expect(positions).toHaveLength(8);
		expect(positions[0].fen).toBe('rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1');
		expect(positions.slice(0, 7).every((p) => !p.over)).toBe(true);
		expect(positions[7].over).toBe(true);
	});
});

describe('GameAnalysis', () => {
	it('searches each position in turn, skips the finished one, and caches the result', async () => {
		const searchers: FakeSearcher[] = [];
		const cache = memoryCache();
		const positions = positionsOf(SCHOLARS_MATE);
		const a = new GameAnalysis('g1', positions, {
			cache,
			createSearcher: () => {
				const s = new FakeSearcher();
				searchers.push(s);
				return s;
			}
		});
		expect(a.status).toBe('idle');
		a.start();
		expect(a.status).toBe('running');
		const answers = [
			cp(30, 'e2e4'),
			cp(-30, 'e7e5'),
			cp(30, 'g1f3'),
			cp(-10, 'b8c6'),
			cp(40, 'f1c4'),
			cp(-60, 'g7g6', 'h5f3'),
			{ best: 'h5f7', score: { kind: 'mate', value: 1 }, pv: ['h5f7'] } satisfies Search
		];
		for (const [i, answer] of answers.entries()) {
			await tick();
			expect(a.done).toBe(i);
			searchers[0].answer(answer);
		}
		await tick();
		expect(a.status).toBe('done');
		expect(a.done).toBe(8);
		// The mated position was never sent to the engine.
		expect(searchers[0].asked).toEqual(positions.slice(0, 7).map((p) => p.fen));
		expect(a.evals[7]).toBeNull();
		expect(searchers[0].disposed).toBe(true);

		const review = reviewGame(SCHOLARS_MATE, a.evals, null);
		expect(review.swings.map((s) => s.played)).toEqual(['3... Nf6']);
		expect(review.swings[0].best).toBe('3... g6 4. Qf3');
		expect(reviewGame(SCHOLARS_MATE, a.evals, 'white').swings).toEqual([]);

		// Opening it again searches nothing.
		const again = new GameAnalysis('g1', positions, {
			cache,
			createSearcher: () => {
				throw new Error('no search needed');
			}
		});
		expect(again.status).toBe('done');
		expect(again.evals).toEqual(a.evals);
	});

	it('stops at once and carries on from where it stopped', async () => {
		const searchers: FakeSearcher[] = [];
		const cache = memoryCache();
		const positions = positionsOf(SCHOLARS_MATE);
		const create = () => {
			const s = new FakeSearcher();
			searchers.push(s);
			return s;
		};
		const a = new GameAnalysis('g2', positions, { cache, createSearcher: create });
		a.start();
		await tick();
		searchers[0].answer(cp(30, 'e2e4'));
		await tick();
		a.stop();
		expect(a.status).toBe('stopped');
		expect(searchers[0].disposed).toBe(true);
		await tick();
		expect(a.done).toBe(1);

		// Leaving and coming back picks up the saved part.
		const b = new GameAnalysis('g2', positions, { cache, createSearcher: create });
		expect(b.status).toBe('idle');
		expect(b.done).toBe(1);
		b.start();
		await tick();
		expect(searchers[1].asked).toEqual([positions[1].fen]);
	});

	it('reports an engine that fails', async () => {
		let searcher!: FakeSearcher;
		const a = new GameAnalysis('g3', positionsOf(SCHOLARS_MATE), {
			cache: memoryCache(),
			createSearcher: () => (searcher = new FakeSearcher())
		});
		a.start();
		await tick();
		searcher.answer(null);
		await tick();
		expect(a.status).toBe('failed');
		expect(a.error).toBeTruthy();
	});
});
