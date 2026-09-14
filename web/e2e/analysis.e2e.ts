import { expect, test } from '@playwright/test';

// Stockfish runs for real here (7 MB of WASM in a worker), so allow time.
test.describe('analysis board', () => {
	test.setTimeout(60_000);

	test('engine follows the position, lines are playable, imports work', async ({ page }) => {
		await page.goto('/analysis');
		const summary = page.locator('.engine .summary');
		const lines = page.locator('.engine .lines li');
		const sq = (name: string) => page.locator(`[data-square="${name}"]`);

		await expect(page.locator('.engine .name')).toHaveText(/Stockfish/, { timeout: 30_000 });
		// The server sends the isolation headers, so the multi-threaded build runs
		// with one thread per spare core (the panel only mentions threads beyond one).
		const cores = await page.evaluate(
			() => [crossOriginIsolated, navigator.hardwareConcurrency] as const
		);
		expect(cores[0]).toBe(true);
		const threads = Math.min(8, Math.max(1, cores[1] - 1));
		if (threads > 1) {
			await expect(page.locator('.engine .threads')).toHaveText(`${threads} threads`);
		} else {
			await expect(page.locator('.engine .threads')).toHaveCount(0);
		}
		await expect(summary).toHaveText(/Depth \d+/, { timeout: 30_000 });
		await expect(lines).toHaveCount(3);
		await expect(page.locator('.board .arrows line')).toHaveCount(1);
		await expect(page.getByTestId('eval-bar').locator('.white')).not.toHaveAttribute(
			'style',
			/height: 50%/
		);

		// Move: the engine switches to the new position (Black to move, lines start "1...").
		await sq('e2').click();
		await sq('e4').click();
		await expect(page.locator('.status')).toHaveText('Black to move');
		await expect(lines.first()).toContainText('1...', { timeout: 30_000 });

		// Clicking a line plays its first move.
		await lines.first().click();
		await expect(page.locator('.move-list .move')).toHaveCount(2);
		await expect(page.locator('.status')).toHaveText('White to move');

		// Moving from history truncates the game.
		await page.locator('.board').focus();
		await page.keyboard.press('ArrowUp');
		await sq('d2').click();
		await sq('d4').click();
		await expect(page.locator('#export-pgn')).toHaveValue('1. d4 *');

		// PGN import with comments, variations and annotations.
		await page
			.locator('#import-pgn')
			.fill('1. e4 {best} e5 2. Nf3 (2. Bc4 Nf6) 2... Nc6 3. Bb5!? a6 4. Ba4 Nf6 5. O-O Be7 1-0');
		await page.locator('#import-pgn ~ .row button').click();
		await expect(page.locator('#export-pgn')).toHaveValue(
			'1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 *'
		);
		await expect(page.locator('.move-list .move.current')).toHaveText('Be7');

		// Bad FEN shows an error; a checkmate FEN idles the engine.
		await page.locator('#import-fen').fill('not a fen');
		await page.locator('#import-fen ~ button').click();
		await expect(page.getByRole('alert')).toContainText('invalid FEN');
		await page
			.locator('#import-fen')
			.fill('rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3');
		await page.locator('#import-fen ~ button').click();
		await expect(page.getByRole('alert')).toHaveCount(0);
		await expect(page.locator('.status')).toHaveText('Checkmate — Black wins');
		await expect(summary).toHaveText('Idle');
		await expect(lines).toHaveCount(0);

		// Engine off.
		await page
			.locator('#import-fen')
			.fill('rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1');
		await page.locator('#import-fen ~ button').click();
		await page.locator('.engine header input').uncheck();
		await expect(summary).toHaveText('Engine off');
	});
});
