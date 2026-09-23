// An email address on the account, and resetting a forgotten password with it.
// The address only counts once the link mailed to it is followed
// (`/verify-email`); a reset link (`/reset-password`) only goes to such an
// address. None of it exists when the server can't send mail.
import type { Account } from '$lib/generated/Account';
import type { EmailAddress } from '$lib/generated/EmailAddress';
import type { EmailChange } from '$lib/generated/EmailChange';
import type { EmailLink } from '$lib/generated/EmailLink';
import type { EmailVerified } from '$lib/generated/EmailVerified';
import type { MailStatus } from '$lib/generated/MailStatus';
import type { PasswordReset } from '$lib/generated/PasswordReset';
import { refusal } from './refusal';

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

const defaultFetch: FetchLike = (input, init) => fetch(input, init);

/** Whether the server can send email. Never throws: no server, no mail. */
export async function mailEnabled(fetchImpl: FetchLike = defaultFetch): Promise<boolean> {
	try {
		const response = await fetchImpl('/api/auth/mail');
		return response.ok && ((await response.json()) as MailStatus).enabled;
	} catch {
		return false;
	}
}

async function send<T>(
	method: string,
	path: string,
	body: object | null,
	fetchImpl: FetchLike
): Promise<T | null> {
	const response = await fetchImpl(path, {
		method,
		headers: body ? { 'content-type': 'application/json' } : undefined,
		body: body ? JSON.stringify(body) : undefined
	});
	if (!response.ok) throw await refusal(response);
	const text = await response.text();
	return text ? (JSON.parse(text) as T) : null;
}

/**
 * Add or change the address; it is pending until its link is followed. A new
 * address needs `current_password` (when the account has one) or a recent
 * sign-in: an older session is refused with `signInAgain`.
 */
export async function changeEmail(
	change: EmailChange,
	fetchImpl: FetchLike = defaultFetch
): Promise<Account> {
	return (await send<Account>('PUT', '/api/me/email', change, fetchImpl)) as Account;
}

/** Take the address (and any pending one) off the account. */
export async function removeEmail(fetchImpl: FetchLike = defaultFetch): Promise<Account> {
	return (await send<Account>('DELETE', '/api/me/email', null, fetchImpl)) as Account;
}

/** Follow a verification link. */
export async function verifyEmail(
	token: string,
	fetchImpl: FetchLike = defaultFetch
): Promise<EmailVerified> {
	const body: EmailLink = { token };
	return (await send<EmailVerified>(
		'POST',
		'/api/auth/verify-email',
		body,
		fetchImpl
	)) as EmailVerified;
}

/**
 * Ask for a reset link. The server says the same whether or not the address
 * has an account, so this resolves either way.
 */
export async function forgotPassword(
	email: string,
	fetchImpl: FetchLike = defaultFetch
): Promise<void> {
	const body: EmailAddress = { email };
	await send('POST', '/api/auth/forgot-password', body, fetchImpl);
}

/** Spend a reset link on a new password; every session is signed out. */
export async function resetPassword(
	reset: PasswordReset,
	fetchImpl: FetchLike = defaultFetch
): Promise<void> {
	await send('POST', '/api/auth/reset-password', reset, fetchImpl);
}

/** What the account page says about the address. */
export function emailState(account: Account): string {
	if (account.email) return 'Verified';
	if (account.pending_email) return 'Not verified yet';
	return 'None';
}
