import { describe, expect, it } from 'vitest';
import { Installer } from './install.svelte';

function promptEvent(outcome: 'accepted' | 'dismissed') {
	const event = new Event('beforeinstallprompt', { cancelable: true }) as Event & {
		prompt: () => Promise<void>;
		userChoice: Promise<{ outcome: typeof outcome }>;
		prompted: boolean;
	};
	event.prompted = false;
	event.prompt = async () => {
		event.prompted = true;
	};
	event.userChoice = Promise.resolve({ outcome });
	return event;
}

describe('Installer', () => {
	it('keeps the prompt event, shows it once, and forgets it after install', async () => {
		const target = new EventTarget();
		const installer = new Installer(target);
		expect(installer.available).toBe(false);
		expect(await installer.install()).toBe('unavailable');

		const event = promptEvent('accepted');
		target.dispatchEvent(event);
		expect(event.defaultPrevented).toBe(true);
		expect(installer.available).toBe(true);
		expect(await installer.install()).toBe('accepted');
		expect(event.prompted).toBe(true);
		expect(installer.available).toBe(false);
		expect(await installer.install()).toBe('unavailable');

		target.dispatchEvent(promptEvent('dismissed'));
		expect(installer.available).toBe(true);
		target.dispatchEvent(new Event('appinstalled'));
		expect(installer.available).toBe(false);
	});
});
