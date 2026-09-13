// The creator of a game holds the opponent's token until it is handed over.
// Session storage keeps it through a reload without leaking it anywhere.
const KEY = (id: string) => `chess:invite:${id}`;

export function rememberInvite(id: string, blackToken: string): void {
	try {
		sessionStorage.setItem(KEY(id), blackToken);
	} catch {
		// Private mode or storage disabled: the link just won't be shown after a reload.
	}
}

export function inviteToken(id: string): string | null {
	try {
		return sessionStorage.getItem(KEY(id));
	} catch {
		return null;
	}
}
