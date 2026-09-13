<script lang="ts">
	import { onDestroy } from 'svelte';
	import Board from '$lib/components/Board.svelte';
	import Controls from '$lib/components/Controls.svelte';
	import EnginePanel from '$lib/components/EnginePanel.svelte';
	import EvalBar from '$lib/components/EvalBar.svelte';
	import ImportPanel from '$lib/components/ImportPanel.svelte';
	import MoveList from '$lib/components/MoveList.svelte';
	import { GameStore } from '$lib/chess/game.svelte';
	import { statusText } from '$lib/chess/status';
	import { scoreForWhite } from '$lib/chess/wasm';
	import { Analyser } from '$lib/engine/analysis.svelte';
	import type { Side } from '$lib/generated/Side';

	// Free analysis: set up any position, step through a game, and let the
	// engine comment. Moves can be made from anywhere in the history.
	const game = new GameStore();
	const analyser = new Analyser({ multipv: 3 });
	let orientation: Side = $state('white');
	let engineOn = $state(true);
	onDestroy(() => {
		analyser.dispose();
		game.dispose();
	});

	// The engine follows the viewed position, unless it's off or the game is over there.
	$effect(() => {
		const view = game.view;
		analyser.request(engineOn && !view.gameOver ? view.fen : null);
	});

	// Only decorate the board with lines that are about the position it shows.
	const best = $derived(analyser.fen === game.view.fen ? analyser.best : undefined);
	const score = $derived(best ? scoreForWhite(best.score, analyser.turn) : null);
	const arrows = $derived(
		best?.pv[0] ? [{ from: best.pv[0].slice(0, 2), to: best.pv[0].slice(2, 4) }] : []
	);
</script>

<svelte:head>
	<title>Analysis · Chess</title>
</svelte:head>

<div class="analysis">
	<div class="board-with-bar">
		<EvalBar {score} {orientation} />
		<Board {game} {orientation} analysis {arrows} />
	</div>
	<aside class="sidebar">
		<EnginePanel {game} {analyser} bind:enabled={engineOn} />
		<p class="status">{statusText(game.view)}</p>
		<MoveList {game} />
		<Controls {game} bind:orientation />
		<ImportPanel {game} />
	</aside>
</div>

<style>
	.analysis {
		display: flex;
		flex-wrap: wrap;
		gap: 1.5rem;
		align-items: flex-start;
	}

	.board-with-bar {
		display: flex;
		gap: 0.6rem;
		align-items: stretch;
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
</style>
