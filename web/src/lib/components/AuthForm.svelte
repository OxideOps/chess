<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { safeNext, withNext } from '$lib/auth/next';
	import { listProviders, startUrl } from '$lib/auth/providers';
	import type { ProviderInfo } from '$lib/generated/ProviderInfo';

	// Sign-up and log-in are the same form with a different verb. On success
	// the visitor goes back to `?next=` (the game they were about to join) or home.
	interface Props {
		mode: 'signup' | 'login';
	}
	let { mode }: Props = $props();

	let username = $state('');
	let password = $state('');
	// An OAuth round trip that failed comes back here with `?error=`.
	let error: string | null = $state(page.url.searchParams.get('error'));
	let busy = $state(false);
	let providers: ProviderInfo[] = $state([]);
	$effect(() => {
		listProviders().then((list) => (providers = list));
	});

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
		<button type="submit" class="btn primary" disabled={busy}>{busy ? `${verb}…` : verb}</button>
	</form>
	{#if error}
		<p class="error" role="alert">{error}</p>
	{/if}
	{#if providers.length > 0}
		<div class="providers">
			<span class="or">or</span>
			{#each providers as provider (provider.id)}
				<a class="provider" href={startUrl(provider, next)} rel="external">
					Continue with {provider.name}
				</a>
			{/each}
		</div>
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
		font-size: var(--type-sm);
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
		font-size: var(--type-sm);
		color: var(--text-muted);
	}

	.btn {
		align-self: flex-start;
	}

	input {
		align-self: flex-start;
		width: 100%;
		max-width: 22rem;
	}

	.error {
		margin: 0;
		color: var(--danger);
	}

	a {
		color: var(--text);
	}

	.providers {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.or {
		color: var(--text-muted);
		font-size: var(--type-sm);
	}

	.provider {
		min-height: var(--tap);
		padding: 0.45rem 0.8rem;
		border: 1px solid var(--panel-border);
		border-radius: var(--radius-sm);
		background: var(--panel);
		color: var(--text);
		text-decoration: none;
		text-align: center;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.provider:hover {
		border-color: var(--text-muted);
	}
</style>
