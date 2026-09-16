import { describe, expect, it } from 'vitest';
import { centre, coords, isLight, squareAt, squaresInDrawOrder } from './squares';

describe('squares', () => {
	it('maps names to coordinates', () => {
		expect(coords('a1')).toEqual({ file: 0, rank: 0 });
		expect(coords('h8')).toEqual({ file: 7, rank: 7 });
		expect(() => coords('i9')).toThrow(/not a square/);
		expect(() => coords('e')).toThrow(/not a square/);
	});

	it('knows square colours', () => {
		expect(isLight('a1')).toBe(false);
		expect(isLight('h1')).toBe(true);
		expect(isLight('e4')).toBe(true);
		expect(isLight('d4')).toBe(false);
	});

	it('draws from the right corner for each orientation', () => {
		expect(squaresInDrawOrder('white').slice(0, 2)).toEqual(['a8', 'b8']);
		expect(squaresInDrawOrder('white').at(-1)).toBe('h1');
		expect(squaresInDrawOrder('black').slice(0, 2)).toEqual(['h1', 'g1']);
		expect(squaresInDrawOrder('black').at(-1)).toBe('a8');
		expect(centre('a8', 'white')).toEqual({ x: 0.5, y: 0.5 });
		expect(centre('a8', 'black')).toEqual({ x: 7.5, y: 7.5 });
		expect(centre('e2', 'white')).toEqual({ x: 4.5, y: 6.5 });
	});

	it('finds the square under a point, from either side', () => {
		expect(squareAt(0.1, 0.1, 'white')).toBe('a8');
		expect(squareAt(4.5, 6.5, 'white')).toBe('e2');
		expect(squareAt(7.99, 7.99, 'white')).toBe('h1');
		expect(squareAt(0.1, 0.1, 'black')).toBe('h1');
		expect(squareAt(4.5, 6.5, 'black')).toBe('d7');
		// Every centre maps back to its own square.
		for (const side of ['white', 'black'] as const) {
			for (const sq of squaresInDrawOrder(side)) {
				const { x, y } = centre(sq, side);
				expect(squareAt(x, y, side)).toBe(sq);
			}
		}
		expect(squareAt(-0.01, 3, 'white')).toBeNull();
		expect(squareAt(3, 8, 'white')).toBeNull();
		expect(squareAt(Number.NaN, 3, 'white')).toBeNull();
	});
});
