import { Game } from '$lib/chess/wasm';
import { GameStore, type Promotion } from '$lib/chess/game.svelte';
import type { PlayResult } from '$lib/chess/wasm';
import type { ClientMessage } from '$lib/generated/ClientMessage';
import type { Clocks } from '$lib/generated/Clocks';
import type { GameEnd } from '$lib/generated/GameEnd';
import type { Players } from '$lib/generated/Players';
import type { ServerMessage } from '$lib/generated/ServerMessage';
import type { Side } from '$lib/generated/Side';

export type ConnectionState = 'connecting' | 'open' | 'reconnecting' | 'closed' | 'failed';

/** What the client needs from a WebSocket; injectable for tests. */
export interface SocketLike {
	send(text: string): void;
	close(): void;
	onopen: (() => void) | null;
	onmessage: ((event: { data: unknown }) => void) | null;
	onclose: (() => void) | null;
	onerror: (() => void) | null;
}

export interface OnlineGameOptions {
	createSocket?: (url: string) => SocketLike;
	/** Clock for the local countdown; defaults to `performance.now`. */
	now?: () => number;
}

/** Remaining time as last reported by the server, and when we heard it. */
interface ServerClocks {
	whiteMs: number;
	blackMs: number;
	at: number;
}

/**
 * A game played on the server. Mirrors the server's `Room` on the client:
 * the `GameStore` holds the moves so far, and everything else here is what
 * the last `Sync`/broadcast said. Moves are applied locally first (the same
 * rules run on both sides) and sent; a `Rejected` reply triggers a resync.
 */
export class OnlineGame {
	readonly game = new GameStore();
	connection: ConnectionState = $state('connecting');
	yourColor: Side | null = $state(null);
	ended: GameEnd | null = $state(null);
	drawOffer: Side | null = $state(null);
	players: Players = $state({ white: null, black: null });
	/** The most recent rejection from the server, for a status line. */
	rejection: string | null = $state(null);
	#clocks: ServerClocks = $state({ whiteMs: 0, blackMs: 0, at: 0 });
	#tick = $state(0);

	readonly #id: string;
	readonly #url: string;
	readonly #createSocket: (url: string) => SocketLike;
	readonly #now: () => number;
	#socket: SocketLike | null = null;
	#closedByUs = false;
	#retryMs = 1000;
	#ticker: ReturnType<typeof setInterval> | null = null;

	constructor(id: string, { createSocket, now }: OnlineGameOptions = {}) {
		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		// The session cookie identifies us; seat holders play, others watch.
		this.#id = id;
		this.#url = `${proto}://${location.host}/api/games/${encodeURIComponent(id)}/ws`;
		this.#createSocket = createSocket ?? ((url) => new WebSocket(url) as unknown as SocketLike);
		this.#now = now ?? (() => performance.now());
		this.#connect();
	}

	/** The Black seat is open and we don't hold White. */
	get canJoin(): boolean {
		return this.players.black === null && this.yourColor === null && !this.ended;
	}

	/** Take the open Black seat, then reconnect so the server seats us. */
	async join(): Promise<void> {
		const response = await fetch(`/api/games/${encodeURIComponent(this.#id)}/join`, {
			method: 'POST'
		});
		if (!response.ok) throw new Error(await response.text());
		this.#resync();
	}

	/** Whose clock is running, if any. */
	get running(): Side | null {
		const view = this.game.view;
		if (this.ended || view.plyCount < 2) return null;
		return view.turn;
	}

	/** Remaining milliseconds for `side`, counting down locally. */
	clockMs(side: Side): number {
		void this.#tick; // re-evaluate while the ticker runs
		const c = this.#clocks;
		const base = side === 'white' ? c.whiteMs : c.blackMs;
		const elapsed = this.running === side ? this.#now() - c.at : 0;
		return Math.max(0, base - elapsed);
	}

	get isMyTurn(): boolean {
		return this.yourColor !== null && !this.ended && this.game.view.turn === this.yourColor;
	}

