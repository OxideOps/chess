<script lang="ts">
	import { onDestroy, onMount, untrack } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import PuzzleView from '$lib/components/PuzzleView.svelte';
	import { session } from '$lib/auth/session.svelte';
	import { formatDay, isDay } from '$lib/puzzles/daily';
	import { PuzzleSession } from '$lib/puzzles/session.svelte';

	// The daily puzzle: the same one for everyone, whatever their rating.
	// `?date=YYYY-MM-DD` shows an earlier day's. It rates like any other
	// puzzle: your first try at it counts, and only that.
	const puzzles = new PuzzleSession();
	const requested = $derived(page.url.searchParams.get('date'));

	// Load once there is a session, and again when the date in the URL
	// changes (the "Today's puzzle" link stays on this page).
	let ready = $state(false);
	onMount(async () => {
		try {
			await session.ensure(); // puzzle ratings belong to a session, guests included
		} catch (e) {
			puzzles.phase = 'error';
			puzzles.error = e instanceof Error ? e.message : String(e);
			return;
		}
		ready = true;
	});
	$effect(() => {
		if (!ready) return;
		const date = isDay(requested) ? requested : undefined;
		untrack(() => void puzzles.daily(date));
	});
	onDestroy(() => puzzles.dispose());

	const pastDay = $derived(isDay(requested) && puzzles.date === requested);
	const link = $derived(
		puzzles.date ? `${page.url.origin}${resolve('/puzzles/daily')}?date=${puzzles.date}` : null
	);
	const today = resolve('/puzzles/daily');

	let copied = $state(false);
	async function copyLink() {
		if (!link) return;
		await navigator.clipboard.writeText(link);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<svelte:head>
	<title>Daily puzzle · Chess</title>
</svelte:head>

<PuzzleView
	{puzzles}
	emptyText={isDay(requested)
		? 'No puzzle for that day: it is still to come, or there are no puzzles yet.'
		: 'No daily puzzle yet: no puzzles have been imported.'}
>
	{#snippet top()}
		<div class="heading">
			<h1>Daily puzzle</h1>
			{#if puzzles.date}
				<p class="day" data-testid="daily-date">{formatDay(puzzles.date)}</p>
			{/if}
		</div>
		{#if link}
			<div class="share">
				<input
					readonly
					value={link}
					aria-label="Link to this puzzle"
					data-testid="daily-link"
					onfocus={(e) => e.currentTarget.select()}
				/>
				<button type="button" class="btn" onclick={copyLink}
					>{copied ? 'Copied' : 'Copy link'}</button
				>
			</div>
		{/if}
		{#if pastDay}
			<p class="note"><a href={today}>Today's puzzle</a></p>
		{/if}
	{/snippet}
	{#snippet after()}
		<button type="button" class="btn primary" onclick={() => goto(resolve('/puzzles'))}
			>More puzzles</button
		>
	{/snippet}
</PuzzleView>

<style>
	.heading {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 0.75rem;
	}

	h1 {
		margin: 0;
		font-size: var(--type-lg);
	}

	.day,
	.note {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.share {
		display: flex;
		gap: 0.5rem;
	}

	.share input {
		flex: 1;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: var(--type-xs);
	}

	.note a {
		color: var(--text-muted);
	}
</style>
