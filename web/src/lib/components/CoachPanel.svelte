<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { withNext } from '$lib/auth/next';
	import { Coach, MIN_DEPTH } from '$lib/coach/coach.svelte';
	import type { GameStore } from '$lib/chess/game.svelte';
	import type { Analyser } from '$lib/engine/analysis.svelte';

	// "Explain this position": the coach talks through the engine's lines.
	// Hidden when the server has no coach.
	interface Props {
		game: GameStore;
		analyser: Analyser;
	}
	let { game, analyser }: Props = $props();

	const coach = new Coach();
	onMount(() => {
		void coach.load();
	});

	const fen = $derived(game.view.fen);
	const lastMove = $derived(
		game.view.cursor > 0 ? game.view.moves[game.view.cursor - 1].san : null
	);
	// Lines about this position, deep enough to say something.
	const ready = $derived(
		analyser.fen === fen && analyser.lines.length > 0 && (analyser.depth ?? 0) >= MIN_DEPTH
	);
	const answer = $derived(coach.key === fen ? coach.text : coach.answerFor(fen));
	const busy = $derived(coach.busy && coach.key === fen);
	const error = $derived(coach.key === fen ? coach.error : null);
	const here = $derived(page.url.pathname);
</script>

{#if coach.available}
	<section class="coach" aria-label="Coach">
		{#if answer}
			<p class="text" data-testid="coach-text">{answer}</p>
		{:else if !session.registered}
			<p class="hint">
				The coach explains positions in plain language.
				<a href={withNext(resolve('/signup'), here)}>Sign up</a> or
				<a href={withNext(resolve('/login'), here)}>log in</a> to ask it.
			</p>
		{:else}
			<button
				type="button"
				disabled={!ready || busy}
				onclick={() => coach.explain(fen, lastMove, analyser.lines)}
			>
				{busy ? 'The coach is thinking…' : 'Explain this position'}
			</button>
			{#if !ready && !busy}
				<p class="hint">Available once the engine has looked a little deeper.</p>
			{/if}
		{/if}
		{#if error}
			<p class="error" role="alert">{error}</p>
		{/if}
	</section>
{/if}

<style>
	.coach {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.6rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: 6px;
	}

	.text {
		margin: 0;
		line-height: 1.45;
		white-space: pre-line;
	}

	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: 0.85rem;
	}

	.hint a {
		color: var(--text);
	}

	.error {
		margin: 0;
		color: #e06c75;
		font-size: 0.85rem;
	}

	button {
		min-height: var(--tap);
		padding: 0.45rem 0.8rem;
		border: 1px solid var(--accent);
		border-radius: 6px;
		background: var(--accent);
		color: #fff;
		font: inherit;
		cursor: pointer;
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
