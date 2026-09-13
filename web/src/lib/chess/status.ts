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
