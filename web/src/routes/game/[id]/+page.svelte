<script lang="ts">
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import Board from '$lib/components/Board.svelte';
	import Clock from '$lib/components/Clock.svelte';
	import GameSidebar from '$lib/components/GameSidebar.svelte';
	import { OnlineGame } from '$lib/online/client.svelte';
	import { seatName } from '$lib/online/listing';
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
</script>

<svelte:head>
	<title>Game · Chess</title>
</svelte:head>

<div class="online board-page">
	<div class="board-column">
		<Clock
			ms={online.clockMs(opponent)}
			side={opponent}
			name={name(opponent)}
			active={online.running === opponent}
		/>
		<Board
			{game}
			{orientation}
			playAs={online.yourColor ?? 'both'}
			onmove={(from, to, promotion) => online.tryMove(from, to, promotion)}
		/>
		<Clock
			ms={online.clockMs(orientation)}
			side={orientation}
			name={name(orientation)}
			active={online.running === orientation}
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
