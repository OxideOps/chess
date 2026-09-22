import { describe, expect, it } from 'vitest';
import {
	changeEmail,
	emailState,
	forgotPassword,
	mailEnabled,
	removeEmail,
	resetPassword,
	verifyEmail
} from './email';
import type { Account } from '$lib/generated/Account';

const account: Account = {
	username: 'kay',
	has_password: true,
	email: null,
	pending_email: null,
	identities: []
};

type Call = [string, RequestInit | undefined];

/** A fetch that records its calls and answers with `status` and `body`. */
function server(status: number, body: string) {
	const calls: Call[] = [];
	const fetchImpl = async (input: string, init?: RequestInit) => {
		calls.push([input, init]);
		return new Response(status === 204 ? null : body, { status });
	};
	return { calls, fetchImpl };
}

describe('email', () => {
	it('knows whether the server can mail, and says no when it can’t be asked', async () => {
		expect(await mailEnabled(server(200, '{"enabled":true}').fetchImpl)).toBe(true);
		expect(await mailEnabled(server(200, '{"enabled":false}').fetchImpl)).toBe(false);
		expect(await mailEnabled(server(404, '').fetchImpl)).toBe(false);
		const down = async () => {
			throw new TypeError('offline');
		};
		expect(await mailEnabled(down)).toBe(false);
	});

	it('adds and removes the address, returning the account', async () => {
		const pending = { ...account, pending_email: 'kay@example.com' };
		const put = server(200, JSON.stringify(pending));
		expect(await changeEmail('kay@example.com', put.fetchImpl)).toEqual(pending);
		expect(put.calls).toEqual([
			[
				'/api/me/email',
				{
					method: 'PUT',
					headers: { 'content-type': 'application/json' },
					body: '{"email":"kay@example.com"}'
				}
			]
		]);
		const del = server(200, JSON.stringify(account));
		expect(await removeEmail(del.fetchImpl)).toEqual(account);
		expect(del.calls[0][1]?.method).toBe('DELETE');
	});

	it('passes a refusal on with its reason', async () => {
		const refuse = server(400, '{"error":"that isn\'t an email address"}');
		await expect(changeEmail('nope', refuse.fetchImpl)).rejects.toMatchObject({
			status: 400,
			message: "that isn't an email address"
		});
		const spent = server(400, '{"error":"that link has expired or was already used"}');
		await expect(
			resetPassword({ token: 't', password: 'new horse!' }, spent.fetchImpl)
		).rejects.toMatchObject({ message: 'that link has expired or was already used' });
	});

	it('asks for a reset and spends links', async () => {
		const accepted = server(202, '');
		await forgotPassword('kay@example.com', accepted.fetchImpl);
		expect(accepted.calls[0][0]).toBe('/api/auth/forgot-password');

		const reset = server(204, '');
		await resetPassword({ token: 'abc', password: 'new horse!' }, reset.fetchImpl);
		expect(reset.calls[0][1]?.body).toBe('{"token":"abc","password":"new horse!"}');

		const verified = server(200, '{"email":"kay@example.com","username":"kay"}');
		expect(await verifyEmail('abc', verified.fetchImpl)).toEqual({
			email: 'kay@example.com',
			username: 'kay'
		});
	});

	it('says whether the address is verified', () => {
		expect(emailState(account)).toBe('None');
		expect(emailState({ ...account, pending_email: 'a@b.co' })).toBe('Not verified yet');
		expect(emailState({ ...account, email: 'a@b.co', pending_email: 'c@d.co' })).toBe('Verified');
	});
});
