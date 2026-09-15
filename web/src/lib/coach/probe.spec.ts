import { describe, expect, it } from 'vitest';
import { Prober } from './probe';
import type { OpponentLike, Search } from '$lib/engine/opponent.svelte';

function engine(result: Search | null) {
	const asked: [string, string[]][] = [];
	let created = 0;
	let disposed = 0;
	const create = (): OpponentLike => {
		created++;
		return {
			search: async (fen, moves) => {
				asked.push([fen, moves]);
				return result;
			},
			dispose: () => {
				disposed++;
			}
		};
	};
	return { create, asked, counts: () => ({ created, disposed }) };
}

describe('Prober', () => {
	it('searches the position after the move, on an engine started on first use', async () => {
		const e = engine({
			best: 'e4e5',
			score: { kind: 'cp', value: 40 },
			pv: ['e4e5', 'f6d5'],
			depth: 17
		});
		const prober = new Prober(e.create);
		expect(e.counts().created).toBe(0);
		expect(await prober.probe('fen', 'g8f6')).toEqual({
			depth: 17,
			score: { kind: 'cp', value: 40 },
			pv: ['e4e5', 'f6d5']
		});
		await prober.probe('fen', 'd7d5');
		expect(e.asked).toEqual([
			['fen', ['g8f6']],
			['fen', ['d7d5']]
		]);
		expect(e.counts()).toEqual({ created: 1, disposed: 0 });
		prober.dispose();
		expect(e.counts().disposed).toBe(1);
	});

	it('has nothing to say without a score', async () => {
		expect(await new Prober(engine(null).create).probe('fen', 'g8f6')).toBeNull();
		const unscored = engine({ best: null, score: null, pv: [] });
		expect(await new Prober(unscored.create).probe('fen', 'g8f6')).toBeNull();
	});
});
