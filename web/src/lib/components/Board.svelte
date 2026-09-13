<script lang="ts">
	import type { GameStore, Promotion } from '$lib/chess/game.svelte';
	import type { PlayResult } from '$lib/chess/wasm';
	import { centre, isLight, squaresInDrawOrder } from '$lib/chess/squares';
	import type { PieceOnSquare } from '$lib/generated/PieceOnSquare';
	import type { Side } from '$lib/generated/Side';
	import { pieceImage, pieceName } from './pieces';
	import PromotionPicker from './PromotionPicker.svelte';

	/**
	 * An interactive board. Click a piece, then click where it should go.
	 *
	 * In play mode the board only accepts moves at the latest position of a
	 * game that isn't over. In `analysis` mode any position can be played
	 * from: moving while viewing history discards the moves after it. Arrow
	 * keys step through the history when the board has focus. `arrows` are
	 * drawn from square to square, e.g. for engine suggestions.
	 */
	interface Props {
		game: GameStore;
		orientation?: Side;
		analysis?: boolean;
		arrows?: { from: string; to: string }[];
		/** Only pieces of this side can be picked up; default both (local play). */
		playAs?: Side | 'both';
		/** Where moves go instead of straight into `game` (online play). */
		onmove?: (from: string, to: string, promotion?: Promotion) => PlayResult;
	}
	let {
		game,
		orientation = 'white',
		analysis = false,
		arrows = [],
		playAs = 'both',
		onmove
	}: Props = $props();

	let selected: string | null = $state(null);
	// A move that needs a promotion piece before it can be played.
	let promotion: { from: string; to: string } | null = $state(null);

	const view = $derived(game.view);
	const interactive = $derived(
		!view.gameOver &&
			(analysis || !view.viewingHistory) &&
			(playAs === 'both' || playAs === view.turn)
	);
	const selectedSquare = $derived(interactive ? selected : null);
	const pieceAt = $derived(new Map<string, PieceOnSquare>(view.pieces.map((p) => [p.square, p])));
	const destinations = $derived.by(() => {
		// Depends on the position too, not just the selection.
		void view.fen;
		return new Set(selectedSquare ? game.legalDestinations(selectedSquare) : []);
	});

	const order = $derived(squaresInDrawOrder(orientation));
	const squares = $derived(
		order.map((square, i) => ({
			square,
			piece: pieceAt.get(square),
			light: isLight(square),
			selected: selectedSquare === square,
			lastMove: view.lastMove?.from === square || view.lastMove?.to === square,
			check: view.checkSquare === square,
			destination: destinations.has(square),
			// Coordinate labels go on the left column and bottom row.
			rankLabel: i % 8 === 0 ? square[1] : null,
			fileLabel: i >= 56 ? square[0] : null
		}))
	);

	const arrowViews = $derived(
		arrows.map(({ from, to }) => {
			const a = centre(from, orientation);
			const b = centre(to, orientation);
			// Stop short of the destination centre so the head sits inside the square.
			const dx = b.x - a.x;
			const dy = b.y - a.y;
			const len = Math.hypot(dx, dy) || 1;
			const shorten = 0.3;
			return { x1: a.x, y1: a.y, x2: b.x - (dx / len) * shorten, y2: b.y - (dy / len) * shorten };
		})
	);

	function play(from: string, to: string, promo?: Promotion): PlayResult {
		if (onmove) return onmove(from, to, promo);
		return analysis ? game.playHere(from, to, promo) : game.play(from, to, promo);
	}

	function ownsPiece(square: string): boolean {
		return pieceAt.get(square)?.color === view.turn;
	}

	function onSquareClick(square: string) {
		if (!interactive) return;
		if (selected === square) {
			selected = null;
			return;
		}
		if (selected) {
			const from = selected;
			switch (play(from, square)) {
				case 'ok':
					selected = null;
					break;
				case 'promotion_required':
					promotion = { from, to: square };
					break;
				// Clicking another of your own pieces selects it instead.
				case 'illegal':
					selected = ownsPiece(square) ? square : null;
					break;
			}
		} else if (ownsPiece(square)) {
			selected = square;
		}
	}

	function onPromotionPick(role: Promotion | null) {
		if (role && promotion && play(promotion.from, promotion.to, role) !== 'ok') {
			console.warn('promotion failed');
		}
		promotion = null;
		selected = null;
	}

	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 'ArrowLeft':
				game.goBack();
				break;
			case 'ArrowRight':
				game.goForward();
				break;
			case 'ArrowUp':
				game.goToStart();
				break;
			case 'ArrowDown':
				game.goToEnd();
				break;
			case 'Escape':
				selected = null;
				promotion = null;
				break;
			default:
				return;
		}
		event.preventDefault();
	}
</script>

