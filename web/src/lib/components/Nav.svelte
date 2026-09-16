<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { safeNext, withNext } from '$lib/auth/next';
	import { installer } from '$lib/pwa/install.svelte';
	import Logo from '$lib/components/Logo.svelte';

	const links = [
		{ href: resolve('/'), label: 'Play' },
		{ href: resolve('/online'), label: 'Online' },
		{ href: resolve('/puzzles'), label: 'Puzzles' },
		{ href: resolve('/lessons'), label: 'Lessons' },
		{ href: resolve('/analysis'), label: 'Analysis' }
	] as const;

	// Where the auth pages send people back to: the page they were on. On the
	// auth pages themselves, pass their `next` along rather than nesting it.
	const onAuthPage = $derived(
		page.url.pathname === resolve('/login') || page.url.pathname === resolve('/signup')
	);
	const here = $derived(
		onAuthPage ? safeNext(page.url.searchParams.get('next')) : page.url.pathname + page.url.search
	);
	const loginHref = $derived(withNext(resolve('/login'), here));
	const signupHref = $derived(withNext(resolve('/signup'), here));

	let leaving = $state(false);
	async function logout() {
		leaving = true;
		try {
			await session.logout();
			await goto(resolve('/'));
		} finally {
			leaving = false;
		}
	}
</script>

<nav id="navbar">
	<a class="brand" href={resolve('/')} aria-label="Chess, home">
		<Logo size={20} />
		<span>Chess</span>
	</a>
	{#each links as link (link.href)}
		<a href={link.href} aria-current={page.url.pathname === link.href ? 'page' : undefined}>
			{link.label}
		</a>
	{/each}
	<span class="account">
		{#if installer.available}
			<button type="button" onclick={() => installer.install()}>Install app</button>
		{/if}
		{#if session.user}
			<a
				href={resolve('/games')}
				class="who"
				class:guest={!session.registered}
				aria-current={page.url.pathname === resolve('/games') ? 'page' : undefined}
				title="My games"
			>
				{session.displayName}
			</a>
		{/if}
		{#if session.registered}
			<button type="button" onclick={logout} disabled={leaving}>Log out</button>
		{:else}
			<a href={loginHref}>Log in</a>
			<a href={signupHref}>Sign up</a>
		{/if}
	</span>
</nav>

<style>
	#navbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem 1.25rem;
		min-height: var(--nav-height);
		padding: 0.5rem var(--gutter);
		background: var(--panel);
		border-bottom: 1px solid var(--panel-border);
	}

	.brand {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		margin-right: 0.5rem;
		color: var(--accent);
		font-family: var(--font-mono);
		font-size: var(--type-base);
		font-weight: 500;
		letter-spacing: -0.02em;
		text-decoration: none;
	}

	/* The wordmark stays readable on the dark bar; only the mark is brass. */
	.brand span {
		color: var(--text);
	}

	a {
		display: inline-flex;
		align-items: center;
		min-height: var(--tap);
		color: var(--text-muted);
		font-size: var(--type-sm);
		text-decoration: none;
	}

	a:hover {
		color: var(--text);
	}

	/* The current page is the one place the nav uses the accent. */
	a[aria-current='page'] {
		color: var(--text);
		box-shadow: inset 0 -2px 0 var(--accent);
	}

	@media (max-width: 700px) {
		#navbar {
			column-gap: 0.9rem;
		}

		/* The mark alone identifies the site; the wordmark costs a row here. */
		.brand span {
			display: none;
		}

		.account {
			gap: 0.75rem;
		}
	}

	.account {
		margin-left: auto;
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.who {
		color: var(--text);
		font-weight: 600;
	}

	.who.guest {
		font-weight: 400;
		font-style: italic;
	}

	button {
		min-height: var(--tap);
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--panel-border);
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--type-sm);
		cursor: pointer;
	}

	button:hover:not(:disabled) {
		color: var(--text);
		border-color: var(--text-muted);
	}

	button:disabled {
		opacity: 0.6;
		cursor: default;
	}
</style>
