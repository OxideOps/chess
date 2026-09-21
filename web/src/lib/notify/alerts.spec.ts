import { describe, expect, it } from 'vitest';
import { shouldShow, wording } from './alerts';

describe('what an alert says', () => {
	it('names who took the game and what it was', () => {
		expect(wording({ kind: 'game-ready', opponent: 'dan', timeControl: '5+3' })).toEqual({
			title: 'Your game is ready',
			body: 'dan took your 5+3 offer.',
			short: 'Your game is ready'
		});
	});

	it('manages without a name or a time control', () => {
		const said = wording({ kind: 'game-ready', opponent: null, timeControl: null });
		expect(said.body).toBe('A guest took your offer.');
	});

	it('says when someone sat down in a game we were waiting in', () => {
		expect(wording({ kind: 'opponent-joined', opponent: 'dan' }).body).toBe(
			'dan joined your game.'
		);
	});
});

describe('whether to raise a notification', () => {
	it('only for a tab nobody is looking at', () => {
		expect(shouldShow({ hidden: true, permission: 'granted' })).toBe(true);
		// The page is on screen: it is already showing the news.
		expect(shouldShow({ hidden: false, permission: 'granted' })).toBe(false);
	});

	it('never without permission', () => {
		expect(shouldShow({ hidden: true, permission: 'denied' })).toBe(false);
		expect(shouldShow({ hidden: true, permission: 'default' })).toBe(false);
	});
});
