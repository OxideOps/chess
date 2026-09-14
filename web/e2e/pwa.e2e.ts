import { expect, test } from '@playwright/test';

test('the app is installable: manifest, icons, theme colour', async ({ page, request }) => {
	await page.goto('/');
	const href = await page.locator('link[rel="manifest"]').getAttribute('href');
	const response = await request.get(new URL(href!, page.url()).toString());
	expect(response.headers()['content-type']).toContain('application/manifest+json');
	const manifest = await response.json();
	expect(manifest).toMatchObject({ name: 'Chess', display: 'standalone', start_url: '/' });
	const purposes = manifest.icons.map((i: { purpose: string }) => i.purpose);
	expect(purposes).toContain('maskable');
	for (const icon of manifest.icons) {
		expect((await request.get(icon.src)).status(), icon.src).toBe(200);
	}
	await expect(page.locator('meta[name="theme-color"]')).toHaveAttribute('content', '#262421');
});

// The service worker caches the shell on install and the engine build the page
// runs; after that, the app works without a network.
test('after one visit, local play and analysis work offline', async ({ page, context }) => {
	await page.goto('/analysis');
	await page.evaluate(() => navigator.serviceWorker.ready);
	await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
	await expect(page.locator('.engine .summary')).toHaveText(/Depth \d+/, { timeout: 30_000 });
	// The page asked the worker to keep its engine build; wait until it has.
	await expect
		.poll(
			() =>
				page.evaluate(async () => {
					const keys = await (await caches.open('engine')).keys();
					return keys.map((r) => new URL(r.url).pathname.split('/').pop()).sort();
				}),
			{ timeout: 30_000 }
		)
		.toEqual(['stockfish-18-lite.js', 'stockfish-18-lite.wasm']);

	await context.setOffline(true);
	await page.reload();
	await expect(page.locator('[data-square]')).toHaveCount(64);
	await expect(page.locator('.engine .summary')).toHaveText(/Depth \d+/, { timeout: 30_000 });
	// Served from the cache with the isolation headers intact: still multi-threaded.
	expect(await page.evaluate(() => crossOriginIsolated)).toBe(true);

	await page.goto('/');
	await page.locator('[data-square="e2"]').click();
	await page.locator('[data-square="e4"]').click();
	await expect(page.locator('.move-list button.move')).toHaveText(['e4']);

	// A game link still opens (from the fallback shell) and says what's wrong.
	await page.goto('/game/offline-test');
	await expect(page.getByText('Connection lost, reconnecting…')).toBeVisible();
});
