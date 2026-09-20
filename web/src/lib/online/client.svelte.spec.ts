import { beforeAll, describe, expect, it } from 'vitest';
import { initChess } from '$lib/chess/wasm';
import { AWAY_NOTICE_DELAY_MS, OnlineGame, type SocketLike } from './client.svelte';
import type { ServerMessage } from '$lib/generated/ServerMessage';

const START = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

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
	say(msg: ServerMessage) {
		this.onmessage?.({ data: JSON.stringify(msg) });
	}
	take(): unknown[] {
		return this.sent.splice(0).map((s) => JSON.parse(s));
	}
}

function setup() {
	const sockets: FakeSocket[] = [];
	let now = 1000;
	const online = new OnlineGame('g1', {
		createSocket: (url) => {
			const s = new FakeSocket(url);
			sockets.push(s);
			return s;
		},
		now: () => now
	});
	return { online, sockets, socket: () => sockets.at(-1)!, tick: (ms: number) => (now += ms) };
}

const clocks = (white_ms: number, black_ms: number) => ({ white_ms, black_ms });

beforeAll(() => initChess());

describe('OnlineGame', () => {
	it('connects and builds the game from Sync', () => {
		const { online, socket } = setup();
		expect(socket().url).toMatch(/\/api\/games\/g1\/ws$/);
		expect(online.connection).toBe('connecting');
		socket().open();
		expect(online.connection).toBe('open');

		socket().say({
			type: 'sync',
			away: null,
			rated: false,
			category: 'blitz',
			rating_diffs: null,
			start_fen: START,
			moves: ['e2e4', 'e7e5'],
			clocks: clocks(60_000, 58_000),
			your_color: 'white',
			ended: null,
			draw_offer: 'black',
			players: { white: { username: 'dan', rating: null }, black: null }
		});
		expect(online.players.white?.username).toBe('dan');
		expect(online.canJoin).toBe(false); // we hold White
		expect(online.yourColor).toBe('white');
		expect(online.game.view.movetext).toBe('1. e4 e5');
		expect(online.drawOffer).toBe('black');
		expect(online.isMyTurn).toBe(true);
		expect(online.running).toBe('white');
		online.dispose();
	});

	it('plays our move locally, sends it, and ignores the echo', () => {
		const { online, socket } = setup();
		socket().open();
		socket().say({
			type: 'sync',
			away: null,
			rated: false,
			category: 'blitz',
			rating_diffs: null,
			start_fen: START,
			moves: [],
			clocks: clocks(60_000, 60_000),
			your_color: 'white',
			ended: null,
			draw_offer: null,
			players: { white: { username: null, rating: null }, black: { username: null, rating: null } }
		});
		expect(online.tryMove('e7', 'e5')).toBe('illegal'); // not our piece
		expect(online.tryMove('e2', 'e4')).toBe('ok');
		expect(socket().take()).toEqual([{ type: 'move', uci: 'e2e4' }]);
		expect(online.isMyTurn).toBe(false);
		expect(online.tryMove('d2', 'd4')).toBe('illegal'); // not our turn

		socket().say({ type: 'move_played', ply: 1, uci: 'e2e4', clocks: clocks(60_000, 60_000) });
		expect(online.game.view.plyCount).toBe(1);
		socket().say({ type: 'move_played', ply: 2, uci: 'e7e5', clocks: clocks(60_000, 60_000) });
		expect(online.game.view.movetext).toBe('1. e4 e5');
		expect(online.isMyTurn).toBe(true);
		online.dispose();
	});

	it('counts the running clock down locally and freezes it at the end', () => {
		const { online, socket, tick } = setup();
		socket().open();
		socket().say({
			type: 'sync',
			away: null,
			rated: false,
			category: 'blitz',
			rating_diffs: null,
			start_fen: START,
			moves: ['e2e4', 'e7e5'],
			clocks: clocks(30_000, 20_000),
			your_color: null,
			ended: null,
			draw_offer: null,
			players: { white: { username: null, rating: null }, black: { username: null, rating: null } }
		});
		tick(5_000);
		expect(online.clockMs('white')).toBe(25_000);
		expect(online.clockMs('black')).toBe(20_000);

		socket().say({ type: 'game_over', result: 'black_wins', reason: 'resignation' });
		expect(online.ended).toEqual({ result: 'black_wins', reason: 'resignation' });
		tick(5_000);
		expect(online.clockMs('white')).toBe(25_000);
		expect(online.running).toBeNull();
		online.dispose();
	});

	it('resyncs after a rejection or a gap, and reconnects when dropped', () => {
		const { online, sockets, socket } = setup();
		socket().open();
		socket().say({
			type: 'sync',
			away: null,
			rated: false,
			category: 'blitz',
			rating_diffs: null,
			start_fen: START,
			moves: [],
			clocks: clocks(60_000, 60_000),
			your_color: 'black',
			ended: null,
			draw_offer: null,
			players: { white: { username: null, rating: null }, black: { username: null, rating: null } }
		});
		// A move two plies ahead means we missed one: drop and let the server resync.
		socket().say({ type: 'move_played', ply: 2, uci: 'e7e5', clocks: clocks(60_000, 60_000) });
		// The replacement is opened at once, rather than waiting for the old
		// socket's close to come back: through a proxy that takes seconds.
		expect(sockets[0].closed).toBe(true);
		expect(sockets).toHaveLength(2);
		expect(online.connection).toBe('reconnecting');
		expect(online.game.view.plyCount).toBe(0);

		socket().open();
		expect(online.connection).toBe('open');
		socket().say({ type: 'rejected', message: 'it is not your turn' });
		expect(online.rejection).toBe('it is not your turn');
		online.dispose();
		expect(online.connection).toBe('closed');
	});

	it('counts an away player down, after a moment, and forgets them when the game ends', () => {
		const { online, socket, tick } = setup();
		socket().open();
		// Nothing is known before the first Sync: no "Join as Black" for the creator.
		expect(online.canJoin).toBe(false);
		socket().say({
			type: 'sync',
			away: { side: 'black', ms: 60_000 },
			rated: false,
			category: 'blitz',
			rating_diffs: null,
			start_fen: START,
			moves: ['e2e4', 'e7e5'],
			clocks: clocks(60_000, 60_000),
			your_color: 'white',
			ended: null,
			draw_offer: null,
			players: { white: { username: null, rating: null }, black: { username: null, rating: null } }
		});
		// A countdown this fresh could be a reload: not mentioned yet.
		expect(online.away).toBeNull();
		tick(AWAY_NOTICE_DELAY_MS);
		expect(online.away).toEqual({ side: 'black', ms: 60_000 - AWAY_NOTICE_DELAY_MS });
		tick(10_000);
		expect(online.away?.ms).toBe(48_000);

		socket().say({ type: 'away_changed', away: null });
		expect(online.away).toBeNull();
		socket().say({ type: 'away_changed', away: { side: 'black', ms: 60_000 } });
		tick(AWAY_NOTICE_DELAY_MS);
		expect(online.away?.side).toBe('black');

		socket().say({ type: 'game_over', result: 'white_wins', reason: 'abandoned' });
		expect(online.away).toBeNull();
		expect(online.ended).toEqual({ result: 'white_wins', reason: 'abandoned' });
		online.dispose();
	});

	it('knows whether the game is rated and what it did to the ratings', () => {
		const { online, socket } = setup();
		socket().open();
		socket().say({
			type: 'sync',
			away: null,
			rated: true,
			category: 'rapid',
			rating_diffs: null,
			start_fen: START,
			moves: [],
			clocks: clocks(600_000, 600_000),
			your_color: 'white',
			ended: null,
			draw_offer: null,
			players: {
				white: { username: 'alice', rating: { value: 1500, provisional: true } },
				black: { username: 'bob', rating: { value: 1623, provisional: false } }
			}
		});
		expect(online.rated).toBe(true);
		expect(online.category).toBe('rapid');
		expect(online.players.black?.rating).toEqual({ value: 1623, provisional: false });
		expect(online.ratingDiffs).toBeNull();
		socket().say({ type: 'game_over', result: 'white_wins', reason: 'resignation' });
		socket().say({ type: 'ratings_changed', white: 170, black: -35 });
		expect(online.ratingDiffs).toEqual({ white: 170, black: -35 });
		online.dispose();
	});

	it('holds a move made while the socket is reopening, and sends it when it is', () => {
		const { online, sockets, socket } = setup();
		socket().open();
		socket().say({
			type: 'sync',
			away: null,
			rated: false,
			category: 'blitz',
			rating_diffs: null,
			start_fen: START,
			moves: [],
			clocks: clocks(60_000, 60_000),
			your_color: 'white',
			ended: null,
			draw_offer: null,
			players: { white: { username: null, rating: null }, black: { username: null, rating: null } }
		});
		socket().take();

		// A gap drops the socket and opens another; the player moves before it
		// is up. Sending on a connecting socket throws, so it waits.
		socket().say({ type: 'move_played', ply: 2, uci: 'e7e5', clocks: clocks(60_000, 60_000) });
		expect(online.connection).toBe('reconnecting');
		expect(online.tryMove('e2', 'e4')).toBe('ok');
		expect(socket().take()).toEqual([]);

		socket().open();
		expect(socket().take()).toEqual([{ type: 'move', uci: 'e2e4' }]);
		expect(sockets).toHaveLength(2);
	});
});
