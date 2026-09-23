// The signed-in account's sign-in methods (`/account`): the connected
// provider identities, the password, and which providers could still be
// connected. Connecting is the ordinary provider flow started while signed in
// (`startUrl(provider, '/account')`); the server links instead of creating.
import type { Account } from '$lib/generated/Account';
import type { LinkedIdentity } from '$lib/generated/LinkedIdentity';
import type { PasswordChange } from '$lib/generated/PasswordChange';
import type { ProviderInfo } from '$lib/generated/ProviderInfo';
import { refusal } from './refusal';

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

const defaultFetch: FetchLike = (input, init) => fetch(input, init);

export async function loadAccount(fetchImpl: FetchLike = defaultFetch): Promise<Account> {
	const response = await fetchImpl('/api/me/account');
	if (!response.ok) throw await refusal(response);
	return (await response.json()) as Account;
}

/** Refused (409, with the reason) when it is the account's last way in. */
export async function disconnect(
	identity: LinkedIdentity,
	fetchImpl: FetchLike = defaultFetch
): Promise<void> {
	const path = `/api/me/identities/${encodeURIComponent(identity.provider)}/${encodeURIComponent(identity.subject)}`;
	const response = await fetchImpl(path, { method: 'DELETE' });
	if (!response.ok) throw await refusal(response);
}

/**
 * Set a first password, or change it (then `current` is required). A first
 * password needs a recent sign-in: an older session is refused with
 * `signInAgain` naming the provider to go back through.
 */
export async function setPassword(
	change: PasswordChange,
	fetchImpl: FetchLike = defaultFetch
): Promise<void> {
	const response = await fetchImpl('/api/me/password', {
		method: 'PUT',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(change)
	});
	if (!response.ok) throw await refusal(response);
}

/** The providers the server offers that this account has no identity with. */
export function connectable(providers: ProviderInfo[], account: Account): ProviderInfo[] {
	const have = new Set(account.identities.map((identity) => identity.provider));
	return providers.filter((provider) => !have.has(provider.id));
}

/** "Lichess — dillon", or just "Lichess" before the label is known. */
export function identityText(identity: LinkedIdentity): string {
	return identity.label ? `${identity.provider_name} — ${identity.label}` : identity.provider_name;
}
