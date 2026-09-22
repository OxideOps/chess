<script lang="ts">
	import { resolve } from '$app/paths';
	import { forgotPassword, mailEnabled } from '$lib/auth/email';

	// Ask for a reset link. The server answers the same whether or not the
	// address has an account, so this page does too.
	let email = $state('');
	let busy = $state(false);
	let sentTo: string | null = $state(null);
	let error: string | null = $state(null);
	let mail: boolean | null = $state(null);
	$effect(() => {
		mailEnabled().then((enabled) => (mail = enabled));
	});

	async function submit() {
		busy = true;
		error = null;
		try {
			await forgotPassword(email);
			sentTo = email.trim();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head>
	<title>Forgot your password · Chess</title>
</svelte:head>

<div class="page-header">
	<h1>Forgot your password</h1>
	<p>We email a link to set a new one, to the address you verified on your account.</p>
</div>

{#if mail === false}
	<p class="muted">
		This server can't send email, so a password can't be reset here. <a href={resolve('/login')}
			>Back to log in</a
		>
	</p>
{:else if sentTo}
	<p class="sent" role="status">
		If {sentTo} is the verified address of an account, we've sent it a link. It works once, for an hour.
	</p>
	<p class="muted">
		Nothing arrived? An account with no verified address has no way to reset its password. <a
			href={resolve('/login')}>Back to log in</a
		>
	</p>
{:else}
	<form
		onsubmit={(event) => {
			event.preventDefault();
			submit();
		}}
	>
		<label>
			Email address
			<input
				name="email"
				type="email"
				bind:value={email}
				autocomplete="email"
				required
				maxlength="254"
			/>
		</label>
		<button type="submit" class="btn primary" disabled={busy}>
			{busy ? 'Sending…' : 'Send a reset link'}
		</button>
	</form>
	{#if error}
		<p class="error" role="alert">{error}</p>
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
	}

	p {
		max-width: 32rem;
	}

	.muted {
		color: var(--text-muted);
	}

	.error {
		color: var(--danger);
	}

	a {
		color: var(--text);
	}
</style>
