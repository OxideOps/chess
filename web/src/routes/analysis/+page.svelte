<script lang="ts">
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import Board from '$lib/components/Board.svelte';
	import CoachPanel from '$lib/components/CoachPanel.svelte';
	import type { Arrow } from '$lib/components/CoachAnswer.svelte';
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
	import { parsePly, parseSide } from '$lib/review/links';

	// Free analysis: set up any position, step through a game, and let the
	// engine comment. Moves can be made from anywhere in the history.
	// `?pgn=…&ply=N&orientation=black` opens a game at a move (the game review
	// links here with its better lines as variations).
	const game = opened();
	const analyser = new Analyser({ multipv: 3 });
	let orientation: Side = $state(parseSide(page.url.searchParams.get('orientation')) ?? 'white');
	let engineOn = $state(true);
	onDestroy(() => {
		analyser.dispose();
		game.dispose();
	});

	function opened(): GameStore {
		const pgn = page.url.searchParams.get('pgn');
		if (pgn) {
			try {
				const store = GameStore.fromPgn(pgn);
				const ply = parsePly(page.url.searchParams.get('ply'));
				if (ply !== null) store.goToPly(ply);
				return store;
			} catch {
				// A mangled link: start from the usual empty board instead.
			}
		}
		return new GameStore();
	}

	// The engine follows the viewed position, unless it's off or the game is over there.
	$effect(() => {
		const view = game.view;
		analyser.request(engineOn && !view.gameOver ? view.fen : null);
	});

	// Only decorate the board with lines that are about the position it shows.
	const best = $derived(analyser.fen === game.view.fen ? analyser.best : undefined);
	const score = $derived(best ? scoreForWhite(best.score, analyser.turn) : null);
	// A move hovered in the coach's answer takes the arrow's place.
	let preview: Arrow | null = $state(null);
	const arrows = $derived(
		preview
			? [preview]
			: best?.pv[0]
				? [{ from: best.pv[0].slice(0, 2), to: best.pv[0].slice(2, 4) }]
				: []
	);
</script>

<svelte:head>
	<title>Analysis · Chess</title>
</svelte:head>

<div class="analysis board-page">
	<div class="board-with-bar">
		<EvalBar {score} {orientation} />
		<Board {game} {orientation} analysis {arrows} />
	</div>
	<aside class="sidebar">
		<EnginePanel {game} {analyser} bind:enabled={engineOn} />
		<CoachPanel {game} {analyser} onpreview={(a) => (preview = a)} />
		<p class="status">{statusText(game.view)}</p>
		<MoveList {game} />
		<div class="variation-actions">
			<button
				type="button"
				class="btn"
				disabled={game.view.mainLine}
				onclick={() => game.promoteVariation()}
				title="Make this variation the main line (one level at a time)">Promote variation</button
			>
			<button
				type="button"
				class="btn"
				disabled={game.view.node === 0}
				onclick={() => game.deleteFromHere()}
				title="Delete this move and everything after it">Delete from here</button
			>
		</div>
		<p class="keys">
			Moves played from earlier positions become variations. Keys: ←/→ step, ↑/↓ start and end,
			Shift+↑/↓ switch variation.
		</p>
		<Controls {game} bind:orientation />
		<ImportPanel {game} />
	</aside>
</div>

<style>
	.analysis {
		/* The eval bar and its gap sit beside the board. */
		--board-beside: 2rem;
	}

	.variation-actions {
		display: flex;
		gap: 0.5rem;
	}

	.variation-actions .btn {
		flex: 1;
	}

	.keys {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-xs);
	}

	.board-with-bar {
		display: flex;
		gap: 0.6rem;
		align-items: stretch;
	}
</style>
