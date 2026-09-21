import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { Sounds } from './sounds.svelte';
import { WebAudioVoice } from './voice';
import type { Cue } from './cue';
import type { Voice } from './voice';
import { GameStore } from '$lib/chess/game.svelte';
import { initChess } from '$lib/chess/wasm';

beforeAll(() => initChess());

/** A voice that writes down what it was asked for instead of making a noise. */
class Heard implements Voice {
	cues: Cue[] = [];
	play(cue: Cue) {
		this.cues.push(cue);
	}
}

function sounded() {
	const voice = new Heard();
	return { voice, sounds: new Sounds({ voice }) };
}

describe('Sounds', () => {
	beforeEach(() => localStorage.removeItem('chess:sound'));

	it('sounds every move played on a game it is attached to', () => {
		const { voice, sounds } = sounded();
		const game = new GameStore();
		sounds.attach(game);

		game.play('e2', 'e4');
		game.play('d7', 'd5');
		game.play('e4', 'd5');
		expect(voice.cues).toEqual(['move', 'move', 'capture']);
		game.dispose();
	});

	it('says when a move gave check, castled, promoted or ended the game', () => {
		const { voice, sounds } = sounded();
		const game = GameStore.fromFen(
			'rnbqk2r/pppp1ppp/5n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1'
		);
		sounds.attach(game);
		game.play('e1', 'g1'); // O-O
		game.play('e8', 'g8'); // O-O
		game.play('c4', 'f7'); // Bxf7+
		expect(voice.cues).toEqual(['castle', 'castle', 'check']);
		game.dispose();

		// A promotion out of the black king's sight, or it would be a check.
		const promoting = GameStore.fromFen('8/4P3/8/2k5/8/8/8/K7 w - - 0 1');
		const heard = new Heard();
		new Sounds({ voice: heard }).attach(promoting);
		promoting.playUci('e7e8r');
		expect(heard.cues).toEqual(['promote']);
		promoting.dispose();

		const mating = GameStore.fromFen('6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1');
		const end = new Heard();
		new Sounds({ voice: end }).attach(mating);
		mating.playUci('a1a8');
		expect(end.cues).toEqual(['end']);
		mating.dispose();
	});

	it('stays quiet for moves that were only looked at, not made', () => {
		const { voice, sounds } = sounded();
		const game = GameStore.fromPgn('1. e4 e5 2. Nf3');
		sounds.attach(game);

		// Loading a game and walking through it are not moves being played.
		expect(voice.cues).toEqual([]);
		game.goToStart();
		game.goForward();
		game.goToEnd();
		game.goBack();
		expect(voice.cues).toEqual([]);

		// An illegal move makes no sound either.
		game.goToEnd();
		expect(game.play('a1', 'a8')).not.toBe('ok');
		expect(voice.cues).toEqual([]);
		game.dispose();
	});

	it('is muted by the toggle, and remembers that next time', () => {
		const { voice, sounds } = sounded();
		const game = new GameStore();
		sounds.attach(game);

		sounds.toggle();
		expect(sounds.on).toBe(false);
		game.play('e2', 'e4');
		expect(voice.cues).toEqual([]);

		// A fresh app in the same browser starts muted.
		expect(new Sounds({ voice: new Heard() }).on).toBe(false);

		// Turning it back on plays one move, so the button proves itself.
		sounds.toggle();
		expect(sounds.on).toBe(true);
		expect(voice.cues).toEqual(['move']);
		expect(new Sounds({ voice: new Heard() }).on).toBe(true);
		game.dispose();
	});

	it('makes one voice on demand, and none at all while muted', () => {
		let made = 0;
		const sounds = new Sounds({
			make: () => {
				made++;
				return new Heard();
			}
		});
		sounds.on = false;
		sounds.play('move');
		expect(made).toBe(0);

		sounds.on = true;
		sounds.play('move');
		sounds.play('capture');
		expect(made).toBe(1);
	});
});

describe('WebAudioVoice', () => {
	// Rendering offline is the only way to tell a sound from silence.
	async function render(cue: Cue): Promise<number> {
		const ctx = new OfflineAudioContext({ numberOfChannels: 1, length: 44100, sampleRate: 44100 });
		new WebAudioVoice(() => ctx).play(cue);
		const buffer = await ctx.startRendering();
		const samples = buffer.getChannelData(0);
		let peak = 0;
		for (const s of samples) peak = Math.max(peak, Math.abs(s));
		return peak;
	}

	const cues: Cue[] = ['move', 'capture', 'castle', 'check', 'promote', 'end', 'low-time', 'ready'];
	for (const cue of cues) {
		it(`${cue} is audible`, async () => {
			const peak = await render(cue);
			expect(peak).toBeGreaterThan(0.01);
			// And not so loud it clips.
			expect(peak).toBeLessThanOrEqual(1);
		});
	}

	it('makes no sound when the browser has no audio at all', () => {
		expect(() => new WebAudioVoice(() => null).play('move')).not.toThrow();
	});
});
