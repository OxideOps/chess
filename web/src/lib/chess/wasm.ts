// chess-core compiled to WebAssembly. `initChess()` must resolve before
// anything else in this directory is used; the root layout awaits it.
//
// wasm-bindgen types JSON-shaped returns as `any`; the wrappers below give
// them the ts-rs generated types so nothing else needs a cast.
import init, * as raw from '$lib/wasm/chess_core';
import type { Drill } from '$lib/generated/Drill';
import type { DrillStatus } from '$lib/generated/DrillStatus';
import type { EngineMessage } from '$lib/generated/EngineMessage';
import type { EngineScore } from '$lib/generated/EngineScore';
import type { PositionEval } from '$lib/generated/PositionEval';
import type { PuzzleVerdict } from '$lib/generated/PuzzleVerdict';
import type { Review } from '$lib/generated/Review';
import type { Side } from '$lib/generated/Side';

export { Game } from '$lib/wasm/chess_core';
export type { PlayResult } from '$lib/wasm/chess_core';

let ready: Promise<void> | undefined;

/** Load and instantiate the chess-core module (once). */
export function initChess(): Promise<void> {
	ready ??= init().then(() => undefined);
	return ready;
}

/** Classify one line of UCI engine output. */
export function parseEngineMessage(line: string): EngineMessage {
	return raw.parseEngineMessage(line) as EngineMessage;
}

/** Convert a score given for the side to move into White's point of view. */
export function scoreForWhite(score: EngineScore, turn: Side): EngineScore {
	return raw.scoreForWhite(score, turn) as EngineScore;
}

/** Share of an eval bar (0 to 1) to fill for the side the score favours. */
export function barFraction(score: EngineScore): number {
	return raw.barFraction(score);
}

/** Whether going from `before` to `after` (from the mover's view) was a mistake. */
export function isMistake(before: EngineScore, after: EngineScore): boolean {
	return raw.isMistake(before, after);
}

/** `+0.35`, `-1.20`, `#3`. */
export function formatScore(score: EngineScore): string {
	return raw.formatScore(score);
}

/**
 * Judge the solver's latest move in a puzzle. `moves` is the puzzle as
 * served (setup move first); `played` every move since the setup, ending
 * with the solver's newest.
 */
export function judgePuzzle(fen: string, moves: string[], played: string[]): PuzzleVerdict {
	return raw.judgePuzzle(fen, moves, played) as PuzzleVerdict;
}

/** Every lesson drill, easiest first. */
export function drills(): Drill[] {
	return raw.drills() as Drill[];
}

/** Where drill `id` stands after `moves` (UCI, both sides, from its FEN). */
export function assessDrill(id: string, moves: string[]): DrillStatus {
	return raw.assessDrill(id, moves) as DrillStatus;
}

/** A principal variation as numbered SAN movetext from the position `fen`. */
export function pvMovetext(fen: string, pv: string[]): string {
	return raw.pvMovetext(fen, pv);
}

/**
 * Review a finished game: its biggest swings, and the game as PGN with the
 * engine's better lines added as variations. `evals` has one entry per
 * position of the main line from the start (`null` where it wasn't
 * searched); `side` keeps only that player's moves.
 */
export function reviewGame(pgn: string, evals: (PositionEval | null)[], side: Side | null): Review {
	return raw.reviewGame(pgn, evals, side) as Review;
}
