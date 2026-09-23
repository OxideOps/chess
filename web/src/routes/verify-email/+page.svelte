<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { verifyEmail } from '$lib/auth/email';
	import type { EmailVerified } from '$lib/generated/EmailVerified';

	// Where a verification link lands (`?token=`). Opening it is the
	// confirmation: the page spends the token as soon as it loads. (Mail
	// scanners that fetch links don't run scripts, so they can't spend it.)
	const token = page.url.searchParams.get('token') ?? '';
	let verified: EmailVerified | null = $state(null);
	let error: string | null = $state(token ? null : 'this page needs the link from the email');

	$effect(() => {
		if (!token) return;
		verifyEmail(token)
			.then((result) => (verified = result))
			.catch((e: unknown) => (error = e instanceof Error ? e.message : String(e)));
	});
</script>

<svelte:head>
	<title>Verify your email · Chess</title>
</svelte:head>

<div class="page-header">
	<h1>Verify your email</h1>
	<p>A verified address is how you get back in if you forget your password.</p>
</div>

{#if verified}
	<p class="saved" role="status">
		{verified.email} is verified for {verified.username}.
	</p>
	<p><a href={resolve('/account')}>Your account</a></p>
{:else if error}
	<p class="error" role="alert">
		Could not verify: {error}. Add the address again on
		<a href={resolve('/account')}>your account</a> for a new link.
	</p>
{:else}
	<p class="muted">Verifying…</p>
{/if}

<style>
	.saved {
		color: var(--good);
	}

	.error {
		color: var(--danger);
	}

	.muted {
		color: var(--text-muted);
	}

	a {
		color: var(--text);
	}
</style>
