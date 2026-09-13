import { base } from '$app/paths';
import type { PieceRole } from '$lib/generated/PieceRole';
import type { Side } from '$lib/generated/Side';

const LETTER: Record<PieceRole, string> = {
	pawn: 'P',
	knight: 'N',
	bishop: 'B',
	rook: 'R',
	queen: 'Q',
	king: 'K'
};

/** URL of a piece image (Colin M.L. Burnett's set, CC BY-SA 3.0; see README). */
export function pieceImage(color: Side, role: PieceRole): string {
	return `${base}/pieces/cburnett/${color[0]}${LETTER[role]}.svg`;
}

export function pieceName(color: Side, role: PieceRole): string {
	return `${color} ${role}`;
}
