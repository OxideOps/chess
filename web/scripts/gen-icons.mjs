// Render the app icons from the site's mark: a rook drawn in the board's own
// right angles (the same shape as src/lib/components/Logo.svelte). Run by hand
// after changing the design; the PNGs are committed. Uses the Chromium that
// Playwright installs (`pnpm browsers`).
// Usage: node scripts/gen-icons.mjs
import { writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const web = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
// Keep these in step with app.css: --panel and --accent.
const CHARCOAL = '#1b1814';
const BRASS = '#c99b3f';

// The rook, on a 512 canvas: battlements, collar, a stepped body, a foot.
// Every edge lands on the square grid; nothing is curved.
const ROOK = [
	[116, 100, 64, 56],
	[224, 100, 64, 56],
	[332, 100, 64, 56],
	[116, 156, 280, 56],
	[144, 212, 224, 28],
	[172, 240, 168, 84],
	[144, 324, 224, 28],
	[88, 352, 336, 60]
]
	.map(([x, y, w, h]) => `<rect x="${x}" y="${y}" width="${w}" height="${h}"/>`)
	.join('');

/**
 * A 512×512 icon. `full` fills the square (maskable, Apple); otherwise the
 * badge has rounded corners. `scale` shrinks the mark about the centre.
 */
function icon({ full, scale }) {
	const bg = full
		? `<rect width="512" height="512" fill="${CHARCOAL}"/>`
		: `<rect width="512" height="512" rx="112" fill="${CHARCOAL}"/>`;
	const mark = `<g fill="${BRASS}" transform="translate(256 256) scale(${scale}) translate(-256 -256)">${ROOK}</g>`;
	return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">${bg}${mark}</svg>`;
}

const any = icon({ full: false, scale: 1 });
// Maskable icons are cropped to a circle of 80% of the width: keep the mark inside it.
const maskable = icon({ full: true, scale: 0.72 });
const apple = icon({ full: true, scale: 0.86 });

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
