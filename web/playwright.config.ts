import { defineConfig } from '@playwright/test';

// End-to-end tests run against the real Rust server serving the production
// build, so the API, the game sockets and the 404 fallback are all exercised.
export default defineConfig({
	webServer: {
		command: 'npm run build && cargo run -p server -- --static-dir build --bind 127.0.0.1:4173',
		port: 4173,
		timeout: 300_000
	},
	testDir: 'e2e',
	testMatch: '**/*.e2e.{ts,js}',
	use: { baseURL: 'http://127.0.0.1:4173' }
});
