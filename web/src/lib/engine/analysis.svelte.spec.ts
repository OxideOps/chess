import { beforeAll, describe, expect, it } from 'vitest';
import { initChess } from '$lib/chess/wasm';
import { Analyser, MAX_DEPTH } from './analysis.svelte';
import type { EngineEvent, EngineLike } from './worker';

const START = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
const AFTER_E4 = 'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1';

/** Records commands and lets the test play the engine's side. */
class FakeEngine implements EngineLike {
	sent: string[] = [];
	terminated = false;
	constructor(private emit: (event: EngineEvent) => void) {}
	send(command: string) {
		this.sent.push(command);
	}
	terminate() {
		this.terminated = true;
	}
	say(text: string) {
		this.emit({ type: 'line', text });
	}
	take(): string[] {
		return this.sent.splice(0);
	}
}

function setup() {
	let engine!: FakeEngine;
	const analyser = new Analyser({
		multipv: 2,
		createEngine: (onEvent) => (engine = new FakeEngine(onEvent))
	});
	return { analyser, engine };
}

beforeAll(() => initChess());

describe('Analyser', () => {
	it('handshakes, then starts the search that was requested while loading', () => {
		const { analyser, engine } = setup();
		expect(engine.take()).toEqual(['uci']);
		expect(analyser.status).toBe('loading');

		analyser.request(START);
		expect(engine.take()).toEqual([]);

		engine.say('id name Stockfish 18');
		engine.say('uciok');
		expect(engine.take()).toEqual(['setoption name MultiPV value 2', 'isready']);
		engine.say('readyok');
		expect(analyser.status).toBe('ready');
		expect(analyser.name).toBe('Stockfish 18');
		expect(engine.take()).toEqual([`position fen ${START}`, `go depth ${MAX_DEPTH}`]);
		expect(analyser.fen).toBe(START);
		expect(analyser.searching).toBe(true);
	});

	it('collects lines per multipv slot and stops on bestmove', () => {
		const { analyser, engine } = setup();
		engine.say('uciok');
		engine.say('readyok');
		analyser.request(AFTER_E4);
		engine.take();

		engine.say('info depth 5 multipv 2 score cp -20 pv c7c5');
		engine.say('info depth 5 multipv 1 score cp -10 pv e7e5 g1f3');
		engine.say('info depth 6 multipv 1 score cp -12 pv e7e5 b1c3');
		expect(analyser.lines.map((l) => [l.multipv, l.depth, l.pv[1]])).toEqual([
			[1, 6, 'b1c3'],
			[2, 5, undefined]
		]);
		expect(analyser.turn).toBe('black');
		expect(analyser.depth).toBe(6);

		engine.say('bestmove e7e5');
		expect(analyser.searching).toBe(false);
		expect(analyser.lines).toHaveLength(2); // results stay on screen
		expect(engine.take()).toEqual([]);

		// Same position again: nothing to do.
		analyser.request(AFTER_E4);
		expect(engine.take()).toEqual([]);
	});

	it('waits for bestmove before switching position, and drops stale lines', () => {
		const { analyser, engine } = setup();
		engine.say('uciok');
		engine.say('readyok');
		analyser.request(START);
		engine.take();
		engine.say('info depth 3 multipv 1 score cp 30 pv e2e4');

		analyser.request(AFTER_E4);
		expect(engine.take()).toEqual(['stop']);
		// Still the old position until the engine confirms it stopped.
		expect(analyser.fen).toBe(START);
		engine.say('info depth 4 multipv 1 score cp 31 pv e2e4 e7e5');
		expect(analyser.lines[0].depth).toBe(4);

		engine.say('bestmove e2e4');
		expect(engine.take()).toEqual([`position fen ${AFTER_E4}`, `go depth ${MAX_DEPTH}`]);
		expect(analyser.fen).toBe(AFTER_E4);
		expect(analyser.lines).toEqual([]);

		// A second change while still stopping only replaces the pending target.
		analyser.request(START);
		expect(engine.take()).toEqual(['stop']);
		analyser.request(AFTER_E4);
		expect(engine.take()).toEqual(['stop']);
		engine.say('bestmove (none)');
		expect(engine.take()).toEqual([`position fen ${AFTER_E4}`, `go depth ${MAX_DEPTH}`]);
	});

	it('pauses on null, ignores late lines, and resumes the same position later', () => {
		const { analyser, engine } = setup();
		engine.say('uciok');
		engine.say('readyok');
		analyser.request(START);
		engine.take();

		analyser.request(null);
		expect(engine.take()).toEqual(['stop']);
		expect(analyser.fen).toBeNull();
		engine.say('info depth 9 multipv 1 score cp 1 pv e2e4');
		expect(analyser.lines).toEqual([]);
		engine.say('bestmove e2e4');

		analyser.request(START);
		expect(engine.take()).toEqual([`position fen ${START}`, `go depth ${MAX_DEPTH}`]);
	});

	it('reports engine failures and can be disposed', () => {
		let emit!: (event: EngineEvent) => void;
		const engine = { send() {}, terminate: () => (disposed = true) };
		let disposed = false;
		const analyser = new Analyser({
			createEngine: (onEvent) => {
				emit = onEvent;
				return engine;
			}
		});
		emit({ type: 'error', message: 'boom' });
		expect(analyser.status).toBe('failed');
		expect(analyser.error).toBe('boom');
		analyser.dispose();
		expect(disposed).toBe(true);
	});
});
