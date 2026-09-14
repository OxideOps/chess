import { expect, test } from '@playwright/test';

// Signing in through a provider, against the server's built-in fake one.
test('continue with a provider from the login page and land back where you were', async ({
	page
}) => {
	const name = `eve_${Date.now().toString(36)}`;
	await page.goto('/online');
	await page.getByRole('button', { name: 'Create game' }).click();
	await expect(page).toHaveURL(/\/game\/[0-9a-f-]+$/);
	const gameUrl = page.url();

	// A guest so far; the join box's link takes them to the login page with a return path.
	await page.getByRole('link', { name: 'Log in' }).first().click();
	await expect(page).toHaveURL(/\/login\?next=%2Fgame%2F/);
	await page.getByRole('link', { name: 'Continue with Fake provider' }).click();
	await expect(page.getByRole('heading', { name: 'Fake provider' })).toBeVisible();
	await page.getByLabel('Sign in as').fill(name);
	await page.getByRole('button', { name: 'Continue' }).click();

	// Back on the game, upgraded in place: same seat, now with a name.
	await expect(page).toHaveURL(gameUrl);
	await expect(page.getByRole('link', { name })).toBeVisible();
	await expect(page.getByLabel('White clock')).toContainText(name);

	// Cancelling at the provider comes back to the login page with the reason.
	await page.getByRole('button', { name: 'Log out' }).click();
	await page.goto('/login');
	await page.getByRole('link', { name: 'Continue with Fake provider' }).click();
	await page.getByRole('button', { name: 'Cancel' }).click();
	await expect(page).toHaveURL(/\/login\?error=/);
	await expect(page.getByRole('alert')).toContainText('you cancelled');
});
