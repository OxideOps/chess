<script lang="ts">
	import { page } from '$app/state';
	import { CATEGORY_NAMES, formatRating } from '$lib/online/ratings';
	import type { PlayerProfile } from '$lib/generated/PlayerProfile';

	// A player's ratings per category.
	const username = $derived(page.params.username ?? '');
	let profile: PlayerProfile | null = $state(null);
	let missing = $state(false);
	let error: string | null = $state(null);

	$effect(() => {
		const name = username;
		let cancelled = false;
		profile = null;
		missing = false;
		error = null;
		fetch(`/api/players/${encodeURIComponent(name)}`)
			.then(async (response) => {
				if (cancelled) return;
				if (response.status === 404) missing = true;
				else if (!response.ok) throw new Error(`the server said ${response.status}`);
				else profile = (await response.json()) as PlayerProfile;
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
	<title>{profile?.username ?? username} · Chess</title>
</svelte:head>

<section class="player">
	{#if missing}
		<h1>No such player</h1>
		<p class="muted">Nobody plays here as {username}.</p>
	{:else if error}
		<p class="error" role="alert">Could not load {username}: {error}</p>
	{:else if profile === null}
		<p class="muted">Loading…</p>
	{:else}
		<h1>{profile.username}</h1>
		{#if profile.ratings.length === 0}
			<p class="muted">No rated games yet.</p>
		{:else}
			<table>
				<thead>
					<tr><th>Category</th><th>Rating</th><th>Games</th></tr>
				</thead>
				<tbody>
					{#each profile.ratings as r (r.category)}
						<tr data-testid="rating-{r.category}">
							<td>{CATEGORY_NAMES[r.category]}</td>
							<td class="num">{formatRating({ value: r.rating, provisional: r.provisional })}</td>
							<td class="num">{r.games}</td>
						</tr>
					{/each}
				</tbody>
			</table>
			{#if profile.ratings.some((r) => r.provisional)}
				<p class="muted note">
					A "?" means the rating is still provisional: a few more games settle it.
				</p>
			{/if}
		{/if}
	{/if}
</section>

<style>
	.player {
		max-width: 28rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	h1 {
		margin: 0;
	}

	.muted {
		margin: 0;
		color: var(--text-muted);
	}

	.note {
		font-size: 0.85rem;
	}

	.error {
		margin: 0;
		color: #e06c75;
	}

	table {
		border-collapse: collapse;
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		overflow: hidden;
	}

	th,
	td {
		padding: 0.5rem 0.9rem;
		text-align: left;
		border-bottom: 1px solid var(--panel-border);
	}

	th {
		color: var(--text-muted);
		font-weight: 400;
		font-size: 0.85rem;
	}

	tr:last-child td {
		border-bottom: none;
	}

	.num {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
</style>
