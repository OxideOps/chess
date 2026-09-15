// Notice when a new version of the site has been deployed, and apply it when
// the user asks.
//
// The service worker caches the app so it loads instantly and offline. After
// a deploy the browser downloads the new worker, but it waits until every
// tab of the site has closed before taking over, so an open page never mixes
// old and new code. Here we notice the waiting worker, offer a reload, and on
// the user's say-so tell it to take over, then reload once it has.

export interface SkipWaiting {
	type: 'skip-waiting';
}

/** The parts of the service worker API this uses; injectable for tests. */
export interface WorkerLike {
	state: string;
	postMessage(message: SkipWaiting): void;
	addEventListener(type: 'statechange', listener: () => void): void;
}

export interface RegistrationLike {
	waiting: WorkerLike | null;
	installing: WorkerLike | null;
	addEventListener(type: 'updatefound', listener: () => void): void;
	update(): Promise<unknown>;
}

export interface ContainerLike {
	/** The worker controlling this page; `null` on a first visit. */
	controller: unknown;
	getRegistration(): Promise<RegistrationLike | undefined>;
	addEventListener(type: 'controllerchange', listener: () => void): void;
}

export interface UpdateOptions {
	container?: ContainerLike | null;
	reload?: () => void;
	/** How often to ask the server for a new version while the page stays open. */
	checkEveryMs?: number;
}

export class Updates {
	/** A new version is downloaded and waiting. */
	available = $state(false);

	#registration: RegistrationLike | null = null;
	#container: ContainerLike | null;
	readonly #reload: () => void;
	readonly #checkEveryMs: number;
	#applying = false;
	#timer: ReturnType<typeof setInterval> | null = null;

	constructor({ container, reload, checkEveryMs = 30 * 60_000 }: UpdateOptions = {}) {
		this.#container =
			container !== undefined
				? container
				: typeof navigator !== 'undefined' && 'serviceWorker' in navigator
					? (navigator.serviceWorker as unknown as ContainerLike)
					: null;
		this.#reload = reload ?? (() => location.reload());
		this.#checkEveryMs = checkEveryMs;
	}

	/** Start watching (once per page). */
	async start(): Promise<void> {
		const container = this.#container;
		if (!container || this.#registration) return;
		const registration = await container.getRegistration();
		if (!registration) return;
		this.#registration = registration;
		// Only the worker we asked to take over reloads the page.
		container.addEventListener('controllerchange', () => {
			if (this.#applying) this.#reload();
		});
		// A first visit has no controller: the "new" worker is just the first one.
		const isUpdate = () => container.controller !== null;
		if (registration.waiting && isUpdate()) this.available = true;
		registration.addEventListener('updatefound', () => {
			const worker = registration.installing;
			worker?.addEventListener('statechange', () => {
				if (worker.state === 'installed' && isUpdate()) this.available = true;
			});
		});
		if (this.#checkEveryMs > 0) {
			this.#timer = setInterval(() => void this.check(), this.#checkEveryMs);
		}
	}

	/** Ask the server now whether there's a new version. */
	async check(): Promise<void> {
		try {
			await this.#registration?.update();
		} catch {
			// Offline, or the server is down: try again next time.
		}
	}

	/** Switch to the new version: the waiting worker takes over, then the page reloads. */
	apply(): void {
		const waiting = this.#registration?.waiting;
		if (!waiting) return;
		this.#applying = true;
		waiting.postMessage({ type: 'skip-waiting' });
	}

	stop(): void {
		if (this.#timer) clearInterval(this.#timer);
		this.#timer = null;
	}
}

export const updates = new Updates();
