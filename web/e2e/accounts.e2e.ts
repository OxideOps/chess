import { expect, test, type Page } from '@playwright/test';

// Two people sign up in separate browsers, one creates a game and the other
// joins it; names show on the clocks and the game lands in both lists.
test('sign up, play with names, list games, log out and back in', async ({ browser }) => {
	const stamp = Date.now().toString(36);
	const alice = `alice_${stamp}`;
	const bob = `bob_${stamp}`;

	const a = await (await browser.newContext()).newPage();
	await signUp(a, alice);
	await expect(a.locator('#navbar').getByRole('link', { name: alice })).toBeVisible();
	await expect(a.getByRole('button', { name: 'Log out' })).toBeVisible();

	await a.goto('/online');
	// Accounts default to rated; this test is about a guest joining, which needs casual.
	await expect(a.getByLabel('Rated')).toBeChecked();
	await a.getByLabel('Rated').uncheck();
	await a.getByRole('button', { name: 'Create a private game' }).click();
	await expect(a).toHaveURL(/\/game\/[0-9a-f-]+$/);
	await expect(a.getByLabel('White clock')).toContainText(alice);
	await expect(a.getByLabel('Black clock')).toContainText('Open seat');
	const invite = await a.getByTestId('invite-link').inputValue();

	// Bob opens the invite before signing up: the join box says he'd be a guest.
	const b = await (await browser.newContext()).newPage();
	await b.goto(invite);
	await expect(b.getByText('You would play as a guest.')).toBeVisible();
	await b.getByRole('link', { name: 'Log in' }).first().click();
	await expect(b).toHaveURL(/\/login\?next=%2Fgame%2F/);
	// Both the form's link and the nav's carry the return path along, unnested.
	const gameNext = /\/signup\?next=%2Fgame%2F[0-9a-f-]+$/;
	await expect(b.locator('#navbar').getByRole('link', { name: 'Sign up' })).toHaveAttribute(
		'href',
		gameNext
	);
	await b.getByText('No account yet?').getByRole('link').click();
	await expect(b).toHaveURL(gameNext);
	await b.getByLabel('Username').fill(bob);
	await b.getByLabel('Password').fill('correct horse battery');
	await b.getByRole('button', { name: 'Sign up' }).click();
	// ...and comes back to the game, signed in.
	await expect(b).toHaveURL(invite);
	await expect(b.locator('#navbar').getByRole('link', { name: bob })).toBeVisible();
	await b.getByRole('button', { name: 'Join as Black' }).click();

	await expect(b.getByLabel('Black clock')).toContainText(bob);
	await expect(b.getByLabel('White clock')).toContainText(alice);
	await expect(a.getByLabel('Black clock')).toContainText(bob);

	await sq(a, 'e2').click();
	await sq(a, 'e4').click();
	await expect(b.locator('.move-list button.move')).toHaveText(['e4']);

	// Both lists show the game from their own side.
	await a.locator('#navbar').getByRole('link', { name: alice }).click();
	await expect(a).toHaveURL('/games');
	const aRow = a.getByRole('link', { name: `You (White) vs ${bob}` });
	await expect(aRow).toContainText('In progress');
	await expect(aRow).toContainText('1 ply');
	await b.goto('/games');
	await expect(b.getByRole('link', { name: `You (Black) vs ${alice}` })).toBeVisible();

	// Log out, then back in.
	await b.getByRole('button', { name: 'Log out' }).click();
	await expect(b).toHaveURL('/');
	await expect(b.getByRole('link', { name: 'Log in' })).toBeVisible();
	await b.goto('/games');
	await expect(b.getByText('to see your games')).toBeVisible();
	await b.goto('/login');
	await b.getByLabel('Username').fill(bob);
	await b.getByLabel('Password').fill('wrong password');
	await b.getByRole('button', { name: 'Log in' }).click();
	await expect(b.getByRole('alert')).toContainText('wrong username or password');
	await b.getByLabel('Password').fill('correct horse battery');
	await b.getByRole('button', { name: 'Log in' }).click();
	await expect(b).toHaveURL('/');
	await expect(b.locator('#navbar').getByRole('link', { name: bob })).toBeVisible();

	// The list survives a reload: the session is a cookie, not page state.
	await a.reload();
	await expect(a.getByRole('link', { name: `You (White) vs ${bob}` })).toBeVisible();
});

test('a guest who signs up keeps their games', async ({ page }) => {
	await page.goto('/online');
	await page.getByRole('button', { name: 'Create a private game' }).click();
	await expect(page).toHaveURL(/\/game\/[0-9a-f-]+$/);
	await expect(page.getByRole('link', { name: 'Guest' })).toBeVisible();
	await expect(page.getByLabel('White clock')).toContainText('Guest');

	const name = `carol_${Date.now().toString(36)}`;
	await signUp(page, name);
	await page.goto('/games');
	await expect(page.getByRole('link', { name: 'You (White) vs Open seat' })).toContainText(
		'Waiting for an opponent'
	);
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
