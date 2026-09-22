import { tick } from 'svelte';
import { beforeAll, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import EnginePanel from './EnginePanel.svelte';
import { GameStore } from '$lib/chess/game.svelte';
import { initChess } from '$lib/chess/wasm';
import { Analyser } from '$lib/engine/analysis.svelte';
import type { EngineEvent, EngineLike } from '$lib/engine/worker';

const START = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
const AFTER_E4 = 'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1';

/** Plays the engine's side of the conversation for the test. */
class FakeEngine implements EngineLike {
	readonly threads = 1;
	constructor(private emit: (event: EngineEvent) => void) {}
	send() {}
	terminate() {}
	say(text: string) {
		this.emit({ type: 'line', text });
	}
}

beforeAll(() => initChess());

describe('EnginePanel', () => {
	it('keeps the same height whether the engine has reported no lines or all of them', async () => {
		let engine!: FakeEngine;
		const analyser = new Analyser({
			multipv: 3,
			createEngine: (onEvent) => (engine = new FakeEngine(onEvent))
		});
		const game = new GameStore();
		render(EnginePanel, { game, analyser, enabled: true });

		const panel = () => document.querySelector('.engine')!;
		const height = () => Math.round(panel().getBoundingClientRect().height);
		const shown = () => panel().querySelectorAll('.lines li:not(.pending)').length;
		const heights: number[] = [];

		// Still loading: room for three lines already.
		await tick();
		heights.push(height());
		expect(panel().querySelectorAll('.lines li')).toHaveLength(3);

		engine.say('uciok');
		engine.say('readyok');
		analyser.request(START);
		await tick();
		heights.push(height());

		// They arrive one at a time; the panel does not grow as they do.
		for (const [i, line] of [
			'info depth 8 multipv 1 score cp 30 pv e2e4 e7e5',
			'info depth 8 multipv 2 score cp 25 pv d2d4 d7d5',
			'info depth 8 multipv 3 score cp 20 pv g1f3 g8f6'
		].entries()) {
			engine.say(line);
			await tick();
			expect(shown()).toBe(i + 1);
			heights.push(height());
		}

		// A new position throws the old lines away. The panel keeps its size.
		analyser.request(AFTER_E4);
		engine.say('bestmove e2e4');
		await tick();
		expect(shown()).toBe(0);
		heights.push(height());

		expect(new Set(heights).size).toBe(1);
		game.dispose();
	});

	it('only offers to play lines it actually has', async () => {
		let engine!: FakeEngine;
		const analyser = new Analyser({
			multipv: 3,
			createEngine: (onEvent) => (engine = new FakeEngine(onEvent))
		});
		const game = new GameStore();
		render(EnginePanel, { game, analyser, enabled: true });
		engine.say('uciok');
		engine.say('readyok');
		analyser.request(START);
		engine.say('info depth 8 multipv 1 score cp 30 pv e2e4 e7e5');
		await tick();

		// One real line to click; the two rows still waiting are not buttons.
		expect(document.querySelectorAll('.engine .lines button')).toHaveLength(1);
		expect(document.querySelectorAll('.engine .lines li.pending')).toHaveLength(2);
		game.dispose();
	});
});
