// Presentation helpers for `GET /api/me/games` rows: pure, so the games page
// stays markup and these get unit tests.
import { sideName } from '$lib/chess/status';
import type { GameListing } from '$lib/generated/GameListing';
import type { GameResult } from '$lib/generated/GameResult';
import type { PlayerInfo } from '$lib/generated/PlayerInfo';

/** What to call whoever sits in a seat, or the fact that nobody does. */
export function seatName(seat: PlayerInfo | null): string {
	if (seat === null) return 'Open seat';
	return seat.username ?? 'Guest';
}

/** The other player's name from the caller's point of view. */
export function opponentName(listing: GameListing): string {
	return seatName(listing.your_color === 'white' ? listing.players.black : listing.players.white);
}

export type Outcome = 'won' | 'lost' | 'draw' | 'aborted' | 'playing' | 'waiting';

/** How the game stands for the caller. */
export function outcome(listing: GameListing): Outcome {
	const { ended, your_color, players } = listing;
	if (ended === null) return players.black === null ? 'waiting' : 'playing';
	const byResult: Record<GameResult, Outcome> = {
		white_wins: your_color === 'white' ? 'won' : 'lost',
		black_wins: your_color === 'black' ? 'won' : 'lost',
		draw: 'draw',
		aborted: 'aborted'
	};
	return byResult[ended.result];
}

export const OUTCOME_TEXT: Record<Outcome, string> = {
	won: 'Won',
	lost: 'Lost',
	draw: 'Draw',
	aborted: 'Aborted',
	playing: 'In progress',
	waiting: 'Waiting for an opponent'
};

/**
 * Whether a game can be reviewed: it has finished with a result (not
 * aborted) and has moves to look at.
 */
export function reviewable(listing: GameListing): boolean {
	return listing.ended !== null && listing.ended.result !== 'aborted' && listing.moves > 0;
}

/** "You (White) vs alice", for a row heading. */
export function matchup(listing: GameListing): string {
	return `You (${sideName(listing.your_color)}) vs ${opponentName(listing)}`;
}

/** "just now", "5 min ago", "3 days ago", else the date. */
export function relativeTime(iso: string, now: number = Date.now()): string {
	const then = Date.parse(iso);
	if (Number.isNaN(then)) return iso;
	const seconds = Math.max(0, Math.round((now - then) / 1000));
	if (seconds < 60) return 'just now';
	const minutes = Math.round(seconds / 60);
	if (minutes < 60) return `${minutes} min ago`;
	const hours = Math.round(minutes / 60);
	if (hours < 24) return `${hours} h ago`;
	const days = Math.round(hours / 24);
	if (days < 30) return `${days} day${days === 1 ? '' : 's'} ago`;
	return new Date(then).toLocaleDateString();
}
