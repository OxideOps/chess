// Build crates/chess-core-wasm with wasm-pack into src/lib/wasm.
// Usage: node scripts/build-wasm.mjs [--dev]
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const web = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const crate = path.join(web, '..', 'crates', 'chess-core-wasm');
const outDir = path.join(web, 'src', 'lib', 'wasm');
const dev = process.argv.includes('--dev');

const args = [
	'build',
	crate,
	'--target',
	'web',
	'--out-dir',
	outDir,
	'--out-name',
	'chess_core',
	'--no-pack',
	dev ? '--dev' : '--release'
];
const result = spawnSync('wasm-pack', args, { stdio: 'inherit' });
if (result.error) {
	console.error(
		`wasm-pack is not installed (${result.error.message}). Install it with\n` +
			'  cargo install wasm-pack --locked\n' +
			'or see https://rustwasm.github.io/wasm-pack/installer/'
	);
	process.exit(1);
}
process.exit(result.status ?? 1);
