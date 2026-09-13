// Write the TypeScript bindings for chess-core's protocol types and the wasm
// wrapper's view types into src/lib/generated (via ts-rs, which exports from
// `cargo test`). CI checks the result is committed.
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { rmSync } from 'node:fs';
import path from 'node:path';

const web = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const outDir = path.join(web, 'src', 'lib', 'generated');
rmSync(outDir, { recursive: true, force: true });

const result = spawnSync(
	'cargo',
	['test', '-p', 'chess-core', '-p', 'chess-core-wasm', '--features', 'ts', 'export_bindings'],
	{
		cwd: path.join(web, '..'),
		stdio: 'inherit',
		env: { ...process.env, TS_RS_EXPORT_DIR: outDir }
	}
);
process.exit(result.status ?? 1);
