<script lang="ts">
	import type { Snippet } from 'svelte';
	import Board from '$lib/components/Board.svelte';
	import { sideName } from '$lib/chess/status';
	import { formatDiff, formatRating } from '$lib/online/ratings';
	import type { PuzzleSession } from '$lib/puzzles/session.svelte';

	// The board and sidebar every puzzle page shares: the status line, the
	// rating and streak, and the actions. Pages add their own bits around it.
	interface Props {
		puzzles: PuzzleSession;
		/** What to say when the server has nothing to serve. */
		emptyText?: string;
		/** Above the status line (a picker, a heading). */
		top?: Snippet;
		/** Under what the puzzle was, once it is done. */
		notes?: Snippet;
		/** The action once a puzzle is done (or failed to load): "Next puzzle", say. */
		after: Snippet;
	}

	let {
		puzzles,
		emptyText = 'No puzzles to serve right now.',
		top,
		notes,
		after
	}: Props = $props();
	const game = $derived(puzzles.game);

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
				return emptyText;
			case 'error':
				return `Could not load a puzzle: ${puzzles.error ?? 'unknown error'}`;
		}
	});
	const yourRating = $derived(puzzles.result?.rating ?? puzzles.puzzle?.your_rating ?? null);
	const showing = $derived(puzzles.phase !== 'loading' && puzzles.phase !== 'empty');
</script>

<div class="puzzles board-page">
	<Board
		{game}
		orientation={puzzles.solver}
		playAs={puzzles.solver}
		onmove={(from, to, promotion) => puzzles.tryMove(from, to, promotion)}
	/>
	<aside class="sidebar">
		{@render top?.()}
		<p
			class="status"
			class:good={puzzles.phase === 'solved' && !puzzles.expected}
			class:bad={puzzles.phase === 'failed'}
			data-testid="puzzle-status"
			role="status"
		>
			{status}
		</p>
		{#if showing && puzzles.puzzle?.tried}
			<p class="note" data-testid="puzzle-tried">
				You have tried this puzzle before, so it won't change your rating or streak.
			</p>
		{/if}
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
		{#if puzzles.streak}
			<p class="rating">
				Streak <strong data-testid="puzzle-streak">{puzzles.streak.current}</strong>
				· best <strong data-testid="puzzle-best-streak">{puzzles.streak.best}</strong>
			</p>
		{/if}
		{#if done && puzzles.puzzle}
			<p class="note">
				Puzzle rated {puzzles.puzzle.rating}{#if puzzles.puzzle.themes.length > 0}
					· {puzzles.puzzle.themes.join(', ')}{/if}
			</p>
			{@render notes?.()}
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
				{@render after()}
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
	.note,
	.credit {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.rating strong {
		color: var(--text);
		font-family: var(--font-mono);
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

	.actions :global(.btn) {
		flex: 1;
	}
</style>
