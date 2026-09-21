/**
 * Telling someone their game is ready when they are not looking at the tab.
 *
 * Three signals, loudest last, and the quiet ones always happen:
 *
 * - a sound, the same voice the board uses;
 * - the tab's title, so a glance at a row of tabs finds it — this is the
 *   one that works when notifications were refused, or were never possible;
 * - a system notification, only when the tab is hidden *and* permission was
 *   granted.
 *
 * The notification goes through the service worker rather than
 * `new Notification()`: that constructor does not exist on iOS, where an
 * installed PWA can only be notified by its worker, and one code path that
 * works everywhere beats two that disagree.
 *
 * Nothing here talks to a push server. The page is open and holding the
 * lobby socket, so it already knows; waking a browser with no tab open is a
 * different and much larger feature (Web Push, VAPID keys, subscriptions on
 * the server).
 */
import { sounds } from '$lib/sound/sounds.svelte';
import { TAG, shouldShow, wording, type Alert } from './alerts';

/** Remembers that we have used our one chance to ask. */
const STORED = 'chess:notify-asked';

/** A notification, ready to show. */
export interface Shown {
	title: string;
	body: string;
	tag: string;
	/** Where clicking it should land. */
	url: string;
}

/** The browser's notifications, or a fake in tests. */
export interface Notifications {
	readonly permission: NotificationPermission;
	request(): Promise<NotificationPermission>;
	show(shown: Shown): Promise<void>;
	clear(tag: string): Promise<void>;
}

/** The tab: whether it is on screen, and what its title says. */
export interface Screen {
	hidden(): boolean;
	title(): string;
	setTitle(title: string): void;
	/** Calls back when the tab comes to the front; returns an unsubscribe. */
	onShown(listener: () => void): () => void;
}

export interface NotifierOptions {
	notifications?: Notifications;
	screen?: Screen;
	/** The sound an alert makes. */
	play?: () => void;
	store?: Pick<Storage, 'getItem' | 'setItem'> | null;
}

export class Notifier {
	/** What the browser says now; 'denied' when it has no notifications at all. */
	permission: NotificationPermission = $state('default');
	/** Whether the one prompt has been spent. */
	asked = $state(false);

	readonly #notifications: Notifications;
	readonly #screen: Screen;
	readonly #play: () => void;
	readonly #store: Pick<Storage, 'getItem' | 'setItem'> | null;
	#unsubscribe: (() => void) | null = null;
	/** The title we put up, and the one we took down to do it. */
	#flagged: string | null = null;
	#before: string | null = null;

	constructor({ notifications, screen, play, store }: NotifierOptions = {}) {
		this.#notifications = notifications ?? new BrowserNotifications();
		this.#screen = screen ?? documentScreen();
		this.#play = play ?? (() => sounds.play('ready'));
		this.#store = store !== undefined ? store : safeStorage();
		this.permission = this.#notifications.permission;
		this.asked = asked(this.#store) || this.permission !== 'default';
		this.#unsubscribe = this.#screen.onShown(() => this.clear());
	}

	/** Worth putting a prompt in front of them? */
	get canAsk(): boolean {
		return !this.asked && this.permission === 'default';
	}

	/**
	 * Ask for permission — from a click, which is the only time a browser
	 * will show the prompt, and never a second time. A dismissed prompt
	 * counts as spent: browsers stop showing it to someone who keeps waving
	 * it away, and a site that keeps trying is the reason they do.
	 */
	async ask(): Promise<void> {
		if (!this.canAsk) return;
		this.asked = true;
		try {
			this.#store?.setItem(STORED, 'yes');
		} catch {
			// Storage blocked: we may ask again next visit, not this one.
		}
		this.permission = await this.#notifications.request();
	}

	/**
	 * Say that `alert` happened, with `url` as where it points. The sound
	 * always plays; the title and the notification are for a tab nobody is
	 * looking at.
	 */
	async raise(alert: Alert, url: string): Promise<void> {
		const said = wording(alert);
		this.#play();
		if (!this.#screen.hidden()) return;
		this.#flag(said.short);
		this.permission = this.#notifications.permission;
		if (!shouldShow({ hidden: true, permission: this.permission })) return;
		try {
			await this.#notifications.show({ title: said.title, body: said.body, tag: TAG, url });
		} catch {
			// No worker, or the browser refused: the title still says it.
		}
	}

	/**
	 * Take it all down. Called when the tab comes back to the front: the
	 * news has been delivered, and a notification still sitting there for a
	 * game that started five minutes ago is worse than none.
	 */
	clear(): void {
		if (this.#flagged !== null) {
			// Only if nothing else has set the title since, e.g. a navigation.
			if (this.#screen.title() === this.#flagged && this.#before !== null) {
				this.#screen.setTitle(this.#before);
			}
			this.#flagged = null;
			this.#before = null;
		}
		void this.#notifications.clear(TAG).catch(() => {});
	}

	dispose(): void {
		this.#unsubscribe?.();
		this.#unsubscribe = null;
	}

	/** Put the news in the tab's title, keeping the real one to put back. */
	#flag(short: string): void {
		if (this.#flagged === null) this.#before = this.#screen.title();
		this.#flagged = `(!) ${short} · Chess`;
		this.#screen.setTitle(this.#flagged);
	}
}

/** Notifications shown by the service worker, which is the only way on iOS. */
export class BrowserNotifications implements Notifications {
	get permission(): NotificationPermission {
		return typeof Notification === 'undefined' ? 'denied' : Notification.permission;
	}

	async request(): Promise<NotificationPermission> {
		if (typeof Notification === 'undefined') return 'denied';
		try {
			return await Notification.requestPermission();
		} catch {
			return 'denied';
		}
	}

	async show({ title, body, tag, url }: Shown): Promise<void> {
		const registration = await this.#registration();
		await registration?.showNotification(title, {
			body,
			tag,
			data: { url },
			icon: '/icons/icon-192.png',
			badge: '/icons/icon-192.png'
		});
	}

	async clear(tag: string): Promise<void> {
		const registration = await this.#registration();
		for (const shown of (await registration?.getNotifications({ tag })) ?? []) shown.close();
	}

	async #registration(): Promise<ServiceWorkerRegistration | null> {
		if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return null;
		return (await navigator.serviceWorker.getRegistration()) ?? null;
	}
}

function documentScreen(): Screen {
	return {
		hidden: () => document.visibilityState === 'hidden',
		title: () => document.title,
		setTitle: (title) => (document.title = title),
		onShown: (listener) => {
			const handler = () => {
				if (document.visibilityState === 'visible') listener();
			};
			document.addEventListener('visibilitychange', handler);
			return () => document.removeEventListener('visibilitychange', handler);
		}
	};
}

/** Whether the prompt was spent in an earlier visit. */
function asked(store: Pick<Storage, 'getItem' | 'setItem'> | null): boolean {
	try {
		return store?.getItem(STORED) === 'yes';
	} catch {
		return false;
	}
}

function safeStorage(): Pick<Storage, 'getItem' | 'setItem'> | null {
	try {
		return localStorage;
	} catch {
		return null;
	}
}

export const notifier = new Notifier();
