import { page } from 'vitest/browser';
import { beforeAll, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Board from './Board.svelte';
import { GameStore } from '$lib/chess/game.svelte';
import { initChess } from '$lib/chess/wasm';

beforeAll(() => initChess());

function square(name: string) {
	const el = document.querySelector(`[data-square="${name}"]`);
	if (!el) throw new Error(`no square ${name}`);
	return page.elementLocator(el);
}

describe('Board.svelte', () => {
	it('plays a move by clicking, showing hints and the last move', async () => {
		const game = new GameStore();
		render(Board, { game });

		await square('e2').click();
		expect(document.querySelectorAll('.move-hint')).toHaveLength(2); // e3, e4
		await square('e4').click();
		expect(game.view.moves.map((m) => m.san)).toEqual(['e4']);
		expect(document.querySelector('[data-square="e4"]')?.classList.contains('last-move')).toBe(
			true
		);
		expect(document.querySelectorAll('.move-hint')).toHaveLength(0);

		// Black's turn: clicking a white piece does nothing.
		await square('d2').click();
		expect(document.querySelectorAll('.move-hint')).toHaveLength(0);
		game.dispose();
	});

	it('asks for a promotion piece', async () => {
		const game = GameStore.fromFen('k7/4P3/8/8/8/8/8/K7 w - - 0 1');
		render(Board, { game });

		await square('e7').click();
		await square('e8').click();
		await page.getByRole('button', { name: 'knight' }).click();
		expect(game.view.moves[0].san).toBe('e8=N');
		game.dispose();
	});

	it('is read-only in history unless in analysis mode', async () => {
		const game = GameStore.fromPgn('1. e4 e5');
		game.goBack();
		// `render` returns a promise in vitest-browser-svelte 3; awaiting works for both.
		const screen = await render(Board, { game });

		await square('e7').click();
		expect(document.querySelectorAll('.move-hint')).toHaveLength(0);
		screen.unmount();

		render(Board, { game, analysis: true });
		await square('c7').click();
		await square('c5').click();
		expect(game.view.movetext).toBe('1. e4 c5');
		game.dispose();
	});

	it('draws the board from the other side and arrows in board coordinates', async () => {
		const game = new GameStore();
		render(Board, { game, orientation: 'black', arrows: [{ from: 'e2', to: 'e4' }] });

		const first = document.querySelector('.square');
		expect(first?.getAttribute('data-square')).toBe('h1');
		const line = document.querySelector('.arrows line');
		expect(line?.getAttribute('x1')).toBe('3.5');
		expect(line?.getAttribute('y1')).toBe('1.5');
		game.dispose();
	});
});
