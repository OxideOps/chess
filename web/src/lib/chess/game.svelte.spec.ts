import { beforeAll, describe, expect, it } from 'vitest';
import { initChess, parseEngineMessage, pvMovetext, scoreForWhite, barFraction } from './wasm';
import { GameStore } from './game.svelte';
import type { EngineMessage } from '$lib/generated/EngineMessage';
import type { EngineScore } from '$lib/generated/EngineScore';

const START = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

beforeAll(() => initChess());

describe('GameStore', () => {
	it('plays moves through chess-core and exposes a view', () => {
		const game = new GameStore();
		expect(game.view.fen).toBe(START);
		expect(game.view.pieces).toHaveLength(32);

		expect(game.play('e2', 'e4')).toBe('ok');
		expect(game.play('e7', 'e5')).toBe('ok');
		expect(game.play('g1', 'f3')).toBe('ok');
		expect(game.movetext()).toBe('1. e4 e5 2. Nf3');
		expect(game.view.moves.map((m) => m.san)).toEqual(['e4', 'e5', 'Nf3']);
		expect(game.view.turn).toBe('black');
		expect(game.view.lastMove).toEqual({ from: 'g1', to: 'f3' });
		expect(game.view.cursor).toBe(3);

		expect(game.play('e5', 'e3')).toBe('illegal');
		expect(game.legalDestinations('b8').sort()).toEqual(['a6', 'c6']);
		game.dispose();
	});

	it('navigates history and plays from the middle of it', () => {
		const game = GameStore.fromPgn('1. e4 e5 2. Nf3 Nc6');
		game.goBack();
		expect(game.view.viewingHistory).toBe(true);
		expect(game.view.cursor).toBe(3);
		game.goToStart();
		expect(game.view.fen).toBe(START);
		expect(game.playHere('d2', 'd4')).toBe('ok');
		expect(game.movetext()).toBe('1. d4');
		expect(game.pgn()).toBe('1. d4 *');
		game.dispose();
	});

	it('reports promotions and checkmate', () => {
		const game = GameStore.fromFen('k7/4P3/8/8/8/8/8/K7 w - - 0 1');
		expect(game.play('e7', 'e8')).toBe('promotion_required');
		expect(game.play('e7', 'e8', 'knight')).toBe('ok');
		expect(game.view.moves[0].san).toBe('e8=N');

		const mate = GameStore.fromPgn('1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6 4. Qxf7#');
		expect(mate.view.status).toBe('checkmate');
		expect(mate.view.winner).toBe('white');
		expect(mate.view.gameOver).toBe(true);
		expect(mate.view.checkSquare).toBe('e8');
		expect(mate.pgn()).toContain('1-0');
		game.dispose();
		mate.dispose();
	});

	it('throws a readable error for bad input', () => {
		expect(() => GameStore.fromFen('not a fen')).toThrow(/invalid FEN/);
		expect(() => GameStore.fromPgn('1. e4 e5 2. Ke2 Ke7 3. Nf6')).toThrow(/illegal move Nf6/);
	});
});

describe('engine helpers', () => {
	it('parses UCI output and presents scores', () => {
		const msg: EngineMessage = parseEngineMessage(
			'info depth 18 multipv 2 score cp -35 nodes 1 pv e7e5 g1f3'
		);
		expect(msg).toEqual({
			type: 'info',
			line: { depth: 18, multipv: 2, score: { kind: 'cp', value: -35 }, pv: ['e7e5', 'g1f3'] }
		});
		expect(parseEngineMessage('bestmove (none)')).toEqual({ type: 'best_move', best: null });

		const forBlack: EngineScore = { kind: 'cp', value: 100 };
		expect(scoreForWhite(forBlack, 'black')).toEqual({ kind: 'cp', value: -100 });
		expect(barFraction({ kind: 'mate', value: -3 })).toBe(0);
		expect(barFraction({ kind: 'cp', value: 0 })).toBe(0.5);

		const afterE4 = 'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1';
		expect(pvMovetext(afterE4, ['e7e5', 'g1f3'])).toBe('1... e5 2. Nf3');
	});
});
