import { expect, test, type Page } from '@playwright/test';

// Two browsers, one game, the real server in between.
test('create a game, invite an opponent, play, resign', async ({ browser }) => {
	const white = await browser.newPage();
	await white.goto('/online');
	await white.getByRole('combobox').selectOption({ label: '3+2 Blitz' });
	await white.getByRole('button', { name: 'Create game' }).click();
	await expect(white).toHaveURL(/\/game\/[0-9a-f-]+$/);
	await expect(white.getByTestId('game-status')).toHaveText('Waiting for an opponent to join');
	const invite = await white.getByTestId('invite-link').inputValue();
	expect(invite).toMatch(/\/game\/[0-9a-f-]+$/);

	// A second browser (its own cookies) opens the link and takes Black.
	const black = await (await browser.newContext()).newPage();
	await black.goto(invite);
	await black.getByRole('button', { name: 'Join as Black' }).click();
	await expect(black.getByTestId('game-status')).toHaveText('Waiting for your opponent');
	await expect(white.getByTestId('game-status')).toHaveText('Your move');
	await expect(white.getByTestId('invite-link')).toHaveCount(0);
	// Black sees the board from their side, and cannot move White's pieces.
	await expect(black.locator('.square').first()).toHaveAttribute('data-square', 'h1');
	await sq(black, 'e2').click();
	await expect(black.locator('.move-hint')).toHaveCount(0);

	await sq(white, 'e2').click();
	await sq(white, 'e4').click();
	await expect(white.locator('.move-list button.move')).toHaveText(['e4']);
	await expect(black.locator('.move-list button.move')).toHaveText(['e4']);
	await expect(black.getByTestId('game-status')).toHaveText('Your move');

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

	// A third browser opening the link is a spectator: no seat to join, finished game shown.
	const watcher = await (await browser.newContext()).newPage();
	await watcher.goto(invite);
	await expect(watcher.locator('.move-list button.move')).toHaveText(['e4', 'e5']);
	await expect(watcher.getByTestId('game-status')).toHaveText('Black wins by resignation');
	await expect(watcher.getByRole('button', { name: 'Join as Black' })).toHaveCount(0);
});

test('an opponent who leaves gets a countdown, and coming back cancels it', async ({ browser }) => {
	const white = await (await browser.newContext()).newPage();
	await white.goto('/online');
	await white.getByRole('button', { name: 'Create game' }).click();
	await expect(white).toHaveURL(/\/game\/[0-9a-f-]+$/);
	const invite = await white.getByTestId('invite-link').inputValue();

	const blackContext = await browser.newContext();
	let black = await blackContext.newPage();
	await black.goto(invite);
	await black.getByRole('button', { name: 'Join as Black' }).click();
	await expect(black.getByTestId('game-status')).toHaveText('Waiting for your opponent');
	await sq(white, 'e2').click();
	await sq(white, 'e4').click();
	await sq(black, 'e7').click();
	await sq(black, 'e5').click();
	await expect(white.getByTestId('game-status')).toHaveText('Your move');
	await expect(white.getByTestId('away')).toHaveCount(0);

	// Black closes the tab: after a moment White is told, with the time left.
	await black.close();
	await expect(white.getByTestId('away')).toHaveText(
		/^Your opponent left\. Unless they come back, you win in \d+ s\.$/
	);

	// Black opens the game again (same browser, same session): the notice goes.
	black = await blackContext.newPage();
	await black.goto(invite);
	await expect(black.getByTestId('game-status')).toHaveText('Waiting for your opponent');
	await expect(white.getByTestId('away')).toHaveCount(0);
});

function sq(page: Page, name: string) {
	return page.locator(`[data-square="${name}"]`);
}
