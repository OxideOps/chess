import { describe, expect, it } from 'vitest';
import { connectable, disconnect, identityText, loadAccount, setPassword } from './account';
import type { Account } from '$lib/generated/Account';

const lichess = {
	provider: 'lichess',
	provider_name: 'Lichess',
	subject: 'dillon',
	label: 'Dillon'
};

const account: Account = {
	username: 'dillon',
	has_password: false,
	email: null,
	pending_email: null,
	identities: [lichess]
};

describe('account', () => {
	it('offers only the providers not yet connected', () => {
		const providers = [
			{ id: 'lichess', name: 'Lichess' },
			{ id: 'google', name: 'Google' }
		];
		expect(connectable(providers, account)).toEqual([{ id: 'google', name: 'Google' }]);
		expect(connectable(providers, { ...account, identities: [] })).toEqual(providers);
	});

	it('names an identity by its provider and label', () => {
		expect(identityText(lichess)).toBe('Lichess — Dillon');
		expect(identityText({ ...lichess, label: null })).toBe('Lichess');
	});

	it('loads the account, and reports the reason when refused', async () => {
		const ok = async () => new Response(JSON.stringify(account));
		expect(await loadAccount(ok)).toEqual(account);
		const guest = async () =>
			new Response(JSON.stringify({ error: 'guests have no account settings; sign up first' }), {
				status: 403
			});
		await expect(loadAccount(guest)).rejects.toMatchObject({
			status: 403,
			message: 'guests have no account settings; sign up first'
		});
	});

	it('disconnects by provider and subject, passing the refusal on', async () => {
		const calls: [string, RequestInit | undefined][] = [];
		const refuse = async (input: string, init?: RequestInit) => {
			calls.push([input, init]);
			return new Response(JSON.stringify({ error: 'this is the only way into your account' }), {
				status: 409
			});
		};
		await expect(disconnect({ ...lichess, subject: 'a/b c' }, refuse)).rejects.toMatchObject({
			status: 409,
			message: 'this is the only way into your account'
		});
		expect(calls).toEqual([['/api/me/identities/lichess/a%2Fb%20c', { method: 'DELETE' }]]);
	});

	it('sends a password change as JSON', async () => {
		let sent: unknown = null;
		const ok = async (_input: string, init?: RequestInit) => {
			sent = JSON.parse(String(init?.body));
			return new Response(null, { status: 204 });
		};
		await setPassword({ current: 'old password', password: 'new password' }, ok);
		expect(sent).toEqual({ current: 'old password', password: 'new password' });
	});

	it('passes on which provider to sign in with again', async () => {
		const stale = async () =>
			new Response(
				JSON.stringify({
					error: 'sign in again with Lichess to set a password',
					sign_in_again: { id: 'lichess', name: 'Lichess' }
				}),
				{ status: 403 }
			);
		await expect(
			setPassword({ current: null, password: 'new password' }, stale)
		).rejects.toMatchObject({
			status: 403,
			message: 'sign in again with Lichess to set a password',
			signInAgain: { id: 'lichess', name: 'Lichess' }
		});
		const wrong = async () =>
			new Response(JSON.stringify({ error: 'your current password is not that' }), {
				status: 403
			});
		await expect(
			setPassword({ current: 'x', password: 'new password' }, wrong)
		).rejects.toMatchObject({ signInAgain: null });
	});
});
