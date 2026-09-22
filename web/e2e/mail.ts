// The fake mailer's outbox: the server Playwright starts runs with
// `--fake-mail --fake-mail-dir MAIL_DIR`, which writes each message there as
// `<millis>-<n>-<to>.txt`. Tests read the links out of it.
import { readdirSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { expect } from '@playwright/test';

export const MAIL_DIR = path.join(tmpdir(), 'chess-e2e-mail');

function inbox(to: string): string[] {
	const suffix = `-${to.toLowerCase()}.txt`;
	let names: string[];
	try {
		names = readdirSync(MAIL_DIR).filter((name) => name.endsWith(suffix));
	} catch {
		return [];
	}
	return names.sort().map((name) => readFileSync(path.join(MAIL_DIR, name), 'utf8'));
}

/** The `count`th message to `to` (waiting for it), and the link in it. */
export async function nthMail(to: string, count: number): Promise<{ text: string; link: string }> {
	let text = '';
	await expect
		.poll(() => {
			text = inbox(to)[count - 1] ?? '';
			return text !== '';
		})
		.toBe(true);
	const link = /https?:\/\/\S+\?token=\S+/.exec(text)?.[0] ?? '';
	return { text, link };
}
