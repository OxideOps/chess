import { beforeEach, describe, expect, it } from 'vitest';
import { Notifier, type Notifications, type Screen, type Shown } from './notifier.svelte';
import { TAG } from './alerts';

/** A browser that says yes, no, or nothing, and remembers what was shown. */
class FakeNotifications implements Notifications {
	permission: NotificationPermission = 'default';
	/** What `request()` will answer. */
	answer: NotificationPermission = 'granted';
	asked = 0;
	shown: Shown[] = [];
	cleared: string[] = [];

	async request(): Promise<NotificationPermission> {
		this.asked++;
		this.permission = this.answer;
		return this.answer;
	}

	async show(shown: Shown): Promise<void> {
		this.shown.push(shown);
	}

	async clear(tag: string): Promise<void> {
		this.cleared.push(tag);
	}
}

class FakeScreen implements Screen {
	visible = true;
	titled = 'Play online · Chess';
	#listeners: (() => void)[] = [];

	hidden = () => !this.visible;
	title = () => this.titled;
	setTitle = (title: string) => (this.titled = title);
	onShown = (listener: () => void) => {
		this.#listeners.push(listener);
		return () => (this.#listeners = this.#listeners.filter((l) => l !== listener));
	};

	/** The tab comes back to the front. */
	look() {
		this.visible = true;
		for (const listener of this.#listeners) listener();
	}
}

function setup(permission: NotificationPermission = 'default') {
	const notifications = new FakeNotifications();
	notifications.permission = permission;
	const screen = new FakeScreen();
	const played: number[] = [];
	const store = new Map<string, string>();
	const notifier = new Notifier({
		notifications,
		screen,
		play: () => played.push(1),
		store: {
			getItem: (key) => store.get(key) ?? null,
			setItem: (key, value) => void store.set(key, value)
		}
	});
	return { notifier, notifications, screen, played, store };
}

const gameReady = { kind: 'game-ready', opponent: 'dan', timeControl: '5+0' } as const;

describe('asking for permission', () => {
	it('asks once and remembers the answer', async () => {
		const { notifier, notifications } = setup();
		expect(notifier.canAsk).toBe(true);
		await notifier.ask();
		expect(notifications.asked).toBe(1);
		expect(notifier.permission).toBe('granted');
		await notifier.ask();
		expect(notifications.asked).toBe(1);
	});

	it('does not ask again after a refusal, this visit or the next', async () => {
		const { notifier, notifications, store } = setup();
		notifications.answer = 'default'; // the prompt was waved away
		await notifier.ask();
		expect(notifier.canAsk).toBe(false);
		// A later visit reads the same storage and stays quiet.
		const again = new Notifier({
			notifications,
			screen: new FakeScreen(),
			play: () => {},
			store: {
				getItem: (key) => store.get(key) ?? null,
				setItem: (key, value) => void store.set(key, value)
			}
		});
		expect(again.canAsk).toBe(false);
	});

	it('does not ask someone who has already answered', async () => {
		const { notifier, notifications } = setup('denied');
		expect(notifier.canAsk).toBe(false);
		await notifier.ask();
		expect(notifications.asked).toBe(0);
	});
});

describe('raising an alert', () => {
	let kit: ReturnType<typeof setup>;
	beforeEach(() => (kit = setup('granted')));

	it('notifies a hidden tab, with somewhere to click through to', async () => {
		const { notifier, notifications, screen, played } = kit;
		screen.visible = false;
		await notifier.raise(gameReady, '/game/g7');
		expect(notifications.shown).toEqual([
			{ title: 'Your game is ready', body: 'dan took your 5+0 offer.', tag: TAG, url: '/game/g7' }
		]);
		expect(screen.titled).toBe('(!) Your game is ready · Chess');
		expect(played).toHaveLength(1);
	});

	it('leaves a tab that is on screen alone, but still makes a sound', async () => {
		const { notifier, notifications, screen, played } = kit;
		await notifier.raise(gameReady, '/game/g7');
		expect(notifications.shown).toEqual([]);
		expect(screen.titled).toBe('Play online · Chess');
		expect(played).toHaveLength(1);
	});

	it('flags the title even when notifications were refused', async () => {
		const { notifier, notifications, screen } = setup('denied');
		screen.visible = false;
		await notifier.raise(gameReady, '/game/g7');
		expect(notifications.shown).toEqual([]);
		expect(screen.titled).toBe('(!) Your game is ready · Chess');
	});

	it('puts the title back and takes the notification down when the tab returns', async () => {
		const { notifier, notifications, screen } = kit;
		screen.visible = false;
		await notifier.raise(gameReady, '/game/g7');
		screen.look();
		expect(screen.titled).toBe('Play online · Chess');
		expect(notifications.cleared).toEqual([TAG]);
	});

	it('does not fight a page that has set its own title since', async () => {
		const { notifier, screen } = kit;
		screen.visible = false;
		await notifier.raise(gameReady, '/game/g7');
		screen.titled = 'Game · Chess';
		screen.look();
		expect(screen.titled).toBe('Game · Chess');
	});

	it('survives a browser that refuses to show it', async () => {
		const { notifier, notifications, screen } = kit;
		notifications.show = () => Promise.reject(new Error('no worker'));
		screen.visible = false;
		await expect(notifier.raise(gameReady, '/game/g7')).resolves.toBeUndefined();
		expect(screen.titled).toBe('(!) Your game is ready · Chess');
	});
});
