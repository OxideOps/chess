// The auth pages take `?next=/some/path` to return to where the visitor came
// from. Only same-site paths are honoured, so a crafted link can't send a
// freshly signed-in user off to another origin. The results are typed as
// `ResolvedPathname` because that is what they are: paths on this site, ready
// for `goto()` and `href`.
import type { ResolvedPathname } from '$app/types';

const HOME = '/' as ResolvedPathname;

export function safeNext(next: string | null, fallback: ResolvedPathname = HOME): ResolvedPathname {
	if (next && next.startsWith('/') && !next.startsWith('//') && !next.startsWith('/\\')) {
		return next as ResolvedPathname;
	}
	return fallback;
}

/** `path`, carrying `next` along unless it is just the home page. */
export function withNext(path: ResolvedPathname, next: string): ResolvedPathname {
	if (next === HOME) return path;
	return `${path}?next=${encodeURIComponent(next)}` as ResolvedPathname;
}
