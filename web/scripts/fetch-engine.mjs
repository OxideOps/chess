// Fetch the Stockfish.js builds into static/engine from the GitHub release,
// verifying each file's SHA-256. The binaries are not committed (14 MB and
// growing with every engine version); the license text is.
// Usage: node scripts/fetch-engine.mjs   (a no-op once the files are present)
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const RELEASE = 'https://github.com/nmrugg/stockfish.js/releases/download/v18.0.0';
// Bumping the engine: change the tag above and the names and hashes below
// (`shasum -a 256` of the release assets).
const FILES = {
	'stockfish-18-lite.js': 'f79e667c9d56ee768aca35e8343f91548ceef6a732f67cd82f267cf9eab7f665',
	'stockfish-18-lite.wasm': 'd50136919dcd90e75eb8df78b255d47d618962b670028b38961343f6eb409174',
	'stockfish-18-lite-single.js': '5243fd9b276cab7dfe3ad1d43ab9ead73568fac76468c614242977a210c4a391',
	'stockfish-18-lite-single.wasm':
		'a8fbc05ec6920b56d7485826dcb02c5ffd2826bcbf751cf973046f237a9096f1'
};

const web = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const dir = path.join(web, 'static', 'engine');

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

async function present(file, hash) {
	try {
		return sha256(await readFile(path.join(dir, file))) === hash;
	} catch {
		return false;
	}
}

await mkdir(dir, { recursive: true });
for (const [file, hash] of Object.entries(FILES)) {
	if (await present(file, hash)) continue;
	const url = `${RELEASE}/${file}`;
	console.log(`fetching ${url}`);
	const response = await fetch(url);
	if (!response.ok) {
		console.error(`could not fetch ${url}: ${response.status} ${response.statusText}`);
		process.exit(1);
	}
	const bytes = new Uint8Array(await response.arrayBuffer());
	const got = sha256(bytes);
	if (got !== hash) {
		console.error(`${file}: SHA-256 ${got} does not match the expected ${hash}`);
		process.exit(1);
	}
	await writeFile(path.join(dir, file), bytes);
}
