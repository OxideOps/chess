<script lang="ts">
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import Board from '$lib/components/Board.svelte';
	import Clock from '$lib/components/Clock.svelte';
	import MoveList from '$lib/components/MoveList.svelte';
	import { sideName, statusText } from '$lib/chess/status';
	import { OnlineGame } from '$lib/online/client.svelte';
	import { inviteLink } from '$lib/online/invites';
	import { ensureSession } from '$lib/online/session';
	import type { GameEnd } from '$lib/generated/GameEnd';
	import type { Side } from '$lib/generated/Side';

	const id = page.params.id!;
	const online = new OnlineGame(id);
	const game = online.game;
	onDestroy(() => online.dispose());
	let joinError: string | null = $state(null);

	async function join() {
		joinError = null;
		try {
			await ensureSession();
			await online.join();
		} catch (e) {
			joinError = e instanceof Error ? e.message : String(e);
		}
	}

	const orientation: Side = $derived(online.yourColor ?? 'white');
	const opponent: Side = $derived(orientation === 'white' ? 'black' : 'white');
	// White sees the invite until someone takes the Black seat.
	const invite = $derived(
		online.yourColor === 'white' && online.players.black === null ? inviteLink(id) : null
	);
	const atStart = $derived(game.view.cursor === 0);
	const atEnd = $derived(!game.view.viewingHistory);

	function endText(end: GameEnd): string {
		const who =
			end.result === 'draw' ? 'Draw' : `${end.result === 'white_wins' ? 'White' : 'Black'} wins`;
		const reason: Record<GameEnd['reason'], string> = {
			checkmate: 'by checkmate',
			resignation: 'by resignation',
			timeout: 'on time',
			stalemate: 'by stalemate',
			insufficient_material: 'by insufficient material',
			agreement: 'by agreement',
			repetition: 'by repetition',
			fifty_moves: 'by the fifty-move rule',
			abandoned: 'by abandonment'
		};
		return `${who} ${reason[end.reason]}`;
	}

	const connectionText: Record<typeof online.connection, string> = {
		connecting: 'Connecting…',
		open: '',
		reconnecting: 'Connection lost, reconnecting…',
		closed: 'Disconnected',
		failed: 'Could not connect to the server'
	};
	let copied = $state(false);
	async function copyInvite() {
		if (!invite) return;
		await navigator.clipboard.writeText(invite);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<svelte:head>
	<title>Game · Chess</title>
</svelte:head>

<div class="online">
	<div class="board-column">
		<Clock
			ms={online.clockMs(opponent)}
			active={online.running === opponent}
			label={sideName(opponent)}
		/>
		<Board
			{game}
			{orientation}
			playAs={online.yourColor ?? 'both'}
			onmove={(from, to, promotion) => online.tryMove(from, to, promotion)}
		/>
		<Clock
			ms={online.clockMs(orientation)}
			active={online.running === orientation}
			label={sideName(orientation)}
		/>
	</div>
	<aside class="sidebar">
		{#if connectionText[online.connection]}
			<p class="status connection" role="status">{connectionText[online.connection]}</p>
		{/if}
		<p class="status" data-testid="game-status">
			{#if online.ended}
				{endText(online.ended)}
			{:else if online.yourColor && online.players.black === null}
				Waiting for an opponent to join
			{:else if online.yourColor}
				{online.isMyTurn ? 'Your move' : 'Waiting for your opponent'}
				{#if game.view.status === 'check'}
					— check{/if}
			{:else}
				{statusText(game.view)}
			{/if}
		</p>
		{#if online.rejection}
			<p class="rejection" role="alert">{online.rejection}</p>
		{/if}
		{#if invite}
			<div class="invite">
				<p>Send this link to your opponent. They play Black.</p>
				<input
					readonly
					value={invite}
					data-testid="invite-link"
					onfocus={(e) => e.currentTarget.select()}
				/>
				<button type="button" onclick={copyInvite}>{copied ? 'Copied' : 'Copy link'}</button>
			</div>
		{/if}
		{#if online.canJoin}
			<div class="invite">
				<p>The Black seat is open.</p>
				<button type="button" onclick={join}>Join as Black</button>
				{#if joinError}
					<p class="rejection" role="alert">{joinError}</p>
				{/if}
			</div>
		{/if}
		{#if online.drawOffer && !online.ended}
			<div class="draw" role="status">
				{#if online.drawOffer === online.yourColor}
					<span>Draw offered. Waiting for a reply.</span>
				{:else if online.yourColor}
					<span>{sideName(online.drawOffer)} offers a draw.</span>
					<button type="button" onclick={() => online.acceptDraw()}>Accept</button>
					<button type="button" onclick={() => online.declineDraw()}>Decline</button>
				{:else}
					<span>{sideName(online.drawOffer)} offers a draw.</span>
				{/if}
			</div>
		{/if}
		<MoveList {game} />
		<div class="controls">
			<button type="button" title="First move" disabled={atStart} onclick={() => game.goToStart()}
				>⏮</button
			>
			<button type="button" title="Previous move" disabled={atStart} onclick={() => game.goBack()}
				>◀</button
			>
			<button type="button" title="Next move" disabled={atEnd} onclick={() => game.goForward()}
				>▶</button
			>
			<button type="button" title="Last move" disabled={atEnd} onclick={() => game.goToEnd()}
				>⏭</button
			>
		</div>
		{#if online.yourColor && !online.ended}
			<div class="controls">
				<button
					type="button"
					onclick={() => online.offerDraw()}
					disabled={online.drawOffer !== null}
				>
					Offer draw
				</button>
				<button type="button" class="danger" onclick={() => online.resign()}>Resign</button>
			</div>
		{/if}
	</aside>
</div>

<style>
	.online {
		display: flex;
		flex-wrap: wrap;
		gap: 1.5rem;
		align-items: flex-start;
	}

	.board-column {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		width: min(92vw, 640px);
	}

	.connection {
		color: var(--text-muted);
		font-weight: 400;
	}

	.rejection {
		margin: 0;
		color: #e06c75;
		font-size: 0.85rem;
	}

	.invite {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding: 0.6rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--accent);
		border-radius: 6px;
		font-size: 0.85rem;
	}

	.invite p {
		margin: 0;
	}

	.invite input {
		width: 100%;
		padding: 0.4rem 0.5rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--bg);
		color: var(--text);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.8rem;
	}

	.draw {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
		padding: 0.5rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		font-size: 0.9rem;
	}

	.controls {
		display: flex;
		gap: 0.5rem;
	}

	button {
		flex: 1;
		padding: 0.45rem 0.6rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--panel);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	.draw button {
		flex: 0 0 auto;
	}

	button:hover:not(:disabled) {
		border-color: var(--text-muted);
	}

	button:disabled {
		opacity: 0.4;
		cursor: default;
	}

	button.danger:hover {
		border-color: #e06c75;
		color: #e06c75;
	}
</style>
