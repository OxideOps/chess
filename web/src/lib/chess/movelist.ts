// Lay the move tree out for the move list: the main line as numbered rows of
// two moves, and each group of variations as a block under the move they
// branch from, written inline with their own nested brackets (the way
// Lichess shows them). Pure presentation; the tree itself comes from
// chess-core as PGN-shaped tokens.
import type { TreeToken } from '$lib/generated/TreeToken';

export type TreeMove = Extract<TreeToken, { kind: 'move' }>;

export interface MoveRow {
	kind: 'row';
	number: number;
	white: TreeMove | null;
	black: TreeMove | null;
}

/** Variations after one main-line move; each is inline tokens with nested brackets. */
export interface VariationBlock {
	kind: 'variations';
	variations: TreeToken[][];
}

export type Block = MoveRow | VariationBlock;

function moveNumber(token: TreeMove): number | null {
	return token.number ? Number.parseInt(token.number, 10) : null;
}

export function layout(tokens: TreeToken[], startFullmove: number): Block[] {
	const blocks: Block[] = [];
	let number = startFullmove;
	let i = 0;
	while (i < tokens.length) {
		const token = tokens[i];
		if (token.kind === 'variation_start') {
			// Collect up to the matching end, nested brackets included.
			let level = 0;
			const variation: TreeToken[] = [];
			for (; i < tokens.length; i++) {
				const t = tokens[i];
				if (t.kind === 'variation_start') level++;
				if (t.kind === 'variation_end') level--;
				if (level === 0) break;
				if (!(t.kind === 'variation_start' && level === 1)) variation.push(t);
			}
			i++; // past the matching end
			const last = blocks.at(-1);
			if (last?.kind === 'variations') last.variations.push(variation);
			else blocks.push({ kind: 'variations', variations: [variation] });
			continue;
		}
		if (token.kind === 'move') {
			number = moveNumber(token) ?? number;
			const white = token.number !== null && !token.number.endsWith('...');
			const last = blocks.at(-1);
			if (white) {
				blocks.push({ kind: 'row', number, white: token, black: null });
			} else if (last?.kind === 'row' && last.black === null && last.white !== null) {
				last.black = token;
			} else {
				// First move of a game from a Black-to-move position, or after variations.
				blocks.push({ kind: 'row', number, white: null, black: token });
			}
		}
		i++;
	}
	return blocks;
}
