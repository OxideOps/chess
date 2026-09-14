// Which lessons this browser has completed. A per-viewer convenience kept in
// localStorage, so it can be missing (private windows, cleared data) and
// everything still works.
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
