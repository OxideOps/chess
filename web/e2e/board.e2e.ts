import { expect, test } from '@playwright/test';

// The smoke test from the run-app skill, against the real build.
test('plays e4 e5 Nf3 and steps back with the keyboard', async ({ page }) => {
	await page.goto('/');
	const sq = (name: string) => page.locator(`[data-square="${name}"]`);

	await sq('e2').click();
	await sq('e4').click();
	await sq('e7').click();
	await sq('e5').click();
	await sq('g1').click();
	await sq('f3').click();
	await expect(page.getByTestId('movetext')).toHaveText('1. e4 e5 2. Nf3');
	await expect(page.locator('.status')).toHaveText('Black to move');

	await page.locator('.board').focus();
	await page.keyboard.press('ArrowLeft');
	await expect(page.locator('.status')).toHaveText('White to move');
	await page.keyboard.press('ArrowDown');
	await expect(page.locator('.status')).toHaveText('Black to move');
});
