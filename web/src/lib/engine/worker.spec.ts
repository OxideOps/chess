import { describe, expect, it } from 'vitest';
import { Engine, type EngineEvent, type WorkerLike } from './worker';

class FakeWorker implements WorkerLike {
	sent: string[] = [];
	terminated = false;
	onmessage: WorkerLike['onmessage'] = null;
	onerror: WorkerLike['onerror'] = null;
	constructor(readonly url: string) {}
	postMessage(message: string) {
		this.sent.push(message);
	}
	terminate() {
		this.terminated = true;
	}
	say(text: string) {
		this.onmessage?.({ data: text });
	}
	fail(message: string) {
		this.onerror?.({ message });
	}
}

function engine(isolated: boolean) {
	const workers: FakeWorker[] = [];
	const events: EngineEvent[] = [];
	const e = new Engine((event) => events.push(event), {
		environment: { crossOriginIsolated: isolated, hardwareConcurrency: 8 },
		spawn: (url) => {
			const w = new FakeWorker(url);
			workers.push(w);
			return w;
		}
	});
	return { e, workers, events };
}

describe('Engine', () => {
	it('loads the build for the environment, with the wasm in the fragment', () => {
		const threaded = engine(true);
		expect(threaded.workers[0].url).toBe(
			'/engine/stockfish-18-lite.js#/engine/stockfish-18-lite.wasm'
		);
		expect(threaded.e.threads).toBe(7);
		const single = engine(false);
		expect(single.workers[0].url).toMatch(/stockfish-18-lite-single\.js#/);
		expect(single.e.threads).toBe(1);
	});

	it('falls back to the single-threaded build if the threaded one fails before answering', () => {
		const { e, workers, events } = engine(true);
		e.send('uci');
		workers[0].fail('Out of memory');
		expect(workers[0].terminated).toBe(true);
		expect(workers).toHaveLength(2);
		expect(workers[1].url).toMatch(/stockfish-18-lite-single\.js#/);
		expect(workers[1].sent).toEqual(['uci']);
		expect(e.threads).toBe(1);
		expect(events).toEqual([]);

		// Late noise from the dead worker is ignored; the new one talks normally.
		workers[0].say('uciok');
		workers[1].say('uciok');
		expect(events).toEqual([{ type: 'line', text: 'uciok' }]);
	});

	it("reports failures after the engine has answered, and the single build's always", () => {
		const threaded = engine(true);
		threaded.workers[0].say('id name Stockfish');
		threaded.workers[0].fail('boom');
		expect(threaded.workers).toHaveLength(1);
		expect(threaded.events.at(-1)).toEqual({ type: 'error', message: 'boom' });

		const single = engine(false);
		single.workers[0].fail('');
		expect(single.workers).toHaveLength(1);
		expect(single.events).toEqual([{ type: 'error', message: 'engine worker failed' }]);
	});
});
