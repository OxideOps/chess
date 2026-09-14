import { defineConfig, devices } from '@playwright/test';

// End-to-end tests run against the real Rust server serving the production
// build, so the API, the game sockets and the 404 fallback are all exercised.
export default defineConfig({
	webServer: {
		// The fixture's 46 real Lichess puzzles are imported first (idempotent).
		command:
			'npm run build && cargo run -p server -- import-puzzles ../crates/server/tests/fixtures/puzzles.csv && cargo run -p server -- --static-dir build --bind 127.0.0.1:4173 --fake-oauth',
		port: 4173,
		timeout: 300_000,
		env: {
			// Online play needs accounts, so the server runs on the test database
			// (`createdb chess_test` locally; CI provides a postgres service).
			DATABASE_URL:
				process.env.TEST_DATABASE_URL ??
				`postgres://${process.env.USER ?? 'postgres'}@localhost/chess_test`
		}
	},
	testDir: 'e2e',
	testMatch: '**/*.e2e.{ts,js}',
	use: { baseURL: 'http://127.0.0.1:4173' },
	projects: [
		{ name: 'desktop', use: { ...devices['Desktop Chrome'] }, testIgnore: '**/phone.e2e.ts' },
		// A phone-sized touch screen (Chromium with Pixel 7 metrics).
		{ name: 'phone', use: { ...devices['Pixel 7'] }, testMatch: '**/phone.e2e.ts' }
	]
});
