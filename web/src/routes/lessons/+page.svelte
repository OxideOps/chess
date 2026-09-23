<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { withNext } from '$lib/auth/next';
	import { drills } from '$lib/chess/wasm';
	import { lessonProgress } from '$lib/lessons/progress.svelte';

	// Short drills against Stockfish, easiest first.
	const all = drills();
	// An account's progress comes from the server (merging in anything this
	// browser finished first); a guest's from this browser.
	onMount(() => {
		void lessonProgress.load();
	});
	const done = lessonProgress.done;
	const finished = $derived(all.filter((drill) => done.has(drill.id)).length);
</script>

<svelte:head>
	<title>Lessons · Chess</title>
</svelte:head>

<div class="page-header">
	<h1>Lessons</h1>
	<p>
		Short drills from set positions, played against Stockfish. Each one teaches something every
		player needs, easiest first: opening the game, the common tactics, the basic checkmates and the
		key king, pawn and rook endings.
	</p>
</div>

<!-- The drills run easiest first, so they are numbered and read as one route
     through the material rather than a shelf of separate cards. -->
<ol class="steps">
	{#each all as drill, i (drill.id)}
		<li class:done={done.has(drill.id)}>
			<a href={resolve('/lessons/[id]', { id: drill.id })} data-testid="lesson-{drill.id}">
				<span class="index notation" aria-hidden="true">{i + 1}</span>
				<span class="title">{drill.title}</span>
				<span class="summary">{drill.summary}</span>
				{#if done.has(drill.id)}
					<span class="tick" aria-label="completed">Done</span>
				{/if}
			</a>
		</li>
	{/each}
</ol>

<p class="progress notation" data-testid="lessons-done">{finished} of {all.length} done</p>
{#if !session.registered}
	<p class="hint">
		Kept in this browser.
		<a href={withNext(resolve('/signup'), resolve('/lessons'))}>Sign up</a> or
		<a href={withNext(resolve('/login'), resolve('/lessons'))}>log in</a> to keep it on every device.
	</p>
{/if}

<style>
	.steps {
		max-width: var(--measure);
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--panel-border);
	}

	li {
		border-bottom: 1px solid var(--panel-border);
	}

	.steps a {
		display: grid;
		grid-template-columns: 2.25rem 1fr auto;
		align-items: baseline;
		gap: 0.15rem 0.75rem;
		min-height: var(--tap);
		padding: 0.85rem 0.5rem;
		color: var(--text);
		text-decoration: none;
	}

	.steps a:hover {
		background: var(--panel);
	}

	/* The step's place in the sequence, not decoration: the drills are ordered. */
	.index {
		grid-row: 1 / span 2;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	li.done .index {
		color: var(--accent);
	}

	.title {
		font-weight: 600;
	}

	.summary {
		grid-column: 2;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.tick {
		grid-column: 3;
		grid-row: 1 / span 2;
		align-self: center;
		color: var(--accent);
		font-size: var(--type-sm);
	}

	.progress {
		margin: 1rem 0 0;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.hint {
		margin: 0.35rem 0 0;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.hint a {
		color: var(--text);
	}
</style>
