import { describe, expect, it } from 'vitest';
import type { ResolvedPathname } from '$app/types';
import { parsePly, parseSide, withQuery } from './links';

describe('review links', () => {
	it('adds a query string, encoded', () => {
		const path = '/analysis' as ResolvedPathname;
		expect(withQuery(path, {})).toBe('/analysis');
		expect(withQuery(path, { pgn: '1. e4 e5 (1... c5) *', ply: '3' })).toBe(
			'/analysis?pgn=1.+e4+e5+%281...+c5%29+*&ply=3'
		);
		const url = new URL(withQuery(path, { pgn: '1. e4 e5 (1... c5) *' }), 'http://x');
		expect(url.searchParams.get('pgn')).toBe('1. e4 e5 (1... c5) *');
	});

	it('reads sides and plies, and nothing else', () => {
		expect(parseSide('white')).toBe('white');
		expect(parseSide('black')).toBe('black');
		expect(parseSide('White')).toBeNull();
		expect(parseSide(null)).toBeNull();
		expect(parsePly('0')).toBe(0);
		expect(parsePly('12')).toBe(12);
		expect(parsePly('-1')).toBeNull();
		expect(parsePly('1.5')).toBeNull();
		expect(parsePly('')).toBeNull();
		expect(parsePly(null)).toBeNull();
	});
});
