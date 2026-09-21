import { expect, test, type Page } from '@playwright/test';

// Being told your game is ready when you are not looking at the tab.
//
// Two things about the browser these run in. Notifications need the full
// Chromium rather than the headless shell the rest of the suite uses, which
// denies the permission outright — hence the channel below. And headless
// Chromium calls every tab visible, there being no desktop for one to be
// behind, so `document.visibilityState` is the one thing faked here, through
// `looking()`. Everything else is real: the lobby socket, the service worker,
// the notification it shows, and the title.
//
// Everyone here is a guest. Signing up is rate limited per address and the
// suite is already close to the ceiling, and none of this needs an account.
// That leaves the seek list — which is global, and shared with whatever else
// is running — with no name to search by, so each test posts at a time
// control nothing else in the suite uses and finds its row by that.
test.use({ channel: 'chromium' });

test('a seek taken while you are looking away comes and finds you', async ({ browser }) => {
	// Permission is granted up front: a browser only shows the real prompt
	// to a person, and what is under test is what happens after the answer.
	const context = await browser.newContext({ permissions: ['notifications'] });
	const poster = await context.newPage();
	await poster.addInitScript(fakeVisibility);
	const taker = await (await browser.newContext()).newPage();
	await poster.goto('/online');
	// The service worker is what shows a notification, so wait for one.
	await poster.evaluate(() => navigator.serviceWorker.ready);
	await taker.goto('/online');

	await poster.getByRole('combobox').selectOption({ label: '15+10 Rapid' });
	await poster.getByTestId('post-seek').click();
	await expect(poster.getByTestId('waiting')).toBeVisible();

	// The poster goes and does something else.
	await looking(poster, false);

	const seek = taker.getByTestId('seek').filter({ hasText: '15+10' });
	await expect(seek).toHaveCount(1);
	await seek.click();

	// The tab nobody is looking at says so in its title...
	await expect.poll(() => poster.title()).toBe('(!) Your game is ready · Chess');
	// ...and it is at the board, not still in the lobby.
	await expect(poster).toHaveURL(/\/game\/[0-9a-f-]+$/);
	// ...and the worker is holding a notification that points at the game.
	await expect.poll(() => heldNotifications(poster)).toBe(1);
	const shown = await poster.evaluate(async () => {
		const registration = await navigator.serviceWorker.getRegistration();
		const held = (await registration?.getNotifications({ tag: 'chess-game' })) ?? [];
		return held.map((n) => ({ title: n.title, body: n.body, data: n.data as { url: string } }));
	});
	expect(shown[0].title).toBe('Your game is ready');
	expect(shown[0].body).toBe('A guest took your 15+10 offer.');
	expect(poster.url()).toContain(shown[0].data.url);

	// Coming back puts the title right and takes the notification down.
	await looking(poster, true);
	await expect.poll(() => poster.title()).toBe('Game · Chess');
	await expect.poll(() => heldNotifications(poster)).toBe(0);
});

// Nobody is notified about a tab they are sitting in front of: the page is
// already showing them the news.
test('a seek taken while you are watching says nothing', async ({ browser }) => {
	const context = await browser.newContext({ permissions: ['notifications'] });
	const poster = await context.newPage();
	const taker = await (await browser.newContext()).newPage();
	await poster.goto('/online');
	await poster.evaluate(() => navigator.serviceWorker.ready);
	await taker.goto('/online');

	await poster.getByRole('combobox').selectOption({ label: '1+0 Bullet' });
	await poster.getByTestId('post-seek').click();
	await taker.getByTestId('seek').filter({ hasText: '1+0' }).click();

	await expect(poster).toHaveURL(/\/game\/[0-9a-f-]+$/);
	await expect(poster.getByLabel('White clock')).toContainText(/\w/);
	expect(await poster.title()).toBe('Game · Chess');
	expect(await heldNotifications(poster)).toBe(0);
});

// The other half: a private game you made, sent the link for, and walked
// away from. Nobody should have to sit and watch an empty seat.
test('someone taking the seat in a game you made comes and finds you', async ({ browser }) => {
	const context = await browser.newContext({ permissions: ['notifications'] });
	const creator = await context.newPage();
	await creator.addInitScript(fakeVisibility);
	await creator.goto('/online');
	await creator.evaluate(() => navigator.serviceWorker.ready);
	await creator.getByRole('button', { name: 'Create a private game' }).click();
	await expect(creator).toHaveURL(/\/game\/[0-9a-f-]+$/);
	const invite = await creator.getByTestId('invite-link').inputValue();

	await looking(creator, false);
	const friend = await (await browser.newContext()).newPage();
	await friend.goto(invite);
	await friend.getByRole('button', { name: 'Join as Black' }).click();
	await expect(friend.getByLabel('Black clock')).toContainText(/\w/);

	await expect.poll(() => creator.title()).toBe('(!) Your opponent is here · Chess');
	await expect.poll(() => heldNotifications(creator)).toBe(1);
	const shown = await creator.evaluate(async () => {
		const registration = await navigator.serviceWorker.getRegistration();
		const held = (await registration?.getNotifications({ tag: 'chess-game' })) ?? [];
		return held.map((n) => ({ title: n.title, body: n.body }));
	});
	expect(shown[0]).toEqual({
		title: 'Your opponent is here',
		body: 'A guest joined your game.'
	});

	await looking(creator, true);
	await expect.poll(() => creator.title()).toBe('Game · Chess');
});

/** Makes `document.visibilityState` something the test can set. */
function fakeVisibility() {
	let state: DocumentVisibilityState = 'visible';
	Object.defineProperty(document, 'visibilityState', {
		configurable: true,
		get: () => state
	});
	Object.defineProperty(window, 'setVisibility', {
		configurable: true,
		value: (next: DocumentVisibilityState) => {
			state = next;
			document.dispatchEvent(new Event('visibilitychange'));
		}
	});
}

/** Look at the tab, or away from it. Needs `fakeVisibility` on the page. */
async function looking(page: Page, at: boolean): Promise<void> {
	await page.evaluate((visible) => {
		(window as unknown as { setVisibility(state: DocumentVisibilityState): void }).setVisibility(
			visible ? 'visible' : 'hidden'
		);
	}, at);
}

async function heldNotifications(page: Page): Promise<number> {
	return page.evaluate(async () => {
		const registration = await navigator.serviceWorker.getRegistration();
		return ((await registration?.getNotifications({ tag: 'chess-game' })) ?? []).length;
	});
}
