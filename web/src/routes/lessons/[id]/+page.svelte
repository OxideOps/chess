<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { session as account } from '$lib/auth/session.svelte';
	import { withNext } from '$lib/auth/next';
	import { Coach, mistakeKey } from '$lib/coach/coach.svelte';
	import { Prober } from '$lib/coach/probe';
	import Board from '$lib/components/Board.svelte';
	import CoachAnswer, { type Arrow } from '$lib/components/CoachAnswer.svelte';
	import CoachFollowUps from '$lib/components/CoachFollowUps.svelte';
	import { drills } from '$lib/chess/wasm';
	import { sideName } from '$lib/chess/status';
	import { Opponent } from '$lib/engine/opponent.svelte';
	import { DrillSession } from '$lib/lessons/drill.svelte';

	const all = drills();
	const index = all.findIndex((d) => d.id === page.params.id);
	const drill = index >= 0 ? all[index] : null;
	const next = index >= 0 ? (all[index + 1] ?? null) : null;

	const session = drill ? new DrillSession(drill, new Opponent()) : null;
	void session?.start();
	onDestroy(() => session?.dispose());

	// The coach, for "why was that a mistake?" (hidden if the server has none).
	const coach = new Coach();
	onMount(() => {
		void coach.load();
	});
	// Stockfish for moves a follow-up asks about (not the drill's opponent).
	const prober = new Prober();
	onDestroy(() => prober.dispose());
	const mistake = $derived(session?.mistake ?? null);
	const key = $derived(mistake ? mistakeKey(mistake) : null);
	const answer = $derived(key ? coach.answerFor(key) : null);
	const asking = $derived(coach.busy && coach.key === key);
	const coachError = $derived(key && coach.key === key ? coach.error : null);
	// A move in the coach's answer, shown on the board (hovered, or tapped).
	let preview: Arrow | null = $state(null);

	const plural = (n: number) => `${n} ${n === 1 ? 'move' : 'moves'}`;
	const goal = $derived.by(() => {
		if (!drill) return '';
		const n = plural(drill.moves);
		switch (drill.goal) {
			case 'checkmate':
				return `Goal: checkmate within ${n}, playing ${sideName(drill.student)}.`;
			case 'promote':
				return `Goal: promote the pawn within ${n}, playing ${sideName(drill.student)}.`;
			case 'draw':
				return `Goal: hold the draw for ${n}, playing ${sideName(drill.student)}.`;
		}
	});
	const status = $derived.by(() => {
		if (!session) return '';
		const s = session.status;
		if (s.state === 'won') return s.reason;
		if (s.state === 'lost') return s.reason;
		if (session.thinking) return 'Stockfish is thinking…';
		return `Your move · ${plural(s.moves_left)} left`;
	});
</script>

<svelte:head>
	<title>{drill?.title ?? 'Lesson'} · Chess</title>
</svelte:head>

{#if drill && session}
	<div class="drill board-page">
		<Board
			game={session.game}
			orientation={drill.student}
			playAs={drill.student}
			onmove={(from, to, promotion) => session.tryMove(from, to, promotion)}
			arrows={preview ? [preview] : []}
		/>
		<aside class="sidebar">
			<h1>{drill.title}</h1>
			<p class="lesson">{drill.lesson}</p>
			<p class="goal">{goal}</p>
			<p
				class="status"
				class:won={session.status.state === 'won'}
				class:lost={session.status.state === 'lost'}
				data-testid="drill-status"
				role="status"
			>
				{status}
			</p>
			{#if session.error}
				<p class="error" role="alert">{session.error}</p>
			{/if}
			{#if mistake && drill}
				<div class="mistake" data-testid="mistake">
					<p>
						<strong>{mistake.playedSan}</strong> was a mistake: Stockfish preferred
						<strong>{mistake.betterSan}</strong>.
					</p>
					{#if answer}
						<CoachAnswer {answer} testid="mistake-coach" onpreview={(a) => (preview = a)} />
						{#if answer.thread && key}
							<CoachFollowUps {coach} {key} probe={prober.probe} onpreview={(a) => (preview = a)} />
						{/if}
					{:else if coach.available && account.registered}
						<button
							type="button"
							disabled={asking}
							onclick={() => coach.explainMistake(mistake, drill.id)}
						>
							{asking ? 'The coach is thinking…' : 'Why was that a mistake?'}
						</button>
					{:else if coach.available}
						<p class="hint">
							<a href={withNext(resolve('/signup'), page.url.pathname)}>Sign up</a> or
							<a href={withNext(resolve('/login'), page.url.pathname)}>log in</a> to ask the coach why.
						</p>
					{/if}
					{#if coachError}
						<p class="error" role="alert">{coachError}</p>
					{/if}
				</div>
			{/if}
			<div class="actions">
				<button
					type="button"
					onclick={() => {
						preview = null;
						void session.restart();
					}}>Restart</button
				>
				{#if session.status.state === 'won' && next}
					<a class="primary" href={resolve('/lessons/[id]', { id: next.id })} data-sveltekit-reload
						>Next: {next.title}</a
					>
				{/if}
			</div>
			<a class="back" href={resolve('/lessons')}>All lessons</a>
		</aside>
	</div>
{:else}
	<section>
		<h1>No such lesson</h1>
		<p><a href={resolve('/lessons')}>See all lessons</a></p>
	</section>
{/if}

<style>
	h1 {
		margin: 0;
		font-size: 1.4rem;
	}

	.lesson,
	.goal {
		margin: 0;
		line-height: 1.45;
	}

	.goal {
		color: var(--text-muted);
	}

	.status.won {
		border-color: var(--accent);
	}

	.status.lost {
		border-color: var(--danger);
	}

	.error {
		margin: 0;
		color: var(--danger);
		font-size: var(--type-sm);
	}

	.mistake {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.6rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--warning);
		border-radius: var(--radius-sm);
	}

	.mistake p {
		margin: 0;
		line-height: 1.45;
	}

	.mistake .hint {
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.mistake .hint a {
		color: var(--text);
	}

	.mistake button {
		align-self: flex-start;
		min-height: var(--tap);
		padding: 0.35rem 0.8rem;
		border: 1px solid var(--accent);
		border-radius: var(--radius-sm);
		background: var(--accent);
		color: var(--on-accent);
		font: inherit;
		cursor: pointer;
	}

	.mistake button:disabled {
		opacity: 0.6;
		cursor: default;
	}

	.actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.actions button,
	.actions a {
		flex: 1;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: var(--tap);
		padding: 0.45rem 0.8rem;
		border: 1px solid var(--panel-border);
		border-radius: var(--radius-sm);
		background: var(--panel);
		color: var(--text);
		font: inherit;
		text-decoration: none;
		cursor: pointer;
	}

	.actions a.primary {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--on-accent);
	}

	.back {
		color: var(--text-muted);
		font-size: var(--type-sm);
	}
</style>
