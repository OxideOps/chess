<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import Board from '$lib/components/Board.svelte';
	import { session } from '$lib/auth/session.svelte';
	import { sideName } from '$lib/chess/status';
	import { formatDiff, formatRating } from '$lib/online/ratings';
	import { PuzzleSession } from '$lib/puzzles/session.svelte';

	// Puzzles from the Lichess database, near your puzzle rating.
	const puzzles = new PuzzleSession();
	const game = puzzles.game;
	onMount(async () => {
		try {
			await session.ensure(); // puzzle ratings belong to a session, guests included
		} catch (e) {
			puzzles.phase = 'error';
			puzzles.error = e instanceof Error ? e.message : String(e);
			return;
		}
		await puzzles.next();
	});
	onDestroy(() => puzzles.dispose());

	const done = $derived(puzzles.phase === 'solved' || puzzles.phase === 'failed');
	const status = $derived.by(() => {
		switch (puzzles.phase) {
			case 'loading':
				return 'Loading a puzzle…';
			case 'setup':
				return game.view.plyCount > 1 ? 'Correct! Keep going…' : 'Your opponent moves…';
			case 'solving':
				return game.view.plyCount > 1
					? 'Correct! Find the next move.'
					: `Find the best move for ${sideName(puzzles.solver)}.`;
			case 'solved':
				return puzzles.expected ? 'That was the solution.' : 'Solved!';
			case 'failed':
				return `Not quite: the move was ${puzzles.expected}.`;
			case 'empty':
				return 'No puzzles to serve right now.';
			case 'error':
				return `Could not load a puzzle: ${puzzles.error ?? 'unknown error'}`;
		}
	});
	const yourRating = $derived(puzzles.result?.rating ?? puzzles.puzzle?.your_rating ?? null);
</script>

<svelte:head>
	<title>Puzzles · Chess</title>
</svelte:head>

<div class="puzzles board-page">
	<Board
		{game}
		orientation={puzzles.solver}
		playAs={puzzles.solver}
		onmove={(from, to, promotion) => puzzles.tryMove(from, to, promotion)}
	/>
	<aside class="sidebar">
		<p
			class="status"
			class:good={puzzles.phase === 'solved' && !puzzles.expected}
			class:bad={puzzles.phase === 'failed'}
			data-testid="puzzle-status"
			role="status"
		>
			{status}
		</p>
		{#if yourRating}
			<p class="rating">
				Your puzzle rating
				<strong data-testid="puzzle-rating">{formatRating(yourRating)}</strong>
				{#if puzzles.result?.counted}
					<span
						class="diff"
						class:up={puzzles.result.diff > 0}
						class:down={puzzles.result.diff < 0}
						data-testid="puzzle-diff">{formatDiff(puzzles.result.diff)}</span
					>
				{/if}
			</p>
		{/if}
		{#if done && puzzles.puzzle}
			<p class="about">
				Puzzle rated {puzzles.puzzle.rating}{#if puzzles.puzzle.themes.length > 0}
					· {puzzles.puzzle.themes.join(', ')}{/if}
			</p>
		{/if}
		{#if puzzles.error && puzzles.phase !== 'error'}
			<p class="error" role="alert">{puzzles.error}</p>
		{/if}
		<div class="actions">
			{#if puzzles.phase === 'failed'}
				<button type="button" class="btn" onclick={() => puzzles.showSolution()}
					>Show solution</button
				>
			{/if}
			{#if done || puzzles.phase === 'error'}
				<button type="button" class="btn primary" onclick={() => puzzles.next()}>Next puzzle</button
				>
			{/if}
		</div>
		<p class="credit">
			Puzzles from the <a href="https://database.lichess.org/#puzzles" rel="external"
				>Lichess puzzle database</a
			> (CC0).
		</p>
	</aside>
</div>

<style>
	.status.good {
		border-color: var(--accent);
	}

	.status.bad {
		border-color: var(--danger);
	}

	.rating,
	.about,
	.credit {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.rating strong {
		color: var(--text);
		font-variant-numeric: tabular-nums;
	}

	.diff {
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}

	.diff.up {
		color: var(--accent);
	}

	.diff.down {
		color: var(--danger);
	}

	.credit {
		font-size: var(--type-xs);
	}

	.credit a {
		color: var(--text-muted);
	}

	.error {
		margin: 0;
		color: var(--danger);
		font-size: var(--type-sm);
	}

	.actions {
		display: flex;
		gap: 0.5rem;
	}

	.actions .btn {
		flex: 1;
	}
</style>
