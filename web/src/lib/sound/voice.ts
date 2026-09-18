import type { Cue } from './cue';

/** Something that can make the sounds. Swapped for a fake in tests. */
export interface Voice {
	play(cue: Cue): void;
}

/**
 * The sounds, synthesised with Web Audio rather than played from files: a
 * piece landing on a board is a short knock, which is a burst of noise
 * through a narrow filter over a low body tone, and that is a few lines of
 * code instead of a folder of samples to license and download.
 *
 * Everything is built from two voices — `knock` for wood, `tone` for the
 * few sounds that say something (check, promotion, the end of the game) —
 * so they sit together as one set.
 */
export class WebAudioVoice implements Voice {
	#ctx: BaseAudioContext | null = null;
	#out: GainNode | null = null;
	#noise: AudioBuffer | null = null;
	#open: () => BaseAudioContext | null;

	/**
	 * `open` makes the audio context. The tests hand in an
	 * `OfflineAudioContext` and render what comes out, which is the only way
	 * to check a sound is a sound and not silence.
	 */
	constructor(open: () => BaseAudioContext | null = defaultContext) {
		this.#open = open;
	}

	/**
	 * The context is made on the first sound, not on page load: browsers
	 * won't start one until a gesture, and the first sound always follows
	 * one (a move). A context that is still suspended is resumed.
	 */
	#audio(): { ctx: BaseAudioContext; out: GainNode } | null {
		if (!this.#ctx) {
			const ctx = this.#open();
			if (!ctx) return null;
			this.#ctx = ctx;
			this.#out = ctx.createGain();
			this.#out.gain.value = 0.35;
			this.#out.connect(ctx.destination);
		}
		// Only a live context needs waking; an offline one starts when it renders.
		if (isLive(this.#ctx) && this.#ctx.state === 'suspended') {
			void this.#ctx.resume().catch(() => {});
		}
		return { ctx: this.#ctx, out: this.#out! };
	}

	/** A second of white noise, made once and reused for every knock. */
	#noiseBuffer(ctx: BaseAudioContext): AudioBuffer {
		if (!this.#noise) {
			this.#noise = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate);
			const samples = this.#noise.getChannelData(0);
			for (let i = 0; i < samples.length; i++) samples[i] = Math.random() * 2 - 1;
		}
		return this.#noise;
	}

	/** Wood: filtered noise for the contact, a low sine for the board under it. */
	#knock(at: number, { pitch, colour, length, gain }: Knock) {
		const audio = this.#audio();
		if (!audio) return;
		const { ctx, out } = audio;
		const start = ctx.currentTime + at;

		const noise = ctx.createBufferSource();
		noise.buffer = this.#noiseBuffer(ctx);
		const band = ctx.createBiquadFilter();
		band.type = 'bandpass';
		band.frequency.value = colour;
		band.Q.value = 1.2;
		const tap = ctx.createGain();
		tap.gain.setValueAtTime(gain, start);
		tap.gain.exponentialRampToValueAtTime(0.0001, start + length);
		noise.connect(band).connect(tap).connect(out);
		noise.start(start);
		noise.stop(start + length);

		const body = ctx.createOscillator();
		body.type = 'sine';
		body.frequency.setValueAtTime(pitch, start);
		body.frequency.exponentialRampToValueAtTime(pitch * 0.7, start + length);
		const thump = ctx.createGain();
		thump.gain.setValueAtTime(gain * 0.5, start);
		thump.gain.exponentialRampToValueAtTime(0.0001, start + length);
		body.connect(thump).connect(out);
		body.start(start);
		body.stop(start + length);
	}

	/** A plain note, for the sounds that mean something rather than land. */
	#tone(at: number, { from, to, length, gain, shape = 'triangle' }: Tone) {
		const audio = this.#audio();
		if (!audio) return;
		const { ctx, out } = audio;
		const start = ctx.currentTime + at;

		const osc = ctx.createOscillator();
		osc.type = shape;
		osc.frequency.setValueAtTime(from, start);
		if (to !== from) osc.frequency.exponentialRampToValueAtTime(to, start + length);
		const level = ctx.createGain();
		// A short fade in as well as out: a square edge on a tone clicks.
		level.gain.setValueAtTime(0.0001, start);
		level.gain.exponentialRampToValueAtTime(gain, start + 0.012);
		level.gain.exponentialRampToValueAtTime(0.0001, start + length);
		osc.connect(level).connect(out);
		osc.start(start);
		osc.stop(start + length);
	}

	play(cue: Cue): void {
		const move: Knock = { pitch: 200, colour: 1500, length: 0.055, gain: 0.3 };
		const heavy: Knock = { pitch: 140, colour: 850, length: 0.1, gain: 0.42 };
		switch (cue) {
			case 'move':
				this.#knock(0, move);
				break;
			// Taking something is the same knock, lower and harder, with the
			// piece it displaced skittering after it.
			case 'capture':
				this.#knock(0, heavy);
				this.#knock(0.035, { pitch: 260, colour: 2200, length: 0.05, gain: 0.16 });
				break;
			// Two pieces land, king then rook.
			case 'castle':
				this.#knock(0, move);
				this.#knock(0.085, move);
				break;
			case 'check':
				this.#knock(0, move);
				this.#tone(0.04, { from: 1174, to: 1174, length: 0.09, gain: 0.14 });
				this.#tone(0.13, { from: 1568, to: 1568, length: 0.11, gain: 0.14 });
				break;
			// A pawn becoming a queen: the same piece, rising.
			case 'promote':
				this.#knock(0, move);
				this.#tone(0.03, { from: 523, to: 1046, length: 0.22, gain: 0.15 });
				break;
			case 'end':
				this.#knock(0, heavy);
				this.#tone(0.06, { from: 784, to: 784, length: 0.16, gain: 0.14 });
				this.#tone(0.2, { from: 523, to: 523, length: 0.34, gain: 0.14 });
				break;
			// Not the board: the clock. Thin and square, so it cuts through.
			case 'low-time':
				this.#tone(0, { from: 1320, to: 1320, length: 0.08, gain: 0.12, shape: 'square' });
				this.#tone(0.12, { from: 1320, to: 1320, length: 0.08, gain: 0.12, shape: 'square' });
				break;
		}
	}
}

interface Knock {
	/** The board under the piece (Hz). */
	pitch: number;
	/** Where the contact sits (Hz): higher is a lighter, sharper tap. */
	colour: number;
	length: number;
	gain: number;
}

interface Tone {
	from: number;
	to: number;
	length: number;
	gain: number;
	shape?: OscillatorType;
}

function isLive(ctx: BaseAudioContext): ctx is AudioContext {
	return typeof AudioContext !== 'undefined' && ctx instanceof AudioContext;
}

function defaultContext(): BaseAudioContext | null {
	const Ctor =
		window.AudioContext ??
		(window as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
	return Ctor ? new Ctor() : null;
}
