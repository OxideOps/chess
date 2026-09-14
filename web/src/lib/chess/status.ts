import type { GameEnd } from '$lib/generated/GameEnd';
import type { GameView } from '$lib/generated/GameView';
import type { Side } from '$lib/generated/Side';

export function sideName(side: Side): string {
	return side === 'white' ? 'White' : 'Black';
}

/** One line describing the viewed position, for a status bar. */
export function statusText(view: Pick<GameView, 'status' | 'turn' | 'winner'>): string {
	switch (view.status) {
		case 'ongoing':
			return `${sideName(view.turn)} to move`;
		case 'check':
			return `${sideName(view.turn)} to move — check`;
		case 'checkmate':
			return `Checkmate — ${sideName(view.winner ?? 'white')} wins`;
		case 'stalemate':
			return 'Draw by stalemate';
		case 'insufficient_material':
			return 'Draw by insufficient material';
		case 'fifty_move_rule':
			return 'Draw by the fifty-move rule';
		case 'threefold_repetition':
			return 'Draw by threefold repetition';
	}
}

/** "Black wins by resignation", "Draw by agreement": how an online game ended. */
export function gameEndText(end: GameEnd): string {
	// Every result listed, so a new one is a type error here rather than a wrong winner.
	const who: Record<GameEnd['result'], string | null> = {
		white_wins: 'White wins',
		black_wins: 'Black wins',
		draw: 'Draw',
		aborted: null
	};
	const outcome = who[end.result];
	if (outcome === null) return 'Game aborted: a player left before both had moved';
	const reason: Record<GameEnd['reason'], string> = {
		checkmate: 'by checkmate',
		resignation: 'by resignation',
		timeout: 'on time',
		stalemate: 'by stalemate',
		insufficient_material: 'by insufficient material',
		agreement: 'by agreement',
		repetition: 'by repetition',
		fifty_moves: 'by the fifty-move rule',
		abandoned: 'by abandonment'
	};
	return `${outcome} ${reason[end.reason]}`;
}

/**
 * What to say while a player who left is counting down to losing the game:
 * to their opponent, what happens if they don't return; to anyone else, who
 * left and how long they have.
 */
export function awayText(
	away: { side: Side; ms: number },
	yourColor: Side | null,
	plies: number
): string {
	const seconds = `${Math.ceil(away.ms / 1000)} s`;
	if (yourColor !== null && yourColor !== away.side) {
		const then = plies < 2 ? 'the game is aborted' : 'you win';
		return `Your opponent left. Unless they come back, ${then} in ${seconds}.`;
	}
	return `${sideName(away.side)} left: ${seconds} to come back.`;
}
