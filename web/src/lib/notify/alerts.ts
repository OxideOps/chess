/**
 * The few things worth interrupting someone for, and the words for them.
 *
 * Only what needs the player *now*: a game of theirs is ready to play. Not
 * "a new seek was posted" — the list on screen is already live, and a
 * notification for something that isn't yours is noise that teaches people
 * to turn notifications off.
 *
 * Pure: no browser, no state, so the decision and the wording are testable
 * on their own.
 */

export type Alert =
	/** Someone took the seek we posted. */
	| { kind: 'game-ready'; opponent: string | null; timeControl: string | null }
	/** Someone joined the private game we created and are sitting in. */
	| { kind: 'opponent-joined'; opponent: string | null };

export interface Wording {
	/** The notification's heading. */
	title: string;
	/** The line under it. */
	body: string;
	/** The same news, short enough for a tab. */
	short: string;
}

/**
 * One tag for every alert: they all mean "go to your game", so a second one
 * replaces the first rather than stacking up behind it.
 */
export const TAG = 'chess-game';

/** Whoever it is, said the way a stranger's absence should be said. */
function who(opponent: string | null): string {
	return opponent ?? 'A guest';
}

export function wording(alert: Alert): Wording {
	switch (alert.kind) {
		case 'game-ready':
			return {
				title: 'Your game is ready',
				body: alert.timeControl
					? `${who(alert.opponent)} took your ${alert.timeControl} offer.`
					: `${who(alert.opponent)} took your offer.`,
				short: 'Your game is ready'
			};
		case 'opponent-joined':
			return {
				title: 'Your opponent is here',
				body: `${who(alert.opponent)} joined your game.`,
				short: 'Your opponent is here'
			};
	}
}

/**
 * Whether to raise a system notification, as opposed to the quieter signals
 * that always happen.
 *
 * A notification is for someone who isn't looking. If the tab is on screen
 * the page itself is already showing the news — sending a desktop pop-up
 * for something visible behind it is the thing that makes people click
 * "block".
 */
export function shouldShow(state: {
	hidden: boolean;
	permission: NotificationPermission;
}): boolean {
	return state.hidden && state.permission === 'granted';
}
