<script lang="ts">
	import type { GameStore } from '$lib/chess/game.svelte';
	import type { MoveView } from '$lib/generated/MoveView';

	// Numbered move pairs. Clicking a move shows the position after it.
	let { game }: { game: GameStore } = $props();

	interface Row {
		number: number;
		white: MoveView | null;
		black: MoveView | null;
	}

	const rows = $derived.by(() => {
		const view = game.view;
		const rows: Row[] = [];
		let number = view.startFullmove;
		view.moves.forEach((move, i) => {
			const whiteMove = (i % 2 === 0) === (view.startTurn === 'white');
			if (whiteMove) {
				rows.push({ number, white: move, black: null });
			} else {
				const last = rows.at(-1);
				if (last && last.black === null) last.black = move;
				// A game that started with Black to move has no white move in its first row.
				else rows.push({ number, white: null, black: move });
				number += 1;
			}
		});
		return rows;
	});
</script>

{#snippet cell(move: MoveView | null)}
	{#if move}
		<button
			type="button"
			class="move"
			class:current={move.ply === game.view.cursor}
			onclick={() => game.goToPly(move.ply)}
		>
			{move.san}
		</button>
	{:else}
		<span class="move empty">…</span>
	{/if}
{/snippet}

<ol class="move-list">
	{#each rows as row (row.number)}
		<li>
			<span class="number">{row.number}.</span>
			{@render cell(row.white)}
			{@render cell(row.black)}
		</li>
	{/each}
</ol>

<style>
	.move-list {
		margin: 0;
		padding: 0.4rem 0;
		list-style: none;
		max-height: 320px;
		overflow-y: auto;
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		font-variant-numeric: tabular-nums;
	}

	li {
		display: grid;
		grid-template-columns: 3rem 1fr 1fr;
		align-items: center;
	}

	.number {
		padding-left: 0.8rem;
		color: var(--text-muted);
	}

	.move {
		padding: 0.25rem 0.5rem;
		border: none;
		border-radius: 4px;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.move:hover {
		background: rgba(255, 255, 255, 0.06);
	}

	.move.current {
		background: var(--accent);
		color: #fff;
	}

	.move.empty {
		color: var(--text-muted);
		cursor: default;
	}
</style>
