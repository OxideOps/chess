import { expect, test, type Page } from '@playwright/test';

// The account page: connect a provider to an account that has a password,
// then sign in with that provider and land on the same account. The fake
// provider plays both roles.
test('connect a provider on the account page, then sign in with it as the same player', async ({
	page
}) => {
	const stamp = Date.now().toString(36);
	const name = `ivy_${stamp}`;
	const elsewhere = `IvyElsewhere_${stamp}`;
	await page.goto('/signup');
	await page.getByLabel('Username').fill(name);
	await page.getByLabel('Password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Sign up' }).click();
	await expect(page).toHaveURL('/');
	const before = await whoAmI(page);

	await page.locator('#navbar').getByRole('link', { name }).click();
	await expect(page).toHaveURL('/account');
	const methods = page.getByTestId('sign-in-methods');
	await expect(page.getByTestId('password-state')).toHaveText('Set');
	await page.getByRole('link', { name: 'Connect Fake provider' }).click();
	await continueAtProvider(page, elsewhere);

	// Back on the account page, still the same player, with the new row.
	await expect(page).toHaveURL('/account');
	await expect(methods).toContainText(`Fake provider — ${elsewhere}`);
	await expect(page.getByRole('link', { name: 'Connect Fake provider' })).toHaveCount(0);
	await expect(page.locator('#navbar').getByRole('link', { name })).toBeVisible();

	// Signed out, the provider now signs in as this account, not a new one.
	await page.getByRole('button', { name: 'Log out' }).click();
	await expect(page.locator('#navbar').getByRole('link', { name: 'Log in' })).toBeVisible();
	await page.goto('/login');
	await page.getByRole('link', { name: 'Continue with Fake provider' }).click();
	await continueAtProvider(page, elsewhere);
	await expect(page.locator('#navbar').getByRole('link', { name })).toBeVisible();
	expect(await whoAmI(page)).toEqual(before);
});

test('the last way in cannot be disconnected until there is a password', async ({ page }) => {
	const name = `jo_${Date.now().toString(36)}`;
	await page.goto('/login');
	await page.getByRole('link', { name: 'Continue with Fake provider' }).click();
	await continueAtProvider(page, name);
	await page.locator('#navbar').getByRole('link', { name }).click();
	await expect(page).toHaveURL('/account');

	const methods = page.getByTestId('sign-in-methods');
	await expect(page.getByTestId('password-state')).toHaveText('Not set');
	const disconnect = page.getByRole('button', { name: `Disconnect Fake provider — ${name}` });
	await disconnect.click();
	await expect(page.getByRole('alert')).toContainText('only way into your account');
	await expect(methods).toContainText(`Fake provider — ${name}`);

	// With a password it can go.
	await page.getByLabel('New password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Set password' }).click();
	await expect(page.getByRole('status')).toContainText('Password set');
	await expect(page.getByTestId('password-state')).toHaveText('Set');
	await disconnect.click();
	await expect(methods).not.toContainText('Fake provider');
	await expect(page.getByRole('link', { name: 'Connect Fake provider' })).toBeVisible();

	// Changing it asks for the current one.
	await expect(page.getByRole('heading', { name: 'Change password' })).toBeVisible();
	await page.getByLabel('Current password').fill('not my password');
	await page.getByLabel('New password').fill('another horse battery');
	await page.getByRole('button', { name: 'Change password' }).click();
	await expect(page.getByRole('alert')).toContainText('current password');
});

async function continueAtProvider(page: Page, name: string) {
	await expect(page.getByRole('heading', { name: 'Fake provider' })).toBeVisible();
	await page.getByLabel('Sign in as').fill(name);
	await page.getByRole('button', { name: 'Continue' }).click();
}

async function whoAmI(page: Page): Promise<unknown> {
	const response = await page.request.get('/api/me');
	expect(response.ok()).toBe(true);
	return response.json();
}
