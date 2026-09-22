import { expect, test, type Page } from '@playwright/test';

// A short finished game, reviewed from /games with the real Stockfish: White
// wins with the Scholar's mate, so White's own moves hold no swing and
// Black's 3... Nf6?? is the one that lost the game.
test('review a finished game: its swing, the better line, and the analysis board', async ({
	browser
}) => {
	test.setTimeout(90_000);
	const white = await (await browser.newContext()).newPage();
	await white.goto('/online');
	await white.getByRole('button', { name: 'Create a private game' }).click();
	await expect(white).toHaveURL(/\/game\/[0-9a-f-]+$/);
	const invite = await white.getByTestId('invite-link').inputValue();
	const black = await (await browser.newContext()).newPage();
	await black.goto(invite);
	await black.getByRole('button', { name: 'Join as Black' }).click();
	await expect(white.getByTestId('game-status')).toHaveText('Your move');

	const moves: [Page, string, string][] = [
		[white, 'e2', 'e4'],
		[black, 'e7', 'e5'],
		[white, 'd1', 'h5'],
		[black, 'b8', 'c6'],
		[white, 'f1', 'c4'],
		[black, 'g8', 'f6'],
		[white, 'h5', 'f7']
	];
	for (const [i, [page, from, to]] of moves.entries()) {
		await expect(page.getByTestId('game-status')).toHaveText('Your move');
		await sq(page, from).click();
		await sq(page, to).click();
		await expect(page.locator('.move-list button.move')).toHaveCount(i + 1);
	}
	await expect(white.getByTestId('game-status')).toHaveText('White wins by checkmate');

	// From the list, the game offers a review.
	await white.goto('/games');
	await white.getByRole('link', { name: 'Review' }).first().click();
	await expect(white).toHaveURL(/\/games\/[0-9a-f-]+\/review\?side=white$/);
	await expect(white.getByRole('heading', { name: 'Game review' })).toBeVisible();

	// The engine goes through the game, then: nothing in White's own moves...
	await expect(white.getByTestId('no-swings')).toContainText('No big swings in your moves', {
		timeout: 60_000
	});
	await expect(white.getByRole('button', { name: 'Your moves' })).toHaveAttribute(
		'aria-pressed',
		'true'
	);

	// ...and Black's blunder once both sides are shown, with Stockfish's line.
	await white.getByRole('button', { name: 'Both sides' }).click();
	const swings = white.getByTestId('swings').locator('li');
	await expect(swings).toHaveCount(1);
	await expect(swings.first().locator('.played')).toHaveText('3... Nf6');
	await expect(swings.first()).toContainText("Black's winning chances");
	await expect(swings.first().getByTestId('better-line')).toHaveText(/^3\.\.\. \S+/);
	const better = (await swings.first().getByTestId('better-line').textContent())!;
	const betterFirst = better.split(' ')[1];
	expect(betterFirst).not.toBe('Nf6');
	// The board shows the position before the mistake, with the better move drawn.
	await expect(white.locator('.board .arrows line')).toHaveCount(1);

	// A second visit shows it at once: the analysis was kept.
	await white.reload();
	await expect(white.getByTestId('no-swings')).toBeVisible({ timeout: 5_000 });
	await expect(white.getByTestId('review-progress')).toHaveCount(0);

	// The better line opens on the analysis board as a variation, at the move.
	await white.getByRole('button', { name: 'Both sides' }).click();
	await white.getByRole('link', { name: 'Open on the analysis board' }).click();
	await expect(white).toHaveURL(/\/analysis\?/);
	await expect(white.locator('.move-list button.move')).toHaveText([
		'e4',
		'e5',
		'Qh5',
		'Nc6',
		'Bc4',
		'Nf6',
		'Qxf7#'
	]);
	const variation = white.locator('.move-list .variation');
	await expect(variation).toContainText(betterFirst);
	await expect(white.locator('.status')).toHaveText('Black to move');
	await expect(white.locator('.move-list button.move.current')).toHaveText('Bc4');
});

test('an unfinished game has nothing to review yet', async ({ page }) => {
	await page.goto('/online');
	await page.getByRole('button', { name: 'Create a private game' }).click();
	await expect(page).toHaveURL(/\/game\/[0-9a-f-]+$/);
	const id = new URL(page.url()).pathname.split('/').at(-1)!;
	await page.goto(`/games/${id}/review`);
	await expect(page.getByText("This game isn't over yet")).toBeVisible();
});

function sq(page: Page, name: string) {
	return page.locator(`[data-square="${name}"]`);
}
