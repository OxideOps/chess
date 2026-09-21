import { page } from 'vitest/browser';
import { tick } from 'svelte';
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

type PointerKind = 'mouse' | 'touch' | 'pen';

/** Send a pointer event at the centre of `target` (a square name) or at a page point. */
function pointer(
	type: string,
	target: string | { x: number; y: number },
	pointerType: PointerKind = 'mouse'
) {
	let el: Element;
	let x: number;
	let y: number;
	if (typeof target === 'string') {
		el = document.querySelector(`[data-square="${target}"]`)!;
		const r = el.getBoundingClientRect();
		x = r.left + r.width / 2;
		y = r.top + r.height / 2;
	} else {
		({ x, y } = target);
		el = document.body;
	}
	el.dispatchEvent(
		new PointerEvent(type, {
			bubbles: true,
			cancelable: true,
			clientX: x,
			clientY: y,
			pointerId: pointerType === 'mouse' ? 1 : 7,
			pointerType,
			isPrimary: true,
			button: type === 'pointermove' ? -1 : 0
		})
	);
}

/** Press on `from`, travel to `to` (a square or a page point), and hold there. */
async function pickUpAndMove(
	from: string,
	to: string | { x: number; y: number },
	kind: PointerKind = 'mouse'
) {
	pointer('pointerdown', from, kind);
	pointer('pointermove', to, kind);
	await tick();
}

async function drop(at: string | { x: number; y: number }, kind: PointerKind = 'mouse') {
	pointer('pointerup', at, kind);
	await tick();
}

/** Press and hold on `square`, sending nothing else. */
async function press(square: string, kind: PointerKind = 'mouse') {
	pointer('pointerdown', square, kind);
	await tick();
}

/**
 * The whole of one press: down, up, and the click the browser sends after
 * them. (`locator.click()` does the same, so it is a press too, not a bare
 * click — which matters for anything that behaves differently the second
 * time.)
 */
async function tap(square: string, kind: PointerKind = 'mouse') {
	await press(square, kind);
	pointer('pointerup', square, kind);
	(document.querySelector(`[data-square="${square}"]`) as HTMLElement).click();
	await tick();
}

