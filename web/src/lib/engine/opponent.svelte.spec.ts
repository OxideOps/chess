import { beforeAll, describe, expect, it } from 'vitest';
import { initChess } from '$lib/chess/wasm';
import { Opponent } from './opponent.svelte';
import type { EngineEvent, EngineLike } from './worker';

class FakeEngine implements EngineLike {
	sent: string[] = [];
	readonly threads = 1;
	constructor(readonly emit: (event: EngineEvent) => void) {}
	send(command: string) {
		this.sent.push(command);
	}
	terminate() {}
	say(text: string) {
		this.emit({ type: 'line', text });
	}
}

beforeAll(() => initChess());

describe('Opponent', () => {
	it('handshakes, then asks for a move from the position and answers with bestmove', async () => {
		let engine!: FakeEngine;
		const o = new Opponent({ movetime: 250, createEngine: (e) => (engine = new FakeEngine(e)) });
		expect(engine.sent).toEqual(['uci']);
		engine.say('uciok');
		engine.say('readyok');
		expect(o.status).toBe('ready');

		const reply = o.move('8/8/4k3/8/4K3/4P3/8/8 w - - 0 1', ['e4d4', 'e6d6']);
		await Promise.resolve();
		await Promise.resolve();
		expect(engine.sent.slice(2)).toEqual([
			'position fen 8/8/4k3/8/4K3/4P3/8/8 w - - 0 1 moves e4d4 e6d6',
			'go movetime 250'
		]);
		engine.say('bestmove e3e4 ponder d6e6');
		expect(await reply).toBe('e3e4');
	});

	it('resolves null when replaced or when the engine fails', async () => {
		let engine!: FakeEngine;
		const o = new Opponent({ createEngine: (e) => (engine = new FakeEngine(e)) });
		engine.say('uciok');
		engine.say('readyok');
		const first = o.move('8/8/8/8/8/8/8/K6k w - - 0 1', []);
		await Promise.resolve();
		const second = o.move('8/8/8/8/8/8/8/K6k w - - 0 1', []);
		expect(await first).toBeNull();
		await Promise.resolve();
		expect(engine.sent).toContain('stop');
		engine.emit({
			type: 'error',
			message: 'boom'
		});
		expect(await second).toBeNull();
		expect(o.status).toBe('failed');
	});
});
