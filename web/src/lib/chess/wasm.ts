// chess-core compiled to WebAssembly. `initChess()` must resolve before
// anything else in this directory is used; the root layout awaits it.
//
// wasm-bindgen types JSON-shaped returns as `any`; the wrappers below give
// them the ts-rs generated types so nothing else needs a cast.
import init, * as raw from '$lib/wasm/chess_core';
import type { EngineMessage } from '$lib/generated/EngineMessage';
import type { EngineScore } from '$lib/generated/EngineScore';
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

/** `+0.35`, `-1.20`, `#3`. */
export function formatScore(score: EngineScore): string {
	return raw.formatScore(score);
}

/** A principal variation as numbered SAN movetext from the position `fen`. */
export function pvMovetext(fen: string, pv: string[]): string {
	return raw.pvMovetext(fen, pv);
}
