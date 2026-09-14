import { describe, expect, it } from 'vitest';
import { matchup, opponentName, outcome, relativeTime, seatName } from './listing';
import type { GameListing } from '$lib/generated/GameListing';

const base: GameListing = {
	id: 'g',
	players: { white: { username: 'alice' }, black: { username: null } },
	your_color: 'white',
	ended: null,
	moves: 4,
	updated_at: '2026-09-13T12:00:00Z'
};

describe('listing helpers', () => {
	it("names seats and opponents from the caller's side", () => {
		expect(seatName(null)).toBe('Open seat');
		expect(seatName({ username: null })).toBe('Guest');
		expect(seatName({ username: 'bob' })).toBe('bob');
		expect(opponentName(base)).toBe('Guest');
		expect(opponentName({ ...base, your_color: 'black' })).toBe('alice');
		expect(matchup({ ...base, your_color: 'black' })).toBe('You (Black) vs alice');
	});

	it('works out the outcome for the caller', () => {
		expect(outcome(base)).toBe('playing');
		expect(outcome({ ...base, players: { ...base.players, black: null } })).toBe('waiting');
		expect(outcome({ ...base, ended: { result: 'white_wins', reason: 'checkmate' } })).toBe('won');
		expect(outcome({ ...base, ended: { result: 'black_wins', reason: 'timeout' } })).toBe('lost');
		expect(
			outcome({ ...base, your_color: 'black', ended: { result: 'black_wins', reason: 'timeout' } })
		).toBe('won');
		expect(outcome({ ...base, ended: { result: 'draw', reason: 'agreement' } })).toBe('draw');
	});

	it('formats how long ago', () => {
		const now = Date.parse('2026-09-13T12:00:30Z');
		expect(relativeTime('2026-09-13T12:00:00Z', now)).toBe('just now');
		expect(relativeTime('2026-09-13T11:55:00Z', now)).toBe('6 min ago');
		expect(relativeTime('2026-09-13T09:00:00Z', now)).toBe('3 h ago');
		expect(relativeTime('2026-09-12T11:00:00Z', now)).toBe('1 day ago');
		expect(relativeTime('2026-09-01T11:00:00Z', now)).toBe('12 days ago');
		expect(relativeTime('nonsense', now)).toBe('nonsense');
	});
});
