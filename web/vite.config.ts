import { defineConfig } from 'vitest/config';
import { playwright } from '@vitest/browser-playwright';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';

// The Rust server (`cargo run -p server`) answers the API and game sockets on
// 8080; in development Vite serves the client and proxies `/api` to it. The
// server refuses game sockets whose `Origin` isn't its own host, so the proxy
// presents itself as that host, the way the production server (which serves
// the page itself) sees it.
const apiProxy = {
	'/api': {
		target: 'http://127.0.0.1:8080',
		ws: true,
		changeOrigin: true, // Host: 127.0.0.1:8080
		headers: { origin: 'http://127.0.0.1:8080' }
	}
};

const isolationHeaders = {
	'Cross-Origin-Opener-Policy': 'same-origin',
	'Cross-Origin-Embedder-Policy': 'require-corp'
};

// Vitest 5.0.0 injects a browser project's `define` values at runtime a
// second time, taking SvelteKit's already-stringified values literally, so
// the base path arrives as '""' and `resolve('/')` as '""#/'. In the browser,
// Vite already provides the defines, so the upstream fix
// (vitest-dev/vitest#11198) gives browser projects no runtime defines; this
// does the same after Vitest has resolved the project's config. Remove it
// once Vitest is past 5.0.0; without it the Nav component tests fail.
const vitest500BrowserDefines = {
	name: 'vitest-5.0.0-browser-defines',
	enforce: 'post' as const,
	configResolved(config: { test?: { browser?: { enabled?: boolean }; defines?: object } }) {
		if (config.test?.browser?.enabled) config.test.defines = {};
	}
};

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			// The app is a static SPA: every route is prerendered as an empty shell
			// (see src/routes/+layout.ts) and rendered in the browser. Unknown paths
			// get the SPA shell as `404.html`, which static hosts serve for misses and
			// the axum server will serve as its fallback later.
			// `precompress` writes .br/.gz siblings that the server hands out as-is.
			adapter: adapter({ fallback: '404.html', precompress: true })
		})
	],
	// In development the Rust server (`cargo run -p server`) answers the API
	// and game sockets on 8080; Vite serves the client and proxies to it.
	// The isolation headers are what the production server sends too: they
	// unlock SharedArrayBuffer, and so the multi-threaded engine.
	server: { proxy: apiProxy, headers: isolationHeaders },
	preview: { headers: isolationHeaders },
	test: {
		expect: { requireAssertions: true },
		projects: [
			{
				extends: './vite.config.ts',
				server: { proxy: apiProxy },
				plugins: [vitest500BrowserDefines],
				test: {
					name: 'client',
					browser: {
						enabled: true,
						provider: playwright(),
						instances: [{ browser: 'chromium', headless: true }]
					},
					include: ['src/**/*.svelte.{test,spec}.{js,ts}'],
					exclude: ['src/lib/server/**']
				}
			},
			{
				extends: './vite.config.ts',
				server: { proxy: apiProxy },
				test: {
					name: 'server',
					environment: 'node',
					include: ['src/**/*.{test,spec}.{js,ts}'],
					exclude: ['src/**/*.svelte.{test,spec}.{js,ts}']
				}
			}
		]
	}
});
