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

		const reply = o.search('8/8/4k3/8/4K3/4P3/8/8 w - - 0 1', ['e4d4', 'e6d6']);
		await Promise.resolve();
		await Promise.resolve();
		expect(engine.sent.slice(2)).toEqual([
			'position fen 8/8/4k3/8/4K3/4P3/8/8 w - - 0 1 moves e4d4 e6d6',
			'go movetime 250'
		]);
		engine.say('info depth 10 multipv 1 score cp 45 pv d4e4 d6e6');
		engine.say('info depth 14 multipv 1 score cp 60 pv e3e4 d6e6 d4c5');
		engine.say('bestmove e3e4 ponder d6e6');
		expect(await reply).toEqual({
			best: 'e3e4',
			score: { kind: 'cp', value: 60 },
			pv: ['e3e4', 'd6e6', 'd4c5'],
			depth: 14
		});
		// No info line (e.g. a position with one legal move): the best move alone.
		const quick = o.search('8/8/8/8/8/8/8/K6k w - - 0 1', []);
		await Promise.resolve();
		await Promise.resolve();
		engine.say('bestmove a1a2');
		expect(await quick).toEqual({ best: 'a1a2', score: null, pv: ['a1a2'] });
	});

	it('searches to a depth when given one', async () => {
		let engine!: FakeEngine;
		const o = new Opponent({ depth: 12, createEngine: (e) => (engine = new FakeEngine(e)) });
		engine.say('uciok');
		engine.say('readyok');
		void o.search('8/8/8/8/8/8/8/K6k w - - 0 1', []);
		await Promise.resolve();
		await Promise.resolve();
		expect(engine.sent.at(-1)).toBe('go depth 12');
	});

	it('resolves null when replaced or when the engine fails', async () => {
		let engine!: FakeEngine;
		const o = new Opponent({ createEngine: (e) => (engine = new FakeEngine(e)) });
		engine.say('uciok');
		engine.say('readyok');
		const first = o.search('8/8/8/8/8/8/8/K6k w - - 0 1', []);
		await Promise.resolve();
		const second = o.search('8/8/8/8/8/8/8/K6k w - - 0 1', []);
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
