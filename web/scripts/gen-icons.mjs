// Render the app icons from the knight in static/pieces/cburnett (Colin M.L.
// Burnett, CC BY-SA 3.0). Run by hand after changing the design; the PNGs are
// committed. Uses the Chromium that Playwright installs (`pnpm browsers`).
// Usage: node scripts/gen-icons.mjs
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const web = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const knight = await readFile(path.join(web, 'static/pieces/cburnett/wN.svg'), 'utf8');
// The piece's drawing, without its own <svg> wrapper (45×45 units).
const drawing = knight.replace(/^<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '');
const GREEN = '#629924';

/** A 512×512 icon. `full` fills the square (maskable, Apple); otherwise rounded corners. */
function icon({ full, knightSize }) {
	const scale = knightSize / 45;
	const offset = (512 - knightSize) / 2;
	const bg = full
		? `<rect width="512" height="512" fill="${GREEN}"/>`
		: `<rect width="512" height="512" rx="104" fill="${GREEN}"/>`;
	return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">${bg}<g transform="translate(${offset} ${offset - knightSize * 0.02}) scale(${scale})">${drawing}</g></svg>`;
}

const any = icon({ full: false, knightSize: 400 });
// Maskable icons are cropped to a circle of 80% of the width: keep the knight inside it.
const maskable = icon({ full: true, knightSize: 300 });
const apple = icon({ full: true, knightSize: 360 });

await writeFile(path.join(web, 'src/lib/assets/favicon.svg'), any + '\n');
await writeFile(path.join(web, 'static/icons/icon.svg'), any + '\n');

const browser = await chromium.launch();
const page = await browser.newPage();
for (const [svg, size, name] of [
	[any, 192, 'icon-192.png'],
	[any, 512, 'icon-512.png'],
	[maskable, 512, 'maskable-512.png'],
	[apple, 180, 'apple-touch-icon.png']
]) {
	await page.setViewportSize({ width: size, height: size });
	await page.setContent(
		`<style>html,body{margin:0;background:transparent}svg{display:block;width:${size}px;height:${size}px}</style>${svg}`
	);
	await page.screenshot({ path: path.join(web, 'static/icons', name), omitBackground: true });
	console.log(`static/icons/${name}`);
}
await browser.close();
