import { describe, expect, it } from 'vitest';
import { statusText } from './status';

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
