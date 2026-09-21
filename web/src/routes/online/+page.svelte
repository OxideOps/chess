<script lang="ts">
	import { onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { TIME_CONTROLS, timeControlLabel } from '$lib/online/clock';
	import { Lobby } from '$lib/online/lobby.svelte';
	import { notifier } from '$lib/notify/notifier.svelte';
	import { CATEGORY_NAMES, formatRating } from '$lib/online/ratings';
	import { session } from '$lib/auth/session.svelte';
	import { withNext } from '$lib/auth/next';
	import type { SeekInfo } from '$lib/generated/SeekInfo';

	// Two ways to get a game: take someone's seek (or post your own and wait),
	// or make a private game and send the link to a friend.
	let selected = $state(2); // 5+0
	// Rated games need an account; offer them to those who have one.
	let rated = $state(session.registered);
	let error: string | null = $state(null);
	let busy = $state(false);

	/**
	 * The time control of the game we are waiting on, remembered for the
	 * notification that says what was taken: the server's "your game
	 * started" says who, not what. Set when we offer or accept one, so it
	 * is only null if a game arrives without us asking for one.
	 */
	let awaiting: string | null = null;

	const lobby = new Lobby({
		onGame: async ({ id, opponent }) => {
			// Navigate first: the game page sets the title, and the alert
			// wants to flag the title it leaves behind.
			await goto(resolve('/game/[id]', { id }));
			void notifier.raise(
				{ kind: 'game-ready', opponent, timeControl: awaiting },
				resolve('/game/[id]', { id })
			);
		}
	});
	onDestroy(() => lobby.dispose());

	/**
	 * A seek needs a seat to sit in, so make a guest first if need be. The
	 * lobby socket said who we were when it connected, so a brand new guest
	 * needs it opened again before the server will take our word for it.
	 */
	async function withSession(act: () => void) {
		error = null;
		try {
			const knownAlready = session.user !== null;
			await session.ensure();
			if (!knownAlready) lobby.reauthenticate();
			act();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	function postSeek() {
		const tc = TIME_CONTROLS[selected];
		awaiting = timeControlLabel(tc.initialMs, tc.incrementMs);
		const wanted = rated && session.registered;
		withSession(() => lobby.post(tc.initialMs, tc.incrementMs, wanted));
	}

	/**
	 * Offering a game is the moment to ask about notifications: it is a
	 * click (browsers show the prompt for nothing else), and it is the one
	 * time the answer is worth something, because the next thing that
	 * happens is waiting. Asked once ever, whatever the answer — see
	 * `Notifier.ask`. It goes before the awaits in the handlers below: a
	 * prompt asked for after one is a prompt the browser has already
	 * decided not to show.
	 */
	function askAboutAlerts() {
		void notifier.ask();
	}

	function timeControlOf(seek: SeekInfo): string {
		return timeControlLabel(seek.initial_ms, seek.increment_ms);
	}

	/** Create a game to send a link for: the old way, still here. */
	async function createPrivate() {
		askAboutAlerts();
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

<div class="page-header">
	<h1>Play online</h1>
	<p>Take a game someone is offering, or offer one and wait for a taker.</p>
</div>

<section class="seeks">
	<h2>Open games</h2>
	{#if lobby.others.length === 0}
		<p class="empty" data-testid="no-seeks">
			{lobby.state === 'open'
				? 'Nobody is waiting for a game right now. Offer one below and you will be sent to the board as soon as someone takes it.'
				: 'Connecting to the lobby…'}
		</p>
	{:else}
		<ul>
			{#each lobby.others as seek (seek.id)}
				<li>
					<button
						type="button"
						class="seek"
						data-testid="seek"
						disabled={lobby.busy}
						onclick={() => {
							awaiting = timeControlOf(seek);
							withSession(() => lobby.accept(seek.id));
						}}
					>
						<span class="who">{seek.username ?? 'Guest'}</span>
						<span class="rating notation">{formatRating(seek.rating) ?? '—'}</span>
						<span class="time notation">{timeControlOf(seek)}</span>
						<span class="kind"
							>{seek.rated ? 'Rated' : 'Casual'} · {CATEGORY_NAMES[seek.category]}</span
						>
						<span class="go">Play</span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<section class="offer">
	<h2>Offer a game</h2>
	<form
		onsubmit={(event) => {
			event.preventDefault();
			askAboutAlerts();
			postSeek();
		}}
	>
		<label>
			Time control
			<select bind:value={selected} disabled={lobby.mine !== null}>
				{#each TIME_CONTROLS as tc, i (tc.label)}
					<option value={i}>{tc.label}</option>
				{/each}
			</select>
		</label>
		<label class="check" class:disabled={!session.registered}>
			<input
				type="checkbox"
				bind:checked={rated}
				disabled={!session.registered || lobby.mine !== null}
			/>
			Rated
		</label>
		{#if lobby.mine === null}
			<button type="submit" class="btn primary" disabled={lobby.busy} data-testid="post-seek">
				Offer a game
			</button>
		{:else}
			<button type="button" class="btn" data-testid="cancel-seek" onclick={() => lobby.cancel()}>
				Cancel
			</button>
		{/if}
	</form>
	{#if lobby.mine !== null}
		<p class="waiting" role="status" data-testid="waiting">
			Waiting for someone to take your game. Leave this page open — the offer goes when you do.
			{#if notifier.permission === 'granted'}
				You can work in another tab; we'll tell you.
			{/if}
		</p>
	{/if}
	{#if !session.registered}
		<p class="hint">
			Rated games need an account: <a href={withNext(resolve('/login'), resolve('/online'))}
				>log in</a
			>
			or <a href={withNext(resolve('/signup'), resolve('/online'))}>sign up</a>.
		</p>
	{/if}
	<p class="hint">
		Playing someone you know? <button
			type="button"
			class="link"
			onclick={createPrivate}
			disabled={busy}>{busy ? 'Creating…' : 'Create a private game'}</button
		> and send them the link.
	</p>
	{#if lobby.rejection}
		<p class="error" role="alert">{lobby.rejection}</p>
	{/if}
	{#if error}
		<p class="error" role="alert">Could not start the game: {error}</p>
	{/if}
</section>

<style>
	.seeks,
	.offer {
		max-width: var(--measure);
	}

	.offer {
		margin-top: 2rem;
	}

	h2 {
		font-size: var(--type-lg);
		margin-bottom: 0.5rem;
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--panel-border);
	}

	li {
		border-bottom: 1px solid var(--panel-border);
	}

	/* A row is one button: the whole thing is the target, on a phone too. */
	.seek {
		display: grid;
		grid-template-columns: 1fr auto auto auto;
		align-items: baseline;
		gap: 0.15rem 0.75rem;
		width: 100%;
		min-height: var(--tap);
		padding: 0.7rem 0.5rem;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.seek:hover:not(:disabled) {
		background: var(--panel);
	}

	.seek:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.who {
		font-weight: 600;
	}

	.rating,
	.time {
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.kind {
		grid-column: 1 / 4;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	/* "Play" sits on the right, across both rows of the entry. */
	.go {
		grid-column: 4;
		grid-row: 1 / span 2;
		align-self: center;
		color: var(--accent);
		font-weight: 600;
	}

	.empty {
		margin: 0;
		color: var(--text-muted);
	}

	.waiting {
		margin: 0;
		color: var(--accent);
	}

	.check {
		flex-direction: row;
		align-items: center;
		gap: 0.4rem;
		min-height: var(--tap);
		align-self: center;
		color: var(--text);
		font-size: var(--type-base);
	}

	.check.disabled {
		opacity: 0.5;
	}

	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.hint a,
	.link {
		color: var(--text);
	}

	/* A button that reads as part of the sentence it sits in. */
	.link {
		padding: 0;
		border: none;
		background: none;
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}

	.link:disabled {
		opacity: 0.6;
		cursor: default;
	}

	form {
		display: flex;
		gap: 0.75rem;
		align-items: flex-end;
		flex-wrap: wrap;
		margin-bottom: 0.75rem;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: var(--type-sm);
		color: var(--text-muted);
	}

	select {
		min-height: var(--tap);
		padding: 0.45rem 0.8rem;
		border: 1px solid var(--panel-border);
		border-radius: var(--radius-sm);
		background: var(--panel);
		color: var(--text);
		font: inherit;
	}

	.error {
		margin: 0;
		color: var(--danger);
	}
</style>
