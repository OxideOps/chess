import { spawn, type ChildProcess } from 'node:child_process';
import { appendFileSync, cpSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { expect, test } from '@playwright/test';

// A deploy, simulated: a second server on a copy of the build, whose
// service worker script then changes. The page must offer the new version
// and switch to it only when asked. (A copy, so the suite's own server and
// the other tests never see the change.)

const PORT = 4175;
const base = `http://127.0.0.1:${PORT}`;
let dir: string;
let server: ChildProcess;

test.beforeAll(async () => {
	dir = mkdtempSync(path.join(tmpdir(), 'chess-update-'));
	cpSync('build', dir, { recursive: true });
	const binary = path.resolve('..', 'target', 'debug', 'chess-server');
	// No database: this server only needs to serve the files.
	const env = { ...process.env };
	delete env.DATABASE_URL;
	server = spawn(binary, ['--static-dir', dir, '--bind', `127.0.0.1:${PORT}`], {
		stdio: ['ignore', 'ignore', 'inherit'],
		env
	});
	for (let i = 0; i < 100; i++) {
		try {
			if ((await fetch(`${base}/healthz`)).ok) return;
		} catch {
			// not up yet
		}
		await new Promise((r) => setTimeout(r, 100));
	}
	throw new Error('the second server did not start');
});

test.afterAll(() => {
	server?.kill();
	rmSync(dir, { recursive: true, force: true });
});

test('a new version is offered, and applied only on request', async ({ page }) => {
	await page.goto(`${base}/analysis`);
	await page.evaluate(() => navigator.serviceWorker.ready);
	await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
	const banner = page.getByText('A new version of the site is available.');
	await expect(banner).toHaveCount(0);

	// "Deploy": the worker script changes on the server. Browsers compare it
	// byte for byte, so any change is a new version.
	// The precompressed copies would still serve the old bytes, so drop them.
	rmSync(path.join(dir, 'service-worker.js.br'), { force: true });
	rmSync(path.join(dir, 'service-worker.js.gz'), { force: true });
	appendFileSync(path.join(dir, 'service-worker.js'), '\n// deployed again\n');
	// The page checks when it becomes visible again and every half hour; ask now.
	await page.evaluate(async () => (await navigator.serviceWorker.getRegistration())?.update());
	await expect(banner).toBeVisible({ timeout: 15_000 });

	// Nothing reloads by itself.
	await page.evaluate(() => ((window as unknown as { marker: number }).marker = 1));
	await page.waitForTimeout(500);
	expect(await page.evaluate(() => (window as unknown as { marker?: number }).marker)).toBe(1);

	// On request, the new worker takes over and the page reloads into it.
	await page.getByRole('button', { name: 'Reload' }).click();
	await page.waitForFunction(() => (window as unknown as { marker?: number }).marker === undefined);
	await expect(page.locator('[data-square]')).toHaveCount(64);
	await expect(banner).toHaveCount(0);
	expect(
		await page.evaluate(async () => (await navigator.serviceWorker.getRegistration())?.waiting)
	).toBeFalsy();
});
