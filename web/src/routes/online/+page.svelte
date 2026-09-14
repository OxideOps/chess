<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { TIME_CONTROLS } from '$lib/online/clock';
	import { session } from '$lib/auth/session.svelte';
	import { withNext } from '$lib/auth/next';

	// Create a game on the server and go to it as White; the page then shows
	// the link to send to the opponent. A guest session is started if needed.
	let selected = $state(2); // 5+0
	// Rated games need an account; offer them to those who have one.
	let rated = $state(session.registered);
	let error: string | null = $state(null);
	let busy = $state(false);

	async function create() {
		busy = true;
		error = null;
		const tc = TIME_CONTROLS[selected];
		try {
			await session.ensure();
			const response = await fetch('/api/games', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({
					initial_ms: tc.initialMs,
					increment_ms: tc.incrementMs,
					rated: rated && session.registered
				})
			});
			if (!response.ok)
				throw new Error((await response.text()) || `server said ${response.status}`);
			const created: { id: string } = await response.json();
			await goto(resolve('/game/[id]', { id: created.id }));
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head>
	<title>Play online · Chess</title>
</svelte:head>

<section class="lobby">
	<h1>Play online</h1>
	<p>Create a game, then send the link to whoever you want to play. You play White.</p>
	<form
		onsubmit={(event) => {
			event.preventDefault();
			create();
		}}
	>
		<label>
			Time control
			<select bind:value={selected}>
				{#each TIME_CONTROLS as tc, i (tc.label)}
					<option value={i}>{tc.label}</option>
				{/each}
			</select>
		</label>
		<label class="check" class:disabled={!session.registered}>
			<input type="checkbox" bind:checked={rated} disabled={!session.registered} />
			Rated
		</label>
		<button type="submit" disabled={busy}>{busy ? 'Creating…' : 'Create game'}</button>
	</form>
	{#if !session.registered}
		<p class="hint">
			Rated games need an account: <a href={withNext(resolve('/login'), resolve('/online'))}
				>log in</a
			>
			or <a href={withNext(resolve('/signup'), resolve('/online'))}>sign up</a>.
		</p>
	{/if}
	{#if error}
		<p class="error" role="alert">Could not create the game: {error}</p>
	{/if}
</section>

<style>
	.lobby {
		max-width: 32rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	h1 {
		margin: 0;
	}

	.check {
		flex-direction: row;
		align-items: center;
		gap: 0.4rem;
		min-height: var(--tap);
		align-self: center;
		color: var(--text);
		font-size: 0.95rem;
	}

	.check.disabled {
		opacity: 0.5;
	}

	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: 0.9rem;
	}

	.hint a {
		color: var(--text);
	}

	form {
		display: flex;
		gap: 0.75rem;
		align-items: flex-end;
		flex-wrap: wrap;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.85rem;
		color: var(--text-muted);
	}

	select,
	button {
		min-height: var(--tap);
		padding: 0.45rem 0.8rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--panel);
		color: var(--text);
		font: inherit;
	}

	button {
		cursor: pointer;
		background: var(--accent);
		border-color: var(--accent);
		color: #fff;
	}

	button:disabled {
		opacity: 0.6;
		cursor: default;
	}

	.error {
		margin: 0;
		color: #e06c75;
	}
</style>
