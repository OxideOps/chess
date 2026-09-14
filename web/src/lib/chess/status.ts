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
	const who =
		end.result === 'draw'
			? 'Draw'
			: `${sideName(end.result === 'white_wins' ? 'white' : 'black')} wins`;
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
	return `${who} ${reason[end.reason]}`;
}
