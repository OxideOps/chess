<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import Board from '$lib/components/Board.svelte';
	import { GameStore } from '$lib/chess/game.svelte';
	import { gameEndText, sideName } from '$lib/chess/status';
	import { formatScore, reviewGame } from '$lib/chess/wasm';
	import type { GameSnapshot } from '$lib/generated/GameSnapshot';
	import type { Side } from '$lib/generated/Side';
	import { seatName } from '$lib/online/listing';
	import { GameAnalysis, positionsOf } from '$lib/review/analysis.svelte';
	import { parseSide, withQuery } from '$lib/review/links';

	// A finished game's biggest swings, with what Stockfish would have played
	// instead. The engine goes through the game here, in the browser, one
	// position at a time; the result is kept per browser, so a second visit
	// shows it straight away.
	const id = page.params.id!;
	// Whose game this is, from the link on /games. Without it, both sides.
	const yours = parseSide(page.url.searchParams.get('side'));
	const orientation: Side = yours ?? 'white';

	let snapshot = $state<GameSnapshot | null>(null);
	let loadError = $state<string | null>(null);
	let board = $state<GameStore | null>(null);
	let analysis = $state<GameAnalysis | null>(null);
	let mineOnly = $state(yours !== null);
	let selected = $state(0);

	onMount(() => {
		let cancelled = false;
		fetch(`/api/games/${encodeURIComponent(id)}`)
			.then(async (response) => {
				if (response.status === 404) throw new Error('there is no such game');
				if (!response.ok) throw new Error(`the server said ${response.status}`);
				const game = (await response.json()) as GameSnapshot;
				if (cancelled) return;
				snapshot = game;
				if (!finished(game)) return;
				board = GameStore.fromPgn(game.movetext);
				analysis = new GameAnalysis(`game.${id}`, positionsOf(game.movetext));
				analysis.start();
			})
			.catch((e: unknown) => {
				if (!cancelled) loadError = e instanceof Error ? e.message : String(e);
			});
		return () => {
			cancelled = true;
		};
	});
	onDestroy(() => {
		analysis?.dispose();
		board?.dispose();
	});

	function finished(game: GameSnapshot): boolean {
		return game.ended !== null && game.ended.result !== 'aborted' && game.movetext !== '';
	}

	const review = $derived(
		snapshot && analysis?.status === 'done'
			? reviewGame(snapshot.movetext, $state.snapshot(analysis.evals), mineOnly ? yours : null)
			: null
	);
	const swing = $derived(review?.swings[selected] ?? null);

	// The board shows the position the selected move was played from.
	$effect(() => {
		if (board && swing) board.goToPly(swing.ply - 1);
	});
	const arrows = $derived(
		swing?.bestUci[0]
			? [{ from: swing.bestUci[0].slice(0, 2), to: swing.bestUci[0].slice(2, 4) }]
			: []
	);

	function showSide(mine: boolean) {
		mineOnly = mine;
		selected = 0;
	}

	const percent = (share: number) => `${Math.round(share * 100)}%`;
	const player = (side: Side) => {
		const seat = snapshot?.players[side] ?? null;
		return `${seatName(seat)} (${sideName(side)})`;
	};
</script>

<svelte:head>
	<title>Game review · Chess</title>
</svelte:head>

