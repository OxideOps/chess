import { describe, expect, it } from 'vitest';
import { Lobby } from './lobby.svelte';
import type { SocketLike } from './client.svelte';
import type { LobbyServerMessage } from '$lib/generated/LobbyServerMessage';
import type { SeekInfo } from '$lib/generated/SeekInfo';
import type { Side } from '$lib/generated/Side';

class FakeSocket implements SocketLike {
	sent: string[] = [];
	closed = false;
	onopen: (() => void) | null = null;
	onmessage: ((event: { data: unknown }) => void) | null = null;
	onclose: (() => void) | null = null;
	onerror: (() => void) | null = null;
	constructor(public url: string) {}
	send(text: string) {
		this.sent.push(text);
	}
	close() {
		this.closed = true;
		this.onclose?.();
	}
	open() {
		this.onopen?.();
	}
	say(msg: LobbyServerMessage) {
		this.onmessage?.({ data: JSON.stringify(msg) });
	}
	take(): unknown[] {
		return this.sent.splice(0).map((s) => JSON.parse(s));
	}
}

function seek(id: string, extra: Partial<SeekInfo> = {}): SeekInfo {
	return {
		id,
		username: 'dana',
		rating: { value: 1500, provisional: false },
		initial_ms: 300_000,
		increment_ms: 0,
		rated: false,
		category: 'blitz',
		...extra
	};
}

function setup() {
	const sockets: FakeSocket[] = [];
	const games: { id: string; yourColor: Side }[] = [];
	const lobby = new Lobby({
		createSocket: (url) => {
			const socket = new FakeSocket(url);
			sockets.push(socket);
			return socket;
		},
		onGame: (game) => games.push(game)
	});
	sockets[0].open();
	return { lobby, sockets, games, socket: () => sockets[sockets.length - 1] };
}

describe('Lobby', () => {
	it('shows what the server sends, and keeps our own seek apart', () => {
		const { lobby, socket } = setup();
		expect(lobby.state).toBe('open');
		expect(lobby.seeks).toEqual([]);

		socket().say({ type: 'seeks', seeks: [seek('a'), seek('b')] });
		expect(lobby.seeks.map((s) => s.id)).toEqual(['a', 'b']);
		// Nothing is ours yet, so both are takeable.
		expect(lobby.others.map((s) => s.id)).toEqual(['a', 'b']);

		lobby.post(300_000, 0, true);
		expect(socket().take()).toEqual([
			{ type: 'post_seek', initial_ms: 300_000, increment_ms: 0, rated: true }
		]);
		expect(lobby.busy).toBe(true);

		socket().say({ type: 'seek_posted', id: 'mine' });
		expect(lobby.mine).toBe('mine');
		expect(lobby.busy).toBe(false);

		socket().say({ type: 'seeks', seeks: [seek('a'), seek('mine'), seek('b')] });
		expect(lobby.others.map((s) => s.id)).toEqual(['a', 'b']);
	});

	it('goes to the game when a seek is taken', () => {
		const { lobby, socket, games } = setup();
		socket().say({ type: 'seeks', seeks: [seek('a')] });

		lobby.accept('a');
		expect(socket().take()).toEqual([{ type: 'accept_seek', id: 'a' }]);
		expect(lobby.busy).toBe(true);

		socket().say({ type: 'game_started', game_id: 'g7', your_color: 'black', opponent: 'dan' });
		// Who took it comes with it: the notification says their name.
		expect(games).toEqual([{ id: 'g7', yourColor: 'black', opponent: 'dan' }]);
		expect(lobby.busy).toBe(false);
		expect(lobby.mine).toBe(null);
	});

	it('reports a refusal and stops waiting', () => {
		const { lobby, socket } = setup();
		lobby.accept('gone');
		socket().say({ type: 'rejected', message: 'that seek is no longer open' });
		expect(lobby.rejection).toBe('that seek is no longer open');
		expect(lobby.busy).toBe(false);

		// The next request clears the old complaint.
		lobby.post(60_000, 0, false);
		expect(lobby.rejection).toBe(null);
	});

	it('forgets our seek when the server stops listing it', () => {
		const { lobby, socket } = setup();
		lobby.post(300_000, 0, false);
		socket().say({ type: 'seek_posted', id: 'mine' });
		socket().say({ type: 'seeks', seeks: [seek('mine')] });
		expect(lobby.mine).toBe('mine');

		// Taken by someone, or dropped: either way it is not ours any more.
		socket().say({ type: 'seeks', seeks: [] });
		expect(lobby.mine).toBe(null);
	});

	it('cancels at once, without waiting to be told', () => {
		const { lobby, socket } = setup();
		lobby.post(300_000, 0, false);
		socket().say({ type: 'seek_posted', id: 'mine' });
		socket().take();

		lobby.cancel();
		expect(lobby.mine).toBe(null);
		expect(socket().take()).toEqual([{ type: 'cancel_seek' }]);
	});

	it('reconnects, and does not pretend the old seek survived', () => {
		const { lobby, sockets, socket } = setup();
		lobby.post(300_000, 0, false);
		socket().say({ type: 'seek_posted', id: 'mine' });
		expect(lobby.mine).toBe('mine');

		// The socket drops: the server has already dropped the seek with it.
		sockets[0].onclose?.();
		expect(lobby.state).toBe('reconnecting');
		expect(lobby.mine).toBe(null);
		expect(lobby.busy).toBe(false);
	});

	it('closes the socket when the page goes, which withdraws the seek', () => {
		const { lobby, sockets } = setup();
		lobby.dispose();
		expect(sockets[0].closed).toBe(true);
		expect(lobby.state).toBe('closed');
		// A close we asked for is not a reason to reconnect.
		expect(sockets).toHaveLength(1);
	});

	it('opens again once there is a session, and sends what was waiting', () => {
		const { lobby, sockets } = setup();
		// What the page does when a signed-out visitor clicks: take a guest
		// session, reopen the socket with it, then ask for the game.
		lobby.reauthenticate();
		lobby.post(300_000, 0, false);
		expect(sockets).toHaveLength(2);
		expect(sockets[0].closed).toBe(true);
		// The old socket closing is not a dropped connection to recover from.
		expect(lobby.state).toBe('connecting');

		sockets[1].open();
		expect(lobby.state).toBe('open');
		expect(sockets[1].take()).toEqual([
			{ type: 'post_seek', initial_ms: 300_000, increment_ms: 0, rated: false }
		]);
	});
});
