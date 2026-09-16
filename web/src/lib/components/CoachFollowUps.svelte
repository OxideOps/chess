<script lang="ts">
	import type { Coach } from '$lib/coach/coach.svelte';
	import type { Probe } from '$lib/coach/probe';
	import CoachAnswer, { type Arrow } from '$lib/components/CoachAnswer.svelte';

	// Follow-up questions under a coach answer: the thread so far, and a box
	// to ask the next one. A question naming a move the engine didn't cover
	// waits for Stockfish's look at it (`probe`) before the coach answers.
	interface Props {
		coach: Coach;
		/** The answer's key in `coach` (a FEN, or a `mistakeKey`). */
		key: string;
		probe: Probe;
		onpreview?: (arrow: Arrow | null) => void;
		onplay?: (path: string[]) => void;
	}
	let { coach, key, probe, onpreview, onplay }: Props = $props();

	let draft = $state('');
	const turns = $derived(coach.followUps(key));
	const left = $derived(coach.leftFor(key));
	const asking = $derived(coach.following?.key === key ? coach.following : null);

	async function ask(event: SubmitEvent) {
		event.preventDefault();
		const question = draft.trim();
		if (!question) return;
		draft = '';
		await coach.followUp(key, question, probe);
	}
</script>

<div class="follow-ups">
	{#each turns as turn, i (i)}
		<p class="question" data-testid="coach-question">{turn.question}</p>
		{#if turn.answer}
			<CoachAnswer answer={turn.answer} testid="coach-follow-up" {onpreview} {onplay} />
		{:else if turn.error}
			<p class="error" role="alert">{turn.error}</p>
		{:else if asking}
			<p class="hint" role="status">
				{asking.stage === 'engine'
					? `Stockfish is looking at ${asking.san}…`
					: 'The coach is thinking…'}
			</p>
		{/if}
	{/each}
	{#if left > 0}
		<form onsubmit={ask}>
			<input
				aria-label="Ask the coach a follow-up question"
				placeholder="Ask a follow-up, e.g. why not Nf6?"
				maxlength="300"
				bind:value={draft}
				disabled={coach.following !== null}
			/>
			<button type="submit" disabled={coach.following !== null || !draft.trim()}>Ask</button>
		</form>
		<p class="hint">{left} {left === 1 ? 'question' : 'questions'} left about this answer</p>
	{:else}
		<p class="hint">That's all the questions about this answer.</p>
	{/if}
</div>

<style>
	.follow-ups {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.question {
		margin: 0.3rem 0 0;
		padding-left: 0.6rem;
		border-left: 2px solid var(--accent);
		color: var(--text-muted);
		font-style: italic;
	}

	form {
		display: flex;
		gap: 0.4rem;
	}

	input {
		flex: 1;
		min-width: 0;
		min-height: var(--tap);
		padding: 0.4rem 0.5rem;
		border: 1px solid var(--panel-border);
		border-radius: var(--radius-sm);
		background: var(--bg);
		color: var(--text);
		font: inherit;
		font-size: var(--type-sm);
	}

	button {
		min-height: var(--tap);
		padding: 0.4rem 0.8rem;
		border: 1px solid var(--panel-border);
		border-radius: var(--radius-sm);
		background: var(--panel);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-xs);
	}

	.error {
		margin: 0;
		color: var(--danger);
		font-size: var(--type-sm);
	}
</style>
