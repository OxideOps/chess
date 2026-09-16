<script lang="ts">
	import type { GameStore } from '$lib/chess/game.svelte';
	import { layout, type TreeMove } from '$lib/chess/movelist';

	// The main line as numbered pairs, with any variations written inline
	// under the move they branch from. Clicking a move shows the position
	// after it (and makes its line current).
	let { game }: { game: GameStore } = $props();

	const blocks = $derived(layout(game.view.tree, game.view.startFullmove));
	// Keys for the keyed each: rows by their first move, blocks by position.
	const keyOf = (i: number) => {
		const b = blocks[i];
		return b.kind === 'row' ? `r${b.white?.id ?? b.black?.id}` : `v${i}`;
	};
</script>

{#snippet cell(move: TreeMove | null)}
	{#if move}
		<button
			type="button"
			class="move"
			class:current={move.current}
			onclick={() => game.goToNode(move.id)}
		>
			{move.san}
		</button>
	{:else}
		<span class="move empty">…</span>
	{/if}
{/snippet}

<ol class="move-list">
	{#if blocks.length === 0}
		<li class="none">No moves yet</li>
	{/if}
	{#each blocks as block, i (keyOf(i))}
		{#if block.kind === 'row'}
			<li>
				<span class="number">{block.number}.</span>
				{@render cell(block.white)}
				{@render cell(block.black)}
			</li>
		{:else}
			<li class="variations">
				{#each block.variations as variation, v (v)}
					<p class="variation">
						{#each variation as token, t (t)}
							{#if token.kind === 'move'}
								{#if token.number}<span class="var-number">{token.number}</span>{/if}
								<button
									type="button"
									class="var-move"
									class:current={token.current}
									class:line={token.line}
									onclick={() => game.goToNode(token.id)}>{token.san}</button
								>
							{:else}
								<span class="paren" class:open={token.kind === 'variation_start'}
									>{token.kind === 'variation_start' ? '(' : ')'}</span
								>
							{/if}
						{/each}
					</p>
				{/each}
			</li>
		{/if}
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
		border-radius: var(--radius);
		font-family: var(--font-mono);
		font-size: var(--type-sm);
		font-variant-numeric: tabular-nums;
	}

	li {
		display: grid;
		grid-template-columns: 3rem 1fr 1fr;
		align-items: center;
	}

	/* An empty list would otherwise be a bare sliver of panel. */
	li.none {
		display: block;
		padding: 0.25rem 0.8rem;
		color: var(--text-muted);
		font-family: var(--font);
	}

	.number {
		padding-left: 0.8rem;
		color: var(--text-muted);
	}

	.move {
		min-height: min(var(--tap), 40px);
		padding: 0.25rem 0.5rem;
		border: none;
		border-radius: 4px;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.move:hover,
	.var-move:hover {
		background: var(--panel-raised);
	}

	.move.current,
	.var-move.current {
		background: var(--accent);
		color: var(--on-accent);
	}

	.move.empty {
		color: var(--text-muted);
		cursor: default;
	}

	li.variations {
		display: block;
		padding: 0.15rem 0.8rem 0.3rem 1.6rem;
		border-left: 2px solid var(--panel-border);
		margin: 0.1rem 0 0.1rem 0.9rem;
	}

	/* Flex, so whitespace between tokens doesn't show; the gap spaces them. */
	.variation {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		column-gap: 0.3rem;
		margin: 0.1rem 0;
		line-height: 1.7;
		font-size: var(--type-sm);
		color: var(--text-muted);
	}

	/* Brackets hug what they enclose: "(2. c3 d5)". */
	.paren.open {
		margin-right: -0.3rem;
	}

	/* A closing bracket follows a move button: also absorb its padding. */
	.paren:not(.open) {
		margin-left: -0.6rem;
	}

	.var-number,
	.paren {
		color: var(--text-muted);
	}

	.var-move {
		min-height: min(var(--tap), 32px);
		padding: 0.05rem 0.3rem;
		border: none;
		border-radius: 4px;
		background: none;
		color: var(--text-muted);
		font: inherit;
		cursor: pointer;
	}

	.var-move.line {
		color: var(--text);
	}
</style>
