// Make sure the browser has a session before doing anything that needs one.
// Guests are created on demand so nobody has to sign up to play.
import type { User } from '$lib/generated/User';

export async function ensureSession(): Promise<User> {
	const response = await fetch('/api/auth/guest', { method: 'POST' });
	if (!response.ok) throw new Error(`could not start a session (${response.status})`);
	return (await response.json()) as User;
}
