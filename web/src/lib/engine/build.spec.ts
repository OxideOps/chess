import { describe, expect, it } from 'vitest';
import { pickBuild } from './build';

describe('pickBuild', () => {
	it('falls back to the single-threaded build without isolation', () => {
		expect(pickBuild({ crossOriginIsolated: false, hardwareConcurrency: 16 })).toEqual({
			stem: 'stockfish-18-lite-single',
			threads: 1
		});
	});

	it('uses the threaded build with one core left over, capped', () => {
		const pick = (cores: number) =>
			pickBuild({ crossOriginIsolated: true, hardwareConcurrency: cores });
		expect(pick(1)).toEqual({ stem: 'stockfish-18-lite', threads: 1 });
		expect(pick(2)).toEqual({ stem: 'stockfish-18-lite', threads: 1 });
		expect(pick(4).threads).toBe(3);
		expect(pick(10).threads).toBe(8);
		expect(pick(64).threads).toBe(8);
		expect(pick(NaN).threads).toBe(1);
	});
});
