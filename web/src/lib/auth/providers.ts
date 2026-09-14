// The OAuth providers this server is configured with (`GET /api/auth/providers`).
import type { ProviderInfo } from '$lib/generated/ProviderInfo';

export async function listProviders(fetchImpl: typeof fetch = fetch): Promise<ProviderInfo[]> {
	try {
		const response = await fetchImpl('/api/auth/providers');
		return response.ok ? ((await response.json()) as ProviderInfo[]) : [];
	} catch {
		return [];
	}
}

/** Where a "Continue with …" button goes; the server redirects back to `next`. */
export function startUrl(provider: ProviderInfo, next: string): string {
	return `/api/auth/${encodeURIComponent(provider.id)}/start?next=${encodeURIComponent(next)}`;
}
