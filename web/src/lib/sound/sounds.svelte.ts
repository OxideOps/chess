import type { GameStore } from '$lib/chess/game.svelte';
import { cueFor, type Cue } from './cue';
import { WebAudioVoice, type Voice } from './voice';

const STORED = 'chess:sound';

/**
 * Whether the board makes a noise, and the making of it. One of these for
 * the whole app (`sounds` below); `attach` points it at a game, which every
 * board does, so a move sounds the same wherever it was played — by you, by
 * the engine, or by an opponent on the other side of a socket.
 */
export class Sounds {
	/** Off mutes everything; remembered in this browser. */
	on = $state(true);
	#voice: Voice | null;
	#make: () => Voice;

	constructor(options: { voice?: Voice; make?: () => Voice } = {}) {
		this.#voice = options.voice ?? null;
		this.#make = options.make ?? (() => new WebAudioVoice());
		this.on = stored() ?? true;
	}

	toggle(): void {
		this.on = !this.on;
		try {
			localStorage.setItem(STORED, this.on ? 'on' : 'off');
		} catch {
			// A browser with storage blocked still gets sound, just not the memory of it.
		}
		// Say what was switched on, so the button proves itself.
		if (this.on) this.play('move');
	}

	play(cue: Cue): void {
		if (!this.on) return;
		this.#voice ??= this.#make();
		this.#voice.play(cue);
	}

	/**
	 * Sound every move played on `game` from now on. Boards call this, so a
	 * page gets it by having a board, and moves that arrive any other way
	 * (loading a PGN, syncing a game from the server) stay silent.
	 */
	attach(game: GameStore): void {
		game.onmove = (san, over) => this.play(cueFor(san, over));
	}
}

function stored(): boolean | null {
	try {
		const saved = localStorage.getItem(STORED);
		return saved === null ? null : saved === 'on';
	} catch {
		return null;
	}
}

export const sounds = new Sounds();
