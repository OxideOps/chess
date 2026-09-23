<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { withNext } from '$lib/auth/next';
	import { listProviders, startUrl } from '$lib/auth/providers';
	import {
		connectable,
		disconnect,
		identityText,
		loadAccount,
		setPassword
	} from '$lib/auth/account';
	import { changeEmail, emailState, mailEnabled, removeEmail } from '$lib/auth/email';
	import { AuthError } from '$lib/auth/refusal';
	import type { Account } from '$lib/generated/Account';
	import type { LinkedIdentity } from '$lib/generated/LinkedIdentity';
	import type { ProviderInfo } from '$lib/generated/ProviderInfo';

	// How this account signs in: connected providers, the password, and the
	// providers it could still connect. Connecting leaves for the provider and
	// comes back here; a failure comes back with `?error=`.
	let account: Account | null = $state(null);
	let providers: ProviderInfo[] = $state([]);
	let loadError: string | null = $state(null);
	let error: string | null = $state(page.url.searchParams.get('error'));
	let removing: string | null = $state(null);
	// Connecting a new provider needs a recent sign-in too; a refused one
	// comes back with `?sign_in_again=<provider id>` (empty when the account
	// has no provider to name, only its password).
	const againId = page.url.searchParams.get('sign_in_again');
	let connectStale = $state(againId !== null);
	const connectAgain = $derived(providers.find((provider) => provider.id === againId) ?? null);

	let current = $state('');
	let password = $state('');
	let saving = $state(false);
	let passwordError: string | null = $state(null);
	let passwordSaved: string | null = $state(null);
	// A first password needs a recent sign-in; when this session is older the
	// server names the provider to go back through, and it returns here.
	let signInAgain: ProviderInfo | null = $state(null);

	let mail = $state(false);
	let email = $state('');
	let emailBusy = $state(false);
	let emailError: string | null = $state(null);
	let emailSent: string | null = $state(null);
	// A new address is a new way in (a reset can go to it), so it takes the
	// password, or on an account without one a recent sign-in.
	let emailPassword = $state('');
	let emailSignInAgain: ProviderInfo | null = $state(null);

	const loginHref = $derived(withNext(resolve('/login'), resolve('/account')));
	const signupHref = $derived(withNext(resolve('/signup'), resolve('/account')));
	const toConnect = $derived(account ? connectable(providers, account) : []);

	const key = (identity: LinkedIdentity) => `${identity.provider}:${identity.subject}`;

	$effect(() => {
		if (!session.registered) return;
		let cancelled = false;
		Promise.all([loadAccount(), listProviders(), mailEnabled()])
			.then(([loaded, offered, canMail]) => {
				if (cancelled) return;
				account = loaded;
				providers = offered;
				mail = canMail;
			})
			.catch((e: unknown) => {
				if (!cancelled) loadError = e instanceof Error ? e.message : String(e);
			});
		return () => {
			cancelled = true;
		};
	});

	async function remove(identity: LinkedIdentity) {
		removing = key(identity);
		error = null;
		connectStale = false;
		try {
			await disconnect(identity);
			account = await loadAccount();
		} catch (e) {
			const reason = e instanceof Error ? e.message : String(e);
			error = `Can't disconnect ${identityText(identity)}: ${reason}.`;
		} finally {
			removing = null;
		}
	}

	async function saveEmail() {
		emailBusy = true;
		emailError = null;
		emailSent = null;
		emailSignInAgain = null;
		try {
			const current_password = account?.has_password ? emailPassword : null;
			account = await changeEmail({ email, current_password });
			emailSent = account.pending_email
				? `We sent a link to ${account.pending_email}. Follow it to verify the address.`
				: null;
			email = '';
			emailPassword = '';
		} catch (e) {
			emailError = e instanceof Error ? e.message : String(e);
			if (e instanceof AuthError) emailSignInAgain = e.signInAgain;
		} finally {
			emailBusy = false;
		}
	}

	async function dropEmail() {
		emailBusy = true;
		emailError = null;
		emailSent = null;
		try {
			account = await removeEmail();
		} catch (e) {
			emailError = e instanceof Error ? e.message : String(e);
		} finally {
			emailBusy = false;
		}
	}

	async function savePassword() {
		if (!account) return;
		saving = true;
		passwordError = null;
		passwordSaved = null;
		signInAgain = null;
		const changing = account.has_password;
		try {
			await setPassword({ current: changing ? current : null, password });
			account = { ...account, has_password: true };
			current = '';
			password = '';
			passwordSaved = changing
				? 'Password changed. Other devices were signed out.'
				: 'Password set. You can now log in with your username and it.';
		} catch (e) {
			passwordError = e instanceof Error ? e.message : String(e);
			if (e instanceof AuthError) signInAgain = e.signInAgain;
		} finally {
			saving = false;
		}
	}
</script>

