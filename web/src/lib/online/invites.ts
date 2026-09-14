// Nothing secret travels in links any more: the game page is the invite.
// Kept as one place to build the link in case that changes again.
import { resolve } from '$app/paths';

export function inviteLink(id: string): string {
	return `${location.origin}${resolve('/game/[id]', { id })}`;
}
