import { describe, expect, it } from 'vitest';
import { isThemeKey, themeName, themeOptions } from './themes';

describe('puzzle themes', () => {
	it('names the known ones the way Lichess does', () => {
		expect(themeName('backRankMate')).toBe('Back rank mate');
		expect(themeName('mateIn2')).toBe('Mate in 2');
		expect(themeName('fork')).toBe('Fork');
	});

	it('makes words out of keys it has never seen', () => {
		expect(themeName('someNewTheme')).toBe('Some new theme');
		expect(themeName('mateIn9')).toBe('Mate in 9');
		expect(themeName('x')).toBe('X');
	});

	it('lists the options by name, with their counts', () => {
		const options = themeOptions([
			{ theme: 'short', count: 30 },
			{ theme: 'fork', count: 7 },
			{ theme: 'backRankMate', count: 2 }
		]);
		expect(options).toEqual([
			{ key: 'backRankMate', name: 'Back rank mate', count: 2 },
			{ key: 'fork', name: 'Fork', count: 7 },
			{ key: 'short', name: 'Short puzzle', count: 30 }
		]);
	});

	it('only passes keys the server accepts', () => {
		expect(isThemeKey('mateIn2')).toBe(true);
		expect(isThemeKey(null)).toBe(false);
		expect(isThemeKey('')).toBe(false);
		expect(isThemeKey('back rank')).toBe(false);
		expect(isThemeKey('fork&x=1')).toBe(false);
	});
});
