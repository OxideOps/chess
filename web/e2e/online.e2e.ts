import { expect, test, type Page } from '@playwright/test';

// Two browsers, one game, the real server in between.
test('create a game, invite an opponent, play, resign', async ({ browser }) => {
	const white = await browser.newPage();
	await white.goto('/online');
	await white.getByRole('combobox').selectOption({ label: '3+2 Blitz' });
	await white.getByRole('button', { name: 'Create game' }).click();
	await expect(white).toHaveURL(/\/game\/[0-9a-f-]+\?token=/);
	await expect(white.getByTestId('game-status')).toHaveText('Your move');
	const invite = await white.getByTestId('invite-link').inputValue();
	expect(invite).toMatch(/\/game\/[0-9a-f-]+\?token=/);

	const black = await browser.newPage();
	await black.goto(invite);
	await expect(black.getByTestId('game-status')).toHaveText('Waiting for your opponent');
	// Black sees the board from their side, and cannot move White's pieces.
	await expect(black.locator('.square').first()).toHaveAttribute('data-square', 'h1');
	await sq(black, 'e2').click();
	await expect(black.locator('.move-hint')).toHaveCount(0);

	await sq(white, 'e2').click();
	await sq(white, 'e4').click();
	await expect(white.locator('.move-list button.move')).toHaveText(['e4']);
	await expect(black.locator('.move-list button.move')).toHaveText(['e4']);
	await expect(black.getByTestId('game-status')).toHaveText('Your move');
	await expect(white.getByTestId('invite-link')).toHaveCount(0);

	await sq(black, 'e7').click();
	await sq(black, 'e5').click();
	await expect(white.locator('.move-list button.move')).toHaveText(['e4', 'e5']);
	// Clocks: 3+2, both moved, White's clock is running.
	await expect(white.getByLabel('White clock')).toHaveClass(/active/);
	await expect(white.getByLabel('Black clock')).toContainText('3:0');

	// Draw offer and decline, then resignation ends it for both.
	await black.getByRole('button', { name: 'Offer draw' }).click();
	await expect(white.getByText('Black offers a draw.')).toBeVisible();
	await white.getByRole('button', { name: 'Decline' }).click();
	await expect(white.getByText('Black offers a draw.')).toHaveCount(0);
	await white.getByRole('button', { name: 'Resign' }).click();
	await expect(white.getByTestId('game-status')).toHaveText('Black wins by resignation');
	await expect(black.getByTestId('game-status')).toHaveText('Black wins by resignation');
	await expect(black.getByRole('button', { name: 'Resign' })).toHaveCount(0);

	// A spectator opening the plain link sees the finished game.
	const watcher = await browser.newPage();
	await watcher.goto(invite.split('?')[0]);
	await expect(watcher.locator('.move-list button.move')).toHaveText(['e4', 'e5']);
	await expect(watcher.getByTestId('game-status')).toHaveText('Black wins by resignation');
});

function sq(page: Page, name: string) {
	return page.locator(`[data-square="${name}"]`);
}
