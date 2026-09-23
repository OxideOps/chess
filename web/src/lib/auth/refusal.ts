// How the account endpoints say no: a status and `{ "error": "..." }`, plus
// `sign_in_again` (a provider) when the fix is a fresh sign-in through it.
import type { ProviderInfo } from '$lib/generated/ProviderInfo';

/** A refusal from the server, with its message (`{ "error": "..." }`). */
export class AuthError extends Error {
	constructor(
		message: string,
		readonly status: number,
		/** The provider to sign in with again before retrying, when that is the fix. */
		readonly signInAgain: ProviderInfo | null = null
	) {
		super(message);
		this.name = 'AuthError';
	}
}

/** The server's reason, or its status when it gave none. */
export async function refusal(response: Response): Promise<AuthError> {
	let message = `the server said ${response.status}`;
	let signInAgain: ProviderInfo | null = null;
	try {
		const body = (await response.json()) as { error?: string; sign_in_again?: ProviderInfo };
		if (typeof body.error === 'string') message = body.error;
		const provider = body.sign_in_again;
		if (provider && typeof provider.id === 'string' && typeof provider.name === 'string') {
			signInAgain = { id: provider.id, name: provider.name };
		}
	} catch {
		// not JSON; keep the status text
	}
	return new AuthError(message, response.status, signInAgain);
}
