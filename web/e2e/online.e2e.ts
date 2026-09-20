import { expect, test, type Page } from '@playwright/test';

// Two browsers, one game, the real server in between.
test('create a game, invite an opponent, play, resign', async ({ browser }) => {
	const white = await browser.newPage();
	await white.goto('/online');
	await white.getByRole('combobox').selectOption({ label: '3+2 Blitz' });
	await white.getByRole('button', { name: 'Create a private game' }).click();
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
	await white.getByRole('button', { name: 'Create a private game' }).click();
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
	// Patient on purpose: a proxy in front of the server can take seconds to
	// report a closed socket, and the client waits `AWAY_NOTICE_DELAY_MS`
	// after that before saying anything. Against a deployment this is the
	// difference between passing and failing.
	await black.close();
	await expect(white.getByTestId('away')).toHaveText(
		/^Your opponent left\. Unless they come back, you win in \d+ s\.$/,
		{ timeout: 30_000 }
	);

	// Black opens the game again (same browser, same session): the notice goes.
	black = await blackContext.newPage();
	await black.goto(invite);
	await expect(black.getByTestId('game-status')).toHaveText('Waiting for your opponent');
	await expect(white.getByTestId('away')).toHaveCount(0, { timeout: 30_000 });
});

function sq(page: Page, name: string) {
	return page.locator(`[data-square="${name}"]`);
}

// The lobby: two strangers meet without anyone sending a link.
//
// The seek list is global and these tests share a server with everything
// else running at the same time, so they never count the whole list —
// each poster signs up under a name nothing else uses, and the assertions
// are about that person's row.
test('post a seek, someone takes it, and both land in the same game', async ({ browser }) => {
	const name = seekerName('sk');
	const poster = await (await browser.newContext()).newPage();
	const taker = await (await browser.newContext()).newPage();
	await signUp(poster, name);
	await poster.goto('/online');
	await taker.goto('/online');

	await poster.getByRole('combobox').selectOption({ label: '3+2 Blitz' });
	await poster.getByLabel('Rated').uncheck();
	await poster.getByTestId('post-seek').click();
	await expect(poster.getByTestId('waiting')).toBeVisible();
	// Posting doesn't create a game: the poster is still on the lobby page.
	await expect(poster).toHaveURL(/\/online$/);

	// The other browser sees it appear, without reloading.
	const seek = taker.getByTestId('seek').filter({ hasText: name });
	await expect(seek).toHaveCount(1);
	await expect(seek).toContainText('3+2');
	await expect(seek).toContainText('Casual · Blitz');
	// Our own seek is not offered back to us.
	await expect(poster.getByTestId('seek').filter({ hasText: name })).toHaveCount(0);

	await seek.click();
	// Both are sent to the same board, on opposite sides.
	await expect(taker).toHaveURL(/\/game\/[0-9a-f-]+$/);
	await expect(poster).toHaveURL(/\/game\/[0-9a-f-]+$/);
	expect(poster.url()).toBe(taker.url());
	// Nobody is waiting for anyone to join: both seats were filled at once.
	await expect(poster.getByTestId('invite-link')).toHaveCount(0);
	await expect(poster.getByLabel('White clock')).toContainText(/\w/);

	// Colours were drawn, so wait to hear which of them is White before
	// deciding who plays the first move.
	const status = poster.getByTestId('game-status');
	await expect(status).toHaveText(/Your move|Waiting for your opponent/);
	const postersTurn = (await status.textContent())?.trim() === 'Your move';
	const mover = postersTurn ? poster : taker;
	const other = postersTurn ? taker : poster;
	await sq(mover, 'e2').click();
	await sq(mover, 'e4').click();
	await expect(other.locator('.move-list button.move')).toHaveText(['e4']);
});

test('a seek disappears when the person offering it leaves', async ({ browser }) => {
	const name = seekerName('lv');
	const poster = await (await browser.newContext()).newPage();
	const watcher = await (await browser.newContext()).newPage();
	await signUp(poster, name);
	await poster.goto('/online');
	await watcher.goto('/online');
	const theirs = watcher.getByTestId('seek').filter({ hasText: name });

	await poster.getByTestId('post-seek').click();
	await expect(theirs).toHaveCount(1);

	// Cancelling withdraws it...
	await poster.getByTestId('cancel-seek').click();
	await expect(theirs).toHaveCount(0);

	// ...and so does closing the tab, so nobody is left clicking a ghost.
	await poster.getByTestId('post-seek').click();
	await expect(theirs).toHaveCount(1);
	await poster.close();
	await expect(theirs).toHaveCount(0);
});

async function signUp(page: Page, username: string) {
	await page.goto('/signup');
	await page.getByLabel('Username').fill(username);
	await page.getByLabel('Password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Sign up' }).click();
	await expect(page).toHaveURL('/');
}

/** A unique name that fits the 3-20 character limit on usernames. */
function seekerName(prefix: string): string {
	const stamp = Date.now().toString(36);
	const salt = Math.floor(Math.random() * 1e4)
		.toString()
		.padStart(4, '0');
	return `${prefix}_${stamp}${salt}`;
}
