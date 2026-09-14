import { expect, test, type Page } from '@playwright/test';

test('win the first drill, see it marked, and move on', async ({ page }) => {
	await page.goto('/lessons');
	await expect(page.locator('ol li')).toHaveCount(6);
	await page.getByTestId('lesson-back-rank-mate').click();
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Back-rank mate');
	const status = page.getByTestId('drill-status');
	await expect(status).toHaveText('Your move · 1 move left');

	await sq(page, 'a1').click();
	await sq(page, 'a8').click();
	await expect(status).toHaveText('Checkmate!');
	await expect(page.getByRole('link', { name: 'Next: Two rooks: the ladder' })).toBeVisible();

	await page.goto('/lessons');
	await expect(page.getByTestId('lesson-back-rank-mate')).toContainText('Done');
	await expect(page.getByTestId('lesson-two-rooks')).not.toContainText('Done');
});

test('Stockfish answers your moves, and restart starts over', async ({ page }) => {
	await page.goto('/lessons/two-rooks');
	const status = page.getByTestId('drill-status');
	await expect(status).toHaveText('Your move · 15 moves left');
	await expect(sq(page, 'd6').locator('img')).toHaveAttribute('alt', 'black king');

	await sq(page, 'a1').click();
	await sq(page, 'a5').click();
	// The real engine (lite Stockfish in a worker) moves the black king.
	await expect(status).toHaveText('Your move · 14 moves left', { timeout: 30_000 });
	await expect(sq(page, 'd6').locator('img')).toHaveCount(0);

	await page.getByRole('button', { name: 'Restart' }).click();
	await expect(status).toHaveText('Your move · 15 moves left');
	await expect(sq(page, 'a1').locator('img')).toHaveAttribute('alt', 'white rook');
});

test('when you defend, the engine moves first and the board faces you', async ({ page }) => {
	await page.goto('/lessons/hold-the-draw');
	await expect(page.getByTestId('drill-status')).toHaveText('Your move · 20 moves left', {
		timeout: 30_000
	});
	// Black's view: h1 is the first square.
	await expect(page.locator('.square').first()).toHaveAttribute('data-square', 'h1');
	await page.goto('/lessons/no-such-drill');
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('No such lesson');
});

function sq(page: Page, name: string) {
	return page.locator(`[data-square="${name}"]`);
}
