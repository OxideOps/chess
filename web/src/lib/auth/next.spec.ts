import { describe, expect, it } from 'vitest';
import type { ResolvedPathname } from '$app/types';
import { safeNext, withNext } from './next';

describe('safeNext', () => {
	it('keeps same-site paths and drops everything else', () => {
		expect(safeNext('/game/abc?x=1')).toBe('/game/abc?x=1');
		expect(safeNext(null)).toBe('/');
		expect(safeNext('')).toBe('/');
		expect(safeNext('https://evil.example/')).toBe('/');
		expect(safeNext('//evil.example/')).toBe('/');
		expect(safeNext('/\\evil.example/')).toBe('/');
		expect(safeNext('relative')).toBe('/');
		expect(safeNext(null, '/games' as ResolvedPathname)).toBe('/games');
	});
});

describe('withNext', () => {
	it('adds the return path as a query parameter, except for home', () => {
		const login = '/login' as ResolvedPathname;
		expect(withNext(login, '/')).toBe('/login');
		expect(withNext(login, '/game/abc?x=1')).toBe('/login?next=%2Fgame%2Fabc%3Fx%3D1');
	});
});
