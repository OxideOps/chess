<script lang="ts">
	import { Game } from '$lib/chess/wasm';
	import type { GameStore } from '$lib/chess/game.svelte';

	// Load a position from FEN or a game from PGN, and copy the current game out.
	let { game }: { game: GameStore } = $props();

	let fenText = $state('');
	let pgnText = $state('');
	let error: string | null = $state(null);

	function load(parse: () => Game) {
		try {
			game.replace(parse());
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}
</script>

<section class="import">
	<form
		onsubmit={(event) => {
			event.preventDefault();
			load(() => Game.fromFen(fenText.trim()));
		}}
	>
		<label for="import-fen">FEN</label>
		<div class="row">
			<input
				id="import-fen"
				placeholder="rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
				spellcheck="false"
				bind:value={fenText}
			/>
			<button type="submit">Load</button>
		</div>
	</form>
	<form
		onsubmit={(event) => {
			event.preventDefault();
			load(() => Game.fromPgn(pgnText));
		}}
	>
		<label for="import-pgn">PGN</label>
		<textarea
			id="import-pgn"
			rows="4"
			placeholder="1. e4 e5 2. Nf3 …"
			spellcheck="false"
			bind:value={pgnText}></textarea>
		<div class="row">
			<button type="submit">Load</button>
		</div>
	</form>
	{#if error}
		<p class="error" role="alert">{error}</p>
	{/if}
	<label for="export-pgn">Current game</label>
	<textarea id="export-pgn" rows="3" readonly value={game.view.pgn}></textarea>
</section>

<style>
	.import {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.row {
		display: flex;
		gap: 0.4rem;
	}

	.row input {
		flex: 1;
		min-width: 0;
	}

	input,
	textarea {
		width: 100%;
		padding: 0.4rem 0.5rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--panel);
		color: var(--text);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.8rem;
		resize: vertical;
	}

	button {
		padding: 0.4rem 0.8rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--panel);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	button:hover {
		border-color: var(--text-muted);
	}

	.error {
		margin: 0;
		color: #e06c75;
	}
</style>