const hasClass = (name: string, cls: string) =>
	document.querySelector(`[data-square="${name}"]`)!.classList.contains(cls);

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

	it('picks the piece up on the press, before the pointer has moved', async () => {
		const game = new GameStore();
		render(Board, { game });

		// Pressed and held, with no movement at all.
		await press('e2');
		expect(document.querySelector('.held')).not.toBeNull();
		expect(hasClass('e2', 'lifted')).toBe(true);
		expect(hasClass('e2', 'selected')).toBe(true);
		expect(document.querySelectorAll('.move-hint')).toHaveLength(2); // e3, e4
		// The square under the pointer is the one it came from: no landing ring.
		expect(hasClass('e2', 'drag-over')).toBe(false);

		// Letting go without moving leaves it selected, ready for a click.
		pointer('pointerup', 'e2');
		(document.querySelector('[data-square="e2"]') as HTMLElement).click();
		await tick();
		expect(document.querySelector('.held')).toBeNull();
		expect(hasClass('e2', 'selected')).toBe(true);
		expect(document.querySelectorAll('.move-hint')).toHaveLength(2);

		await tap('e4');
		expect(game.view.moves.map((m) => m.san)).toEqual(['e4']);
		game.dispose();
	});

	it('puts a piece down when it is pressed a second time', async () => {
		const game = new GameStore();
		render(Board, { game });

		await tap('e2');
		expect(hasClass('e2', 'selected')).toBe(true);

		// The second press on the same square is the one that puts it down.
		await tap('e2');
		expect(hasClass('e2', 'selected')).toBe(false);
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
		// In analysis the old move stays, as the main line.
		expect(game.view.movetext).toBe('1. e4 e5 (1... c5)');
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

	it('plays a move by dragging, with the piece following the pointer', async () => {
		const game = new GameStore();
		render(Board, { game });

		await pickUpAndMove('g1', 'f3');
		// Held: a copy follows the pointer, the original is a ghost, hints show.
		expect(document.querySelector('.held')).not.toBeNull();
		expect(hasClass('g1', 'lifted')).toBe(true);
		expect(hasClass('f3', 'drag-over')).toBe(true);
		expect(document.querySelectorAll('.move-hint')).toHaveLength(2); // f3, h3
		await drop('f3');

		expect(game.view.moves.map((m) => m.san)).toEqual(['Nf3']);
		expect(document.querySelector('.held')).toBeNull();
		expect(document.querySelectorAll('.move-hint')).toHaveLength(0);
		game.dispose();
	});

	it('drags with a finger just as with a mouse', async () => {
		const game = new GameStore();
		render(Board, { game });

		await pickUpAndMove('e2', 'e4', 'touch');
		await drop('e4', 'touch');
		expect(game.view.moves.map((m) => m.san)).toEqual(['e4']);
		game.dispose();
	});

	it('snaps back from an illegal square or off the board', async () => {
		const game = new GameStore();
		render(Board, { game });

		await pickUpAndMove('e2', 'e5');
		await drop('e5');
		expect(game.view.moves).toHaveLength(0);
		expect(document.querySelector('.held')).toBeNull();
		expect(hasClass('e2', 'selected')).toBe(false);

		const board = document.querySelector('.board')!.getBoundingClientRect();
		const outside = { x: board.right + 40, y: board.top + 10 };
		await pickUpAndMove('e2', outside);
		await drop(outside);
		expect(game.view.moves).toHaveLength(0);
		expect(hasClass('e2', 'selected')).toBe(false);
		game.dispose();
	});

	it('keeps a piece picked up when it is put back, so a click can finish the move', async () => {
		const game = new GameStore();
		render(Board, { game });

		// Travel away and come back to the same square.
		await pickUpAndMove('d2', 'd4');
		pointer('pointermove', 'd2');
		await drop('d2');
		expect(hasClass('d2', 'selected')).toBe(true);
		expect(game.view.moves).toHaveLength(0);

		await square('d4').click();
		expect(game.view.moves.map((m) => m.san)).toEqual(['d4']);
		game.dispose();
	});

	it("won't pick up the other side's pieces, or anything in read-only history", async () => {
		const game = new GameStore();
		render(Board, { game });

		await pickUpAndMove('e7', 'e5');
		expect(document.querySelector('.held')).toBeNull();
		await drop('e5');
		expect(game.view.moves).toHaveLength(0);
		expect(hasClass('e7', 'movable')).toBe(false);
		expect(hasClass('e2', 'movable')).toBe(true);
		game.dispose();

		const played = GameStore.fromPgn('1. e4 e5');
		played.goBack();
		const screen = await render(Board, { game: played });
		await pickUpAndMove('g8', 'f6');
		expect(document.querySelector('.held')).toBeNull();
		await drop('f6');
		expect(played.view.movetext).toBe('1. e4 e5');
		screen.unmount();
		played.dispose();
	});

	it('asks for a promotion piece after a drag, too', async () => {
		const game = GameStore.fromFen('k7/4P3/8/8/8/8/8/K7 w - - 0 1');
		render(Board, { game });

		await pickUpAndMove('e7', 'e8');
		await drop('e8');
		await page.getByRole('button', { name: 'queen' }).click();
		expect(game.view.moves[0].san).toBe('e8=Q+');
		game.dispose();
	});

	it('a press that barely moves is still a click', async () => {
		const game = new GameStore();
		render(Board, { game });

		const el = document.querySelector('[data-square="b1"]')!.getBoundingClientRect();
		const nudge = { x: el.left + el.width / 2 + 2, y: el.top + el.height / 2 + 1 };
		pointer('pointerdown', 'b1');
		pointer('pointermove', nudge);
		await tick();
		// Held, as any press is — but a nudge this small is not a drag, so
		// letting go leaves the piece picked up rather than dropping it.
		expect(document.querySelector('.held')).not.toBeNull();
		pointer('pointerup', nudge);
		(document.querySelector('[data-square="b1"]') as HTMLElement).click();
		await tick();
		expect(hasClass('b1', 'selected')).toBe(true);
		await tap('c3');
		expect(game.view.moves.map((m) => m.san)).toEqual(['Nc3']);
		game.dispose();
	});
});