<!-- The ways back to a fresh session, which then returns here: through a
     provider the account has, or its password at the login form. -->
{#snippet signInAgainLinks(provider: ProviderInfo | null)}
	<div class="connect">
		{#if provider}
			<a class="btn" href={startUrl(provider, resolve('/account'))} rel="external">
				Sign in again with {provider.name}
			</a>
		{/if}
		{#if account?.has_password}
			<a class="btn" href={loginHref}>Log in again with your password</a>
		{/if}
	</div>
{/snippet}

<svelte:head>
	<title>Account · Chess</title>
</svelte:head>

<div class="page-header">
	<h1>{account?.username ?? 'Your account'}</h1>
	<p>
		How you sign in, and how you get back in. Your games are under <a href={resolve('/games')}
			>My games</a
		>.
	</p>
</div>

{#if !session.user}
	<p class="muted">
		<a href={loginHref}>Log in</a> to see how your account signs in.
	</p>
{:else if !session.registered}
	<p class="muted">
		You are playing as a guest. <a href={signupHref}>Sign up</a> to keep your games and your name.
	</p>
{:else if loadError}
	<p class="error" role="alert">Could not load your account: {loadError}</p>
{:else if account === null}
	<p class="muted">Loading…</p>
{:else}
	<div class="sections">
		<section class="panel" aria-labelledby="methods">
			<h2 id="methods">Sign-in methods</h2>
			<ul data-testid="sign-in-methods">
				{#each account.identities as identity (key(identity))}
					<li>
						<span>{identityText(identity)}</span>
						<button
							type="button"
							class="btn"
							aria-label="Disconnect {identityText(identity)}"
							disabled={removing !== null}
							onclick={() => remove(identity)}
						>
							{removing === key(identity) ? 'Disconnecting…' : 'Disconnect'}
						</button>
					</li>
				{/each}
				<li>
					<span>Password</span>
					<span class="muted" data-testid="password-state"
						>{account.has_password ? 'Set' : 'Not set'}</span
					>
				</li>
			</ul>
			{#if error}
				<p class="error" role="alert">{error}</p>
			{/if}
			{#if connectStale}
				{@render signInAgainLinks(connectAgain)}
			{/if}
			{#if toConnect.length > 0}
				<div class="connect">
					{#each toConnect as provider (provider.id)}
						<a class="btn" href={startUrl(provider, resolve('/account'))} rel="external">
							Connect {provider.name}
						</a>
					{/each}
				</div>
			{/if}
		</section>

		{#if mail || account.email}
			<section class="panel" aria-labelledby="email">
				<h2 id="email">Email</h2>
				<ul>
					<li>
						<span>{account.email ?? account.pending_email ?? 'No address'}</span>
						<span class={account.email ? 'saved' : 'muted'} data-testid="email-state"
							>{emailState(account)}</span
						>
					</li>
					{#if account.email && account.pending_email}
						<li>
							<span>Changing to {account.pending_email}</span>
							<span class="muted">Not verified yet</span>
						</li>
					{/if}
				</ul>
				{#if !account.email}
					<p class="muted" data-testid="no-recovery">
						{account.pending_email
							? 'Until you follow the link we sent, a forgotten password can’t be recovered.'
							: 'Without a verified address, a forgotten password can’t be recovered.'}
					</p>
				{/if}
				{#if mail}
					<form
						onsubmit={(event) => {
							event.preventDefault();
							saveEmail();
						}}
					>
						<label>
							{account.email ? 'New address' : 'Email address'}
							<input
								name="email"
								type="email"
								bind:value={email}
								autocomplete="email"
								required
								maxlength="254"
							/>
						</label>
						{#if account.has_password}
							<label>
								Your password
								<input
									name="email-current-password"
									type="password"
									bind:value={emailPassword}
									autocomplete="current-password"
									required
								/>
							</label>
						{/if}
						<div class="row">
							<button type="submit" class="btn" disabled={emailBusy}>
								{account.email || account.pending_email ? 'Change email' : 'Add email'}
							</button>
							{#if account.email || account.pending_email}
								<button type="button" class="btn" disabled={emailBusy} onclick={dropEmail}>
									Remove email
								</button>
							{/if}
						</div>
					</form>
				{/if}
				{#if emailError}
					<p class="error" role="alert">{emailError}</p>
				{/if}
				{#if emailSignInAgain}
					{@render signInAgainLinks(emailSignInAgain)}
				{/if}
				{#if emailSent}
					<p class="saved" role="status">{emailSent}</p>
				{/if}
			</section>
		{/if}

		<section class="panel" aria-labelledby="password">
			<h2 id="password">{account.has_password ? 'Change password' : 'Set a password'}</h2>
			{#if !account.has_password}
				<p class="muted">
					Then you can log in with your username too, and disconnect a provider without losing your
					way in.
				</p>
			{/if}
			<form
				onsubmit={(event) => {
					event.preventDefault();
					savePassword();
				}}
			>
				{#if account.has_password}
					<label>
						Current password
						<input
							name="current-password"
							type="password"
							bind:value={current}
							autocomplete="current-password"
							required
						/>
					</label>
				{/if}
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
				<button type="submit" class="btn primary" disabled={saving}>
					{account.has_password ? 'Change password' : 'Set password'}
				</button>
			</form>
			{#if passwordError}
				<p class="error" role="alert">{passwordError}</p>
			{/if}
			{#if signInAgain}
				{@render signInAgainLinks(signInAgain)}
			{/if}
			{#if passwordSaved}
				<p class="saved" role="status">{passwordSaved}</p>
			{/if}
		</section>
	</div>
{/if}

<style>
	.sections {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		max-width: 32rem;
	}

	h2 {
		margin: 0 0 0.75rem;
		font-size: var(--type-lg);
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		min-height: var(--tap);
		padding: 0.4rem 0;
		border-bottom: 1px solid var(--panel-border);
	}

	li:last-child {
		border-bottom: none;
	}

	/* A long email wraps rather than widening the page on a phone. */
	li span:first-child {
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.connect {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-top: 0.75rem;
	}

	.connect a {
		text-decoration: none;
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

	input {
		width: 100%;
		max-width: 22rem;
	}

	form .btn {
		align-self: flex-start;
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	p {
		margin: 0.75rem 0 0;
	}

	.muted {
		color: var(--text-muted);
	}

	section > p.muted:first-of-type {
		margin: 0 0 0.75rem;
		font-size: var(--type-sm);
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