<!-- The board is the keyboard target for history navigation; squares are plain click targets. -->
<div class="board" class:interactive role="grid" tabindex="0" onkeydown={onKeyDown}>
	{#each squares as s (s.square)}
		<button
			type="button"
			class="square"
			class:light={s.light}
			class:dark={!s.light}
			class:selected={s.selected}
			class:last-move={s.lastMove}
			class:check={s.check}
			data-square={s.square}
			aria-label={s.piece ? `${s.square}, ${pieceName(s.piece.color, s.piece.role)}` : s.square}
			onclick={() => onSquareClick(s.square)}
		>
			{#if s.piece}
				<img
					class="piece"
					src={pieceImage(s.piece.color, s.piece.role)}
					alt={pieceName(s.piece.color, s.piece.role)}
					draggable="false"
				/>
			{/if}
			{#if s.destination}
				<span class={s.piece ? 'capture-hint' : 'move-hint'}></span>
			{/if}
			{#if s.rankLabel}
				<span class="coord rank">{s.rankLabel}</span>
			{/if}
			{#if s.fileLabel}
				<span class="coord file">{s.fileLabel}</span>
			{/if}
		</button>
	{/each}

	{#if arrowViews.length > 0}
		<svg class="arrows" viewBox="0 0 8 8" aria-hidden="true">
			<defs>
				<marker
					id="arrowhead"
					viewBox="0 0 10 10"
					refX="5"
					refY="5"
					markerWidth="4"
					markerHeight="4"
					orient="auto"
				>
					<polygon points="0,0 10,5 0,10" />
				</marker>
			</defs>
			{#each arrowViews as a, i (i)}
				<line x1={a.x1} y1={a.y1} x2={a.x2} y2={a.y2} marker-end="url(#arrowhead)" />
			{/each}
		</svg>
	{/if}

	{#if promotion}
		<PromotionPicker color={view.turn} onpick={onPromotionPick} />
	{/if}
</div>

<style>
	.board {
		position: relative;
		display: grid;
		grid-template-columns: repeat(8, 1fr);
		width: min(92vw, 640px);
		aspect-ratio: 1;
		user-select: none;
		-webkit-user-select: none;
		outline: none;
		border-radius: 4px;
		overflow: hidden;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
	}

	.board:focus-visible {
		box-shadow:
			0 0 0 3px var(--accent),
			0 8px 24px rgba(0, 0, 0, 0.45);
	}

	.square {
		position: relative;
		aspect-ratio: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		/* Squares are buttons for accessibility; they should not look like them. */
		padding: 0;
		border: none;
		outline: none;
		font: inherit;
		cursor: default;
	}

	.square.light {
		background: var(--square-light);
	}

	.square.dark {
		background: var(--square-dark);
	}

	.board.interactive .square:has(.piece),
	.board.interactive .square:has(.move-hint),
	.board.interactive .square:has(.capture-hint) {
		cursor: pointer;
	}

	/* Highlights are painted in a pseudo-element so they sit under the piece. */
	.square.selected::before,
	.square.last-move::before,
	.square.check::before {
		content: '';
		position: absolute;
		inset: 0;
	}

	.square.last-move::before {
		background: var(--highlight-last-move);
	}

	.square.selected::before {
		background: var(--highlight-selected);
	}

	.square.check::before {
		background: radial-gradient(
			circle at center,
			rgba(255, 0, 0, 1) 0%,
			rgba(231, 0, 0, 1) 25%,
			rgba(169, 0, 0, 0) 89%,
			rgba(158, 0, 0, 0) 100%
		);
	}

	.piece {
		position: relative;
		z-index: 2;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	.move-hint {
		position: absolute;
		z-index: 1;
		width: 32%;
		height: 32%;
		border-radius: 50%;
		background: var(--highlight-destination);
	}

	.capture-hint {
		position: absolute;
		z-index: 1;
		inset: 0;
		border-radius: 50%;
		border: 0.4rem solid var(--highlight-destination);
		box-sizing: border-box;
	}

	.coord {
		position: absolute;
		z-index: 3;
		font-size: 0.7rem;
		font-weight: 600;
		line-height: 1;
		opacity: 0.85;
	}

	.coord.rank {
		top: 3px;
		left: 4px;
	}

	.coord.file {
		bottom: 3px;
		right: 4px;
	}

	.light .coord {
		color: var(--square-dark);
	}

	.dark .coord {
		color: var(--square-light);
	}

	/* Arrows float above the pieces but don't take clicks. */
	.arrows {
		position: absolute;
		z-index: 4;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	.arrows line {
		stroke: rgba(21, 120, 27, 0.75);
		stroke-width: 0.16;
		stroke-linecap: round;
	}

	.arrows polygon {
		fill: rgba(21, 120, 27, 0.75);
	}
</style>
