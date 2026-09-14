import { expect, test, type Page } from '@playwright/test';
import type { PuzzleData } from '../src/lib/generated/PuzzleData';

// The server imported the fixture puzzles at start-up. Which one is served is
// random, so the test reads it from the response and plays its solution.

async function loadPuzzle(page: Page, action: () => Promise<unknown>): Promise<PuzzleData> {
	const [response] = await Promise.all([
		page.waitForResponse((r) => r.url().endsWith('/api/puzzles/next') && r.ok()),
		action()
	]);
	const puzzle = (await response.json()) as PuzzleData;
	await expect(page.getByTestId('puzzle-status')).toHaveText(
		/^Find the best move for (White|Black)\.$/
	);
	return puzzle;
}

async function play(page: Page, uci: string) {
	await sq(page, uci.slice(0, 2)).click();
	await sq(page, uci.slice(2, 4)).click();
	const promotion = uci.slice(4);
	if (promotion) {
		const role = { q: 'queen', r: 'rook', b: 'bishop', n: 'knight' }[promotion]!;
		await page.locator(`button[title="${role}"]`).click();
	}
}

test('solve a puzzle, then miss one and see the solution', async ({ page }) => {
	const puzzle = await loadPuzzle(page, () => page.goto('/puzzles'));
	await expect(page.getByTestId('puzzle-rating')).toHaveText('1500?');
	// The board faces the solver: the side that doesn't play the setup move.
	const solverIsWhite = puzzle.fen.split(' ')[1] === 'b';
	await expect(page.locator('.square').first()).toHaveAttribute(
		'data-square',
		solverIsWhite ? 'a8' : 'h1'
	);

	const solution = puzzle.moves.slice(1);
	for (let i = 0; i < solution.length; i += 2) {
		await play(page, solution[i]);
		if (i + 1 < solution.length) {
			await expect(page.getByTestId('puzzle-status')).toHaveText('Correct! Find the next move.');
		}
	}
	await expect(page.getByTestId('puzzle-status')).toHaveText('Solved!');
	await expect(page.getByTestId('puzzle-diff')).toHaveText(/^\+\d+$/);
	await expect(page.getByText(`Puzzle rated ${puzzle.rating}`)).toBeVisible();

	// The next one: play a legal move that isn't the solution.
	const next = await loadPuzzle(page, () =>
		page.getByRole('button', { name: 'Next puzzle' }).click()
	);
	expect(next.id).not.toBe(puzzle.id);
	const wanted = next.moves[1];
	await wrongMove(page, wanted);
	await expect(page.getByTestId('puzzle-status')).toHaveText(/^Not quite: the move was .+\.$/);
	await expect(page.getByTestId('puzzle-diff')).toHaveText(/^−\d+$/);
	await page.getByRole('button', { name: 'Show solution' }).click();
	await expect(page.getByTestId('puzzle-status')).toHaveText('That was the solution.');
	// The whole solution was played out: its last move is the one highlighted.
	const last = next.moves.at(-1)!;
	await expect(sq(page, last.slice(0, 2))).toHaveClass(/last-move/);
	await expect(sq(page, last.slice(2, 4))).toHaveClass(/last-move/);
});

/** Any legal move of the solver's other than `wanted`, found through the board's hints. */
async function wrongMove(page: Page, wanted: string) {
	const from = wanted.slice(0, 2);
	const pieces = [
		from,
		...(await page.locator('[data-square]').evaluateAll((squares, first) => {
			const turn = document
				.querySelector('[data-square="' + first + '"]')!
				.getAttribute('aria-label')!
				.includes('white')
				? 'white'
				: 'black';
			return squares
				.filter((s) => (s.getAttribute('aria-label') ?? '').includes(`, ${turn} `))
				.map((s) => s.getAttribute('data-square')!);
		}, from))
	];
	for (const square of pieces) {
		await sq(page, square).click();
		const targets = await page
			.locator('[data-square]:has(.move-hint), [data-square]:has(.capture-hint)')
			.evaluateAll((els) => els.map((e) => e.getAttribute('data-square')!));
		const other = targets.find((t) => square + t !== wanted.slice(0, 4));
		if (other) {
			await sq(page, other).click();
			// A promotion asks for a piece; any will do.
			const picker = page.locator('button[title="queen"]');
			if (await picker.isVisible()) await picker.click();
			return;
		}
		await sq(page, square).click(); // deselect
	}
	throw new Error('no legal alternative move found');
}

function sq(page: Page, name: string) {
	return page.locator(`[data-square="${name}"]`);
}
