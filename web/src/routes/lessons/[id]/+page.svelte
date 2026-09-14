<script lang="ts">
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import Board from '$lib/components/Board.svelte';
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
			<div class="actions">
				<button type="button" onclick={() => session.restart()}>Restart</button>
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
		border-color: #e06c75;
	}

	.error {
		margin: 0;
		color: #e06c75;
		font-size: 0.85rem;
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
		border-radius: 6px;
		background: var(--panel);
		color: var(--text);
		font: inherit;
		text-decoration: none;
		cursor: pointer;
	}

	.actions a.primary {
		background: var(--accent);
		border-color: var(--accent);
		color: #fff;
	}

	.back {
		color: var(--text-muted);
		font-size: 0.9rem;
	}
</style>
