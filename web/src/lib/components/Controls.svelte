<script lang="ts">
	import { Game } from '$lib/chess/wasm';
	import type { GameStore } from '$lib/chess/game.svelte';
	import type { Side } from '$lib/generated/Side';

	// History navigation plus board-level actions.
	let { game, orientation = $bindable() }: { game: GameStore; orientation: Side } = $props();

	const atStart = $derived(game.view.cursor === 0);
	const atEnd = $derived(!game.view.viewingHistory);
</script>

<!-- Stepping through the game is one control, so the four buttons are one bar. -->
<div class="history" role="group" aria-label="Move through the game">
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
<div class="actions">
	<button
		type="button"
		class="btn"
		onclick={() => (orientation = orientation === 'white' ? 'black' : 'white')}
	>
		Flip board
	</button>
	<button type="button" class="btn" onclick={() => game.replace(new Game())}>New game</button>
</div>

<style>
	.history {
		display: flex;
		border: 1px solid var(--panel-border);
		border-radius: var(--radius-sm);
		overflow: hidden;
	}

	.history button {
		flex: 1;
		min-height: var(--tap);
		padding: 0.45rem 0.6rem;
		border: none;
		border-left: 1px solid var(--panel-border);
		background: var(--panel);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	.history button:first-child {
		border-left: none;
	}

	.history button:hover:not(:disabled) {
		background: var(--panel-raised);
	}

	.history button:disabled {
		opacity: 0.4;
		cursor: default;
	}

	.actions {
		display: flex;
		gap: 0.5rem;
	}

	.actions .btn {
		flex: 1;
	}
</style>
