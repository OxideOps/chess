import { describe, expect, it } from 'vitest';
import { layout, type Block } from './movelist';
import type { TreeToken } from '$lib/generated/TreeToken';

let nextId = 1;
const m = (san: string, number: string | null = null, depth = 0): TreeToken => ({
	kind: 'move',
	id: nextId++,
	number,
	san,
	depth,
	current: false,
	line: depth === 0
});
const open: TreeToken = { kind: 'variation_start' };
const close: TreeToken = { kind: 'variation_end' };

/** Blocks as text, to compare shapes at a glance. */
function show(blocks: Block[]): string[] {
	return blocks.map((b) =>
		b.kind === 'row'
			? `${b.number}. ${b.white?.san ?? '…'} ${b.black?.san ?? '…'}`
			: b.variations
					.map((v) =>
						v
							.map((t) =>
								t.kind === 'move'
									? `${t.number ? t.number + ' ' : ''}${t.san}`
									: t.kind === 'variation_start'
										? '('
										: ')'
							)
							.join(' ')
					)
					.join(' | ')
	);
}

describe('layout', () => {
	it('puts the main line in numbered rows', () => {
		const tokens = [m('e4', '1.'), m('e5'), m('Nf3', '2.')];
		expect(show(layout(tokens, 1))).toEqual(['1. e4 e5', '2. Nf3 …']);
	});

	it('breaks a row for variations and resumes with an empty white cell', () => {
		// 1. e4 e5 (1... c5 2. Nf3 (2. c3)) (1... e6) 2. Nf3
		const tokens = [
			m('e4', '1.'),
			m('e5'),
			open,
			m('c5', '1...', 1),
			m('Nf3', '2.', 1),
			open,
			m('c3', '2.', 2),
			close,
			close,
			open,
			m('e6', '1...', 1),
			close,
			m('Nf3', '2.')
		];
		expect(show(layout(tokens, 1))).toEqual([
			'1. e4 e5',
			'1... c5 2. Nf3 ( 2. c3 ) | 1... e6',
			'2. Nf3 …'
		]);

		// After a white move's variation, Black continues on a new row.
		const afterWhite = [m('e4', '1.'), open, m('d4', '1.', 1), close, m('e5', '1...')];
		expect(show(layout(afterWhite, 1))).toEqual(['1. e4 …', '1. d4', '1. … e5']);
	});

	it('starts from a Black-to-move position with an empty white cell', () => {
		expect(show(layout([m('e5', '12...'), m('Nf3', '13.')], 12))).toEqual([
			'12. … e5',
			'13. Nf3 …'
		]);
		expect(layout([], 1)).toEqual([]);
	});
});
