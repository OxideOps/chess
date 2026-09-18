/**
 * Which sound a move asks for. Everything needed is already in the move's
 * SAN and whether the game ended, so this is a pure function with no chess
 * knowledge of its own: `chess-core` wrote the SAN, and the notation says
 * what happened (`x` took something, `+` gave check, `=` promoted, `O-O`
 * castled).
 */
export type Cue = 'move' | 'capture' | 'castle' | 'check' | 'promote' | 'end' | 'low-time';

/**
 * The cue for a move written as `san`, in a game that `over` says has now
 * finished. Loudest thing first: the end of the game beats a check, a check
 * beats what the move did to get there.
 */
export function cueFor(san: string, over: boolean): Cue {
	if (over) return 'end';
	if (san.includes('+')) return 'check';
	if (san.includes('=')) return 'promote';
	if (san.includes('x')) return 'capture';
	if (san.startsWith('O-O')) return 'castle';
	return 'move';
}
