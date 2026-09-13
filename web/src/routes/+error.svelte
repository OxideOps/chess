<script lang="ts">
	import { page } from '$app/state';
</script>

<svelte:head>
	<title>{page.status === 404 ? 'Page not found' : 'Error'} · Chess</title>
</svelte:head>

<section class="error">
	{#if page.status === 404}
		<h1>Page not found</h1>
		<p>There is nothing at {page.url.pathname}.</p>
	{:else}
		<h1>Something went wrong</h1>
		<p>{page.error?.message ?? page.status}</p>
	{/if}
</section>

<style>
	.error {
		max-width: 40rem;
	}
</style>
