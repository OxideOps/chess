<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { withNext } from '$lib/auth/next';
	import { Coach, MIN_DEPTH } from '$lib/coach/coach.svelte';
	import CoachAnswer, { type Arrow } from '$lib/components/CoachAnswer.svelte';
	import type { GameStore } from '$lib/chess/game.svelte';
	import type { Analyser } from '$lib/engine/analysis.svelte';

	// "Explain this position": the coach talks through the engine's lines.
	// Moves in the answer preview as an arrow (`onpreview`) and, clicked, play
	// their line from the explained position as a variation. The answer stays
	// up while the board is on such a line, with a way back. Hidden when the
	// server has no coach.
	interface Props {
		game: GameStore;
		analyser: Analyser;
		onpreview?: (arrow: Arrow | null) => void;
	}
	let { game, analyser, onpreview }: Props = $props();

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
	const answer = $derived(coach.key === fen ? coach.answer : coach.answerFor(fen));
	const busy = $derived(coach.busy && coach.key === fen);
	const error = $derived(coach.key === fen ? coach.error : null);
	const here = $derived(page.url.pathname);

	/** The explained position a move from its answer was played from. */
	let origin = $state<{ fen: string; node: number } | null>(null);
	const away = $derived(!answer && origin ? coach.answerFor(origin.fen) : null);

	function play(path: string[]) {
		const from = answer ? { fen, node: game.view.node } : origin;
		if (!from) return;
		game.goToNode(from.node);
		for (const uci of path) {
			if (game.playHereUci(uci) !== 'ok') break;
		}
		origin = from;
	}

	function back() {
		if (origin) game.goToNode(origin.node);
	}
</script>

{#if coach.available}
	<section class="coach" aria-label="Coach">
		{#if answer}
			<CoachAnswer {answer} testid="coach-text" {onpreview} onplay={play} />
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
		{#if away}
			<div class="away" data-testid="coach-away">
				<button type="button" class="back" onclick={back}>↩ Back to the explained position</button>
				<CoachAnswer answer={away} testid="coach-away-text" {onpreview} onplay={play} />
			</div>
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

	.away {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding-top: 0.5rem;
		border-top: 1px solid var(--panel-border);
		color: var(--text-muted);
	}

	.back {
		align-self: flex-start;
		border-color: var(--panel-border);
		background: none;
		color: var(--text);
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
