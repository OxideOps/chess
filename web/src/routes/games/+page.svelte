<script lang="ts">
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { gameEndText } from '$lib/chess/status';
	import { OUTCOME_TEXT, matchup, outcome, relativeTime } from '$lib/online/listing';
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

<section class="games">
	<h1>My games</h1>
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
					<a href={resolve('/game/[id]', { id: game.id })}>
						<span class="matchup">{matchup(game)}</span>
						<span class="outcome {state}">{OUTCOME_TEXT[state]}</span>
						<span class="detail">
							{#if game.ended}
								{gameEndText(game.ended)} ·
							{/if}
							{game.moves}
							{game.moves === 1 ? 'ply' : 'plies'} · {relativeTime(game.updated_at)}
						</span>
					</a>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.games {
		max-width: 40rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	h1 {
		margin: 0;
	}

	.empty {
		margin: 0;
		color: var(--text-muted);
	}

	.error {
		margin: 0;
		color: #e06c75;
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	li a {
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 0.15rem 1rem;
		padding: 0.6rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		color: var(--text);
		text-decoration: none;
	}

	li a:hover {
		border-color: var(--text-muted);
	}

	.matchup {
		font-weight: 600;
	}

	.outcome {
		grid-row: span 2;
		align-self: center;
		font-size: 0.85rem;
		color: var(--text-muted);
	}

	.outcome.won {
		color: var(--accent);
	}

	.outcome.lost {
		color: #e06c75;
	}

	.detail {
		font-size: 0.85rem;
		color: var(--text-muted);
	}

	.empty a {
		color: var(--text);
	}
</style>
