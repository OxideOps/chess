import { page } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Nav from './Nav.svelte';
import { session } from '$lib/auth/session.svelte';

describe('Nav.svelte', () => {
	afterEach(() => {
		session.user = null;
	});

	it('links to the play and analysis pages, and to signing in when nobody is', async () => {
		await render(Nav);

		await expect.element(page.getByRole('link', { name: 'Play' })).toHaveAttribute('href', '/');
		await expect
			.element(page.getByRole('link', { name: 'Analysis' }))
			.toHaveAttribute('href', '/analysis');
		await expect.element(page.getByRole('link', { name: 'Log in' })).toBeVisible();
		await expect.element(page.getByRole('link', { name: 'Sign up' })).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Log out' })).not.toBeInTheDocument();
	});

	it('shows a guest as Guest, still with the sign-in links', async () => {
		session.user = { id: 'g', username: null, is_guest: true };
		await render(Nav);

		await expect
			.element(page.getByRole('link', { name: 'Guest' }))
			.toHaveAttribute('href', '/games');
		await expect.element(page.getByRole('link', { name: 'Sign up' })).toBeVisible();
	});

	it('shows a registered user by name, linking to their account, with a log-out button', async () => {
		session.user = { id: 'u', username: 'alice', is_guest: false };
		await render(Nav);

		await expect
			.element(page.getByRole('link', { name: 'alice' }))
			.toHaveAttribute('href', '/account');
		await expect.element(page.getByRole('button', { name: 'Log out' })).toBeVisible();
		await expect.element(page.getByRole('link', { name: 'Sign up' })).not.toBeInTheDocument();
	});
});
