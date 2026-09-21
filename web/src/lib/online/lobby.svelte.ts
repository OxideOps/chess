import type { LobbyClientMessage } from '$lib/generated/LobbyClientMessage';
import type { LobbyServerMessage } from '$lib/generated/LobbyServerMessage';
import type { SeekInfo } from '$lib/generated/SeekInfo';
import type { Side } from '$lib/generated/Side';
import type { SocketLike } from './client.svelte';

export interface LobbyOptions {
	createSocket?: (url: string) => SocketLike;
	/** Called when a seek of ours is taken: go and play. */
	onGame?: (game: { id: string; yourColor: Side; opponent: string | null }) => void;
}

export type LobbyState = 'connecting' | 'open' | 'reconnecting' | 'closed';

/**
 * The seek list, mirrored from the server.
 *
 * The server sends the whole list whenever it changes, so there is nothing
 * to merge here. Our own seek is remembered by id (`mine`), which is how the
 * list can offer "cancel" on one row and "play" on the others.
 *
 * The socket *is* the seek: closing it withdraws what we posted, so this
 * store holds it open while the lobby is on screen and drops it on
 * `dispose`. Nothing is kept across a reload.
 */
export class Lobby {
	seeks: SeekInfo[] = $state([]);
	/** The id of our own open seek, if we have one. */
	mine: string | null = $state(null);
	state: LobbyState = $state('connecting');
	/** The server's last refusal, e.g. a seek that someone else took first. */
	rejection: string | null = $state(null);
	/** True from asking for a game until the server answers. */
	busy = $state(false);

	readonly #url: string;
	readonly #createSocket: (url: string) => SocketLike;
	readonly #onGame: (game: { id: string; yourColor: Side; opponent: string | null }) => void;
	#socket: SocketLike | null = null;
	#closedByUs = false;
	#retryMs = 1000;
	#retry: ReturnType<typeof setTimeout> | null = null;
	/** Anything asked for before the socket was ready, sent once it is. */
	#queued: LobbyClientMessage[] = [];

	constructor({ createSocket, onGame }: LobbyOptions = {}) {
		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		this.#url = `${proto}://${location.host}/api/lobby/ws`;
		this.#createSocket = createSocket ?? ((url) => new WebSocket(url) as unknown as SocketLike);
		this.#onGame = onGame ?? (() => {});
		this.#connect();
	}

	/** Everyone else's seeks: the ones there is any point clicking. */
	get others(): SeekInfo[] {
		return this.seeks.filter((seek) => seek.id !== this.mine);
	}

	post(initialMs: number, incrementMs: number, rated: boolean): void {
		this.rejection = null;
		this.busy = true;
		this.#send({ type: 'post_seek', initial_ms: initialMs, increment_ms: incrementMs, rated });
	}

	cancel(): void {
		this.mine = null;
		this.#send({ type: 'cancel_seek' });
	}

	accept(id: string): void {
		this.rejection = null;
		this.busy = true;
		this.#send({ type: 'accept_seek', id });
	}

	/**
	 * Open the socket again with whatever session the browser has now.
	 *
	 * The server reads the session cookie when the socket connects, so a
	 * visitor who was signed out when the page loaded — and became a guest
	 * only when they clicked something — is still nobody on the old
	 * connection. Anything sent meanwhile is queued and goes out on the new
	 * one.
	 */
	reauthenticate(): void {
		const old = this.#socket;
		this.#socket = null;
		this.state = 'connecting';
		if (old) {
			// Our own close: it must not schedule a reconnect of its own.
			old.onclose = null;
			old.close();
		}
		this.#connect();
	}

	dispose(): void {
		this.#closedByUs = true;
		if (this.#retry !== null) clearTimeout(this.#retry);
		this.#socket?.close();
		this.#socket = null;
		this.state = 'closed';
	}

	#send(message: LobbyClientMessage): void {
		if (this.#socket === null || this.state !== 'open') {
			this.#queued.push(message);
			return;
		}
		this.#socket.send(JSON.stringify(message));
	}

	#connect(): void {
		const socket = this.#createSocket(this.#url);
		this.#socket = socket;
		socket.onopen = () => {
			this.state = 'open';
			this.#retryMs = 1000;
			const queued = this.#queued.splice(0);
			for (const message of queued) socket.send(JSON.stringify(message));
		};
		socket.onmessage = (event) => {
			if (typeof event.data !== 'string') return;
			this.#handle(JSON.parse(event.data) as LobbyServerMessage);
		};
		socket.onerror = () => {};
		socket.onclose = () => {
			if (this.#closedByUs) return;
			// Our seek died with the socket, so don't claim we still have one.
			this.mine = null;
			this.busy = false;
			this.state = 'reconnecting';
			this.#retry = setTimeout(() => this.#connect(), this.#retryMs);
			this.#retryMs = Math.min(this.#retryMs * 2, 10_000);
		};
	}

	#handle(message: LobbyServerMessage): void {
		switch (message.type) {
			case 'seeks':
				this.seeks = message.seeks;
				// A seek of ours that is no longer listed was taken or dropped.
				if (this.mine !== null && !message.seeks.some((s) => s.id === this.mine)) {
					this.mine = null;
				}
				break;
			case 'seek_posted':
				this.mine = message.id;
				this.busy = false;
				break;
			case 'game_started':
				this.mine = null;
				this.busy = false;
				this.#onGame({
					id: message.game_id,
					yourColor: message.your_color,
					opponent: message.opponent
				});
				break;
			case 'rejected':
				this.rejection = message.message;
				this.busy = false;
				break;
			case 'pong':
				break;
		}
	}
}
