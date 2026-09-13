import { expect, test } from '@playwright/test';

// Each route must work when opened directly, not only via client-side
// navigation: that is what the prerendered shells are for.
test('play page opens directly', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Play');
});

test('analysis page opens directly and the nav links back', async ({ page }) => {
	await page.goto('/analysis');
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Analysis');
	await page.getByRole('link', { name: 'Play' }).click();
	await expect(page).toHaveURL('/');
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Play');
});
