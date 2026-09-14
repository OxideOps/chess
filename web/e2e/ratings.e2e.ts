import { expect, test, type Page } from '@playwright/test';

// Two accounts play a rated game; a guest can't take the seat; ratings move
// and show on the clocks, in the games list and on the profile page.
test('a rated game between two accounts moves both ratings', async ({ browser }) => {
	const stamp = Date.now().toString(36);
	const alice = `ra_${stamp}`;
	const bob = `rb_${stamp}`;

	const a = await (await browser.newContext()).newPage();
	await signUp(a, alice);
	await a.goto('/online');
	await expect(a.getByLabel('Rated')).toBeChecked();
	await a.getByRole('button', { name: 'Create game' }).click();
	await expect(a).toHaveURL(/\/game\/[0-9a-f-]+$/);
	await expect(a.getByTestId('game-kind')).toHaveText('Rated · Blitz');
	await expect(a.getByLabel('White clock').getByTestId('rating')).toHaveText('1500?');
	const invite = await a.getByTestId('invite-link').inputValue();

	// A guest opening the invite is told to sign in instead of offered the seat.
	const b = await (await browser.newContext()).newPage();
	await b.goto(invite);
	await expect(b.getByText('this is a rated game')).toBeVisible();
	await expect(b.getByRole('button', { name: 'Join as Black' })).toHaveCount(0);
	await b.locator('.invite').getByRole('link', { name: 'sign up' }).click();
	await b.getByLabel('Username').fill(bob);
	await b.getByLabel('Password').fill('correct horse battery');
	await b.getByRole('button', { name: 'Sign up' }).click();
	await expect(b).toHaveURL(invite);
	await b.getByRole('button', { name: 'Join as Black' }).click();
	await expect(a.getByLabel('Black clock')).toContainText(bob);
	await expect(a.getByLabel('Black clock').getByTestId('rating')).toHaveText('1500?');

	await sq(a, 'e2').click();
	await sq(a, 'e4').click();
	await expect(b.getByTestId('game-status')).toHaveText('Your move');
	await sq(b, 'e7').click();
	await sq(b, 'e5').click();
	await expect(a.getByTestId('game-status')).toHaveText('Your move');
	await b.getByRole('button', { name: 'Resign' }).click();
	await expect(a.getByTestId('game-status')).toHaveText('White wins by resignation');

	// Both see the changes on the clocks: up for Alice, down by as much for Bob.
	const up = a.getByLabel('White clock').getByTestId('rating-diff');
	const down = a.getByLabel('Black clock').getByTestId('rating-diff');
	await expect(up).toHaveText(/^\+\d+$/);
	await expect(down).toHaveText(/^−\d+$/);
	const gain = Number((await up.textContent())!.slice(1));
	expect(Number((await down.textContent())!.slice(1))).toBe(gain);
	await expect(b.getByLabel('Black clock').getByTestId('rating-diff')).toHaveText(`−${gain}`);

	// The list shows it, and the name on the clock leads to the profile.
	await a.goto('/games');
	await expect(a.getByTestId('your-diff').first()).toHaveText(`+${gain}`);
	await b.getByLabel('White clock').getByRole('link', { name: alice }).click();
	await expect(b).toHaveURL(`/players/${alice}`);
	await expect(b.getByRole('heading', { level: 1 })).toHaveText(alice);
	await expect(b.getByTestId('rating-blitz')).toContainText(`${1500 + gain}?`);
	await expect(b.getByTestId('rating-blitz')).toContainText('1');
});

test('guests can only create casual games', async ({ page }) => {
	await page.goto('/online');
	await expect(page.getByLabel('Rated')).toBeDisabled();
	await expect(page.getByText('Rated games need an account')).toBeVisible();
	await page.getByRole('button', { name: 'Create game' }).click();
	await expect(page.getByTestId('game-kind')).toHaveText('Casual · Blitz');
	await page.goto('/players/nobody_here_xyz');
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('No such player');
});

async function signUp(page: Page, username: string) {
	await page.goto('/signup');
	await page.getByLabel('Username').fill(username);
	await page.getByLabel('Password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Sign up' }).click();
	await expect(page).toHaveURL('/');
}

function sq(page: Page, name: string) {
	return page.locator(`[data-square="${name}"]`);
}
