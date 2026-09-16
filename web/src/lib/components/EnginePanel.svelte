<script lang="ts">
	import type { GameStore } from '$lib/chess/game.svelte';
	import { formatScore, pvMovetext, scoreForWhite } from '$lib/chess/wasm';
	import type { Analyser } from '$lib/engine/analysis.svelte';

	// Engine on/off switch, progress, and its best lines. Clicking a line
	// plays its first move on the board.
	interface Props {
		game: GameStore;
		analyser: Analyser;
		enabled: boolean;
	}
	let { game, analyser, enabled = $bindable() }: Props = $props();

	const name = $derived(analyser.name ?? 'Stockfish');
	const lines = $derived(
		analyser.fen === null
			? []
			: analyser.lines.map((line) => ({
					key: line.multipv,
					score: formatScore(scoreForWhite(line.score, analyser.turn)),
					movetext: pvMovetext(analyser.fen!, line.pv),
					firstMove: line.pv[0] ?? ''
				}))
	);
	const summary = $derived.by(() => {
		if (analyser.status === 'failed') return `Engine unavailable: ${analyser.error ?? ''}`;
		if (analyser.status === 'loading') return 'Loading engine…';
		if (!enabled) return 'Engine off';
		// Nothing to analyse: the game is over at the viewed position.
		if (analyser.fen === null) return 'Idle';
		const depth = analyser.depth;
		if (depth === undefined) return 'Thinking…';
		return analyser.searching ? `Depth ${depth}…` : `Depth ${depth}`;
	});

	function playLine(uci: string) {
		if (uci && game.playHereUci(uci) !== 'ok') console.warn('engine line: could not play', uci);
	}
</script>

<section class="engine">
	<header>
		<label>
			<input type="checkbox" bind:checked={enabled} disabled={analyser.status === 'failed'} />
			<span class="name">{name}</span>
		</label>
		<span class="summary">{summary}</span>
		{#if analyser.threads > 1}
			<span class="threads" title="Multi-threaded build: the page is cross-origin isolated"
				>{analyser.threads} threads</span
			>
		{/if}
	</header>
	{#if enabled && lines.length > 0}
		<ol class="lines">
			{#each lines as line (line.key)}
				<li>
					<button
						type="button"
						title="Play {line.firstMove}"
						onclick={() => playLine(line.firstMove)}
					>
						<span class="score">{line.score}</span>
						<span class="moves">{line.movetext}</span>
					</button>
				</li>
			{/each}
		</ol>
	{/if}
</section>

<style>
	.engine {
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: var(--radius);
		overflow: hidden;
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		padding: 0.5rem 0.8rem;
	}

	header label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 600;
		cursor: pointer;
	}

	header input {
		accent-color: var(--accent);
	}

	.threads {
		color: var(--text-muted);
		font-size: var(--type-xs);
		white-space: nowrap;
	}

	.summary {
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: var(--type-sm);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	.lines {
		margin: 0;
		padding: 0;
		list-style: none;
		border-top: 1px solid var(--panel-border);
	}

	.lines li + li {
		border-top: 1px solid var(--panel-border);
	}

	/* An engine line is notation: mono, so the moves line up between lines. */
	.lines button {
		min-height: var(--tap);
		display: flex;
		align-items: center;
		gap: 0.6rem;
		width: 100%;
		padding: 0.4rem 0.8rem;
		border: none;
		background: none;
		color: var(--text);
		font-family: var(--font-mono);
		font-size: var(--type-sm);
		text-align: left;
		cursor: pointer;
	}

	.lines button:hover {
		background: var(--panel-raised);
	}

	.score {
		flex: 0 0 3.2rem;
		font-weight: 500;
		font-variant-numeric: tabular-nums;
	}

	.moves {
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
</style>
