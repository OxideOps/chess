<script lang="ts">
	import { onDestroy } from 'svelte';
	import Board from '$lib/components/Board.svelte';
	import Controls from '$lib/components/Controls.svelte';
	import MoveList from '$lib/components/MoveList.svelte';
	import { GameStore } from '$lib/chess/game.svelte';
	import { statusText } from '$lib/chess/status';
	import type { Side } from '$lib/generated/Side';

	// A local two-player game: both sides are moved from this screen.
	const game = new GameStore();
	let orientation: Side = $state('white');
	onDestroy(() => game.dispose());
</script>

<svelte:head>
	<title>Chess</title>
</svelte:head>

<div class="play board-page">
	<Board {game} {orientation} />
	<aside class="sidebar">
		<p class="status">{statusText(game.view)}</p>
		<MoveList {game} />
		<Controls {game} bind:orientation />
		<label class="fen">
			<span>FEN</span>
			<input readonly value={game.view.fen} />
		</label>
	</aside>
</div>

<style>
	.fen {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: var(--type-xs);
		color: var(--text-muted);
	}

	.fen input {
		padding: 0.4rem 0.5rem;
		border: 1px solid var(--panel-border);
		border-radius: var(--radius-sm);
		background: var(--panel);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: var(--type-xs);
	}
</style>
