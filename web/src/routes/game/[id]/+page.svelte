<script lang="ts">
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import Board from '$lib/components/Board.svelte';
	import Clock from '$lib/components/Clock.svelte';
	import GameSidebar from '$lib/components/GameSidebar.svelte';
	import { OnlineGame } from '$lib/online/client.svelte';
	import { seatName } from '$lib/online/listing';
	import { notifier } from '$lib/notify/notifier.svelte';
	import type { Side } from '$lib/generated/Side';

	// An online game: the board and clocks, with everything else in the sidebar.
	const id = page.params.id!;
	const online = new OnlineGame(id);
	const game = online.game;
	onDestroy(() => online.dispose());

	const orientation: Side = $derived(online.yourColor ?? 'white');
	const opponent: Side = $derived(orientation === 'white' ? 'black' : 'white');
	const name = (side: Side) => {
		const seat = online.players[side];
		return seat === null ? null : seatName(seat);
	};
	/**
	 * Someone taking the open seat is news to whoever has been sitting here
	 * waiting for them — which is the private-game flow: create, send the
	 * link, go and do something else.
	 *
	 * Only after the seat has been seen empty, so a game that was already
	 * full when the page loaded (one the lobby matched) says nothing.
	 */
	let seatWasOpen = false;
	$effect(() => {
		if (!online.synced || online.yourColor === null) return;
		const seat = online.players[opponent];
		if (seat === null) {
			seatWasOpen = true;
			return;
		}
		if (!seatWasOpen) return;
		seatWasOpen = false;
		void notifier.raise(
			{ kind: 'opponent-joined', opponent: seat.username },
			resolve('/game/[id]', { id })
		);
	});

	const clockProps = (side: Side) => {
		const seat = online.players[side];
		return {
			side,
			name: name(side),
			href: seat?.username ? resolve('/players/[username]', { username: seat.username }) : null,
			rating: seat?.rating ?? null,
			diff: online.ratingDiffs?.[side] ?? null,
			active: online.running === side
		};
	};
</script>

<svelte:head>
	<title>Game · Chess</title>
</svelte:head>

<div class="online board-page">
	<div class="board-column">
		<Clock ms={online.clockMs(opponent)} {...clockProps(opponent)} />
		<Board
			{game}
			{orientation}
			playAs={online.yourColor ?? 'both'}
			onmove={(from, to, promotion) => online.tryMove(from, to, promotion)}
		/>
		<Clock
			ms={online.clockMs(orientation)}
			{...clockProps(orientation)}
			yours={orientation === online.yourColor}
		/>
	</div>
	<GameSidebar {id} {online} />
</div>

<style>
	.online {
		/* A clock above and below the board. */
		--board-around: 7.5rem;
	}

	.board-column {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		width: var(--board-size);
	}
</style>
