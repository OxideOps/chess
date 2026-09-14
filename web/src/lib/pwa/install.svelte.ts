// "Install app" for browsers that offer it (Chrome, Edge, Android): keep the
// `beforeinstallprompt` event and show it when the user asks. Safari has no
// such event; there it's Share → Add to Home Screen.

/** Not in lib.dom yet. */
interface BeforeInstallPromptEvent extends Event {
	prompt(): Promise<void>;
	userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
}

export class Installer {
	/** The browser will install the app if asked. */
	available = $state(false);
	#event: BeforeInstallPromptEvent | null = null;

	constructor(target: EventTarget | null = typeof window === 'undefined' ? null : window) {
		target?.addEventListener('beforeinstallprompt', (event) => {
			event.preventDefault(); // we show our own button instead of the mini-infobar
			this.#event = event as BeforeInstallPromptEvent;
			this.available = true;
		});
		target?.addEventListener('appinstalled', () => {
			this.#event = null;
			this.available = false;
		});
	}

	/** Show the browser's install dialog. It can only be shown once per event. */
	async install(): Promise<'accepted' | 'dismissed' | 'unavailable'> {
		const event = this.#event;
		if (!event) return 'unavailable';
		this.#event = null;
		this.available = false;
		await event.prompt();
		return (await event.userChoice).outcome;
	}
}

export const installer = new Installer();
