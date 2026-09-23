// Which lessons this browser has completed. A per-viewer convenience kept in
// localStorage, so it can be missing (private windows, cleared data) and
// everything still works.
//
// For a guest this is their progress. For an account it is only an outbox:
// what hasn't reached the server yet (a failed write, or what they finished
// as a guest before signing in), merged in and forgotten by
// `LessonProgress.load` in `progress.svelte.ts`.
const KEY = 'chess.lessons.done';

export function completed(): Set<string> {
	try {
		const raw = localStorage.getItem(KEY);
		return new Set(raw ? (JSON.parse(raw) as string[]) : []);
	} catch {
		return new Set();
	}
}

export function markCompleted(id: string): void {
	try {
		const done = completed();
		done.add(id);
		localStorage.setItem(KEY, JSON.stringify([...done]));
	} catch {
		// storage unavailable: nothing to remember it in
	}
}

/** Drop `ids` (now kept by the account); anything added meanwhile stays. */
export function forgetCompleted(ids: Iterable<string>): void {
	try {
		const done = completed();
		for (const id of ids) done.delete(id);
		if (done.size === 0) localStorage.removeItem(KEY);
		else localStorage.setItem(KEY, JSON.stringify([...done]));
	} catch {
		// storage unavailable: nothing to forget
	}
}
