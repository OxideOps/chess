import { describe, expect, it } from 'vitest';
import { awayText, gameEndText, statusText } from './status';

describe('statusText', () => {
	it('describes every status', () => {
		expect(statusText({ status: 'ongoing', turn: 'white', winner: null })).toBe('White to move');
		expect(statusText({ status: 'check', turn: 'black', winner: null })).toBe(
			'Black to move — check'
		);
		expect(statusText({ status: 'checkmate', turn: 'black', winner: 'white' })).toBe(
			'Checkmate — White wins'
		);
		expect(statusText({ status: 'stalemate', turn: 'white', winner: null })).toBe(
			'Draw by stalemate'
		);
		expect(statusText({ status: 'fifty_move_rule', turn: 'white', winner: null })).toMatch(
			/fifty-move/
		);
	});
});

describe('gameEndText', () => {
	it('names the winner and how, or says the game was aborted', () => {
		expect(gameEndText({ result: 'black_wins', reason: 'resignation' })).toBe(
			'Black wins by resignation'
		);
		expect(gameEndText({ result: 'white_wins', reason: 'abandoned' })).toBe(
			'White wins by abandonment'
		);
		expect(gameEndText({ result: 'draw', reason: 'agreement' })).toBe('Draw by agreement');
		expect(gameEndText({ result: 'aborted', reason: 'abandoned' })).toBe(
			'Game aborted: a player left before both had moved'
		);
	});
});

describe('awayText', () => {
	it('tells the opponent what happens, and everyone else who left', () => {
		const away = { side: 'black' as const, ms: 41_200 };
		expect(awayText(away, 'white', 4)).toBe(
			'Your opponent left. Unless they come back, you win in 42 s.'
		);
		expect(awayText(away, 'white', 1)).toBe(
			'Your opponent left. Unless they come back, the game is aborted in 42 s.'
		);
		expect(awayText(away, null, 4)).toBe('Black left: 42 s to come back.');
	});
});
