import { expect, test, type Page } from '@playwright/test';

// Runs in the `phone` project: a Pixel 7 sized touch screen.

/** Nothing sticks out sideways, and the board fills the width less the gutters. */
async function fitsTheScreen(page: Page) {
	// Pages render once the WASM has loaded.
	await expect(page.locator('.board')).toBeVisible();
	await expect(page.locator('.sidebar')).toBeVisible();
	const m = await page.evaluate(() => {
		const board = document.querySelector('.board')!.getBoundingClientRect();
		const sidebar = document.querySelector('.sidebar')!.getBoundingClientRect();
		return {
			width: innerWidth,
			scrollWidth: document.documentElement.scrollWidth,
			boardLeft: board.left,
			boardRight: board.right,
			boardBottom: board.bottom,
			sidebarTop: sidebar.top,
			sidebarWidth: sidebar.width
		};
	});
	expect(m.scrollWidth).toBeLessThanOrEqual(m.width);
	expect(m.boardLeft).toBeGreaterThanOrEqual(0);
	expect(m.boardRight).toBeLessThanOrEqual(m.width);
	// Stacked: the panels come after the board and use the full width.
	expect(m.sidebarTop).toBeGreaterThan(m.boardBottom);
	expect(m.sidebarWidth).toBeGreaterThan(m.width * 0.9);
	return m;
}

test('the play page stacks, fills the width, and plays by tapping', async ({ page }) => {
	await page.goto('/');
	const m = await fitsTheScreen(page);
	expect(m.boardRight - m.boardLeft).toBeGreaterThan(m.width * 0.9);

	await page.locator('[data-square="e2"]').tap();
	await page.locator('[data-square="e4"]').tap();
	await expect(page.locator('.move-list button.move')).toHaveText(['e4']);

	// Buttons are big enough to hit with a thumb.
	const heights = await page
		.locator('.history button, .actions .btn')
		.evaluateAll((buttons) => buttons.map((b) => b.getBoundingClientRect().height));
	expect(heights.length).toBeGreaterThan(0);
	for (const h of heights) expect(h).toBeGreaterThanOrEqual(44);
});

test('the analysis page fits, with long engine lines truncated', async ({ page }) => {
	await page.goto('/analysis');
	await expect(page.locator('.engine .lines li')).toHaveCount(3, { timeout: 30_000 });
	await fitsTheScreen(page);
});

test('the nav fits and the game page keeps both clocks with the board', async ({ page }) => {
	await page.goto('/online');
	const nav = await page.locator('#navbar').boundingBox();
	const width = page.viewportSize()!.width;
	expect(nav!.width).toBeLessThanOrEqual(width);
	for (const name of ['Play', 'Online', 'Analysis', 'Log in', 'Sign up']) {
		const box = await page.getByRole('link', { name, exact: true }).boundingBox();
		expect(box!.x + box!.width, name).toBeLessThanOrEqual(width);
	}

	await page.getByRole('button', { name: 'Create game' }).tap();
	await expect(page).toHaveURL(/\/game\/[0-9a-f-]+$/);
	await fitsTheScreen(page);
	const board = (await page.locator('.board').boundingBox())!;
	for (const side of ['White', 'Black']) {
		const clock = (await page.getByLabel(`${side} clock`).boundingBox())!;
		expect(Math.round(clock.width)).toBe(Math.round(board.width));
	}
});

// Real touch input (Chromium's DevTools protocol), so this checks what a
// synthetic event can't: a finger on a piece drags it instead of scrolling.
test('a finger drags a piece without scrolling the page', async ({ page }) => {
	await page.goto('/');
	await expect(page.locator('.board')).toBeVisible();
	const cdp = await page.context().newCDPSession(page);
	const centre = async (name: string) => {
		const box = (await page.locator(`[data-square="${name}"]`).boundingBox())!;
		return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
	};
	const touch = (type: string, points: { x: number; y: number }[]) =>
		cdp.send('Input.dispatchTouchEvent', { type, touchPoints: points });

	const from = await centre('e2');
	const to = await centre('e4');
	const scrollBefore = await page.evaluate(() => scrollY);
	await touch('touchStart', [from]);
	for (let i = 1; i <= 8; i++) {
		await touch('touchMove', [
			{ x: from.x + ((to.x - from.x) * i) / 8, y: from.y + ((to.y - from.y) * i) / 8 }
		]);
	}
	await expect(page.locator('.board .held')).toBeVisible();
	await touch('touchEnd', []);

	await expect(page.locator('.move-list button.move')).toHaveText(['e4']);
	expect(await page.evaluate(() => scrollY)).toBe(scrollBefore);
});
