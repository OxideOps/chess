// How the account endpoints say no: a status and `{ "error": "..." }`.

/** A refusal from the server, with its message (`{ "error": "..." }`). */
export class AuthError extends Error {
	constructor(
		message: string,
		readonly status: number
	) {
		super(message);
		this.name = 'AuthError';
	}
}

/** The server's reason, or its status when it gave none. */
export async function refusal(response: Response): Promise<AuthError> {
	let message = `the server said ${response.status}`;
	try {
		const body = (await response.json()) as { error?: string };
		if (typeof body.error === 'string') message = body.error;
	} catch {
		// not JSON; keep the status text
	}
	return new AuthError(message, response.status);
}
