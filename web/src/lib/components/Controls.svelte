<script lang="ts">
	import { Game } from '$lib/chess/wasm';
	import type { GameStore } from '$lib/chess/game.svelte';
	import type { Side } from '$lib/generated/Side';

	// History navigation plus board-level actions.
	let { game, orientation = $bindable() }: { game: GameStore; orientation: Side } = $props();

	const atStart = $derived(game.view.cursor === 0);
	const atEnd = $derived(!game.view.viewingHistory);
</script>

<div class="controls">
	<button type="button" title="First move" disabled={atStart} onclick={() => game.goToStart()}
		>⏮</button
	>
	<button type="button" title="Previous move" disabled={atStart} onclick={() => game.goBack()}
		>◀</button
	>
	<button type="button" title="Next move" disabled={atEnd} onclick={() => game.goForward()}
		>▶</button
	>
	<button type="button" title="Last move" disabled={atEnd} onclick={() => game.goToEnd()}>⏭</button>
</div>
<div class="controls">
	<button type="button" onclick={() => (orientation = orientation === 'white' ? 'black' : 'white')}>
		Flip board
	</button>
	<button type="button" onclick={() => game.replace(new Game())}>New game</button>
</div>

<style>
	.controls {
		display: flex;
		gap: 0.5rem;
	}

	button {
		min-height: var(--tap);
		flex: 1;
		padding: 0.45rem 0.6rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--panel);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	button:hover:not(:disabled) {
		border-color: var(--text-muted);
	}

	button:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
