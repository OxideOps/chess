import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Nav from './Nav.svelte';

describe('Nav.svelte', () => {
	it('links to the play and analysis pages', async () => {
		render(Nav);

		await expect.element(page.getByRole('link', { name: 'Play' })).toHaveAttribute('href', '/');
		await expect
			.element(page.getByRole('link', { name: 'Analysis' }))
			.toHaveAttribute('href', '/analysis');
	});
});
