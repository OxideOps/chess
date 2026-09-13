import { expect, test } from '@playwright/test';

// Each route must work when opened directly, not only via client-side
// navigation: that is what the prerendered shells are for.
test('play page opens directly', async ({ page }) => {
	await page.goto('/');
	await expect(page.locator('.board')).toBeVisible();
	await expect(page.locator('[data-square]')).toHaveCount(64);
});

test('analysis page opens directly and the nav links back', async ({ page }) => {
	await page.goto('/analysis');
	await expect(page.locator('.engine')).toBeVisible();
	await expect(page.getByTestId('eval-bar')).toBeVisible();
	await page.getByRole('link', { name: 'Play' }).click();
	await expect(page).toHaveURL('/');
	await expect(page.locator('.board')).toBeVisible();
});

test('unknown paths show the 404 page', async ({ page }) => {
	await page.goto('/no/such/page');
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Page not found');
	await expect(page.getByText('There is nothing at /no/such/page.')).toBeVisible();
	await page.getByRole('link', { name: 'Analysis' }).click();
	await expect(page.locator('.engine')).toBeVisible();
});
