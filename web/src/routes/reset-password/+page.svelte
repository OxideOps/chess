<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { resetPassword } from '$lib/auth/email';
	import { session } from '$lib/auth/session.svelte';

	// Where a reset link lands (`?token=`): choose the new password. The link
	// works once; using it signs the account out everywhere, this browser too.
	const token = page.url.searchParams.get('token') ?? '';
	let password = $state('');
	let busy = $state(false);
	let done = $state(false);
	let error: string | null = $state(null);

	async function submit() {
		busy = true;
		error = null;
		try {
			await resetPassword({ token, password });
			done = true;
			// Every session of the account is gone; the nav should say so.
			await session.load();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head>
	<title>Choose a new password · Chess</title>
</svelte:head>

<div class="page-header">
	<h1>Choose a new password</h1>
	<p>Setting it signs your account out on every device.</p>
</div>

{#if !token}
	<p class="error" role="alert">
		This page needs the link from the email. <a href={resolve('/forgot-password')}
			>Ask for a new one</a
		>
	</p>
{:else if done}
	<p class="saved" role="status">Password changed. Every device was signed out.</p>
	<p><a class="btn primary" href={resolve('/login')}>Log in</a></p>
{:else}
	<form
		onsubmit={(event) => {
			event.preventDefault();
			submit();
		}}
	>
		<label>
			New password
			<input
				name="new-password"
				type="password"
				bind:value={password}
				autocomplete="new-password"
				required
				minlength="8"
				maxlength="128"
			/>
		</label>
		<button type="submit" class="btn primary" disabled={busy}>
			{busy ? 'Saving…' : 'Set new password'}
		</button>
	</form>
	{#if error}
		<p class="error" role="alert">
			{error}. <a href={resolve('/forgot-password')}>Ask for a new link</a>
		</p>
	{/if}
{/if}

<style>
	form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-width: 22rem;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: var(--type-sm);
		color: var(--text-muted);
	}

	input {
		width: 100%;
	}

	.btn {
		align-self: flex-start;
		text-decoration: none;
	}

	.error {
		color: var(--danger);
	}

	.saved {
		color: var(--good);
	}

	a {
		color: var(--text);
	}
</style>
