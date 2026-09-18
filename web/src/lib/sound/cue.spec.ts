import { describe, expect, it } from 'vitest';
import { cueFor } from './cue';

describe('cueFor', () => {
	it('reads what happened out of the notation', () => {
		expect(cueFor('e4', false)).toBe('move');
		expect(cueFor('Nf3', false)).toBe('move');
		expect(cueFor('exd5', false)).toBe('capture');
		expect(cueFor('Qxf7', false)).toBe('capture');
		expect(cueFor('O-O', false)).toBe('castle');
		expect(cueFor('O-O-O', false)).toBe('castle');
		expect(cueFor('Bb5+', false)).toBe('check');
		expect(cueFor('e8=Q', false)).toBe('promote');
	});

	it('gives the loudest thing about a move', () => {
		// A move can be several of these at once; the order matters.
		expect(cueFor('exd8=Q+', false)).toBe('check');
		expect(cueFor('e8=Q', false)).toBe('promote');
		expect(cueFor('O-O+', false)).toBe('check');
		expect(cueFor('Qxh7#', true)).toBe('end');
		// Any move that ends the game sounds the same, mate or not.
		expect(cueFor('Kb6', true)).toBe('end');
		expect(cueFor('Rxd8+', true)).toBe('end');
	});
});
