<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { session } from '$lib/auth/session.svelte';
	import { safeNext, withNext } from '$lib/auth/next';

	const links = [
		{ href: resolve('/'), label: 'Play' },
		{ href: resolve('/online'), label: 'Online' },
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
	<span class="brand">Chess</span>
	{#each links as link (link.href)}
		<a href={link.href} aria-current={page.url.pathname === link.href ? 'page' : undefined}>
			{link.label}
		</a>
	{/each}
	<span class="account">
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
		align-items: center;
		gap: 1.25rem;
		padding: 0.6rem 1.25rem;
		background: var(--panel);
		border-bottom: 1px solid var(--panel-border);
	}

	.brand {
		font-weight: 700;
		letter-spacing: 0.02em;
	}

	a {
		color: var(--text-muted);
		text-decoration: none;
	}

	a:hover,
	a[aria-current='page'] {
		color: var(--text);
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
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: transparent;
		color: var(--text-muted);
		font: inherit;
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
