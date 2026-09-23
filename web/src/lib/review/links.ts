// Links in and out of a game review. Pure, so they get unit tests.
import type { ResolvedPathname } from '$app/types';
import type { Side } from '$lib/generated/Side';

/** `path` with `params` as its query string. */
export function withQuery(
	path: ResolvedPathname,
	params: Record<string, string>
): ResolvedPathname {
	const query = new URLSearchParams(params).toString();
	return (query ? `${path}?${query}` : path) as ResolvedPathname;
}

/** A `?side=` value, or `null` if it doesn't name one. */
export function parseSide(value: string | null): Side | null {
	return value === 'white' || value === 'black' ? value : null;
}

/** A `?ply=` value: a whole number of half-moves, or `null`. */
export function parsePly(value: string | null): number | null {
	if (value === null || !/^\d+$/.test(value)) return null;
	return Number(value);
}