{#if board && analysis && snapshot}
	<div class="review board-page">
		<Board game={board} {orientation} {arrows} onmove={() => 'illegal'} />
		<aside class="sidebar">
			<h1>Game review</h1>
			<p class="players">
				{[
					`${player('white')} vs ${player('black')}`,
					snapshot.ended ? gameEndText(snapshot.ended) : null
				]
					.filter(Boolean)
					.join(' · ')}
			</p>

			{#if analysis.status === 'running' || analysis.status === 'idle'}
				<div class="progress panel" data-testid="review-progress">
					<p role="status">
						Stockfish is going through the game: <span class="notation"
							>{analysis.done} of {analysis.total}</span
						> positions.
					</p>
					<progress max={analysis.total} value={analysis.done}></progress>
					<button type="button" class="btn" onclick={() => analysis?.stop()}>Stop</button>
				</div>
			{:else if analysis.status === 'stopped'}
				<div class="progress panel">
					<p role="status">
						Stopped after <span class="notation">{analysis.done} of {analysis.total}</span> positions.
						What's done is kept.
					</p>
					<button type="button" class="btn primary" onclick={() => analysis?.start()}
						>Carry on</button
					>
				</div>
			{:else if analysis.status === 'failed'}
				<div class="progress panel">
					<p class="error" role="alert">{analysis.error}</p>
					<button type="button" class="btn primary" onclick={() => analysis?.start()}
						>Try again</button
					>
				</div>
			{:else if review}
				{#if yours}
					<div class="sides" role="group" aria-label="Whose moves">
						<button type="button" class="btn" aria-pressed={mineOnly} onclick={() => showSide(true)}
							>Your moves</button
						>
						<button
							type="button"
							class="btn"
							aria-pressed={!mineOnly}
							onclick={() => showSide(false)}>Both sides</button
						>
					</div>
				{/if}
				{#if review.swings.length === 0}
					<p class="empty" data-testid="no-swings">
						No big swings{mineOnly ? ' in your moves' : ''}: no move gave much away while the game
						was still open.
					</p>
				{:else}
					<ol class="swings" data-testid="swings">
						{#each review.swings as s, i (s.ply)}
							<li class:selected={i === selected}>
								<button
									type="button"
									class="swing"
									aria-pressed={i === selected}
									onclick={() => (selected = i)}
								>
									<span class="played notation">{s.played}</span>
									<span class="evals notation"
										>{formatScore(s.before)} → {formatScore(s.after)}</span
									>
									<span class="chances">
										{sideName(s.mover)}'s winning chances {percent(s.beforeChance)} → {percent(
											s.afterChance
										)}
									</span>
								</button>
								<p class="better">
									Stockfish preferred <span class="notation" data-testid="better-line"
										>{s.best}</span
									>
								</p>
								<a
									class="open"
									href={withQuery(resolve('/analysis'), {
										pgn: review.pgn,
										ply: String(s.ply - 1),
										orientation
									})}>Open on the analysis board</a
								>
							</li>
						{/each}
					</ol>
				{/if}
			{/if}
			<a class="back" href={resolve('/games')}>All my games</a>
		</aside>
	</div>
{:else}
	<section class="review-page">
		<div class="page-header">
			<h1>Game review</h1>
		</div>
		{#if loadError}
			<p class="error" role="alert">Could not load the game: {loadError}</p>
		{:else if snapshot}
			<p class="empty">
				{#if snapshot.ended === null}
					This game isn't over yet. Its review will be here once it has finished.
				{:else}
					There is nothing to review: the game ended before it began.
				{/if}
			</p>
			<a class="back" href={resolve('/game/[id]', { id })}>Go to the game</a>
		{:else}
			<p class="empty">Loading…</p>
		{/if}
	</section>
{/if}

<style>
	h1 {
		margin: 0;
		font-size: var(--type-xl);
	}

	.players {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.progress {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.progress p {
		margin: 0;
	}

	.progress .btn {
		align-self: flex-start;
	}

	progress {
		width: 100%;
		accent-color: var(--accent);
	}

	.sides {
		display: flex;
		gap: 0.5rem;
	}

	.sides .btn {
		flex: 1;
	}

	.sides .btn[aria-pressed='true'] {
		border-color: var(--accent);
		background: var(--panel-raised);
	}

	.swings {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.swings li {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		padding: 0.6rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: var(--radius);
	}

	.swings li.selected {
		border-color: var(--accent);
	}

	.swing {
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 0.15rem 0.75rem;
		min-height: var(--tap);
		padding: 0;
		border: 0;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.played {
		font-weight: 600;
		font-size: var(--type-lg);
	}

	.evals {
		align-self: center;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.chances {
		grid-column: 1 / -1;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.better {
		margin: 0;
		/* Engine lines don't break; let them wrap between moves instead of widening the page. */
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.open,
	.back {
		display: inline-flex;
		align-items: center;
		min-height: var(--tap);
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.open:hover,
	.back:hover {
		color: var(--text);
	}

	.empty {
		margin: 0;
		color: var(--text-muted);
	}

	.error {
		margin: 0;
		color: var(--danger);
	}

	.review-page {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-width: var(--measure);
	}

	.review-page .page-header {
		margin-bottom: 0;
	}
</style>
