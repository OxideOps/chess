<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import type { ResolvedPathname } from '$app/types';
	import PuzzleView from '$lib/components/PuzzleView.svelte';
	import { session } from '$lib/auth/session.svelte';
	import type { PuzzleTheme } from '$lib/generated/PuzzleTheme';
	import { PuzzleSession } from '$lib/puzzles/session.svelte';
	import { isThemeKey, themeName, themeOptions } from '$lib/puzzles/themes';

	// Puzzles from the Lichess database, near your puzzle rating, of one theme
	// if you pick one. The theme is in the URL (`?theme=fork`), so a drill
	// can be bookmarked or shared.
	const puzzles = new PuzzleSession();
	let themes: PuzzleTheme[] = $state([]);
	const options = $derived(themeOptions(themes));

	onMount(async () => {
		const wanted = page.url.searchParams.get('theme');
		puzzles.theme = isThemeKey(wanted) ? wanted : null;
		void loadThemes();
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

	async function loadThemes() {
		try {
			const response = await fetch('/api/puzzles/themes');
			if (response.ok) themes = (await response.json()) as PuzzleTheme[];
		} catch {
			// No picker then; the puzzles still work.
		}
	}

	async function pick(theme: string) {
		puzzles.theme = theme === '' ? null : theme;
		const base = resolve('/puzzles');
		const href = (
			puzzles.theme ? `${base}?theme=${encodeURIComponent(puzzles.theme)}` : base
		) as ResolvedPathname;
		await goto(href, { replaceState: true, keepFocus: true, noScroll: true });
		await puzzles.next();
	}

	const themeLabel = $derived(puzzles.theme ? themeName(puzzles.theme) : null);
	const emptyText = $derived(
		themeLabel
			? `No “${themeLabel}” puzzles left for you. Pick another theme.`
			: 'No puzzles to serve right now.'
	);
	// A theme can run dry near your rating: the server then widens the range
	// rather than serve nothing, and this says so.
	const farOff = $derived(
		puzzles.theme !== null &&
			puzzles.puzzle !== null &&
			Math.abs(puzzles.puzzle.rating - puzzles.puzzle.your_rating.value) > 250
	);
</script>

<svelte:head>
	<title>Puzzles · Chess</title>
</svelte:head>

<PuzzleView {puzzles} {emptyText}>
	{#snippet top()}
		<div class="top">
			<label class="theme">
				<span>Theme</span>
				<select
					data-testid="puzzle-theme"
					value={puzzles.theme ?? ''}
					onchange={(e) => pick(e.currentTarget.value)}
				>
					<option value="">All themes</option>
					{#if puzzles.theme && !options.some((o) => o.key === puzzles.theme)}
						<option value={puzzles.theme}>{themeLabel}</option>
					{/if}
					{#each options as option (option.key)}
						<option value={option.key}>{option.name} ({option.count})</option>
					{/each}
				</select>
			</label>
			<a class="daily" href={resolve('/puzzles/daily')}>Daily puzzle</a>
		</div>
	{/snippet}
	{#snippet notes()}
		{#if farOff}
			<p class="note" data-testid="puzzle-far-off">
				Few “{themeLabel}” puzzles are left near your rating, so this one was further off.
			</p>
		{/if}
	{/snippet}
	{#snippet after()}
		<button type="button" class="btn primary" onclick={() => puzzles.next()}>Next puzzle</button>
	{/snippet}
</PuzzleView>

<style>
	.top {
		display: flex;
		align-items: flex-end;
		gap: 0.75rem;
	}

	.theme {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 0.25rem;
		min-width: 0;
		color: var(--text-muted);
		font-size: var(--type-xs);
	}

	.theme select {
		min-height: var(--tap);
	}

	.daily {
		display: inline-flex;
		align-items: center;
		min-height: var(--tap);
		color: var(--text-muted);
		font-size: var(--type-sm);
		white-space: nowrap;
	}

	.note {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}
</style>
