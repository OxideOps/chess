import { expect, test } from '@playwright/test';

// The smoke test from the run-app skill, against the real build.
test('plays e4 e5 Nf3 and steps back with the keyboard', async ({ page }) => {
	await page.goto('/');
	const sq = (name: string) => page.locator(`[data-square="${name}"]`);

	await sq('e2').click();
	await sq('e4').click();
	await sq('e7').click();
	await sq('e5').click();
	await sq('g1').click();
	await sq('f3').click();
	await expect(page.locator('.move-list button.move')).toHaveText(['e4', 'e5', 'Nf3']);
	await expect(page.locator('.move-list .number')).toHaveText(['1.', '2.']);
	await expect(page.locator('.status')).toHaveText('Black to move');
	await expect(page.locator('.fen input')).toHaveValue(
		'rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2'
	);

	await page.locator('.board').focus();
	await page.keyboard.press('ArrowLeft');
	await expect(page.locator('.status')).toHaveText('White to move');
	await page.keyboard.press('ArrowDown');
	await expect(page.locator('.status')).toHaveText('Black to move');

	// Sidebar controls: click a move in the list, step with the buttons, start over.
	await page.locator('.move-list .move', { hasText: 'e4' }).click();
	await expect(page.locator('.status')).toHaveText('Black to move');
	await expect(page.locator('.move-list .move.current')).toHaveText('e4');
	await page.getByTitle('Last move').click();
	await expect(page.locator('.move-list .move.current')).toHaveText('Nf3');
	await page.getByRole('button', { name: 'New game' }).click();
	await expect(page.locator('.move-list button.move')).toHaveCount(0);
	await expect(page.locator('.status')).toHaveText('White to move');
});

// A real mouse: press, travel, release. Flipped too, so the drop square is
// worked out from the board as drawn, not as White sees it.
test('moves pieces by dragging them, from either side of the board', async ({ page }) => {
	await page.goto('/');
	const sq = (name: string) => page.locator(`[data-square="${name}"]`);
	const dragTo = async (from: string, to: string) => {
		const a = (await sq(from).boundingBox())!;
		const b = (await sq(to).boundingBox())!;
		await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
		await page.mouse.down();
		await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, { steps: 8 });
		await expect(page.locator('.board .held')).toBeVisible();
		await page.mouse.up();
		await expect(page.locator('.board .held')).toHaveCount(0);
	};

	await dragTo('e2', 'e4');
	await page.getByRole('button', { name: 'Flip board' }).click();
	await dragTo('c7', 'c5');
	await dragTo('g1', 'f3');
	await expect(page.locator('.move-list button.move')).toHaveText(['e4', 'c5', 'Nf3']);

	// An illegal drop puts the piece back; clicking still works after a drag.
	await dragTo('b8', 'b4');
	await expect(page.locator('.move-list button.move')).toHaveCount(3);
	await sq('b8').click();
	await sq('c6').click();
	await expect(page.locator('.move-list button.move')).toHaveText(['e4', 'c5', 'Nf3', 'Nc6']);
});
