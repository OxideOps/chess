// The daily puzzle's date. `/puzzles/daily` is today's (UTC);
// `/puzzles/daily?date=YYYY-MM-DD` is that day's for good: the server
// remembers each day's pick, so the dated link is the one to share.

/** A `YYYY-MM-DD` string (whether the day exists is the server's call). */
export function isDay(text: string | null): text is string {
	return text !== null && /^\d{4}-\d{2}-\d{2}$/.test(text);
}

/** "22 September 2026" for `2026-09-22`: the day as the server counts it, in UTC. */
export function formatDay(date: string): string {
	return new Date(`${date}T00:00:00Z`).toLocaleDateString('en-GB', {
		day: 'numeric',
		month: 'long',
		year: 'numeric',
		timeZone: 'UTC'
	});
}
