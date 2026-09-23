<script lang="ts">
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { gameEndText } from '$lib/chess/status';
	import { OUTCOME_TEXT, matchup, outcome, relativeTime, reviewable } from '$lib/online/listing';
	import { formatDiff } from '$lib/online/ratings';
	import { withQuery } from '$lib/review/links';
	import type { GameListing } from '$lib/generated/GameListing';

	// Every game the signed-in user has a seat in, newest activity first.
	let games: GameListing[] | null = $state(null);
	let error: string | null = $state(null);

	$effect(() => {
		if (!session.user) return;
		let cancelled = false;
		fetch('/api/me/games')
			.then(async (response) => {
				if (!response.ok) throw new Error(`the server said ${response.status}`);
				const rows = (await response.json()) as GameListing[];
				if (!cancelled) games = rows;
			})
			.catch((e: unknown) => {
				if (!cancelled) error = e instanceof Error ? e.message : String(e);
			});
		return () => {
			cancelled = true;
		};
	});
</script>

<svelte:head>
	<title>My games · Chess</title>
</svelte:head>

<div class="page-header">
	<h1>My games</h1>
	<p>Every game you have a seat in. Review a finished one to see where it turned.</p>
</div>

<section class="games">
	{#if !session.user}
		<p class="empty">
			<a href={resolve('/login')}>Log in</a> to see your games, or
			<a href={resolve('/online')}>start one</a>.
		</p>
	{:else if error}
		<p class="error" role="alert">Could not load your games: {error}</p>
	{:else if games === null}
		<p class="empty">Loading…</p>
	{:else if games.length === 0}
		<p class="empty">
			No games yet. <a href={resolve('/online')}>Start one</a> and send the link to a friend.
		</p>
	{:else}
		<ul>
			{#each games as game (game.id)}
				{@const state = outcome(game)}
				<li>
					<a class="game" href={resolve('/game/[id]', { id: game.id })}>
						<span class="matchup">{matchup(game)}</span>
						<span class="outcome {state}">{OUTCOME_TEXT[state]}</span>
						<span class="detail">
							{#if game.ended}
								{gameEndText(game.ended)} ·
							{/if}
							{#if game.rated}
								Rated{#if game.rating_diffs}
									<span data-testid="your-diff"
										>{formatDiff(game.rating_diffs[game.your_color])}</span
									>{/if} ·
							{/if}
							{game.moves}
							{game.moves === 1 ? 'ply' : 'plies'} · {relativeTime(game.updated_at)}
						</span>
					</a>
					{#if reviewable(game)}
						<a
							class="review"
							href={withQuery(resolve('/games/[id]/review', { id: game.id }), {
								side: game.your_color
							})}>Review</a
						>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.games {
		max-width: var(--measure);
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.empty {
		margin: 0;
		color: var(--text-muted);
	}

	.error {
		margin: 0;
		color: var(--danger);
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--panel-border);
	}

	li {
		display: flex;
		align-items: stretch;
		border-bottom: 1px solid var(--panel-border);
	}

	a.game {
		flex: 1;
		min-width: 0;
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 0.15rem 1rem;
		min-height: var(--tap);
		padding: 0.7rem 0.5rem;
		color: var(--text);
		text-decoration: none;
	}

	li a:hover {
		background: var(--panel);
	}

	a.review {
		display: flex;
		align-items: center;
		min-height: var(--tap);
		padding: 0 0.75rem;
		border-left: 1px solid var(--panel-border);
		color: var(--text-muted);
		font-size: var(--type-sm);
		text-decoration: none;
	}

	a.review:hover {
		color: var(--text);
	}

	.matchup {
		font-weight: 600;
	}

	.outcome {
		grid-row: span 2;
		align-self: center;
		font-size: var(--type-sm);
		color: var(--text-muted);
	}

	.outcome.won {
		color: var(--good);
	}

	.outcome.lost {
		color: var(--danger);
	}

	.detail {
		font-size: var(--type-sm);
		font-variant-numeric: tabular-nums;
		color: var(--text-muted);
	}

	.empty a {
		color: var(--text);
	}
</style>
