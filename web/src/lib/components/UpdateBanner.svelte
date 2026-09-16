<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { page } from '$app/state';
	import { updates } from '$lib/pwa/updates.svelte';

	// "A new version is available": shown when a deploy is downloaded and
	// waiting. Never reloads by itself, and stays out of the way during an
	// online game (it shows again afterwards, on any other page).
	onMount(() => {
		void updates.start();
		const check = () => {
			if (document.visibilityState === 'visible') void updates.check();
		};
		document.addEventListener('visibilitychange', check);
		return () => document.removeEventListener('visibilitychange', check);
	});
	onDestroy(() => updates.stop());

	const inGame = $derived(page.route.id === '/game/[id]');
</script>

{#if updates.available && !inGame}
	<div class="update" role="status">
		<span>A new version of the site is available.</span>
		<button type="button" onclick={() => updates.apply()}>Reload</button>
	</div>
{/if}

<style>
	.update {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-wrap: wrap;
		gap: 0.75rem;
		padding: 0.4rem var(--gutter);
		background: var(--panel);
		border-bottom: 1px solid var(--accent);
		font-size: var(--type-sm);
	}

	button {
		min-height: var(--tap);
		padding: 0.25rem 0.8rem;
		border: 1px solid var(--accent);
		border-radius: var(--radius-sm);
		background: var(--accent);
		color: var(--on-accent);
		font: inherit;
		cursor: pointer;
	}
</style>
