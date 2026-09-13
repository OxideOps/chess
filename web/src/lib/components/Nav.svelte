<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';

	const links = [
		{ href: resolve('/'), label: 'Play' },
		{ href: resolve('/analysis'), label: 'Analysis' }
	] as const;
</script>

<nav id="navbar">
	<span class="brand">Chess</span>
	{#each links as link (link.href)}
		<a href={link.href} aria-current={page.url.pathname === link.href ? 'page' : undefined}>
			{link.label}
		</a>
	{/each}
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
</style>
