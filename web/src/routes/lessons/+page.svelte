<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { drills } from '$lib/chess/wasm';
	import { completed } from '$lib/lessons/progress';

	// Short drills against Stockfish, easiest first.
	const all = drills();
	let done: Set<string> = $state(new Set());
	onMount(() => {
		done = completed();
	});
</script>

<svelte:head>
	<title>Lessons · Chess</title>
</svelte:head>

<section class="lessons">
	<h1>Lessons</h1>
	<p class="intro">
		Short drills from set positions, played against Stockfish. Each one teaches a technique every
		player needs: the basic checkmates and the key king and pawn endings.
	</p>
	<ol>
		{#each all as drill (drill.id)}
			<li>
				<a href={resolve('/lessons/[id]', { id: drill.id })} data-testid="lesson-{drill.id}">
					<span class="title">{drill.title}</span>
					{#if done.has(drill.id)}
						<span class="done" aria-label="completed">✓ Done</span>
					{/if}
					<span class="summary">{drill.summary}</span>
				</a>
			</li>
		{/each}
	</ol>
</section>

<style>
	.lessons {
		max-width: 40rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	h1 {
		margin: 0;
	}

	.intro {
		margin: 0;
		color: var(--text-muted);
	}

	ol {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	a {
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 0.15rem 1rem;
		padding: 0.7rem 0.9rem;
		min-height: var(--tap);
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		color: var(--text);
		text-decoration: none;
	}

	a:hover {
		border-color: var(--text-muted);
	}

	.title {
		grid-column: 1;
		font-weight: 600;
	}

	.done {
		grid-column: 2;
		grid-row: 1 / span 2;
		align-self: center;
		color: var(--accent);
		font-size: 0.85rem;
	}

	.summary {
		grid-column: 1;
		color: var(--text-muted);
		font-size: 0.9rem;
	}
</style>