	/** Board callback: play locally if it's our move, then tell the server. */
	tryMove(from: string, to: string, promotion?: Promotion): PlayResult {
		if (!this.isMyTurn || this.game.view.viewingHistory) return 'illegal';
		const result = this.game.play(from, to, promotion);
		if (result === 'ok') {
			const uci = this.game.view.moves.at(-1)!.uci;
			this.#send({ type: 'move', uci });
		}
		return result;
	}

	resign(): void {
		this.#send({ type: 'resign' });
	}

	offerDraw(): void {
		this.#send({ type: 'offer_draw' });
	}

	acceptDraw(): void {
		this.#send({ type: 'accept_draw' });
	}

	declineDraw(): void {
		this.#send({ type: 'decline_draw' });
	}

	dispose(): void {
		this.#closedByUs = true;
		this.#stopTicker();
		this.#socket?.close();
		this.#socket = null;
		this.game.dispose();
	}

	#connect(): void {
		const socket = this.#createSocket(this.#url);
		this.#socket = socket;
		socket.onopen = () => {
			this.connection = 'open';
			this.#retryMs = 1000;
		};
		socket.onmessage = (event) => {
			if (typeof event.data !== 'string') return;
			this.#handle(JSON.parse(event.data) as ServerMessage);
		};
		socket.onerror = () => {
			if (this.connection === 'connecting') this.connection = 'failed';
		};
		socket.onclose = () => {
			if (this.#closedByUs || this.ended) {
				this.connection = 'closed';
				return;
			}
			this.connection = 'reconnecting';
			setTimeout(() => this.#connect(), this.#retryMs);
			this.#retryMs = Math.min(this.#retryMs * 2, 10_000);
		};
	}

	#send(msg: ClientMessage): void {
		this.#socket?.send(JSON.stringify(msg));
	}

	#handle(msg: ServerMessage): void {
		switch (msg.type) {
			case 'sync': {
				const game = Game.fromFen(msg.start_fen);
				for (const uci of msg.moves) game.playUci(uci);
				this.game.replace(game);
				this.yourColor = msg.your_color;
				this.ended = msg.ended;
				this.drawOffer = msg.draw_offer;
				this.players = msg.players;
				this.#setClocks(msg.clocks);
				break;
			}
			case 'players_changed':
				this.players = msg.players;
				break;
			case 'move_played': {
				const ply = this.game.view.plyCount;
				if (msg.ply === ply + 1) {
					this.game.goToEnd();
					this.game.playUci(msg.uci);
				} else if (msg.ply > ply + 1) {
					// We missed something; the server will send a fresh Sync.
					this.#resync();
					return;
				}
				// msg.ply <= ply: our own move echoed back (already applied).
				this.drawOffer = null;
				this.#setClocks(msg.clocks);
				break;
			}
			case 'draw_offered':
				this.drawOffer = msg.by;
				break;
			case 'draw_declined':
				this.drawOffer = null;
				break;
			case 'game_over':
				// Freeze the clocks where they are (before `ended` stops the countdown).
				this.#clocks = {
					whiteMs: this.clockMs('white'),
					blackMs: this.clockMs('black'),
					at: this.#now()
				};
				this.ended = { result: msg.result, reason: msg.reason };
				this.drawOffer = null;
				this.#stopTicker();
				break;
			case 'rejected':
				this.rejection = msg.message;
				// Our local state may have diverged (e.g. a move after the flag).
				this.#resync();
				break;
			case 'pong':
				break;
		}
	}

	#resync(): void {
		this.#socket?.close(); // onclose reconnects, and the server Syncs on connect
	}

	#setClocks(clocks: Clocks): void {
		this.#clocks = { whiteMs: clocks.white_ms, blackMs: clocks.black_ms, at: this.#now() };
		if (this.running) this.#startTicker();
		else this.#stopTicker();
	}

	#startTicker(): void {
		if (this.#ticker !== null) return;
		this.#ticker = setInterval(() => {
			this.#tick += 1;
		}, 100);
	}

	#stopTicker(): void {
		if (this.#ticker !== null) clearInterval(this.#ticker);
		this.#ticker = null;
	}
}
