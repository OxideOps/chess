import { describe, expect, it } from 'vitest';
import { formatDiff, formatRating, gameKind } from './ratings';

describe('rating display', () => {
	it('marks provisional ratings and signs changes', () => {
		expect(formatRating({ value: 1512, provisional: false })).toBe('1512');
		expect(formatRating({ value: 1500, provisional: true })).toBe('1500?');
		expect(formatRating(null)).toBeNull();
		expect(formatDiff(12)).toBe('+12');
		expect(formatDiff(-7)).toBe('−7');
		expect(formatDiff(0)).toBe('±0');
		expect(gameKind(true, 'blitz')).toBe('Rated · Blitz');
		expect(gameKind(false, 'classical')).toBe('Casual · Classical');
	});
});
