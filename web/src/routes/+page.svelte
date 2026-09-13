<script lang="ts">
	import { onDestroy } from 'svelte';
	import Board from '$lib/components/Board.svelte';
	import { GameStore } from '$lib/chess/game.svelte';
	import { statusText } from '$lib/chess/status';
	import type { Side } from '$lib/generated/Side';

	// A local two-player game: both sides are moved from this screen.
	// Move list, controls and the rest of the sidebar arrive in phase 4.
	const game = new GameStore();
	let orientation: Side = $state('white');
	onDestroy(() => game.dispose());
</script>

<svelte:head>
	<title>Chess</title>
</svelte:head>

<div class="play">
	<Board {game} {orientation} />
	<aside class="sidebar">
		<p class="status">{statusText(game.view)}</p>
		<p class="movetext" data-testid="movetext">{game.view.movetext}</p>
		<button
			type="button"
			onclick={() => (orientation = orientation === 'white' ? 'black' : 'white')}
		>
			Flip board
		</button>
	</aside>
</div>

<style>
	.play {
		display: flex;
		flex-wrap: wrap;
		gap: 1.5rem;
		align-items: flex-start;
	}

	.sidebar {
		flex: 1 1 260px;
		max-width: 420px;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.status {
		margin: 0;
		padding: 0.6rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		font-weight: 600;
	}

	.movetext {
		margin: 0;
		min-height: 1.5em;
		color: var(--text-muted);
	}

	button {
		padding: 0.45rem 0.6rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--panel);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}
</style>
