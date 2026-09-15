import { expect, test, type Page } from '@playwright/test';

// The server runs with --fake-coach: answers are built from the engine's lines
// without any API, so this tests the whole path except the model itself.

test('guests are invited to sign up; accounts get an explanation', async ({ page }) => {
	await page.goto('/analysis');
	const coach = page.getByRole('region', { name: 'Coach' });
	await expect(coach).toContainText('Sign up');

	await signUp(page, `coach_${Date.now().toString(36)}`);
	await page.goto('/analysis');
	const ask = coach.getByRole('button', { name: 'Explain this position' });
	await expect(ask).toBeEnabled({ timeout: 30_000 });
	await ask.click();
	const text = page.getByTestId('coach-text');
	await expect(text).toContainText('Practice coach');
	// It talks about the engine's best line, in SAN from the starting position.
	await expect(text).toContainText(/best line is 1\. [a-hKQRBN]/);
	const said = await text.textContent();

	// The moves it names are marked: hovering one draws it on the board in
	// place of the engine's arrow; clicking plays its line from here.
	const moves = text.locator('.move-ref');
	await expect(moves.first()).toBeVisible();
	// (The second move is Black's reply, so its arrow can't be the engine's.)
	const arrow = page.locator('.board .arrows line');
	const at = () =>
		arrow.evaluate((l) => ['x1', 'y1', 'x2', 'y2'].map((a) => l.getAttribute(a)).join());
	const engineArrow = await at();
	await moves.nth(1).hover();
	await expect.poll(at).not.toBe(engineArrow);
	const second = (await moves.nth(1).textContent())!;
	await moves.nth(1).click();
	// The line is played up to it; the explanation stays up, with a way back.
	await expect(page.locator('.move-list button.move')).toHaveCount(2);
	await expect(page.locator('.move-list')).toContainText(second);
	await expect(page.getByTestId('coach-away-text')).toHaveText(said!);
	await page.getByRole('button', { name: /Back to the explained position/ }).click();
	await expect(page.getByTestId('coach-text')).toHaveText(said!);

	// Follow-up questions carry the conversation on.
	const box = page.getByLabel('Ask the coach a follow-up question');
	await box.fill('Why that move?');
	await box.press('Enter');
	const followUps = page.getByTestId('coach-follow-up');
	await expect(followUps.first()).toContainText('you asked \u201cWhy that move?\u201d');
	await expect(page.getByText('4 questions left about this answer')).toBeVisible();
	// A move the engine's lines don't start with: Stockfish looks at it first.
	await box.fill('What about Nh3?');
	await box.press('Enter');
	await expect(followUps.nth(1)).toContainText('Stockfish answers it with', { timeout: 30_000 });
	await expect(page.getByText('3 questions left about this answer')).toBeVisible();

	// A new position gets its own explanation; going back shows the old one.
	await page.locator('.engine .lines li button').first().click();
	await expect(page.getByTestId('coach-text')).toHaveCount(0);
	await page.locator('.board').press('ArrowLeft');
	await expect(page.getByTestId('coach-text')).toHaveText(said!);
});

async function signUp(page: Page, username: string) {
	await page.goto('/signup');
	await page.getByLabel('Username').fill(username);
	await page.getByLabel('Password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Sign up' }).click();
	await expect(page).toHaveURL('/');
}
