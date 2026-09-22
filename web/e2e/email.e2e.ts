import { expect, test } from '@playwright/test';
import { nthMail } from './mail';

// Email and password reset, end to end, reading the links from the fake
// mailer's outbox: add an address on the account page, verify it, forget the
// password, reset it by the emailed link, and find the other device signed out.
// The account starts at a provider and sets a password later (the case the
// issue calls out), which also keeps this test off the signup rate limit the
// rest of the suite already leans on.
test('verify an email, then reset a forgotten password with it', async ({ page, browser }) => {
	const stamp = Date.now().toString(36);
	const name = `kay_${stamp}`;
	const email = `kay_${stamp}@example.com`;
	await page.goto('/login');
	await page.getByRole('link', { name: 'Continue with Fake provider' }).click();
	await page.getByLabel('Sign in as').fill(name);
	await page.getByRole('button', { name: 'Continue' }).click();
	await page.locator('#navbar').getByRole('link', { name }).click();
	await expect(page).toHaveURL('/account');
	await page.getByLabel('New password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Set password' }).click();
	await expect(page.getByTestId('password-state')).toHaveText('Set');

	// No address yet: the account page says there is no way back in.
	const state = page.getByTestId('email-state');
	await expect(state).toHaveText('None');
	await expect(page.getByTestId('no-recovery')).toContainText('can’t be recovered');

	await page.getByLabel('Email address').fill(email);
	await page.getByRole('button', { name: 'Add email' }).click();
	await expect(
		page.getByRole('status').filter({ hasText: `We sent a link to ${email}` })
	).toBeVisible();
	await expect(state).toHaveText('Not verified yet');

	// Following the link verifies it.
	const verification = await nthMail(email, 1);
	expect(verification.text).toContain('Subject: Confirm your email address');
	await page.goto(verification.link);
	await expect(page.getByRole('status')).toContainText(`${email} is verified for ${name}`);
	await page.goto('/account');
	await expect(state).toHaveText('Verified');
	await expect(page.getByTestId('no-recovery')).toHaveCount(0);

	// Signed in on another device too.
	const other = await browser.newContext();
	const otherPage = await other.newPage();
	const login = await otherPage.request.post('/api/auth/login', {
		data: { username: name, password: 'correct horse battery' }
	});
	expect(login.ok()).toBe(true);
	expect((await otherPage.request.get('/api/me')).status()).toBe(200);

	// Forgotten: log out and ask for a link.
	await page.getByRole('button', { name: 'Log out' }).click();
	await expect(page.locator('#navbar').getByRole('link', { name: 'Log in' })).toBeVisible();
	await page.goto('/login');
	await page.getByRole('link', { name: 'Forgot your password?' }).click();
	await expect(page).toHaveURL('/forgot-password');
	await page.getByLabel('Email address').fill(email);
	await page.getByRole('button', { name: 'Send a reset link' }).click();
	await expect(page.getByRole('status')).toContainText(`If ${email} is the verified address`);

	const reset = await nthMail(email, 2);
	expect(reset.text).toContain('Subject: Reset your password');
	await page.goto(reset.link);
	await page.getByLabel('New password').fill('a brand new horse');
	await page.getByRole('button', { name: 'Set new password' }).click();
	await expect(page.getByRole('status')).toContainText('Every device was signed out');

	// The other device is out, and the link is spent.
	expect((await otherPage.request.get('/api/me')).status()).toBe(401);
	await other.close();
	await page.goto(reset.link);
	await page.getByLabel('New password').fill('yet another horse');
	await page.getByRole('button', { name: 'Set new password' }).click();
	await expect(page.getByRole('alert')).toContainText('expired or was already used');

	// Only the new password works.
	await page.goto('/login');
	await page.getByLabel('Username').fill(name);
	await page.getByLabel('Password').fill('correct horse battery');
	await page.getByRole('button', { name: 'Log in' }).click();
	await expect(page.getByRole('alert')).toContainText('wrong username or password');
	await page.getByLabel('Password').fill('a brand new horse');
	await page.getByRole('button', { name: 'Log in' }).click();
	await expect(page.locator('#navbar').getByRole('link', { name })).toBeVisible();
});
