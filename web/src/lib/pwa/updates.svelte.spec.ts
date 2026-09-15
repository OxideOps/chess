import { describe, expect, it } from 'vitest';
import {
	Updates,
	type ContainerLike,
	type RegistrationLike,
	type WorkerLike
} from './updates.svelte';

class FakeWorker implements WorkerLike {
	sent: unknown[] = [];
	#listeners: (() => void)[] = [];
	constructor(public state = 'installing') {}
	postMessage(message: unknown) {
		this.sent.push(message);
	}
	addEventListener(_type: 'statechange', listener: () => void) {
		this.#listeners.push(listener);
	}
	become(state: string) {
		this.state = state;
		for (const l of this.#listeners) l();
	}
}

class FakeRegistration implements RegistrationLike {
	waiting: FakeWorker | null = null;
	installing: FakeWorker | null = null;
	updates = 0;
	#found: (() => void)[] = [];
	addEventListener(_type: 'updatefound', listener: () => void) {
		this.#found.push(listener);
	}
	async update() {
		this.updates++;
	}
	/** The browser found a new worker script and starts installing it. */
	found(worker: FakeWorker) {
		this.installing = worker;
		for (const l of this.#found) l();
	}
}

function container(registration: FakeRegistration, controller: unknown = {}) {
	const listeners: (() => void)[] = [];
	const c: ContainerLike & { takeOver: () => void } = {
		controller,
		getRegistration: async () => registration,
		addEventListener: (_type, listener) => void listeners.push(listener),
		takeOver: () => listeners.forEach((l) => l())
	};
	return c;
}

describe('Updates', () => {
	it('offers a new version once it is installed, and reloads after it takes over', async () => {
		const registration = new FakeRegistration();
		const c = container(registration);
		let reloads = 0;
		const updates = new Updates({ container: c, reload: () => reloads++, checkEveryMs: 0 });
		await updates.start();
		expect(updates.available).toBe(false);

		const worker = new FakeWorker();
		registration.found(worker);
		expect(updates.available).toBe(false); // still installing
		worker.become('installed');
		registration.waiting = worker;
		expect(updates.available).toBe(true);

		updates.apply();
		expect(worker.sent).toEqual([{ type: 'skip-waiting' }]);
		expect(reloads).toBe(0);
		c.takeOver();
		expect(reloads).toBe(1);
	});

	it('notices a version already waiting, but not the very first install', async () => {
		const registration = new FakeRegistration();
		registration.waiting = new FakeWorker('installed');
		const updates = new Updates({ container: container(registration), checkEveryMs: 0 });
		await updates.start();
		expect(updates.available).toBe(true);

		// First visit: no controller, so an installed worker isn't an update.
		const fresh = new FakeRegistration();
		const first = new Updates({ container: container(fresh, null), checkEveryMs: 0 });
		await first.start();
		const worker = new FakeWorker();
		fresh.found(worker);
		worker.become('installed');
		expect(first.available).toBe(false);
	});

	it("doesn't reload on a takeover it didn't ask for, and checks on request", async () => {
		const registration = new FakeRegistration();
		const c = container(registration);
		let reloads = 0;
		const updates = new Updates({ container: c, reload: () => reloads++, checkEveryMs: 0 });
		await updates.start();
		c.takeOver(); // e.g. another tab applied the update
		expect(reloads).toBe(0);
		await updates.check();
		expect(registration.updates).toBe(1);
		updates.apply(); // nothing waiting: nothing happens
		expect(reloads).toBe(0);
		// Without service workers at all, nothing breaks.
		const none = new Updates({ container: null });
		await none.start();
		await none.check();
		none.apply();
		expect(none.available).toBe(false);
	});
});
