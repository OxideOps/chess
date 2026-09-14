<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { safeNext, withNext } from '$lib/auth/next';

	// Sign-up and log-in are the same form with a different verb. On success
	// the visitor goes back to `?next=` (the game they were about to join) or home.
	interface Props {
		mode: 'signup' | 'login';
	}
	let { mode }: Props = $props();

	let username = $state('');
	let password = $state('');
	let error: string | null = $state(null);
	let busy = $state(false);

	const verb = $derived(mode === 'signup' ? 'Sign up' : 'Log in');
	const next = $derived(safeNext(page.url.searchParams.get('next')));
	const otherHref = $derived(withNext(resolve(mode === 'signup' ? '/login' : '/signup'), next));

	async function submit() {
		busy = true;
		error = null;
		try {
			if (mode === 'signup') await session.signup(username, password);
			else await session.login(username, password);
			await goto(next);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

<section class="auth">
	<h1>{verb}</h1>
	{#if mode === 'signup' && session.user && !session.registered}
		<p class="note">You are playing as a guest. Sign up to keep your games and your name.</p>
	{/if}
	<form
		onsubmit={(event) => {
			event.preventDefault();
			submit();
		}}
	>
		<label>
			Username
			<input
				name="username"
				bind:value={username}
				autocomplete="username"
				required
				minlength="3"
				maxlength="20"
				pattern="[A-Za-z0-9_]+"
				title="3–20 letters, digits or underscores"
			/>
		</label>
		<label>
			Password
			<input
				name="password"
				type="password"
				bind:value={password}
				autocomplete={mode === 'signup' ? 'new-password' : 'current-password'}
				required
				minlength="8"
				maxlength="128"
			/>
		</label>
		<button type="submit" disabled={busy}>{busy ? `${verb}…` : verb}</button>
	</form>
	{#if error}
		<p class="error" role="alert">{error}</p>
	{/if}
	<p class="switch">
		{#if mode === 'signup'}
			Already have an account? <a href={otherHref}>Log in</a>
		{:else}
			No account yet? <a href={otherHref}>Sign up</a>
		{/if}
	</p>
</section>

<style>
	.auth {
		max-width: 22rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	h1 {
		margin: 0;
	}

	.note,
	.switch {
		margin: 0;
		color: var(--text-muted);
		font-size: 0.9rem;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.85rem;
		color: var(--text-muted);
	}

	input,
	button {
		padding: 0.45rem 0.8rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--panel);
		color: var(--text);
		font: inherit;
	}

	button {
		align-self: flex-start;
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

	a {
		color: var(--text);
	}
</style>
