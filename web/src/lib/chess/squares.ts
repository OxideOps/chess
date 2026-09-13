// Board geometry: nothing here knows the rules, only where squares are.
import type { Side } from '$lib/generated/Side';

export const FILES = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'] as const;
export const RANKS = ['1', '2', '3', '4', '5', '6', '7', '8'] as const;

export type File = (typeof FILES)[number];
export type Rank = (typeof RANKS)[number];

/** 0-based file and rank of a square like `e4` (`e` → 4, `4` → 3). */
export function coords(square: string): { file: number; rank: number } {
	const file = FILES.indexOf(square[0] as File);
	const rank = RANKS.indexOf(square[1] as Rank);
	if (file < 0 || rank < 0 || square.length !== 2) throw new Error(`not a square: ${square}`);
	return { file, rank };
}

export function isLight(square: string): boolean {
	const { file, rank } = coords(square);
	return (file + rank) % 2 === 1;
}

/**
 * Centre of a square on an 8x8 canvas whose origin is the top-left corner as
 * drawn for `orientation` (a8 for White, h1 for Black).
 */
export function centre(square: string, orientation: Side): { x: number; y: number } {
	const { file, rank } = coords(square);
	return orientation === 'white'
		? { x: file + 0.5, y: 7.5 - rank }
		: { x: 7.5 - file, y: rank + 0.5 };
}

/** The 64 squares in drawing order (rows top to bottom) for `orientation`. */
export function squaresInDrawOrder(orientation: Side): string[] {
	const files = orientation === 'white' ? [...FILES] : [...FILES].reverse();
	const ranks = orientation === 'white' ? [...RANKS].reverse() : [...RANKS];
	return ranks.flatMap((rank) => files.map((file) => file + rank));
}
