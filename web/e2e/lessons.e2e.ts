import { expect, test, type Page } from '@playwright/test';

test('win the first drill, see it marked, and move on', async ({ page }) => {
	await page.goto('/lessons');
	await expect(page.locator('ol li')).toHaveCount(18);
	await page.getByTestId('lesson-back-rank-mate').click();
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Back-rank mate');
	const status = page.getByTestId('drill-status');
	await expect(status).toHaveText('Your move · 1 move left');

	await sq(page, 'a1').click();
	await sq(page, 'a8').click();
	await expect(status).toHaveText('Checkmate!');
	await expect(page.getByRole('link', { name: 'Next: Develop and castle' })).toBeVisible();

	await page.goto('/lessons');
	await expect(page.getByTestId('lesson-back-rank-mate')).toContainText('Done');
	await expect(page.getByTestId('lesson-develop-and-castle')).not.toContainText('Done');
	await expect(page.getByTestId('lessons-done')).toHaveText('1 of 18 done');
});

test('a tactic is won by the material once Stockfish has answered', async ({ page }) => {
	await page.goto('/lessons/knight-fork');
	await expect(page.getByText('Goal: win at least 6 points of material')).toBeVisible();
	const status = page.getByTestId('drill-status');
	await expect(status).toHaveText('Your move · 2 moves left', { timeout: 30_000 });
	await sq(page, 'd5').click();
	await sq(page, 'e7').click(); // Ne7+ forks the king and the queen
	await expect(status).toHaveText('Your move · 1 move left', { timeout: 30_000 });
	await sq(page, 'e7').click();
	await sq(page, 'c6').click();
	await expect(status).toHaveText(/^You're \d+ points up on the start: a won game\.$/, {
		timeout: 30_000
	});
	await expect(page.getByRole('link', { name: 'Next: Pin' })).toBeVisible();
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

test('a blunder is pointed out, and an account can ask the coach why', async ({ page }) => {
	// A guest sees the better move, and is invited to sign up for the explanation.
	await page.goto('/lessons/queen-mate');
	const status = page.getByTestId('drill-status');
	await expect(status).toHaveText('Your move · 20 moves left', { timeout: 30_000 });
	await sq(page, 'h2').click();
	await sq(page, 'e5').click(); // Qe5+?? the king takes the queen
	const mistake = page.getByTestId('mistake');
	await expect(mistake).toContainText(/Qe5\+ was a mistake: Stockfish preferred \d+\. \S+\./, {
		timeout: 30_000
	});
	await expect(mistake).toContainText('to ask the coach why');
	await expect(status).toContainText('insufficient material');

	// Signed up, the same blunder gets an explanation (the offline stand-in coach here).
	await page.goto('/signup');
	await page.getByLabel('Username').fill(`drill_${Date.now().toString(36)}`);
	await page.getByLabel('Password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Sign up' }).click();
	await expect(page).toHaveURL('/');
	await page.goto('/lessons/queen-mate');
	await expect(status).toHaveText('Your move · 20 moves left', { timeout: 30_000 });
	await sq(page, 'h2').click();
	await sq(page, 'e5').click();
	await page.getByRole('button', { name: 'Why was that a mistake?' }).click();
	const answer = page.getByTestId('mistake-coach');
	await expect(answer).toContainText('Practice coach');
	await expect(answer).toContainText('after Qe5+');
	// A move it names shows on the board when tapped (phones have no hover).
	await expect(page.locator('.board .arrows line')).toHaveCount(0);
	await answer.locator('.move-ref', { hasText: 'Qe5+' }).click();
	await expect(page.locator('.board .arrows line')).toHaveCount(1);

	// Restarting clears it.
	await page.getByRole('button', { name: 'Restart' }).click();
	await expect(mistake).toHaveCount(0);
});

test("a guest's progress carries into their account and follows it to another device", async ({
	page,
	browser
}) => {
	// A guest finishes the first drill: kept in this browser.
	await page.goto('/lessons/back-rank-mate');
	const status = page.getByTestId('drill-status');
	await expect(status).toHaveText('Your move · 1 move left');
	await sq(page, 'a1').click();
	await sq(page, 'a8').click();
	await expect(status).toHaveText('Checkmate!');
	await page.goto('/lessons');
	await expect(page.getByTestId('lessons-done')).toHaveText('1 of 18 done');
	await expect(page.getByText('Kept in this browser.')).toBeVisible();

	// They sign up, and it is the account's now.
	const username = `lsn_${Date.now().toString(36)}`;
	await page.locator('main').getByRole('link', { name: 'Sign up' }).click();
	await expect(page).toHaveURL(/\/signup\?next=%2Flessons/);
	await page.getByLabel('Username').fill(username);
	await page.getByLabel('Password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Sign up' }).click();
	await expect(page).toHaveURL('/lessons');
	await expect(page.getByTestId('lessons-done')).toHaveText('1 of 18 done');
	await expect(page.getByTestId('lesson-back-rank-mate')).toContainText('Done');
	await expect(page.getByText('Kept in this browser.')).toHaveCount(0);
	await expect
		.poll(() => page.evaluate(() => localStorage.getItem('chess.lessons.done')))
		.toBeNull();

	// Another device: nothing in its browser, so the progress comes from the account.
	const other = await browser.newContext();
	const phone = await other.newPage();
	await phone.goto('/lessons');
	await expect(phone.getByTestId('lessons-done')).toHaveText('0 of 18 done');
	await phone.goto('/login');
	await phone.getByLabel('Username').fill(username);
	await phone.getByLabel('Password').fill('correct horse battery');
	await phone.getByRole('button', { name: 'Log in' }).click();
	await expect(phone).toHaveURL('/');
	await phone.goto('/lessons');
	await expect(phone.getByTestId('lessons-done')).toHaveText('1 of 18 done');
	await expect(phone.getByTestId('lesson-back-rank-mate')).toContainText('Done');
	await other.close();
});

function sq(page: Page, name: string) {
	return page.locator(`[data-square="${name}"]`);
}
