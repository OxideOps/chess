// Lichess puzzle themes, as people read them. The database writes them as
// camelCase keys (`backRankMate`, `mateIn2`); these are the names Lichess
// shows for them. A key missing here still reads sensibly (`someNewTheme`
// becomes "Some new theme").
import type { PuzzleTheme } from '$lib/generated/PuzzleTheme';

const NAMES: Record<string, string> = {
	advancedPawn: 'Advanced pawn',
	advantage: 'Advantage',
	anastasiaMate: "Anastasia's mate",
	arabianMate: 'Arabian mate',
	attackingF2F7: 'Attacking f2 or f7',
	attraction: 'Attraction',
	backRankMate: 'Back rank mate',
	balestraMate: 'Balestra mate',
	bishopEndgame: 'Bishop endgame',
	blindSwineMate: 'Blind swine mate',
	bodenMate: "Boden's mate",
	capturingDefender: 'Capture the defender',
	castling: 'Castling',
	clearance: 'Clearance',
	cornerMate: 'Corner mate',
	crushing: 'Crushing',
	defensiveMove: 'Defensive move',
	deflection: 'Deflection',
	discoveredAttack: 'Discovered attack',
	discoveredCheck: 'Discovered check',
	doubleBishopMate: 'Double bishop mate',
	doubleCheck: 'Double check',
	dovetailMate: 'Dovetail mate',
	endgame: 'Endgame',
	enPassant: 'En passant',
	equality: 'Equality',
	exposedKing: 'Exposed king',
	fork: 'Fork',
	hangingPiece: 'Hanging piece',
	hookMate: 'Hook mate',
	interference: 'Interference',
	intermezzo: 'Intermezzo',
	killBoxMate: 'Kill box mate',
	kingsideAttack: 'Kingside attack',
	knightEndgame: 'Knight endgame',
	long: 'Long puzzle',
	master: 'Master games',
	masterVsMaster: 'Master vs master games',
	mate: 'Checkmate',
	mateIn1: 'Mate in 1',
	mateIn2: 'Mate in 2',
	mateIn3: 'Mate in 3',
	mateIn4: 'Mate in 4',
	mateIn5: 'Mate in 5 or more',
	middlegame: 'Middlegame',
	morphysMate: "Morphy's mate",
	oneMove: 'One-move puzzle',
	opening: 'Opening',
	operaMate: 'Opera mate',
	pawnEndgame: 'Pawn endgame',
	pillsburysMate: "Pillsbury's mate",
	pin: 'Pin',
	promotion: 'Promotion',
	queenEndgame: 'Queen endgame',
	queenRookEndgame: 'Queen and rook endgame',
	queensideAttack: 'Queenside attack',
	quietMove: 'Quiet move',
	rookEndgame: 'Rook endgame',
	sacrifice: 'Sacrifice',
	short: 'Short puzzle',
	skewer: 'Skewer',
	smotheredMate: 'Smothered mate',
	superGM: 'Super GM games',
	swallowstailMate: "Swallow's tail mate",
	trappedPiece: 'Trapped piece',
	triangleMate: 'Triangle mate',
	underPromotion: 'Underpromotion',
	veryLong: 'Very long puzzle',
	vukovicMate: 'Vuković mate',
	xRayAttack: 'X-ray attack',
	zugzwang: 'Zugzwang'
};

/** "Back rank mate" for `backRankMate`. */
export function themeName(key: string): string {
	const known = NAMES[key];
	if (known) return known;
	const words = key
		.replace(/([a-z])([A-Z0-9])/g, '$1 $2')
		.replace(/([0-9])([A-Za-z])/g, '$1 $2')
		.toLowerCase();
	return words.charAt(0).toUpperCase() + words.slice(1);
}

/** The picker's options: every theme, by name. */
export function themeOptions(
	themes: PuzzleTheme[]
): { key: string; name: string; count: number }[] {
	return themes
		.map((t) => ({ key: t.theme, name: themeName(t.theme), count: t.count }))
		.sort((a, b) => a.name.localeCompare(b.name));
}

/** A theme key as Lichess writes them, safe to pass to the server. */
export function isThemeKey(key: string | null): key is string {
	return key !== null && /^[A-Za-z0-9]{1,40}$/.test(key);
}
